import { cleanup, render } from "@testing-library/svelte";
import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import { base } from "$app/paths";

import { resolveAfter } from "$lib/dusk/promise";
import { settingsStore } from "$lib/stores";
import { transactions } from "$lib/mock-data";
import { sortByHeightDesc } from "$lib/transactions";
import {
  createFeeFormatter,
  createTransferFormatter,
} from "$lib/dusk/currency";

import Transactions from "../Transactions/Transactions.svelte";

global.ResizeObserver = vi.fn().mockImplementation(() => ({
  disconnect: vi.fn(),
  observe: vi.fn(),
  unobserve: vi.fn(),
}));

vi.useFakeTimers();

describe("Transactions", () => {
  const transactionsPromise = resolveAfter(1000, transactions);
  const emptyTransactionsPromise = resolveAfter(1000, []);
  const blockExplorerBaseUrl = "/explorer/transactions/transaction?id=";
  const highestTransactionID = sortByHeightDesc(transactions)[0].id;
  const settings = get(settingsStore);
  const language = settings.language;
  const transferFormatter = createTransferFormatter(language);
  const feeFormatter = createFeeFormatter(language);

  const baseProps = {
    isSyncing: false,
    items: transactionsPromise,
    language,
    syncError: null,
  };

  afterEach(() => {
    cleanup();
  });

  afterAll(() => {
    vi.useRealTimers();
  });

  it("renders a loading indicator after a successful sync if the transaction promise isn't resolved yet", async () => {
    const props = {
      ...baseProps,
      isSyncing: true,
    };
    const { getByRole, getByText, rerender } = render(Transactions, {
      props: props,
    });
    const notice = getByText("Data will load after a successful sync.");

    expect(notice).toBeInTheDocument();

    await rerender({ ...baseProps });

    const spinner = getByRole("progressbar");

    expect(spinner).toBeInTheDocument();
  });

  it("renders transactions correctly when items are fulfilled", async () => {
    const props = {
      ...baseProps,
      limit: 1,
    };

    const { getByText, container } = render(Transactions, props);

    await vi.advanceTimersToNextTimerAsync();

    const transaction = sortByHeightDesc(transactions)[0];

    const transactionAmount = getByText(transferFormatter(transaction.amount));
    const transactionBlockHeight = getByText(
      new Intl.NumberFormat(language).format(transaction.block_height)
    );
    const transactionType = getByText(transaction.tx_type.toUpperCase());
    const transactionFee = getByText(feeFormatter(transaction.fee));

    expect(container.firstChild).toMatchSnapshot();

    expect(transactionAmount).toBeInTheDocument();
    expect(transactionBlockHeight).toBeInTheDocument();
    expect(transactionType).toBeInTheDocument();
    expect(transactionFee).toBeInTheDocument();
  });

  it("renders the correct amount of Transactions, as the limit supplied", async () => {
    const props = {
      ...baseProps,
      limit: 3,
    };

    const { getAllByText, container } = render(Transactions, props);

    await vi.advanceTimersToNextTimerAsync();

    expect(container.firstChild).toMatchSnapshot();

    const transactionHashes = getAllByText("Hash");

    expect(transactionHashes).toHaveLength(3);
  });

  it("renders the Transactions in descending order", async () => {
    const props = {
      ...baseProps,
    };
    const { container } = render(Transactions, props);

    await vi.advanceTimersToNextTimerAsync();

    const sortedTransactions = sortByHeightDesc(transactions);

    const transactionElements =
      container.querySelectorAll(".transactions-list");

    sortedTransactions.forEach((transaction, index) => {
      expect(transactionElements[index]).toHaveTextContent(transaction.id);
    });
  });

  it("renders the Transactions with the correct CTA to the Block Explorer", async () => {
    const props = {
      ...baseProps,
      limit: 1,
    };

    const { getByRole } = render(Transactions, props);

    await vi.advanceTimersToNextTimerAsync();

    const transactionHrefBlockExplorerAnchor = getByRole("link", {
      name: highestTransactionID,
    });

    expect(transactionHrefBlockExplorerAnchor).toBeInTheDocument();
    expect(transactionHrefBlockExplorerAnchor).toHaveAttribute(
      "href",
      `${blockExplorerBaseUrl}${highestTransactionID}`
    );
  });

  it("displays empty state when no transactions are present", async () => {
    const props = {
      ...baseProps,
      items: emptyTransactionsPromise,
    };

    const { getByText } = render(Transactions, props);

    await vi.advanceTimersToNextTimerAsync();

    const emptyState = getByText("You have no transaction history");

    expect(emptyState).toBeInTheDocument();
  });

  it('displays the "All transactions" CTA if limit is supplied', async () => {
    const props = {
      ...baseProps,
      items: emptyTransactionsPromise,
      limit: 1,
    };

    const { getByRole } = render(Transactions, props);
    const allTransactionAnchor = getByRole("link", {
      name: "All transactions",
    });

    expect(allTransactionAnchor).toBeInTheDocument();
    expect(allTransactionAnchor).toHaveAttribute(
      "href",
      `${base}/dashboard/transactions`
    );
  });

  it('displays the "Back" CTA if no limit is supplied', async () => {
    const props = {
      ...baseProps,
      items: emptyTransactionsPromise,
    };

    const { getByRole } = render(Transactions, props);
    const backAnchor = getByRole("link", { name: "Back" });

    expect(backAnchor).toBeInTheDocument();
    expect(backAnchor).toHaveAttribute("href", `${base}/dashboard`);
  });

  it("handles error state when items are rejected", async () => {
    const promiseReject = Promise.reject(new Error("An error has occurred"));
    const props = {
      ...baseProps,
      items: promiseReject,
    };

    const { getByText } = render(Transactions, props);

    await vi.advanceTimersToNextTimerAsync();

    const errorState = getByText("Error getting transactions");

    expect(errorState).toBeInTheDocument();
  });
});
