import { pages } from "@plumb/web/meta";
import Home from "@plumb/web/views/index.svelte";
import { render } from "svelte/server";
import { expect, test } from "vitest";

test("home", () => {
	const markup = render(Home).body;
	expect(markup).toContain("plumb");
	expect(markup).toContain('class="frame');
});

test("meta", () => {
	expect(pages.map((page) => page.path)).toEqual(["/"]);
	for (const page of pages) {
		expect(page.title.length).toBeGreaterThan(0);
		expect(page.text.length).toBeGreaterThan(0);
	}
});
