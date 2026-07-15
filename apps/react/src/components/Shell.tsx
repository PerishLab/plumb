import { Frame, Nav } from "@open-web/components";
import { NavLink, Outlet } from "react-router";

export function Shell() {
	return (
		<Frame>
			<Nav>
				<NavLink to="/" end>
					open-web
				</NavLink>
				<NavLink to="/negentropy">negentropy</NavLink>
				<NavLink to="/runseal">runseal</NavLink>
				<NavLink to="/sidecar">sidecar</NavLink>
				<NavLink to="/constitution">constitution</NavLink>
				<NavLink to="/vocabulary">vocabulary</NavLink>
			</Nav>
			<Outlet />
		</Frame>
	);
}
