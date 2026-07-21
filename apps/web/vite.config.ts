import { design } from "@jsr/perish__vite-plugin-design";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
	plugins: [design(), react()],
	server: {
		host: "127.0.0.1",
		port: Number(process.env.SIDECAR_PORT ?? 5173),
		strictPort: true,
	},
});
