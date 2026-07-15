import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { renderToString } from "react-dom/server";
import { MemoryRouter, useRoutes } from "react-router";
import { type Page, pages } from "./lib/meta";
import { routes } from "./lib/routes";

function Site() {
	return useRoutes(routes);
}

function render(path: string): string {
	return renderToString(
		<MemoryRouter initialEntries={[path]}>
			<Site />
		</MemoryRouter>,
	);
}

function head(page: Page): string {
	const home = "https://harness.perish.uk";
	return [
		`<title>${page.title}</title>`,
		`<meta name="description" content="${page.text}" />`,
		`<meta property="og:title" content="${page.title}" />`,
		`<meta property="og:description" content="${page.text}" />`,
		`<meta property="og:image" content="${home}/og.png" />`,
		`<meta property="og:url" content="${home}${page.path}" />`,
		`<meta property="og:type" content="website" />`,
		`<meta name="twitter:card" content="summary_large_image" />`,
	].join("\n\t\t");
}

const template = readFileSync("dist/index.html", "utf8");
for (const page of pages) {
	const html = template
		.replace("<title>open-web</title>", head(page))
		.replace(
			'<div id="root"></div>',
			`<div id="root">${render(page.path)}</div>`,
		);
	const dir = page.path === "/" ? "dist" : `dist${page.path}`;
	mkdirSync(dir, { recursive: true });
	writeFileSync(`${dir}/index.html`, html);
}
console.log(`prerender: ${pages.length} routes`);
