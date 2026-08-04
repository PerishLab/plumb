import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { renderToString } from "react-dom/server";
import { type Page, pages } from "./lib/meta";
import Home from "./views/index";

function render(path: string): string {
	return path === "/" ? renderToString(<Home />) : "";
}

function head(page: Page): string {
	const home = "https://plumb.perish.uk";
	return [
		`<title>${page.title}</title>`,
		`<meta name="description" content="${page.text}" />`,
		`<meta property="og:title" content="${page.title}" />`,
		`<meta property="og:description" content="${page.text}" />`,
		`<meta property="og:image" content="${home}/og.png" />`,
		`<meta property="og:url" content="${home}${
			page.path === "/" ? "/" : `${page.path}/`
		}" />`,
		`<meta property="og:type" content="website" />`,
		`<meta name="twitter:card" content="summary_large_image" />`,
	].join("\n\t\t");
}

const guide = `# open-web

> a workshop of four tools for code that AI agents maintain: rules live in
> checkable structure and a reviewed vocabulary, not in prose comments.

## tools

- [ectropy](https://plumb.perish.uk/ectropy/): structural checker, fifteen
  mechanical laws, four language adapters. install:
  curl -fsSL https://releases.ectropy.perish.uk/manage.sh | sh
- [runseal](https://plumb.perish.uk/runseal/): operator toolbelt - explicit
  profile injection for external commands. install:
  curl -fsSL https://runseal.perish.uk/manage.sh | sh
- [sidecar](https://plumb.perish.uk/sidecar/): local process manager -
  manifest lifecycle, stamped identity, chosen free ports. install:
  curl -fsSL https://sidecar.perish.uk/manage.sh | sh
- [shield](https://plumb.perish.uk/shield/): fault substrate for typescript -
  native throw, family-scoped kinds, no Result monad, zero deps. import:
  import { family } from "jsr:@perish/shield";

## law

- [constitution](https://plumb.perish.uk/constitution/): the eleven laws and
  why each exists
- [vocabulary](https://plumb.perish.uk/vocabulary/): every declared name,
  counted; written verdicts for contested words

## source

- https://git.perish.top/PerishFire/ectropy
- https://git.perish.top/PerishFire/runseal
- https://git.perish.top/PerishFire/sidecar
- https://git.perish.top/PerishFire/shield
`;

const manual = `# open-web - full reference for agents

> the deep mirror of https://plumb.perish.uk for agent readers; /llms.txt is
> the short card. rules live in checkable structure and a reviewed vocabulary,
> not in prose comments.

## ectropy - structural checker

what: fifteen mechanical laws judged over one parser substrate; rust,
typescript/tsx, scss, and markdown adapters. every finding is an error and
fails the run.

install (see /llms.txt for the live platform list; sha256 sums sit in
checksums.txt beside every artifact):
  curl -fsSL https://releases.ectropy.perish.uk/manage.sh | sh
installs under ~/.local/share/ectropy and links into ~/.local/bin.

surface:
  ectropy [ROOT]                judge every configured law (ROOT defaults to .)
  ectropy shape [ROOT]          print structure JSON lines
  ectropy vocabulary [ROOT]     print the living dictionary
  ectropy cookbook [ENTRY]      print procedural moves and their EXIT clauses
  ectropy skill <COMMAND>       manage the ectropy agent brief
exit codes: 0 = clean; 1 = one or more findings; 2 = invalid input,
configuration, schema, or source IO.

config (ectropy.toml at the repo root; without one, defaults judge the
whole tree):
  [scan] include / exclude globs
  [module] roots (the depth coordinate system)
  [limit] block = 4, path = 4, param = 4; markup = 8 counts its own axis
  [comment] allow = false
  [word] single = true
  [[grant]] syntax = "test" | "style", paths - confine a syntax class
  [[boundary]] paths, allow, note - a declared exemption
  [[vocabulary.term]] name / description registers one explained compound;
  both fields are required and duplicates are rejected.
source: https://git.perish.top/PerishFire/ectropy

## runseal - operator toolbelt

what: run an explicit external command inside a small profile that injects
environment, arguments, and symlinks. declared resource identities remain
profile data rather than repository-authored commands.

install (windows: manage.ps1; sha256 sums in checksums.txt):
  curl -fsSL https://runseal.perish.uk/manage.sh | sh

surface:
  runseal [-p <PROFILE>] <cmd> [args...]   run an explicit external command
  runseal [-p <PROFILE>] @tool            run the atomic profile tool

every invocation names either the external command or atomic @tool operation;
the profile supplies only env, argv, symlink, and resource identity data.
source: https://git.perish.top/PerishFire/runseal

## sidecar - local process manager

what: one manifest per project; stamped process identity; a chosen free port
handed to each target as SIDECAR_PORT; one loopback tcp broker per namespace;
an inspect bridge over unix sockets.

install (windows: manage.ps1; sha256 sums in checksums.txt):
  curl -fsSL https://sidecar.perish.uk/manage.sh | sh

surface:
  sidecar doctor | plan | start | restart | stop | status | list | reset
    [--config <path>] [--format text|json] [-p <project>]
  sidecar inspect <sidecar> <event> [<json-payload>]

manifest (sidecar.toml at the repo root):
  [project] name, namespace, root
  [app] and [[sidecars]]: name, command, args, cwd, mode, env, inherits_env,
  inspect_socket, port (0 = pick a free loopback port), health_url (a {port}
  template), ready
the packed --sidecar-stamp arg is the only identity contract; state lives in
targets.json and logs under the data home.
source: https://git.perish.top/PerishFire/sidecar

## shield - fault substrate

what: a typescript fault substrate below harness; native throw, no Result
monad, zero runtime deps; a domain declares its failure vocabulary once and
that declaration types both the throw site and every handler.

import (jsr, runtime-pure):
  import { assert, family, kind, run } from "jsr:@perish/shield";

surface:
  family(name, spec) - declare a domain's fault kinds; the spec is the single
    source of truth for the factories and every handler's meta type
  assert(expr, mint) - throw a minted fault when a check fails
  run(fn) - the boundary; absorbs every non-fault into a foreign kind
  first(...tries) - sequential fallback; total failure throws exhausted
  <family>.consume(table) - exhaustive terminal disposition; every kind handled
  <family>.attempt(fn, table) - local recovery over named kinds
a business brings its own schema (zod, or the zero-dep kind phantom); shield
consumes the inferred shape and never validates. adapters at the harness seam
mint faults from native errors.
source: https://git.perish.top/PerishFire/shield

## law

constitution: https://plumb.perish.uk/constitution/ - eleven laws and why
each exists. vocabulary: https://plumb.perish.uk/vocabulary/ - every
declared name counted per owning folder; contested words carry written
verdicts.
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

const home = "https://plumb.perish.uk";
const robots = `User-agent: *
Allow: /

Sitemap: ${home}/sitemap.xml
`;
writeFileSync("dist/robots.txt", robots);
writeFileSync("dist/llms-full.txt", manual);
mkdirSync("dist/.well-known", { recursive: true });
writeFileSync("dist/.well-known/llms.txt", guide);

const spots = pages
	.map(
		(page) =>
			`\t<url><loc>${home}${
				page.path === "/" ? "/" : `${page.path}/`
			}</loc></url>`,
	)
	.join("\n");
const atlas = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${spots}
</urlset>
`;
writeFileSync("dist/sitemap.xml", atlas);
console.log(
	`prerender: ${pages.length} routes + llms + manual + robots + sitemap`,
);
