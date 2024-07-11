import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";

import { apiMarketData, gqlTransaction } from "$lib/mock-data";
import { transformTransaction } from "$lib/chain-info";

import { TransactionDetails } from "..";

describe("Transaction Details", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 20));

  const baseProps = {
    data: transformTransaction(gqlTransaction.tx),
    error: null,
    loading: false,
    market: {
      currentPrice: apiMarketData.market_data.current_price,
      marketCap: apiMarketData.market_data.market_cap,
    },
    payload:
      "db0794770322802a22905c4364511f3186e6184085f875dbb9f11a3ae914766c020000000000000014bc23b875c67d0dbecfdd45f5964f3fea7188aff2035730c8802",
  };

  afterEach(cleanup);

  afterAll(() => {
    vi.useRealTimers();
  });

  it("renders the Transaction Details component", () => {
    const { container } = render(TransactionDetails, baseProps);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("renders the Transaction Details component with the payload visible", async () => {
    const { container, getByRole } = render(TransactionDetails, baseProps);

    await fireEvent.click(getByRole("switch"));

    expect(container.firstChild).toMatchSnapshot();
  });
});
