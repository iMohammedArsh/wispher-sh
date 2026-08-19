import { spawn } from "node:child_process";

// This is deliberately not port 1420: that port is commonly retained by an
// older WSL Vite server. A native Tauri process must not silently render UI
// code from a different development environment.
const url = "http://127.0.0.1:1430/";

async function hasViteServer() {
  try {
    const response = await fetch(url, { signal: AbortSignal.timeout(800) });
    const page = await response.text();
    return response.ok && page.includes("/@vite/client");
  } catch {
    return false;
  }
}

if (await hasViteServer()) {
  console.log("[dev] Reusing the native Vite server on port 1430.");
  process.exit(0);
}

// Node 26 on Windows can reject a detached `npm.cmd` child with EINVAL.
// Run Vite's local JavaScript entry point with this same Node process instead.
const child = spawn(process.execPath, ["node_modules/vite/bin/vite.js", "--host", "127.0.0.1", "--port", "1430", "--strictPort"], {
  cwd: process.cwd(),
  detached: true,
  stdio: "ignore",
  windowsHide: true,
});
child.unref();

for (let attempt = 0; attempt < 40; attempt += 1) {
  await new Promise((resolve) => setTimeout(resolve, 250));
  if (await hasViteServer()) {
    console.log("[dev] Native Vite server started on port 1430.");
    process.exit(0);
  }
}

console.error("[dev] Native Vite did not become available on port 1430.");
process.exit(1);
