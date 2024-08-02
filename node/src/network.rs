// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::net::{AddrParseError, SocketAddr};
use std::sync::Arc;

use crate::{BoxedFilter, Message};
use async_trait::async_trait;
use kadcast::config::Config;
use kadcast::{MessageInfo, Peer};
use metrics::counter;
use node_data::message::payload::{GetResource, Inv};
use node_data::message::Metadata;
use node_data::message::{AsyncQueue, Topics};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::{self, Instant};
use tracing::{error, info, trace, warn};

mod frame;

const MAX_PENDING_SENDERS: u64 = 1000;

/// Number of alive peers randomly selected which a `flood_request` is sent to
const REDUNDANCY_PEER_COUNT: usize = 8;

type RoutesList<const N: usize> = [Option<AsyncQueue<Message>>; N];
type FilterList<const N: usize> = [Option<BoxedFilter>; N];

pub struct Listener<const N: usize> {
    routes: Arc<RwLock<RoutesList<N>>>,
    filters: Arc<RwLock<FilterList<N>>>,

    /// Number of awaiting senders.
    pending_senders: Arc<AtomicU64>,
}

impl<const N: usize> Listener<N> {
    fn reroute(&self, topic: u8, msg: Message) -> anyhow::Result<()> {
        if self.pending_senders.fetch_add(1, Ordering::Relaxed)
            >= MAX_PENDING_SENDERS
        {
            // High value of this field means either a message consumer is
            // blocked or it's too slow on processing a wire msg
            self.pending_senders.store(0, Ordering::Relaxed);
            warn!("too many sender jobs: {}", MAX_PENDING_SENDERS);
        }

        let counter = self.pending_senders.clone();
        let routes = self.routes.clone();

        // Sender task
        tokio::spawn(async move {
            if let Some(Some(queue)) = routes.read().await.get(topic as usize) {
                queue.try_send(msg);
            };

            counter.fetch_sub(1, Ordering::Relaxed);
        });

        Ok(())
    }

    fn call_filters(
        &self,
        topic: impl Into<u8>,
        msg: &Message,
    ) -> anyhow::Result<()> {
        let topic = topic.into() as usize;

        match self.filters.try_write()?.get_mut(topic) {
            Some(Some(f)) => f.filter(msg),
            _ => Ok(()),
        }
    }
}

impl<const N: usize> kadcast::NetworkListen for Listener<N> {
    fn on_message(&self, blob: Vec<u8>, md: MessageInfo) {
        let msg_size = blob.len();
        match frame::Pdu::decode(&mut &blob.to_vec()[..]) {
            Ok(d) => {
                let mut msg = d.payload;

                counter!("dusk_bytes_recv").increment(msg_size as u64);
                counter!(format!("dusk_inbound_{:?}_size", msg.topic()))
                    .increment(msg_size as u64);
                counter!(format!("dusk_inbound_{:?}_count", msg.topic()))
                    .increment(1);

                // Update Transport Data
                msg.metadata = Some(Metadata {
                    height: md.height(),
                    src_addr: md.src(),
                });

                // Allow upper layers to fast-discard a message before queueing
                if let Err(e) = self.call_filters(msg.topic(), &msg) {
                    info!("discard message due to {e}");
                    return;
                }

                // Reroute message to the upper layer
                if let Err(e) = self.reroute(msg.topic().into(), msg) {
                    error!("could not reroute due to {e}");
                }
            }
            Err(err) => {
                // Dump message blob and topic number
                let topic = blob.get(node_data::message::TOPIC_FIELD_POS);
                error!("err: {err}, msg_topic: {topic:?}",);
            }
        };
    }
}

pub struct Kadcast<const N: usize> {
    peer: Peer,
    routes: Arc<RwLock<RoutesList<N>>>,
    filters: Arc<RwLock<FilterList<N>>>,
    conf: Config,

    counter: AtomicU64,

    /// Represents a parsed conf.public_addr
    public_addr: SocketAddr,
}

impl<const N: usize> Kadcast<N> {
    pub fn new(conf: Config) -> Result<Self, AddrParseError> {
        const INIT: Option<AsyncQueue<Message>> = None;
        let routes = Arc::new(RwLock::new([INIT; N]));

        const INIT_FN: Option<BoxedFilter> = None;
        let filters = Arc::new(RwLock::new([INIT_FN; N]));

        info!(
            "Loading network with public_address {} and private_address {:?}",
            &conf.public_address, &conf.listen_address
        );
        let listener = Listener {
            routes: routes.clone(),
            filters: filters.clone(),
            pending_senders: Arc::new(AtomicU64::new(0)),
        };
        let peer = Peer::new(conf.clone(), listener)?;
        let public_addr = conf
            .public_address
            .parse::<SocketAddr>()
            .expect("valid kadcast public address");

        Ok(Kadcast {
            routes,
            filters,
            peer,
            conf,
            counter: AtomicU64::new(0),
            public_addr,
        })
    }

    pub fn route_internal(&self, msg: Message) {
        let topic = msg.topic() as usize;
        let routes = self.routes.clone();

        tokio::spawn(async move {
            if let Some(Some(queue)) = routes.read().await.get(topic) {
                queue.try_send(msg.clone());
            };
        });
    }

    /// Removes a route, if exists, for a given topic.
    async fn remove_route(&mut self, topic: u8) {
        let mut guard = self.routes.write().await;

        if let Some(Some(_)) = guard.get_mut(topic as usize) {
            guard[topic as usize] = None;
        }
    }

    pub async fn alive_nodes(&self, amount: usize) -> Vec<SocketAddr> {
        self.peer.alive_nodes(amount).await
    }

    pub fn conf(&self) -> &Config {
        &self.conf
    }

    async fn send_with_metrics(&self, bytes: &Vec<u8>, recv_addr: SocketAddr) {
        counter!("dusk_bytes_sent").increment(bytes.len() as u64);
        self.peer.send(bytes, recv_addr).await;
    }
}

#[async_trait]
impl<const N: usize> crate::Network for Kadcast<N> {
    async fn broadcast(&self, msg: &Message) -> anyhow::Result<()> {
        let height = match msg.metadata {
            Some(Metadata { height: 0, .. }) => return Ok(()),
            Some(Metadata { height, .. }) => Some(height - 1),
            None => None,
        };

        let encoded = frame::Pdu::encode(msg, 0).map_err(|err| {
            error!("could not encode message {msg:?}: {err}");
            anyhow::anyhow!("failed to broadcast: {err}")
        })?;

        counter!("dusk_bytes_cast").increment(encoded.len() as u64);
        counter!(format!("dusk_outbound_{:?}_size", msg.topic()))
            .increment(encoded.len() as u64);

        trace!("broadcasting msg ({:?})", msg.topic());
        self.peer.broadcast(&encoded, height).await;

        Ok(())
    }

    /// Broadcast a GetResource request.
    ///
    /// By utilizing the randomly selected peers per bucket in Kadcast, this
    /// broadcast does follow the so-called "Flood with Random Walk" blind
    /// search (resource discovery).
    ///
    /// A receiver of this message is supposed to look up the resource and
    /// either return it or, if not found, rebroadcast the message to the next
    /// Kadcast bucket
    ///
    /// * `ttl_as_sec` - Defines the lifespan of the request in seconds
    ///
    /// * `hops_limit` - Defines maximum number of hops to receive the request
    async fn flood_request(
        &self,
        msg_inv: &Inv,
        ttl_as_sec: Option<u64>,
        hops_limit: u16,
    ) -> anyhow::Result<()> {
        let ttl_as_sec = ttl_as_sec.map_or_else(
            || u64::MAX,
            |v| {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
                    + v
            },
        );

        self.send_to_alive_peers(
            &Message::new_get_resource(GetResource::new(
                msg_inv.clone(),
                self.public_addr,
                ttl_as_sec,
                hops_limit,
            )),
            REDUNDANCY_PEER_COUNT,
        )
        .await
    }

    /// Sends an encoded message to a given peer.
    async fn send_to_peer(
        &self,
        msg: &Message,
        recv_addr: SocketAddr,
    ) -> anyhow::Result<()> {
        // rnd_count is added to bypass kadcast dupemap
        let rnd_count = self.counter.fetch_add(1, Ordering::SeqCst);
        let encoded = frame::Pdu::encode(msg, rnd_count)
            .map_err(|err| anyhow::anyhow!("failed to send_to_peer: {err}"))?;
        let topic = msg.topic();

        info!("sending msg ({topic:?}) to peer {recv_addr}");
        self.send_with_metrics(&encoded, recv_addr).await;

        Ok(())
    }

    /// Sends to random set of alive peers.
    async fn send_to_alive_peers(
        &self,
        msg: &Message,
        amount: usize,
    ) -> anyhow::Result<()> {
        let encoded = frame::Pdu::encode(msg, 0)
            .map_err(|err| anyhow::anyhow!("failed to encode: {err}"))?;
        let topic = msg.topic();

        counter!(format!("dusk_requests_{:?}", topic)).increment(1);

        for recv_addr in self.peer.alive_nodes(amount).await {
            trace!("sending msg ({topic:?}) to peer {recv_addr}");
            self.send_with_metrics(&encoded, recv_addr).await;
        }

        Ok(())
    }

    /// Route any message of the specified type to this queue.
    async fn add_route(
        &mut self,
        topic: u8,
        queue: AsyncQueue<Message>,
    ) -> anyhow::Result<()> {
        let mut guard = self.routes.write().await;

        let route = guard
            .get_mut(topic as usize)
            .ok_or_else(|| anyhow::anyhow!("topic out of range: {topic}"))?;

        debug_assert!(route.is_none(), "topic already registered");

        *route = Some(queue);

        Ok(())
    }

    async fn send_and_wait(
        &mut self,
        request_msg: &Message,
        response_msg_topic: Topics,
        timeout_millis: u64,
        recv_peers_count: usize,
    ) -> anyhow::Result<Message> {
        self.remove_route(response_msg_topic.into()).await;

        let res = {
            let queue = AsyncQueue::bounded(2, "temp_queue");
            // register a temporary route that will be unregister on drop
            self.add_route(response_msg_topic.into(), queue.clone())
                .await?;

            self.send_to_alive_peers(request_msg, recv_peers_count)
                .await?;

            let deadline =
                Instant::now() + Duration::from_millis(timeout_millis);

            // Wait for a response message or a timeout
            match time::timeout_at(deadline, queue.recv()).await {
                // Got a response message
                Ok(Ok(msg)) => Ok(msg),
                // Failed to receive a response message
                Ok(Err(_)) => anyhow::bail!("failed to receive"),
                // Timeout expired
                Err(_) => anyhow::bail!("timeout err"),
            }
        };

        self.remove_route(response_msg_topic.into()).await;
        res
    }

    async fn add_filter(
        &mut self,
        msg_type: u8,
        filter_fn: BoxedFilter,
    ) -> anyhow::Result<()> {
        let mut guard = self.filters.write().await;

        let filter = guard
            .get_mut(msg_type as usize)
            .expect("should be valid type");

        *filter = Some(filter_fn);

        Ok(())
    }

    // TODO: Duplicated func
    fn get_info(&self) -> anyhow::Result<String> {
        Ok(self.conf.public_address.to_string())
    }

    fn public_addr(&self) -> &SocketAddr {
        &self.public_addr
    }

    async fn alive_nodes_count(&self) -> usize {
        // TODO: This call should be replaced with no-copy Kadcast API
        self.peer.alive_nodes(u16::MAX as usize).await.len()
    }
}
