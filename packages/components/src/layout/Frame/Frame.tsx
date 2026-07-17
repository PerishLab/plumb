import type { ReactNode } from "react";
import "@fontsource/spectral/600.css";
import "../../tokens.scss";
import "../../themes/dark.scss";
import "../../themes/light.scss";
import "./Frame.scss";

type Props = {
	children: ReactNode;
};

export function Frame(props: Props) {
	return <div className="frame">{props.children}</div>;
}
