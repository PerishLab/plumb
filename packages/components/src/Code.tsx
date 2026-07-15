import { Copy } from "./Copy";
import "./Code.scss";

type Props = {
	children: string;
	copy?: boolean;
};

export function Code(props: Props) {
	return (
		<div className="code">
			<pre>
				<code>{props.children}</code>
			</pre>
			{props.copy === true ? <Copy text={props.children} /> : null}
		</div>
	);
}
