import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render } from "@testing-library/svelte";
import { mdiHome } from "@mdi/js";
import { getAsHTMLElement } from "$lib/dusk/test-helpers";
import { Tabs } from "..";

vi.useFakeTimers();

describe("Tabs", () => {
  const rafSpy = vi.spyOn(window, "requestAnimationFrame");
  const cafSpy = vi.spyOn(window, "cancelAnimationFrame");
  const scrollBySpy = vi.spyOn(HTMLUListElement.prototype, "scrollBy");
  const scrollIntoViewSpy = vi.spyOn(HTMLLIElement.prototype, "scrollIntoView");
  const scrollLeftSpy = vi
    .spyOn(HTMLUListElement.prototype, "scrollLeft", "get")
    .mockReturnValue(0);
  const scrollToSpy = vi.spyOn(HTMLUListElement.prototype, "scrollTo");
  const scrollWidthSpy = vi
    .spyOn(HTMLUListElement.prototype, "scrollWidth", "get")
    .mockReturnValue(640);
  const ulClientWidthSpy = vi
    .spyOn(HTMLUListElement.prototype, "clientWidth", "get")
    .mockReturnValue(320);

  const items = [
    "Dashboard",
    "User Settings",
    "User Profile",
    "Notifications",
    "Direct Messaging",
    "Task Manager",
    "Event Calendar",
    "Analytics",
    "Team Management",
    "Help",
  ].map((v) => ({ id: v.toLowerCase().replace(/ /g, "-"), label: v }));

  /** @type {TabItem[]} */
  const itemsWithTextAndIcon = items.map((item, idx) => ({
    ...item,
    icon: { path: mdiHome, position: idx % 2 === 0 ? "before" : "after" },
  }));

  /** @type {TabItem[]} */
  const itemsWithIcon = itemsWithTextAndIcon.map(({ id, icon }) => ({
    icon,
    id,
  }));
  const itemsWithIdOnly = items.map(({ id }) => ({ id }));

  const baseProps = {
    items,
    selectedTab: "user-settings",
  };

  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(() => {
    cleanup();
    rafSpy.mockClear();
    cafSpy.mockClear();
    scrollBySpy.mockClear();
    scrollIntoViewSpy.mockClear();
    scrollLeftSpy.mockClear();
    scrollToSpy.mockClear();
    scrollWidthSpy.mockClear();
    ulClientWidthSpy.mockClear();
  });

  afterAll(() => {
    rafSpy.mockRestore();
    cafSpy.mockRestore();
    scrollBySpy.mockRestore();
    scrollIntoViewSpy.mockRestore();
    scrollLeftSpy.mockRestore();
    scrollToSpy.mockRestore();
    scrollWidthSpy.mockRestore();
    ulClientWidthSpy.mockRestore();
    vi.useRealTimers();
  });

  it('should render a "Tabs" component and reset its scroll status if no tab is selected', () => {
    const props = {
      ...baseProps,
      selectedTab: undefined,
    };
    const { container } = render(Tabs, { ...baseOptions, props });
    const tabsList = getAsHTMLElement(container, ".dusk-tabs-list");

    expect(tabsList.scrollTo).toHaveBeenCalledTimes(1);
    expect(tabsList.scrollTo).toHaveBeenCalledWith(0, 0);
    expect(container.firstChild).toMatchSnapshot();
  });

  it("should scroll the selected tab into view if there's a selection", async () => {
    const { container } = render(Tabs, baseOptions);
    const tab = getAsHTMLElement(
      container,
      `[data-tabid="${baseProps.selectedTab}"]`
    );

    expect(tab.scrollIntoView).toHaveBeenCalledTimes(1);
  });

  it("should be able to render tabs with icon and text", () => {
    const props = {
      ...baseProps,
      items: itemsWithTextAndIcon,
    };
    const { container } = render(Tabs, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should be able to render tabs with icons only", () => {
    const props = {
      ...baseProps,
      items: itemsWithIcon,
    };
    const { container } = render(Tabs, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should use the id as label if the tab hasn't one and is without icon", () => {
    const props = {
      ...baseProps,
      items: itemsWithIdOnly,
    };
    const { container } = render(Tabs, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should observe the tab list resize on mounting and stop observing when unmounting", () => {
    const observeSpy = vi.spyOn(ResizeObserver.prototype, "observe");
    const disconnectSpy = vi.spyOn(ResizeObserver.prototype, "disconnect");
    const { container, unmount } = render(Tabs, baseOptions);
    const tabsList = container.querySelector(".dusk-tabs-list");

    expect(observeSpy).toHaveBeenCalledTimes(1);
    expect(observeSpy).toHaveBeenCalledWith(tabsList);

    unmount();

    expect(disconnectSpy).toHaveBeenCalledTimes(1);

    observeSpy.mockRestore();
    disconnectSpy.mockRestore();
  });

  it("should pass additional class names and attributes to the root element", () => {
    const props = {
      ...baseProps,
      className: "foo bar",
      id: "some-id",
    };
    const { container } = render(Tabs, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should fire a change event when a tab is selected and it's not the current selection", async () => {
    const { component, getAllByRole } = render(Tabs, baseOptions);
    const tabs = getAllByRole("tab");

    let expectedTab = tabs[0];

    expect.assertions(3);

    component.$on("change", (event) => {
      expect(event.detail).toBe(expectedTab.dataset.tabid);
    });

    // does nothing as it's currently selected
    await fireEvent.click(tabs[1]);

    await fireEvent.click(expectedTab);

    expectedTab = tabs[1];

    await fireEvent.keyDown(expectedTab, { key: "Enter" });

    expectedTab = tabs[2];

    await fireEvent.keyDown(expectedTab, { key: " " });

    // does nothing as neither space or Enter are pressed
    await fireEvent.keyDown(tabs[1], { key: "f" });
  });

  it("should scroll a tab into view when it gains focus", async () => {
    const { getAllByRole } = render(Tabs, baseOptions);
    const tabs = getAllByRole("tab");

    scrollIntoViewSpy.mockClear();

    await fireEvent.focusIn(tabs[0]);

    expect(tabs[0].scrollIntoView).toHaveBeenCalledTimes(1);
  });

  it("should hide and disable the scroll buttons if there is enough horizontal space", () => {
    scrollWidthSpy.mockReturnValueOnce(0);

    const { container } = render(Tabs, baseOptions);
    const leftBtn = getAsHTMLElement(
      container,
      ".dusk-tab-scroll-button:first-of-type"
    );
    const rightBtn = getAsHTMLElement(
      container,
      ".dusk-tab-scroll-button:last-of-type"
    );

    expect(leftBtn.getAttribute("hidden")).toBe("true");
    expect(leftBtn.getAttribute("disabled")).toBe("");
    expect(rightBtn.getAttribute("hidden")).toBe("true");
    expect(rightBtn.getAttribute("disabled")).toBe("");
  });

  it("should show the scroll buttons when there isn't enough horizontal space and enable the appropriate ones", async () => {
    const originalObserver = ResizeObserver;

    let callback;

    /**
     * We don't have a proper mock for the observer right now,
     * so we use the proxy to memorize the callback received by the
     * observer's constructor.
     * This way we can call it at will, simulating updates.
     */
    global.ResizeObserver = new Proxy(originalObserver, {
      construct(Target, args) {
        callback = args[0];

        return new Target(args[0]);
      },
    });

    const { container } = render(Tabs, baseOptions);
    const tabsList = getAsHTMLElement(container, ".dusk-tabs-list");

    let leftBtn = getAsHTMLElement(
      container,
      ".dusk-tab-scroll-button:first-of-type"
    );
    let rightBtn = getAsHTMLElement(
      container,
      ".dusk-tab-scroll-button:last-of-type"
    );

    expect(leftBtn.getAttribute("hidden")).toBe("false");
    expect(leftBtn.getAttribute("disabled")).toBe("");
    expect(rightBtn.getAttribute("hidden")).toBe("false");
    expect(rightBtn.getAttribute("disabled")).toBeNull();

    await fireEvent.mouseDown(rightBtn, { buttons: 1 });

    expect(rafSpy).toHaveBeenCalledTimes(1);
    expect(tabsList.scrollBy).toHaveBeenCalledTimes(1);
    expect(tabsList.scrollBy).toHaveBeenCalledWith(5, 0);

    scrollBySpy.mockClear();
    rafSpy.mockClear();

    vi.advanceTimersToNextTimer();

    expect(rafSpy).toHaveBeenCalledTimes(1);
    expect(tabsList.scrollBy).toHaveBeenCalledTimes(1);
    expect(tabsList.scrollBy).toHaveBeenCalledWith(5, 0);

    await fireEvent.mouseUp(rightBtn);

    expect(cafSpy).toHaveBeenCalledTimes(1);

    scrollLeftSpy.mockReturnValue(320);

    // we don't care for callback parameters right now
    await act(callback);

    leftBtn = getAsHTMLElement(
      container,
      ".dusk-tab-scroll-button:first-of-type"
    );
    rightBtn = getAsHTMLElement(
      container,
      ".dusk-tab-scroll-button:last-of-type"
    );

    expect(leftBtn.getAttribute("hidden")).toBe("false");
    expect(leftBtn.getAttribute("disabled")).toBeNull();
    expect(rightBtn.getAttribute("hidden")).toBe("false");
    expect(rightBtn.getAttribute("disabled")).toBe("");

    scrollBySpy.mockClear();
    rafSpy.mockClear();

    await fireEvent.mouseDown(leftBtn, { buttons: 1 });

    expect(rafSpy).toHaveBeenCalledTimes(1);
    expect(tabsList.scrollBy).toHaveBeenCalledTimes(1);
    expect(tabsList.scrollBy).toHaveBeenCalledWith(-5, 0);

    global.ResizeObserver = originalObserver;
  });

  it("should ignore mouse down events if the primary button isn't the only one pressed", async () => {
    const { container } = render(Tabs, baseOptions);
    const tabsList = getAsHTMLElement(container, ".dusk-tabs-list");
    const leftBtn = getAsHTMLElement(
      container,
      ".dusk-tab-scroll-button:first-of-type"
    );
    const rightBtn = getAsHTMLElement(
      container,
      ".dusk-tab-scroll-button:last-of-type"
    );

    await fireEvent.mouseDown(leftBtn, { buttons: 2 });

    await fireEvent.mouseDown(leftBtn, { buttons: 3 });

    await fireEvent.mouseDown(rightBtn, { buttons: 2 });

    await fireEvent.mouseDown(rightBtn, { buttons: 3 });

    expect(rafSpy).not.toHaveBeenCalled();
    expect(tabsList.scrollBy).not.toHaveBeenCalled();
  });

  it("should bring the nearest tab into view on mouse clicks on scroll buttons", async () => {
    const { container } = render(Tabs, baseOptions);
    const tabsList = getAsHTMLElement(container, ".dusk-tabs-list");
    const leftBtn = getAsHTMLElement(
      container,
      ".dusk-tab-scroll-button:first-of-type"
    );
    const rightBtn = getAsHTMLElement(
      container,
      ".dusk-tab-scroll-button:last-of-type"
    );
    const firstTab = getAsHTMLElement(container, "[role='tab']:first-of-type");
    const lastTab = getAsHTMLElement(container, "[role='tab']:last-of-type");

    const tabsListGetRectSpy = vi
      .spyOn(tabsList, "getBoundingClientRect")
      .mockReturnValue(DOMRect.fromRect({ width: tabsList.clientWidth, x: 0 }));
    const firstTabGetRectSpy = vi
      .spyOn(firstTab, "getBoundingClientRect")
      .mockReturnValue(DOMRect.fromRect({ width: 100, x: -100 }));
    const lastTabGetRectSpy = vi
      .spyOn(lastTab, "getBoundingClientRect")
      .mockReturnValue(
        DOMRect.fromRect({ width: 100, x: tabsList.clientWidth })
      );

    scrollIntoViewSpy.mockClear();

    await fireEvent.click(rightBtn);

    expect(lastTab.scrollIntoView).toHaveBeenCalledTimes(1);

    scrollIntoViewSpy.mockClear();

    await fireEvent.click(leftBtn);

    expect(firstTab.scrollIntoView).toHaveBeenCalledTimes(1);

    tabsListGetRectSpy.mockRestore();
    firstTabGetRectSpy.mockRestore();
    lastTabGetRectSpy.mockRestore();
  });
});
