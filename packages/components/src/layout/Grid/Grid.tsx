import type { ReactNode } from "react";
import "./Grid.scss";

type Props = {
	children: ReactNode;
};

export function Grid(props: Props) {
	return <div className="grid">{props.children}</div>;
}
