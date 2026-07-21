export type Page = {
	path: string;
	title: string;
	text: string;
};

export const pages: Page[] = [
	{
		path: "/",
		title: "plumb — hold a repo against the skeleton",
		text: "a plumb line for repositories: the living skeleton this workshop builds from, run rather than copied.",
	},
];
