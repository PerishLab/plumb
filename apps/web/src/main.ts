import source from "virtual:perish/views";
import { Views } from "@perishlab/design";
import { hydrate, mount } from "svelte";
import Home from "./views/index.svelte";

const target = document.getElementById("root");
if (target !== null) {
	if (globalThis.location.pathname === "/" && target.hasChildNodes()) {
		hydrate(Home, { target });
	} else {
		mount(Views, { target, props: { source } });
	}
}
