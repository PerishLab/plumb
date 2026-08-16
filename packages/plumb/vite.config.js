import { defineConfig } from "vite";

// The carrier holds no export yet, so the library build exists to fix the shape
// a published module takes: one entry, one format, one emitted seat.
export default defineConfig({
	build: {
		emptyOutDir: true,
		lib: {
			entry: "index.js",
			fileName: () => "index.js",
			formats: ["es"],
		},
		outDir: "dist",
	},
});
