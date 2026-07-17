import type { ReactNode } from "react";
import "./Button.scss";

type Props = {
	children: ReactNode;
};

export function Button(props: Props) {
	return (
		<button type="button" className="button">
			{props.children}
		</button>
	);
}
