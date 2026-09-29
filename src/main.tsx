import React from "react";
import ReactDOM from "react-dom/client";
import "@fontsource/dm-sans/400.css";
import "@fontsource/dm-sans/500.css";
import "@fontsource/dm-sans/600.css";
import "@fontsource/dm-sans/700.css";
import "@fontsource/dm-serif-display/400.css";
import "@fontsource/dm-serif-display/400-italic.css";
import App from "./App";
import "./index.css";
import "./refinement.css";
import "./accessibility.css";
import "./titlebar.css";
import "./workspace.css";
import { currentWindowLabel } from "./tauri";

// This runs before React mounts. It prevents the island from ever painting with
// the main application's light page styling while the React tree is starting.
if (currentWindowLabel() === "island") {
  document.body.classList.add("overlay-window");
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
