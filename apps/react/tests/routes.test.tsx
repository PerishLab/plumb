import { renderToStaticMarkup } from "react-dom/server";
import { createMemoryRouter, RouterProvider } from "react-router";
import { expect, test } from "vitest";
import { routes } from "../src/lib/routes";

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
	expect(render("/vocabulary")).toContain("living dictionary");
});

function render(path: string): string {
	const router = createMemoryRouter(routes, { initialEntries: [path] });
	return renderToStaticMarkup(<RouterProvider router={router} />);
}
