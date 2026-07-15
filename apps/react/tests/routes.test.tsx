import { renderToStaticMarkup } from "react-dom/server";
import { createMemoryRouter, RouterProvider } from "react-router";
import { expect, test } from "vitest";
import vocabulary from "../src/data/vocabulary.json";
import { routes } from "../src/lib/routes";
import { sift } from "../src/lib/sift";
import { Jump } from "../src/views/Vocabulary";

test("home", () => {
	const markup = render("/");
	expect(markup).toContain("<h1>open-web</h1>");
	expect(markup).toContain("negentropy");
});

test("negentropy", () => {
	expect(render("/negentropy")).toContain("structural checker");
});

test("runseal", () => {
	expect(render("/runseal")).toContain("explicit profile");
});

test("sidecar", () => {
	expect(render("/sidecar")).toContain("--sidecar-stamp");
});

test("constitution", () => {
	const markup = render("/constitution");
	expect(markup).toContain("<h3>dispatch</h3>");
	expect(markup).toContain("stable is a promise");
});

test("vocabulary", () => {
	const markup = render("/vocabulary");
	expect(markup).toContain("living dictionary");
	expect(markup).toContain('href="#negentropy"');
	expect(markup).toContain('href="#runseal"');
	expect(markup).toContain('href="#sidecar"');
	expect(markup).toContain('href="#open-web"');
	expect(markup).toContain('id="negentropy"');
	expect(markup).toContain('id="open-web"');
});

test("vocabulary jumps track filter", () => {
	const entries = sift(vocabulary, "__test");
	expect(entries.map((entry) => entry.repo)).toEqual(["sidecar"]);
	const markup = renderToStaticMarkup(<Jump entries={entries} />);
	expect(markup).toContain('href="#sidecar"');
	expect(markup).not.toContain('href="#negentropy"');
	expect(markup).not.toContain('href="#runseal"');
	expect(markup).not.toContain('href="#open-web"');
});

function render(path: string): string {
	const router = createMemoryRouter(routes, { initialEntries: [path] });
	return renderToStaticMarkup(<RouterProvider router={router} />);
}
