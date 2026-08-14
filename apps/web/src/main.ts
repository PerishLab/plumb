import source from "virtual:perish/views";
import { Views } from "@perish/design";
import { hydrate, mount } from "svelte";

const target = document.getElementById("root");
if (target !== null) {
	const props = { source };
	(target.hasChildNodes() ? hydrate : mount)(Views, { target, props });
}
