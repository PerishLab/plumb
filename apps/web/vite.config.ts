import { design } from "@jsr/perish__vite-plugin-design";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
	plugins: [design({ login: false, serve: false }), react()],
});
