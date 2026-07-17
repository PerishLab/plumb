import { useState } from "react";
import "./Copy.scss";

type Props = {
	text: string;
};

export function Copy(props: Props) {
	const [done, flip] = useState(false);
	const grab = () => {
		navigator.clipboard.writeText(props.text);
		flip(true);
		setTimeout(() => flip(false), 1600);
	};
	return (
		<button
			className={done ? "copy done" : "copy"}
			type="button"
			onClick={grab}
		>
			{done ? "copied" : "copy"}
		</button>
	);
}
