import "./Ledger.scss";

type Atom = {
	word: string;
	count: number;
};

type Props = {
	atoms: Atom[];
};

export function Ledger(props: Props) {
	return (
		<ul className="ledger">
			{props.atoms.map((atom) => (
				<li key={atom.word}>
					<span>{atom.word}</span>
					<span className="tail" /> <span className="tally">{atom.count}</span>
				</li>
			))}
		</ul>
	);
}
