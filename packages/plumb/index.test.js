import { expect, test } from "vitest";
import { carrier } from "./index.js";

test("the carrier names itself", () => {
	expect(carrier).toBe("plumb");
});
