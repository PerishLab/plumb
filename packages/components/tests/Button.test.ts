import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { expect, test } from "vitest";
import { Button } from "../src/lib";

test("button", () => {
	const markup = renderToStaticMarkup(createElement(Button, null, "press"));
	expect(markup).toContain("press");
});
