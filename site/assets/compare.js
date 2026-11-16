// The releases compared (docs/design.md, "How heavy each release is"). The switch picks the
// release the multiples are against; pointing at a release lights it in every chart. Without
// script the page shows the multiples against the release before the newest.
(() => {
  const s = document.querySelector(".cmp");
  if (!s) return;
  const sw = s.querySelector(".vs");
  const said = s.querySelector('[data-k="against"]');
  if (sw) {
    sw.hidden = false;
    if (said) said.hidden = true;
    sw.addEventListener("click", (e) => {
      const b = e.target.closest("button[data-base]");
      if (!b) return;
      const base = b.dataset.base;
      for (const x of sw.querySelectorAll("button")) x.setAttribute("aria-pressed", String(x === b));
      for (const em of s.querySelectorAll("em[data-vs]")) em.hidden = em.dataset.vs !== base;
    });
  }
  const light = (r) => {
    s.classList.toggle("hl", r !== null);
    for (const li of s.querySelectorAll("li[data-r]")) li.classList.toggle("on", li.dataset.r === r);
  };
  for (const li of s.querySelectorAll("li[data-r]")) {
    li.addEventListener("pointerenter", () => light(li.dataset.r));
    li.addEventListener("pointerleave", () => light(null));
  }
})();
