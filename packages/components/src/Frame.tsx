import type { ReactNode } from "react";
import "./Frame.scss";

type Props = {
	children: ReactNode;
};

export function Frame(props: Props) {
	return <div className="frame">{props.children}</div>;
}
