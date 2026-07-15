import type { ReactNode } from "react";
import "./Footer.scss";

type Props = {
	children?: ReactNode;
};

export function Footer(props: Props) {
	return (
		<footer className="footer">
			<span>
				a PerishLab workshop — this site is MIT and guarded by its own
				constitution.
			</span>
			<nav>{props.children}</nav>
		</footer>
	);
}
