import "./Search.scss";

type Props = {
	value: string;
	change: (next: string) => void;
	hint?: string;
};

export function Search(props: Props) {
	return (
		<input
			className="search"
			type="search"
			value={props.value}
			placeholder={props.hint}
			onChange={(event) => props.change(event.target.value)}
		/>
	);
}
