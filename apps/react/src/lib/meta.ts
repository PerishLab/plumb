export type Page = {
	path: string;
	title: string;
	text: string;
};

export const pages: Page[] = [
	{
		path: "/",
		title: "open-web — a workshop for agent-maintained code",
		text: "three sharp tools on one premise: code maintained by AI agents needs laws, not comments. a structural checker, an operator toolbelt, and a process manager.",
	},
	{
		path: "/negentropy",
		title: "negentropy — keep entropy down",
		text: "a grammar-first structural checker for agent-maintained codebases: nine laws, a living vocabulary, no target compilers, no target toolchains.",
	},
	{
		path: "/runseal",
		title: "runseal — keep flows explicit",
		text: "run commands inside a small explicit profile: named wrappers, declared resources, and forge tools. no hidden orchestration.",
	},
	{
		path: "/sidecar",
		title: "sidecar — keep local runtimes named",
		text: "a manifest-driven local process manager: stamped identity, leased ports, one broker per namespace. shallow isolation without space isolation.",
	},
	{
		path: "/constitution",
		title: "constitution — nine laws, three classes",
		text: "the laws every repo in this workshop answers to, why each one exists, and the stability promise that prices change.",
	},
	{
		path: "/vocabulary",
		title: "vocabulary — the living dictionary",
		text: "every atom declared across the three tools and this site, counted on every bake and judged by case law.",
	},
];
