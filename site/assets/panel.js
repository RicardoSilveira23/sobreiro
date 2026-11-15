// The details panel's switches: the one picture that matches the pressed service and style shows.
(function () {
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
