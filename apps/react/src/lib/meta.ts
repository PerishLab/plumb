export type Page = {
	path: string;
	title: string;
	text: string;
};

export const pages: Page[] = [
	{
		path: "/",
		title: "open-web — a workshop for agent-maintained code",
		text: "four sharp tools on one premise: code maintained by AI agents needs laws, not comments. a structural checker, an operator toolbelt, a process manager, and a fault substrate.",
	},
	{
		path: "/negentropy",
		title: "negentropy — keep entropy down",
		text: "a grammar-first structural checker for agent-maintained codebases: eleven laws, a living vocabulary, no target compilers, no target toolchains.",
	},
	{
		path: "/runseal",
		title: "runseal — keep flows explicit",
		text: "run commands inside a small explicit profile: named wrappers, declared resources, and forge tools. no hidden orchestration.",
	},
	{
		path: "/sidecar",
		title: "sidecar — keep local runtimes named",
		text: "a manifest-driven local process manager: stamped identity, chosen free ports, one broker per namespace. shallow isolation without space isolation.",
	},
	{
		path: "/shield",
		title: "shield — keep faults named",
		text: "a fault substrate for typescript: native throw, family-scoped kinds, no Result monad, zero runtime deps. one declaration types the throw site and every handler.",
	},
	{
		path: "/constitution",
		title: "constitution — eleven laws, three classes",
		text: "the laws every repo in this workshop answers to, why each one exists, and the stability promise that prices change.",
	},
	{
		path: "/vocabulary",
		title: "vocabulary — the living dictionary",
		text: "every name declared across the three tools and this site, recounted on every site build and judged by written case law.",
	},
];
