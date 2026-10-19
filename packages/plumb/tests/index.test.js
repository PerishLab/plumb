import { carrier } from "@perishlab/plumb";
import { expect, test } from "vitest";

test("the carrier names itself", () => {
	expect(carrier).toBe("plumb");
});
