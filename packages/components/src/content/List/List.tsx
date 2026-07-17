import type { ReactNode } from "react";
import "./List.scss";

type Props = {
	children: ReactNode;
};

export function List(props: Props) {
	return <ul className="list">{props.children}</ul>;
}
