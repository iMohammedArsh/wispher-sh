import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// The Hams landing page. Its own Vite root, sharing the app's node_modules and
// its real Heard/Written component and word diff (imported from ../src).
export default defineConfig({
  root: fileURLToPath(new URL(".", import.meta.url)),
  base: "./",
  plugins: [svelte()],
  server: { host: "127.0.0.1", port: 1431, strictPort: true, fs: { allow: [".."] } },
  build: { outDir: "dist", emptyOutDir: true, target: "es2022" },
});
