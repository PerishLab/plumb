import { expect, test } from "vitest";
import type { Entry } from "../src/lib/sift";
import { sift } from "../src/lib/sift";

const entries: Entry[] = [
	{
		repo: "negentropy",
		roots: [
			{
				root: "crates/kernel/src",
				atoms: [
					{ word: "scan", count: 3 },
					{ word: "law", count: 7 },
				],
			},
		],
	},
	{
		repo: "sidecar",
		roots: [
			{
				root: "crates/core/src",
				atoms: [{ word: "stamp", count: 5 }],
			},
		],
	},
];

test("empty", () => {
	expect(sift(entries, "")).toEqual(entries);
});

test("match", () => {
	const found = sift(entries, "law");
	expect(found).toHaveLength(1);
	expect(found[0].repo).toBe("negentropy");
	expect(found[0].roots[0].atoms).toEqual([{ word: "law", count: 7 }]);
});

test("miss", () => {
	expect(sift(entries, "broker")).toEqual([]);
});
