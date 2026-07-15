import { Button } from "@open-web/components";
import { title } from "../lib/title";

export function Home() {
	return (
		<main>
			<h1>{title()}</h1>
			<Button>press</Button>
		</main>
	);
}
