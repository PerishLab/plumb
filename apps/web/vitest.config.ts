import { defineConfig, mergeConfig } from "vitest/config";
import app from "./vite.config";

export default mergeConfig(
	app,
	defineConfig({
		test: {
			include: ["tests/**/*.test.ts"],
			server: {
				deps: {
					inline: ["@perishlab/crest", "@perishlab/design", "@perishlab/sign"],
				},
			},
		},
	}),
);
