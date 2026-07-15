import type { ReactNode } from "react";
import { Copy } from "./Copy";
import "./Code.scss";

type Props = {
	children: string;
	name?: string;
	copy?: boolean;
};

function order(text: string, index: number): ReactNode {
	if (!text.startsWith("$ ")) {
		return `${text}\n`;
	}
	return (
		<span className="order" key={index}>
			<span className="sign">$</span>
			{`${text.slice(1)}\n`}
		</span>
	);
}

function body(text: string): ReactNode {
	if (!text.split("\n").some((line) => line.startsWith("$ "))) {
		return text;
	}
	return text.split("\n").map((line, index) => order(line, index));
}

export function Code(props: Props) {
	const topped = props.name !== undefined || props.copy === true;
	return (
		<div className="code">
			{topped ? (
				<header>
					<span>{props.name}</span>
					{props.copy === true ? <Copy text={props.children} /> : null}
				</header>
			) : null}
			<pre>
				<code>{body(props.children)}</code>
			</pre>
		</div>
	);
}
