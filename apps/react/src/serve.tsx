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
  manifest lifecycle, stamped identity, chosen free ports. install:
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

const manual = `# open-web - full reference for agents

> the deep mirror of https://harness.perish.uk for agent readers; /llms.txt is
> the short card. rules live in checkable structure and a reviewed vocabulary,
> not in prose comments.

## negentropy - structural checker

what: nine mechanical laws judged over one parser substrate; five languages
(rust, typescript, tsx, scss, markdown). a violation lands as fault (fails the
run), debt (reported, tolerated), or blindspot (unparsed region).

install (linux x86_64 today):
  curl -fsSL https://releases.negentropy.perish.uk/manage.sh | sh
installs under ~/.local/share/negentropy and links into ~/.local/bin.

surface:
  negentropy [OPTIONS] [ROOT]   (ROOT defaults to .)
    --strict       blindspots become fatal
    --debt         list tolerated debt lines
    --json         print structure trees, one {"path","tree"} line per file
    --vocabulary   print the living dictionary per module root
exit codes: 0 = clean or debt only; 1 = any fault, or any blindspot under
--strict.

config (negentropy.toml at the repo root; without one, defaults judge the
whole tree):
  [scan] include / exclude globs
  [module] roots (the depth coordinate system)
  [limit] block = 4, path = 4; markup = 8 counts its own axis
  [comment] allow = false
  [word] single = true
  [[grant]] syntax = "test" | "style", paths - confine a syntax class
  [[boundary]] paths, allow, note - a declared exemption
  vocabulary.toml: [compound] name = "rationale" registers a compound;
  an empty rationale does not register.
source: https://github.com/PerishCode/negentropy

## runseal - operator toolbelt

what: run commands inside a small explicit profile - env, argv, symlinks,
declared resources; repo-authored wrappers become verbs.

install (linux x86_64 today; windows: manage.ps1):
  curl -fsSL https://runseal.perish.uk/manage.sh | sh

surface:
  runseal <cmd>     run an external command inside the profile
  runseal :<name>   run a profile wrapper (.ts under the deno policy, or .sh)
  runseal @profile | @resources | @resolve <uri> | @tool | @wrappers |
  @which :<name>
  -p, --profile <PROFILE>   explicit profile path

profile (runseal.toml at the repo root; discovery walks upward, then
~/.runseal/profiles/default.toml):
  [resources] root
  [[injections]] type = "env" (+ [injections.vars] NAME = "resource://path")
  [deno] permissions = ["--allow-..."] - required for .ts wrappers
source: https://github.com/PerishCode/runseal

## sidecar - local process manager

what: one manifest per project; stamped process identity; a chosen free port
handed to each target as SIDECAR_PORT; one loopback tcp broker per namespace;
an inspect bridge over unix sockets.

install (linux x86_64, macos intel and apple silicon; windows: manage.ps1):
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
source: https://github.com/PerishCode/sidecar

## law

constitution: https://harness.perish.uk/constitution/ - nine laws and why
each exists. vocabulary: https://harness.perish.uk/vocabulary/ - every
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

const home = "https://harness.perish.uk";
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
			`\t<url><loc>${home}${page.path === "/" ? "/" : `${page.path}/`}</loc></url>`,
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
