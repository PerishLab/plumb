import { StrictMode } from "react";
import { createRoot, hydrateRoot } from "react-dom/client";
import { createBrowserRouter, RouterProvider } from "react-router";
import { routes } from "./lib/routes";

const router = createBrowserRouter(routes);

const app = (
	<StrictMode>
		<RouterProvider router={router} />
	</StrictMode>
);

const root = document.getElementById("root");
if (root !== null) {
	if (root.firstChild === null) {
		createRoot(root).render(app);
	} else {
		hydrateRoot(root, app);
	}
}
