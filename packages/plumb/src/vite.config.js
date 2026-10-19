import { defineConfig } from "vite";

export default defineConfig({
	build: {
		emptyOutDir: true,
		lib: {
			entry: "src/index.js",
			fileName: () => "index.js",
			formats: ["es"],
		},
		outDir: "dist",
	},
});
