// The releases compared (docs/design.md, "How heavy each release is"): pointing at a card, or
// tapping it, drains its inner ring from 1.1's level to 2.0's. The rings already stand at 2.0's
// figures without it, and the stylesheet plays nothing for reduced motion.
(() => {
  for (const m of document.querySelectorAll(".cmp .m")) {
    const play = () => {
      m.classList.remove("play");
      void m.offsetWidth; // restart the animation
      m.classList.add("play");
    };
    m.addEventListener("pointerenter", play);
    m.addEventListener("click", play);
    m.addEventListener("animationend", () => m.classList.remove("play"));
  }
})();
