import { renderToStaticMarkup } from "react-dom/server";
import { expect, test } from "vitest";
import { Code } from "../src/lib";

test("code", () => {
	const markup = renderToStaticMarkup(<Code>clean</Code>);
	expect(markup).toContain('class="code"');
	expect(markup).toContain("<code>clean</code>");
});
