/* The live pill: each time the answer dot reaches it, step to the next reading. The page reads
   the same without this script (15%, 45%, 17%), and it does nothing under reduced motion. */
(function () {
  if (matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  /* session %, week %, Copilot %: a session filling up, then resetting */
  var S = [[15, 45, 17], [38, 46, 17], [61, 48, 18], [74, 49, 18], [86, 51, 19], [93, 52, 19]];
  function k(n) { return document.querySelector('[data-k="' + n + '"]'); }
  function tone(p) { return p >= 90 ? 'r' : p >= 70 ? 'a' : 'p'; }
  var txt = [k('v5'), k('v7'), k('va')], ring = [k('r5'), k('r7')], bar = k('ba'), pill = k('pill'), mk = k('mk');
  var dot = document.querySelector('.wab .dot');
  if (!dot || !pill) return;
  var cur = S[0].slice(), i = 0, raf = 0, timer = 0;
  function step(v) {
    ring[0].style.strokeDashoffset = 100 - v[0]; ring[0].dataset.t = tone(v[0]);
    ring[1].style.strokeDashoffset = 100 - v[1]; ring[1].dataset.t = tone(v[1]);
    bar.style.height = v[2] + '%'; bar.dataset.t = tone(v[2]);
    mk.style.left = v[0] + '%';
    var from = cur.slice(), t0 = performance.now();
    cancelAnimationFrame(raf);
    (function tick(t) {
      var p = Math.min(1, Math.max(0, (t - t0) / 800));
      for (var j = 0; j < 3; j++) { cur[j] = from[j] + (v[j] - from[j]) * p; txt[j].textContent = Math.round(cur[j]) + '%'; }
      if (p < 1) raf = requestAnimationFrame(tick);
    })(t0);
    pill.classList.remove('hit'); void pill.offsetWidth; pill.classList.add('hit');
  }
  /* The answer dot lands 78% of the way through its 5 second loop */
  function arm() { clearTimeout(timer); timer = setTimeout(function () { i = (i + 1) % S.length; step(S[i]); }, 3900); }
  dot.addEventListener('animationstart', arm);
  dot.addEventListener('animationiteration', arm);
})();
