import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";

import { rejectAfter, resolveAfter } from "$lib/dusk/promise";

import { OperationResult } from "..";

vi.useFakeTimers();

describe("OperationResult", () => {
  const delay = 1000;

  const onBeforeLeave = vi.fn();

  const baseProps = {
    onBeforeLeave,
    operation: resolveAfter(delay, ""),
  };

  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(() => {
    cleanup();
    onBeforeLeave.mockClear();
  });

  it("should be able to render the `OperationResult` component in a pending state", () => {
    const { container } = render(OperationResult, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should accept a custom message for the pending state", () => {
    const props = {
      ...baseProps,
      pendingMessage: "Transaction pending",
    };
    const { container } = render(OperationResult, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should be able to render the `OperationResult` in a successful state", async () => {
    const { container } = render(OperationResult, baseOptions);

    await vi.advanceTimersByTimeAsync(delay);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should accept a custom message for the successful state", async () => {
    const props = {
      ...baseProps,
      successMessage: "Transaction created",
    };

    const { container } = render(OperationResult, { ...baseOptions, props });

    await vi.advanceTimersByTimeAsync(delay);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should call the `onBeforeLeave` function when the home button is clicked", async () => {
    const { getByRole } = render(OperationResult, baseOptions);

    await vi.advanceTimersByTimeAsync(delay);

    const homeBtn = getByRole("link");

    // prevents the browser from attempting to navigate
    // to the link's href, which jsdom cannot handle
    homeBtn.addEventListener("click", (event) => event.preventDefault());

    await fireEvent.click(homeBtn);

    expect(baseProps.onBeforeLeave).toHaveBeenCalledTimes(1);
  });

  it("should be able to render the `OperationResult` in a failure state", async () => {
    const props = {
      ...baseProps,
      operation: rejectAfter(delay, new Error("some error")),
    };

    const { container } = render(OperationResult, { ...baseOptions, props });

    await vi.advanceTimersByTimeAsync(delay);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should accept a custom message for the failure state", async () => {
    const props = {
      ...baseProps,
      errorMessage: "Transaction failed",
      operation: rejectAfter(delay, new Error("some error")),
    };

    const { container } = render(OperationResult, { ...baseOptions, props });

    await vi.advanceTimersByTimeAsync(delay);

    expect(container.firstChild).toMatchSnapshot();
  });
});
