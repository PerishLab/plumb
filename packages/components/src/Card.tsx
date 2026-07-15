import type { ReactNode } from "react";
import "./Card.scss";

type Props = {
	title: string;
	children: ReactNode;
};

export function Card(props: Props) {
	return (
		<section className="card">
			<h3>{props.title}</h3>
			{props.children}
		</section>
	);
}
