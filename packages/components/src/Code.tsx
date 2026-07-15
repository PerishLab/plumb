import "./Code.scss";

type Props = {
	children: string;
};

export function Code(props: Props) {
	return (
		<pre className="code">
			<code>{props.children}</code>
		</pre>
	);
}
