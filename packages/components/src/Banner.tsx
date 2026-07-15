import type { ReactNode } from "react";
import "./Banner.scss";

type Props = {
	mark: string;
	title: string;
	line: string;
	children?: ReactNode;
};

export function Banner(props: Props) {
	return (
		<header className="banner">
			<img src={props.mark} alt="" width="56" height="56" />
			<div>
				<h1>{props.title}</h1>
				<p>{props.line}</p>
			</div>
			<nav>{props.children}</nav>
			<img className="ghost" src={props.mark} alt="" width="224" height="224" />
		</header>
	);
}
