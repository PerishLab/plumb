export type Atom = {
	word: string;
	count: number;
};

export type Root = {
	root: string;
	atoms: Atom[];
};

export type Entry = {
	repo: string;
	roots: Root[];
};

function prune(root: Root, term: string): Root {
	const atoms = root.atoms.filter((atom) =>
		atom.word.toLowerCase().includes(term),
	);
	return { root: root.root, atoms };
}

export function sift(entries: Entry[], query: string): Entry[] {
	const term = query.trim().toLowerCase();
	if (term === "") {
		return entries;
	}
	return entries
		.map((entry) => ({
			repo: entry.repo,
			roots: entry.roots
				.map((root) => prune(root, term))
				.filter((root) => root.atoms.length > 0),
		}))
		.filter((entry) => entry.roots.length > 0);
}
