import type { ReactNode } from "react";
import { Home } from "../views/Home";

export const routes: { path: string; element: ReactNode }[] = [
	{ path: "/", element: <Home /> },
];
