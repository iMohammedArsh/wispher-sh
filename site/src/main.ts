import "./site.css";
import { hydrate, mount } from "svelte";
import App from "./App.svelte";

// Entrances only hide content once this script is running, so a failed load never hides the page.
document.documentElement.classList.add("js");
const target = document.getElementById("app")!;
// The built page arrives prerendered (prerender.mjs); in `vite dev` it's empty.
if (target.firstElementChild) hydrate(App, { target });
else mount(App, { target });
