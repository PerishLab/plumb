import type { RouteObject } from "react-router";
import { Shell } from "../components/Shell";
import { Constitution } from "../views/Constitution";
import { Home } from "../views/Home";
import { Negentropy } from "../views/Negentropy";
import { Runseal } from "../views/Runseal";
import { Shield } from "../views/Shield";
import { Sidecar } from "../views/Sidecar";
import { Vocabulary } from "../views/Vocabulary";

export const routes: RouteObject[] = [
	{
		Component: Shell,
		children: [
			{ path: "/", Component: Home },
			{ path: "/negentropy", Component: Negentropy },
			{ path: "/runseal", Component: Runseal },
			{ path: "/sidecar", Component: Sidecar },
			{ path: "/shield", Component: Shield },
			{ path: "/constitution", Component: Constitution },
			{ path: "/vocabulary", Component: Vocabulary },
		],
	},
];
