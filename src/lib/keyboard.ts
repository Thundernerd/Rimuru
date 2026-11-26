const KEYS = new Set(["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End"]);

export function arrowNavigation(node: HTMLElement) {
  function onkeydown(e: KeyboardEvent) {
    if (!KEYS.has(e.key) || e.altKey || e.metaKey || e.ctrlKey) return;
    const cards = [...node.querySelectorAll<HTMLElement>("button.card")];
    const index = cards.indexOf(document.activeElement as HTMLElement);
    if (index === -1) return;
    const firstTop = Math.round(cards[0].getBoundingClientRect().top);
    const columns = Math.max(1, cards.filter((c) => Math.round(c.getBoundingClientRect().top) === firstTop).length);
    const target = {
      ArrowLeft: index - 1,
      ArrowRight: index + 1,
      ArrowUp: index - columns,
      ArrowDown: index + columns,
      Home: 0,
      End: cards.length - 1,
    }[e.key]!;
    if (target === index || target < 0 || target >= cards.length) return;
    e.preventDefault();
    cards[target].focus();
    cards[target].scrollIntoView({ block: "nearest", inline: "nearest" });
  }
  node.addEventListener("keydown", onkeydown);
  return {
    destroy() {
      node.removeEventListener("keydown", onkeydown);
    },
  };
}
