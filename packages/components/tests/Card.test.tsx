import { renderToStaticMarkup } from "react-dom/server";
import { expect, test } from "vitest";
import { Card } from "../src/lib";

test("card", () => {
	const markup = renderToStaticMarkup(
		<Card title="laws">
			<p>eight of them</p>
		</Card>,
	);
	expect(markup).toContain('class="card"');
	expect(markup).toContain("<h3>laws</h3>");
	expect(markup).toContain("eight of them");
});
