import source from "virtual:perish/views";
import { Views } from "@perish/react-components";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

const app = (
	<StrictMode>
		<Views source={source} />
	</StrictMode>
);

const root = document.getElementById("root");
if (root !== null) {
	createRoot(root).render(app);
}
