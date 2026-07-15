import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Home } from "./views/Home";

const root = document.getElementById("root");
if (root !== null) {
	createRoot(root).render(
		<StrictMode>
			<Home />
		</StrictMode>,
	);
}
