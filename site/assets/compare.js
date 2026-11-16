// The releases compared (docs/design.md, "Why 2.0 moved to Rust"). Each loop of 2.0's bar
// (site.css, "shrink") the figure at its tip counts down from 1.1's to 2.0's while the bar
// shrinks, and the multiple counts up with it: it is always 1.1's figure over the one shown.
// Without script, or with reduced motion (no animation runs), the final figures stand.
(() => {
  const DELAY = 400, LENGTH = 900; // the bar shrinks from 8% to 26% of its 5-second loop

  // The page's easing, cubic-bezier(.3, .7, .2, 1): progress for a share of the time.
  const ease = (t) => {
    const x1 = 0.3, y1 = 0.7, x2 = 0.2, y2 = 1;
    const at = (a, b, u) => 3 * a * u * (1 - u) ** 2 + 3 * b * u * u * (1 - u) + u ** 3;
    let lo = 0, hi = 1, u = t;
    for (let i = 0; i < 24; i++) {
      u = (lo + hi) / 2;
      if (at(x1, x2, u) < t) lo = u;
      else hi = u;
    }
    return at(y1, y2, u);
  };
  const times = (r) => (r >= 10 ? String(Math.round(r)) : r.toFixed(1)) + "×";

  for (const m of document.querySelectorAll(".cmp .m[data-a]")) {
    const bar = m.querySelector(".new i");
    const fig = m.querySelector(".new em");
    const big = m.querySelector(".big b");
    if (!bar || !fig || !big) continue;
    const a = Number(m.dataset.a), b = Number(m.dataset.b), dp = Number(m.dataset.dp), unit = m.dataset.u;
    const endFig = fig.textContent, endBig = big.textContent;
    const counts = endBig !== "≈"; // "about the same" has no multiple to count
    let run = 0;

    const show = (v) => {
      fig.textContent = `${v.toFixed(dp)} ${unit}`;
      if (counts) big.textContent = times(Math.max(a, v) / Math.min(a, v));
    };
    const start = () => {
      const id = ++run;
      show(a);
      const t0 = performance.now() + DELAY;
      const step = (now) => {
        if (id !== run) return;
        const t = Math.min(1, Math.max(0, (now - t0) / LENGTH));
        if (t >= 1) {
          fig.textContent = endFig;
          big.textContent = endBig;
          return;
        }
        show(a + (b - a) * ease(t));
        requestAnimationFrame(step);
      };
      requestAnimationFrame(step);
    };
    bar.addEventListener("animationstart", start);
    bar.addEventListener("animationiteration", start);
  }
})();
