import type { ReactNode } from "react";
import "./Nav.scss";

type Props = {
	children: ReactNode;
};

export function Nav(props: Props) {
	return <nav className="nav">{props.children}</nav>;
}
