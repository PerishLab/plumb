import type { ReactNode } from "react";
import "./Badge.scss";

type Props = {
	children: ReactNode;
};

export function Badge(props: Props) {
	return <span className="badge">{props.children}</span>;
}
