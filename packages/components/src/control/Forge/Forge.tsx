import "./Forge.scss";

const drawn =
	"M9.5 3.25a2.25 2.25 0 1 1 3 2.122V6A2.5 2.5 0 0 1 10 8.5H6a1 1 0 0 0-1 1v1.128a2.251 2.251 0 1 1-1.5 0V5.372a2.25 2.25 0 1 1 1.5 0v1.836A2.493 2.493 0 0 1 6 7h4a1 1 0 0 0 1-1v-.628a2.25 2.25 0 0 1-1.5-2.122ZM4.25 12a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5ZM3.5 3.25a.75.75 0 1 1 1.5 0 .75.75 0 0 1-1.5 0Zm8.25-.75a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5Z";

type Props = {
	repo: string;
};

export function Forge(props: Props) {
	return (
		<a className="forge" href={`https://git.perish.top/${props.repo}`}>
			<svg
				viewBox="0 0 16 16"
				width="16"
				height="16"
				role="img"
				aria-label="forge"
			>
				<path fill="currentColor" d={drawn} />
			</svg>
			{props.repo}
		</a>
	);
}
