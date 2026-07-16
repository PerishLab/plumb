import "./Rail.scss";

type Stop = {
	mark: string;
	name: string;
	text: string;
};

type Props = {
	stops: Stop[];
};

export function Rail(props: Props) {
	return (
		<ol className="rail">
			{props.stops.map((stop) => (
				<li key={stop.name}>
					<img src={stop.mark} alt="" width="40" height="40" />
					<b>{stop.name}</b>
					<span>{stop.text}</span>
				</li>
			))}
		</ol>
	);
}
