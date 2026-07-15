import { renderToStaticMarkup } from "react-dom/server";
import { expect, test } from "vitest";
import { Nav } from "../src/lib";

test("nav", () => {
	const markup = renderToStaticMarkup(
		<Nav>
			<a href="/">home</a>
		</Nav>,
	);
	expect(markup).toContain('class="nav"');
	expect(markup).toContain("home");
});
