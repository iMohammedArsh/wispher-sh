// Bakes the rendered page into dist/index.html, so the words are there before
// any script runs (fast first paint, readable without JavaScript, crawlable).
// The client bundle then hydrates it.
import { readFileSync, rmSync, writeFileSync } from "node:fs";
import { fileURLToPath, pathToFileURL } from "node:url";

const dir = fileURLToPath(new URL(".", import.meta.url));
const { renderPage } = await import(pathToFileURL(`${dir}dist-ssr/entry-server.js`).href);
const { head, body } = renderPage();
const file = `${dir}dist/index.html`;
const html = readFileSync(file, "utf8");
if (!html.includes("<!--app-html-->")) throw new Error("dist/index.html has no <!--app-html--> placeholder");
writeFileSync(file, html.replace("<!--app-head-->", head).replace("<!--app-html-->", body));
rmSync(`${dir}dist-ssr`, { recursive: true, force: true });
console.log("Prerendered dist/index.html");
