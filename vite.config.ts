import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Two entry points: the main workspace and the tiny always-on-top overlay pill.
// The overlay must stay small, so it never imports the main app's routes or styles.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    port: 1430,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    target: "safari15",
    rollupOptions: {
      input: {
        main: new URL("./main.html", import.meta.url).pathname,
        overlay: new URL("./overlay.html", import.meta.url).pathname,
      },
    },
  },
  test: {
    include: ["src/**/*.test.ts"],
  },
});
