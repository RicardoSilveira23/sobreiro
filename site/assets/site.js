// The theme switch, the header's hairline and the details panel's switches. The page is complete without either.
(function () {
  var root = document.documentElement;
  var theme = document.querySelector('.theme');
  if (theme) {
    theme.hidden = false;
    theme.addEventListener('click', function () {
      var dark = root.dataset.theme ? root.dataset.theme === 'dark' : !matchMedia('(prefers-color-scheme: light)').matches;
      root.dataset.theme = dark ? 'light' : 'dark';
      try { localStorage.setItem('theme', root.dataset.theme); } catch (e) {}
    });
  }
  function update() { root.classList.toggle('scrolled', window.scrollY > 8); }
  update();
  addEventListener('scroll', update, { passive: true });
  // The details panel's switches: one picture matches the pressed service and style.
  var groups = document.querySelectorAll('.styles');
  var images = document.querySelectorAll('.panel img');
  function show() {
    var want = {};
    groups.forEach(function (g) {
      var on = g.querySelector('[aria-pressed="true"]');
      want[g.dataset.group] = on ? on.dataset[g.dataset.group] : null;
    });
    images.forEach(function (i) { i.hidden = i.dataset.services !== want.services || i.dataset.style !== want.style; });
  }
  groups.forEach(function (g) {
    var buttons = g.querySelectorAll('button');
    buttons.forEach(function (b) {
      b.addEventListener('click', function () {
        buttons.forEach(function (o) { o.setAttribute('aria-pressed', String(o === b)); });
        show();
      });
    });
    g.hidden = false;
  });
})();
