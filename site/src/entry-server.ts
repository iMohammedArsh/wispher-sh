import { render } from "svelte/server";
import App from "./App.svelte";

export function renderPage() {
  return render(App);
}
