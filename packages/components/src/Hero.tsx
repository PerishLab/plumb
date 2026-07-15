import type { ReactNode } from "react";
import "./Hero.scss";

type Props = {
	title: string;
	text: string;
	children?: ReactNode;
};

export function Hero(props: Props) {
	return (
		<header className="hero">
			<h1>{props.title}</h1>
			<p>{props.text}</p>
			{props.children}
		</header>
	);
}
