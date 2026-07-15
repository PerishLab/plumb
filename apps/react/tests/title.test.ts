import { expect, test } from "vitest";
import { title } from "../src/lib/title";

test("title", () => {
	expect(title()).toBe("open-web");
});
