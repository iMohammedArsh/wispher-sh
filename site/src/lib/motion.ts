// The page's motion helpers. Everything here runs only in the browser (inside
// actions and effects), so the prerender in prerender.mjs never touches it.
// Every move is transform and opacity, and site.css drops the travel under
// prefers-reduced-motion. Without JavaScript nothing is hidden (see the `.js` gate).

export const reduced = () => typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;

/** Calls `enter` once, the first time the node is about 10% into the viewport. */
export function onceInView(node: Element, enter: () => void, rootMargin = "0px 0px -8% 0px") {
  const io = new IntersectionObserver(
    (entries) => {
      if (!entries.some((e) => e.isIntersecting)) return;
      io.disconnect();
      enter();
    },
    { rootMargin, threshold: 0.05 },
  );
  io.observe(node);
  return () => io.disconnect();
}

/**
 * The parts of a block arrive one after another in reading order: each direct
 * child (or each `[data-part]` inside, when there are any) rises with a delay
 * of `gap` ms times its place. `use:stagger` or `use:stagger={{ gap: 60, from: 2 }}`.
 */
export function stagger(node: HTMLElement, opts: { gap?: number; from?: number; margin?: string } = {}) {
  const gap = opts.gap ?? 80;
  const from = opts.from ?? 0;
  const parts = node.querySelectorAll<HTMLElement>("[data-part]");
  let list = parts.length ? Array.from(parts) : (Array.from(node.children) as HTMLElement[]);
  if (!list.length) list = [node];
  list.forEach((el, i) => {
    el.dataset.rise = "";
    el.style.setProperty("--delay", `${(from + i) * gap}ms`);
  });
  node.dataset.stagger = "";
  const stop = onceInView(node, () => node.classList.add("is-in"), opts.margin);
  return { destroy: stop };
}

/**
 * A headline whose lines rise from behind their own baselines. Author each
 * line as `<span class="line"><span>…</span></span>`; `use:lines` sets the
 * per-line delay and starts it when the headline is on screen.
 */
export function lines(node: HTMLElement, opts: { gap?: number; delay?: number } = {}) {
  const gap = opts.gap ?? 110;
  const delay = opts.delay ?? 0;
  node.querySelectorAll<HTMLElement>(".line > span").forEach((el, i) => el.style.setProperty("--delay", `${delay + i * gap}ms`));
  node.dataset.lines = "";
  // A `display: contents` heading has no box of its own, so watch its parent instead.
  const watched = getComputedStyle(node).display === "contents" ? node.parentElement! : node;
  return { destroy: onceInView(watched, () => node.classList.add("is-in")) };
}

/**
 * Types `text` out one character at a time through `write`, the way someone
 * dictating would see it appear. Returns a stop function. Under reduced motion
 * it writes the whole text at once.
 */
export function typeOut(text: string, write: (s: string) => void, { speed = 34, done }: { speed?: number; done?: () => void } = {}) {
  if (reduced()) {
    write(text);
    done?.();
    return () => {};
  }
  let i = 0;
  let timer: ReturnType<typeof setTimeout>;
  const tick = () => {
    i += 1;
    write(text.slice(0, i));
    if (i >= text.length) return done?.();
    // A short pause after punctuation, like a breath.
    const ch = text[i - 1];
    timer = setTimeout(tick, /[,.?!]/.test(ch) ? speed * 6 : speed + Math.random() * speed * 0.6);
  };
  timer = setTimeout(tick, speed);
  return () => clearTimeout(timer);
}
