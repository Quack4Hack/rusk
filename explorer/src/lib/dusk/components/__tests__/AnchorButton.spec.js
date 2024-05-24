import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { mdiFolderOutline } from "@mdi/js";

import { AnchorButton } from "..";

describe("AnchorButton", () => {
  const baseProps = {
    href: "/some-url",
    text: "some text",
  };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };
  const iconPositions = /** @type {const} */ (["after", "before", undefined]);

  afterEach(cleanup);

  it("should render the AnchorButton component", () => {
    const { container } = render(AnchorButton, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should add a disabled class if the related property is `true`", () => {
    const props = {
      ...baseProps,
      disabled: true,
    };
    const { container } = render(AnchorButton, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should pass additional class names and attributes to the rendered element", () => {
    const props = {
      ...baseProps,
      className: "foo bar",
      id: "some-id",
    };
    const { container } = render(AnchorButton, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should render a AnchorButton without a text", () => {
    const props = {
      ...baseProps,
      text: "",
    };
    const { container } = render(AnchorButton, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should be able to render a AnchorButton with an icon and text", () => {
    iconPositions.forEach((position) => {
      const props = {
        ...baseProps,
        icon: {
          path: mdiFolderOutline,
          position,
        },
      };
      const { container } = render(AnchorButton, { ...baseOptions, props });

      expect(container.firstChild).toMatchSnapshot();
    });
  });

  it("should be able to render a AnchorButton with an icon only", () => {
    iconPositions.forEach((position) => {
      const props = {
        ...baseProps,
        icon: {
          path: mdiFolderOutline,
          position,
        },
        text: "",
      };
      const { container } = render(AnchorButton, { ...baseOptions, props });

      expect(container.firstChild).toMatchSnapshot();
    });
  });
});
