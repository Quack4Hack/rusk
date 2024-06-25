import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";

import { addresses } from "$lib/mock-data";

import { AddressPicker } from "..";

global.ResizeObserver = vi.fn().mockImplementation(() => ({
  disconnect: vi.fn(),
  observe: vi.fn(),
  unobserve: vi.fn(),
}));

describe("AddressPicker", () => {
  const currentAddress = addresses[0];

  const props = { addresses, currentAddress };

  beforeEach(() => {
    Object.assign(navigator, {
      clipboard: {
        writeText: vi.fn().mockResolvedValue(undefined),
      },
    });
  });

  afterEach(cleanup);

  it("renders the AddressPicker component", () => {
    const { container } = render(AddressPicker, props);

    expect(container.firstElementChild).toMatchSnapshot();
  });

  it("copies the current address on Copy button click", async () => {
    const { getByRole } = render(AddressPicker, props);

    const component = getByRole("button", { name: "Copy Address" });

    await fireEvent.click(component);

    expect(navigator.clipboard.writeText).toHaveBeenCalledWith(currentAddress);
  });
});
