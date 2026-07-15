import "./Copy.scss";

type Props = {
	text: string;
};

export function Copy(props: Props) {
	return (
		<button
			className="copy"
			type="button"
			onClick={() => navigator.clipboard.writeText(props.text)}
		>
			copy
		</button>
	);
}
