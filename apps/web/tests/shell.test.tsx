import { renderToStaticMarkup } from "react-dom/server";
import { expect, test } from "vitest";
import { pages } from "../src/lib/meta";
import Home from "../src/views/index";

test("home", () => {
	const markup = renderToStaticMarkup(<Home />);
	expect(markup).toContain("plumb");
	expect(markup).toContain('class="frame"');
});

test("meta", () => {
	expect(pages.map((page) => page.path)).toEqual(["/"]);
	for (const page of pages) {
		expect(page.title.length).toBeGreaterThan(0);
		expect(page.text.length).toBeGreaterThan(0);
	}
});
