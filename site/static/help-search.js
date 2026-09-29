(() => {
  const input = document.getElementById('help-search');
  if (!input) return;
  const cards = [...document.querySelectorAll('.help-card')];
  const buttons = [...document.querySelectorAll('[data-filter]')];
  const count = document.getElementById('help-count');
  const empty = document.getElementById('help-empty');
  let category = 'all';
  function update() {
    const query = input.value.trim().toLocaleLowerCase();
    let visible = 0;
    for (const card of cards) {
      const matches = (category === 'all' || card.dataset.category === category)
        && (!query || card.dataset.search.includes(query));
      card.hidden = !matches;
      if (matches) visible++;
    }
    count.textContent = `${visible} ${visible === 1 ? 'article' : 'articles'}`;
    empty.hidden = visible !== 0;
  }
  input.addEventListener('input', update);
  for (const button of buttons) button.addEventListener('click', () => {
    category = button.dataset.filter;
    for (const item of buttons) item.setAttribute('aria-pressed', String(item === button));
    update();
  });
})();
