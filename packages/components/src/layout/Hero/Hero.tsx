import "./Hero.scss";

type Props = {
	title: string;
	text: string;
	mark?: string;
};

export function Hero(props: Props) {
	return (
		<header className="hero">
			{props.mark !== undefined ? (
				<img
					className="ghost"
					src={props.mark}
					alt=""
					width="224"
					height="224"
				/>
			) : null}
			<h1>{props.title}</h1>
			<p>{props.text}</p>
		</header>
	);
}
