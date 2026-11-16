// The releases compared (docs/design.md, "How heavy each release is"): pointing at a card, or
// tapping it, shrinks 2.0's bar from 1.1's length to its own. The bars already stand at their
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
