import { Frame, Hero } from "@perish/react-components";
import type { ReactNode } from "react";

export default function Home(): ReactNode {
	return (
		<Frame>
			<Hero title="plumb" text="a plumb line for repositories" mark="◆" />
		</Frame>
	);
}
