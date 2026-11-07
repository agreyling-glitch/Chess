// Progressively enhance the study links into a keyboard-accessible preview.
// Without JavaScript, the same links open the corresponding help guides.
(() => {
  const choices = document.querySelector('[data-study-choices]');
  const preview = document.querySelector('#study-preview');
  if (!choices || !preview) return;

  const scenes = {
    analysis: {
      title: 'Critical Analysis',
      image: '/gallery/full-analysis.png', width: 2137, height: 1297,
      alt: 'Ironwood’s annotated board beside Critical Analysis, turning-point details, move-quality bars, and the engine’s preferred line',
      caption: 'See the turning point. Compare your decisions. Keep the lesson.',
      guide: '/help/saved-analysis/', link: 'Explore analysis',
    },
    practice: {
      title: 'Training progress',
      image: '/gallery/training-progress.png', width: 999, height: 1147,
      alt: 'Ironwood training profile with rating history, results for both colors, and suggested puzzle themes',
      caption: 'Follow your progress with both colors and find themes to practice next.',
      guide: '/help/training-vs-ai/', link: 'Find your training routine',
    },
    explore: {
      title: 'Opening Explorer',
      image: '/gallery/opening-explorer.png', width: 1274, height: 855,
      alt: 'Ironwood’s board beside Opening Explorer with Masters move counts and White, draw, and Black result bars',
      caption: 'Compare common continuations. Follow an idea further.',
      guide: '/help/opening-explorer/', link: 'Explore openings',
    },
  };

  const tabs = [...choices.querySelectorAll('[data-study-scene]')].map(link => {
    const button = document.createElement('button');
    button.type = 'button';
    button.id = link.id;
    button.className = link.className;
    button.dataset.studyScene = link.dataset.studyScene;
    button.append(...link.childNodes);
    button.setAttribute('role', 'tab');
    button.setAttribute('aria-controls', preview.id);
    button.setAttribute('aria-selected', String(button.dataset.studyScene === 'analysis'));
    button.tabIndex = button.dataset.studyScene === 'analysis' ? 0 : -1;
    link.replaceWith(button);
    return button;
  });
  choices.setAttribute('role', 'tablist');
  preview.setAttribute('role', 'tabpanel');
  const horizontalLayout = window.matchMedia('(min-width: 541px) and (max-width: 780px)');
  const setOrientation = () => choices.setAttribute('aria-orientation', horizontalLayout.matches ? 'horizontal' : 'vertical');
  setOrientation();
  horizontalLayout.addEventListener('change', setOrientation);

  const image = preview.querySelector('[data-preview-image]');
  const fullSize = preview.querySelector('[data-preview-full]');
  const title = preview.querySelector('[data-preview-title]');
  const caption = preview.querySelector('[data-preview-caption]');
  const guide = preview.querySelector('[data-preview-guide]');

  function select(tab) {
    const scene = scenes[tab.dataset.studyScene];
    for (const item of tabs) {
      const selected = item === tab;
      item.classList.toggle('is-selected', selected);
      item.setAttribute('aria-selected', String(selected));
      item.tabIndex = selected ? 0 : -1;
    }
    preview.setAttribute('aria-labelledby', tab.id);
    image.src = scene.image;
    image.width = scene.width;
    image.height = scene.height;
    image.alt = scene.alt;
    fullSize.href = scene.image;
    fullSize.setAttribute('aria-label', `View ${scene.title} screenshot at full size`);
    title.textContent = scene.title;
    caption.textContent = scene.caption;
    guide.href = scene.guide;
    guide.firstChild.textContent = `${scene.link} `;
  }

  for (const tab of tabs) {
    tab.addEventListener('click', () => select(tab));
    tab.addEventListener('keydown', event => {
      const previous = horizontalLayout.matches ? 'ArrowLeft' : 'ArrowUp';
      const next = horizontalLayout.matches ? 'ArrowRight' : 'ArrowDown';
      let index = tabs.indexOf(tab);
      if (event.key === previous) index = (index + tabs.length - 1) % tabs.length;
      else if (event.key === next) index = (index + 1) % tabs.length;
      else if (event.key === 'Home') index = 0;
      else if (event.key === 'End') index = tabs.length - 1;
      else return;
      event.preventDefault();
      select(tabs[index]);
      tabs[index].focus();
    });
  }
})();
