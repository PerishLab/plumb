import { Footer, Forge, Frame, Nav } from "@open-web/components";
import { useEffect } from "react";
import { NavLink, Outlet, useLocation } from "react-router";
import { pages } from "../lib/meta";

export function Shell() {
	const spot = useLocation();
	useEffect(() => {
		const page = pages.find((entry) => entry.path === spot.pathname);
		if (page !== undefined) {
			document.title = page.title;
		}
	}, [spot]);
	return (
		<Frame>
			<a className="skip" href="#main">
				skip to content
			</a>
			<Nav>
				<NavLink to="/" end>
					<img src="/favicon.svg" alt="" width="18" height="18" />
					open-web
				</NavLink>
				<NavLink to="/negentropy">negentropy</NavLink>
				<NavLink to="/runseal">runseal</NavLink>
				<NavLink to="/sidecar">sidecar</NavLink>
				<NavLink to="/constitution">constitution</NavLink>
				<NavLink to="/vocabulary">vocabulary</NavLink>
			</Nav>
			<main id="main">
				<Outlet />
			</main>
			<Footer>
				<Forge repo="PerishFire/negentropy" />
				<Forge repo="PerishFire/runseal" />
				<Forge repo="PerishFire/sidecar" />
			</Footer>
		</Frame>
	);
}
