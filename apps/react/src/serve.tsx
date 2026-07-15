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
		`<meta property="og:url" content="${home}${page.path === "/" ? "/" : `${page.path}/`}" />`,
		`<meta property="og:type" content="website" />`,
		`<meta name="twitter:card" content="summary_large_image" />`,
	].join("\n\t\t");
}

const guide = `# open-web

> a workshop of three tools for code that AI agents maintain: rules live in
> checkable structure and a reviewed vocabulary, not in prose comments.

## tools

- [negentropy](https://harness.perish.uk/negentropy/): structural checker, nine
  mechanical laws, five languages. install:
  curl -fsSL https://releases.negentropy.perish.uk/manage.sh | sh
- [runseal](https://harness.perish.uk/runseal/): operator toolbelt - explicit
  profile, named wrappers, forge tools. install:
  curl -fsSL https://runseal.perish.uk/manage.sh | sh
- [sidecar](https://harness.perish.uk/sidecar/): local process manager -
  manifest lifecycle, stamped identity, leased ports. install:
  curl -fsSL https://sidecar.perish.uk/manage.sh | sh

## law

- [constitution](https://harness.perish.uk/constitution/): the nine laws and
  why each exists
- [vocabulary](https://harness.perish.uk/vocabulary/): every declared name,
  counted; written verdicts for contested words

## source

- https://github.com/PerishCode/negentropy
- https://github.com/PerishCode/runseal
- https://github.com/PerishCode/sidecar
`;

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
writeFileSync("dist/llms.txt", guide);
console.log(`prerender: ${pages.length} routes + llms.txt`);
