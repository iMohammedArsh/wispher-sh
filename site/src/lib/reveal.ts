// Sections rise in as they scroll into view, once. Each part waits a little
// longer than the one before it (`use:reveal={i}`). Without JavaScript, or with
// reduced motion, everything is simply there (see site.css).
let observer: IntersectionObserver | undefined;

function shared(): IntersectionObserver {
  observer ??= new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        entry.target.classList.add("is-in");
        observer?.unobserve(entry.target);
      }
    },
    { rootMargin: "0px 0px -10% 0px", threshold: 0.1 },
  );
  return observer;
}

export function reveal(node: HTMLElement, order = 0) {
  node.dataset.reveal = "";
  node.style.transitionDelay = `${Math.min(order, 4) * 80}ms`;
  const io = shared();
  io.observe(node);
  return { destroy: () => io.unobserve(node) };
}
