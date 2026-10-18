// Hopper window UI. The core works out each answer; this file shows it, one
// job per screen: one mod, a whole pack, the most-wanted board and the watchlist.
(function () {
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const fmt = (n) => n.toLocaleString('en-US');
  const $ = (id) => document.getElementById(id);
  const ease = (t) => 1 - Math.pow(1 - t, 3);

  const svg = (d) => `<svg class="i" viewBox="0 0 24 24">${d}</svg>`;
  const icon = {
    min: '<svg viewBox="0 0 10 10"><path d="M0 5.5h10"/></svg>',
    max: '<svg viewBox="0 0 10 10"><rect x="0.5" y="0.5" width="9" height="9"/></svg>',
    close: '<svg viewBox="0 0 10 10"><path d="M0 0l10 10M10 0L0 10"/></svg>',
    search: svg('<circle cx="11" cy="11" r="6.5"/><path d="M20 20l-4.2-4.2"/>'),
    drop: svg('<path d="M12 3.5v11M7.5 10l4.5 4.5 4.5-4.5"/><path d="M4 14.5v2.5a3 3 0 0 0 3 3h10a3 3 0 0 0 3-3v-2.5"/>'),
    folder: svg('<path d="M3.5 7.5a2 2 0 0 1 2-2h3.8l2 2.2h7.2a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z"/>'),
    check: svg('<path d="M5 12.5l4.5 4.5L19 7.5"/>'),
    bell: svg('<path d="M6.5 16.5V11a5.5 5.5 0 0 1 11 0v5.5l1.5 1.5h-14z"/><path d="M10 20.5a2 2 0 0 0 4 0"/>'),
    out: svg('<path d="M14 5h5v5M19 5l-8 8"/><path d="M17 14v4a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V9a2 2 0 0 1 2-2h4"/>'),
    chev: svg('<path d="M7 10l5 5 5-5"/>'),
    plus: svg('<path d="M12 5v14M5 12h14"/>'),
    x: svg('<path d="M6.5 6.5l11 11M17.5 6.5l-11 11"/>'),
    install: svg('<path d="M12 4v11M7.5 10.5L12 15l4.5-4.5M5 19.5h14"/>'),
  };

  // The mark: a hopper with a small block dropping in. Same drawing as the app icon.
  const HOPPER = '<rect x="27" y="8.5" width="10" height="10" rx="2" fill="#7a5af8" transform="rotate(14 32 13.5)"/>'
    + '<rect x="11" y="23.5" width="42" height="9.5" rx="2.6" fill="#2b59ff"/><rect x="16" y="26.5" width="32" height="3.2" rx="1.6" fill="#1a3ab8"/>'
    + '<path d="M15.5 35.5h33L39 46.5H25z" fill="#2b59ff" stroke="#2b59ff" stroke-width="1.6" stroke-linejoin="round"/>'
    + '<rect x="27.5" y="46" width="9" height="8.5" rx="1.6" fill="#2b59ff"/>';
  const MARK = `<svg class="mark" viewBox="0 0 64 64"><path d="M32 2.5C54.5 2.5 61.5 9.5 61.5 32S54.5 61.5 32 61.5 2.5 54.5 2.5 32 9.5 2.5 32 2.5z" fill="#fff" stroke="rgba(17,19,24,.12)"/>${HOPPER}</svg>`;

  const LABEL = { builtin: 'Built in', ported: 'Ported', works: 'Works', notyet: 'Not yet' };

  // Each mod's icon, as the core hands them over: cached files from Modrinth's
  // CDN, keyed by project slug, with px set for pixel art. Until an icon has
  // loaded, and for mods without one, the tile shows a monogram.
  const ICONS = window.hopperIcons || {};
  Object.values(ICONS).forEach((i) => { new Image().src = i.src; });

  function tile(name, size = '', slug = '') {
    const words = name.replace(/['’]/g, '').split(/\s+/).filter((w) => /^[A-Za-z]/.test(w) && !/^(and|of|the)$/i.test(w));
    const letters = words.length > 1 ? words[0][0] + words[1][0] : words[0].slice(0, 2);
    const icon = ICONS[slug];
    const img = icon ? `<img src="${icon.src}" alt="" class="${icon.px ? 'px' : ''}">` : '';
    return `<span class="tile ${size}"><span class="m">${letters[0].toUpperCase()}${letters.slice(1)}</span>${img}</span>`;
  }

  // Swaps a monogram for its icon once the icon has loaded; a broken one is dropped.
  function light(root = document) {
    root.querySelectorAll('.tile img').forEach((img) => {
      const on = () => img.parentNode.classList.add('lit');
      if (img.complete && img.naturalWidth) on();
      else {
        img.addEventListener('load', on, { once: true });
        img.addEventListener('error', () => img.remove(), { once: true });
      }
    });
  }

  // Answers, as the core returns them for these mods.
  const MODS = {
    sodium: {
      name: 'Sodium', slug: 'sodium', kind: 'builtin', meta: 'Performance',
      big: 'Horizon is already this fast.',
      why: 'You don’t need it. What Sodium did is part of Horizon’s own engine.',
      details: [['Found by', 'Its mod id, <code>sodium</code>'], ['Why', 'Horizon’s docs list Sodium with the speed mods Horizon replaces.']],
    },
    'distant horizons': {
      name: 'Distant Horizons', kind: 'builtin', meta: 'View distance', button: 'How to turn it on',
      big: 'Horizon does this itself.',
      why: 'Far terrain is built in. Turn up LOD Distance and you can see up to 1024 chunks.',
      details: [['How', 'Options, then Horizon Settings, then Graphics. Raise LOD Distance.'], ['Why', 'Horizon’s README: distant terrain up to 1024 chunks.']],
    },
    create: {
      name: 'Create', slug: 'create', kind: 'notyet', meta: 'Machines and contraptions', votes: 1240,
      why: 'Nobody has made a Horizon version yet. Your vote tells porters what to make next.',
      details: [['Horizon’s check', 'Its machines need Java code, which Horizon doesn’t run.'], ['Votes', 'One per install. The count is close, not exact.']],
    },
    'applied energistics 2': { name: 'Applied Energistics 2', slug: 'ae2', kind: 'notyet', meta: 'Storage networks', votes: 860, why: 'Nobody has made a Horizon version yet. Your vote tells porters what to make next.', details: [] },
    "farmer's delight": { name: 'Farmer’s Delight', slug: 'farmers-delight', kind: 'notyet', meta: 'Cooking and farming', votes: 590, why: 'Nobody has made a Horizon version yet. Your vote tells porters what to make next.', details: [] },
    veinminer: {
      name: 'Veinminer', slug: 'veinminer', kind: 'ported', meta: 'Mining',
      big: 'There’s a Horizon version.',
      why: 'It’s on Horizon’s own list of mods, checked by its team. It installs in one click.',
      details: [['File', '<code>veinminer-2.1.0.hnmod</code>, signed by its author'], ['Checked', 'Size, fingerprint and signature, before it’s installed']],
    },
    waystones: {
      name: 'Waystones', slug: 'waystones', kind: 'ported', meta: 'Fast travel',
      big: 'There’s a Horizon version.',
      why: 'It arrived this week. It’s on Horizon’s own list of mods and installs in one click.',
      details: [],
    },
    "macaw's fences and walls": {
      name: 'Macaw’s Fences and Walls', slug: 'macaws-fences-and-walls', kind: 'works', meta: 'Decoration',
      big: 'Its blocks come over.',
      why: 'Horizon’s own check says its fences, walls and recipes work as they are.',
      details: [['Horizon’s check', 'Works. Blocks, items and recipes come over.'], ['Left out', 'Nothing']],
    },
  };

  const PACK = { name: 'Our SMP pack', total: 60, works: 24, ported: 6, builtin: 14, notyet: 16 };
  const PACK_LIST = [
    ['30 come with you', 'Veinminer, Waystones, Macaw’s Fences and Walls, Macaw’s Roofs, Handcrafted and 25 more'],
    ['14 you don’t need anymore', 'Sodium, Lithium, FerriteCore, JEI, Xaero’s Minimap, Mouse Tweaks and 8 more'],
    ['16 not yet', 'Create, Applied Energistics 2, Farmer’s Delight, Sophisticated Backpacks and 12 more'],
  ];

  const WANTED = [
    ['Create', 'create', 'Machines and contraptions', 1240],
    ['Applied Energistics 2', 'ae2', 'Storage networks', 860, 'mine'],
    ['Waystones', 'waystones', 'Fast travel', 615, 'here'],
    ['Farmer’s Delight', 'farmers-delight', 'Cooking and farming', 590, 'mine'],
    ['Sophisticated Backpacks', 'sophisticated-backpacks', 'Backpacks with upgrades', 470],
    ['Tinkers’ Construct', 'tinkers-construct', 'Tools you build yourself', 455],
    ['Iron’s Spells ’n Spellbooks', 'irons-spells-n-spellbooks', 'Spells and magic gear', 402],
    ['Botania', 'botania', 'Flower magic', 368],
    ['Mekanism', 'mekanism', 'Machines and power', 331],
    ['Supplementaries', 'supplementaries', 'Decorations and gadgets', 287],
  ];

  const WATCHING = [
    ['Create', 'create', 1240, '12 Oct'],
    ['Applied Energistics 2', 'ae2', 860, '12 Oct'],
    ['Farmer’s Delight', 'farmers-delight', 590, '19 Oct'],
  ];

  // Shell -------------------------------------------------------------------
  function shell() {
    return `
    <div class="win">
      <header class="titlebar">
        <div class="brand">${MARK}<span>Hopper</span></div>
        <nav class="tabs" id="tabs">
          <button data-tab="check">Check</button>
          <button data-tab="wanted">Most wanted</button>
          <button data-tab="watching">Watching <em>1</em></button>
        </nav>
        <div class="winctl"><span>${icon.min}</span><span>${icon.max}</span><span>${icon.close}</span></div>
      </header>
      <main class="view" id="view"></main>
      <div class="toast" id="toast"><span class="dot"></span><span><b>Waystones</b> is on Horizon now</span><button class="primary">${icon.install}<span>Install</span></button></div>
    </div>`;
  }

  function checkView() {
    return `
    <section class="check" id="check">
      <div class="ask">
        ${MARK}
        <h1>Is my mod on Horizon?</h1>
        <label class="search" id="search">${icon.search}<input id="q" placeholder="Type a mod’s name" autocomplete="off" spellcheck="false"></label>
        <p class="hint">${icon.drop}or drop a jar, a mods folder or a modpack</p>
      </div>
      <div class="answer" id="answer"></div>
      <p class="smelt" id="smelt"><a class="quiet" href="https://github.com/UpperSupervisor/smelt">Port it yourself with Smelt ${icon.out}</a></p>
      <div class="pack" id="pack"></div>
      <div class="drop" id="drop"><div class="ring">${icon.drop}</div><b>Drop to check the whole pack</b><span>A jar, a mods folder or a modpack</span></div>
    </section>`;
  }

  function answerCard(m) {
    const big = m.kind === 'notyet' ? `<span class="count" id="count">${fmt(m.votes)}</span> players want it.` : m.big;
    const acts = {
      builtin: m.button ? `<button class="primary" id="act">${m.button}</button>` : '<button class="ghost" id="again">Check another mod</button>',
      ported: `<button class="primary" id="act">${icon.install}<span>Install</span></button>`,
      works: `<button class="primary" id="act">${icon.plus}<span>Add to Horizon</span></button>`,
      notyet: `<button class="primary" id="want">Want it too</button><button class="ghost" id="watch">${icon.bell}<span>Tell me when it’s here</span></button>`,
    }[m.kind];
    return `
      <div class="a-head">${tile(m.name, 'big', m.slug)}<div class="who"><b>${m.name}</b><small>${m.meta}</small></div><span class="pill ${m.kind}">${LABEL[m.kind]}</span></div>
      <p class="a-big">${big}</p>
      <p class="a-why">${m.why}</p>
      <div class="a-acts">${acts}${m.details.length ? `<button class="quiet" id="more">Show details ${icon.chev}</button>` : ''}</div>
      <dl class="a-more" id="a-more">${m.details.map(([k, v]) => `<div><dt>${k}</dt><dd>${v}</dd></div>`).join('')}</dl>`;
  }

  function packView() {
    const P = PACK;
    const w = (n) => `${((n / P.total) * 100).toFixed(2)}%`;
    return `
      <div class="p-head"><span class="folder">${icon.folder}</span><div class="who"><b>${P.name}</b><small>${P.total} mods</small></div>
        <button class="quiet" id="again2">${icon.x}Check something else</button></div>
      <div class="nums">
        <div class="num"><b data-to="${P.works + P.ported}">0</b><span>come with you</span><div class="bar"><i class="works" data-w="${w(P.works)}"></i><i class="ported" data-w="${w(P.ported)}"></i></div></div>
        <div class="num"><b data-to="${P.builtin}">0</b><span>you don’t need anymore</span><div class="bar"><i class="builtin" data-w="${w(P.builtin)}"></i></div></div>
        <div class="num"><b data-to="${P.notyet}">0</b><span>not yet</span><div class="bar"><i class="notyet" data-w="${w(P.notyet)}"></i></div></div>
      </div>
      <div class="p-reading"><div class="prog"><i id="pprog"></i></div><span id="pstat">Checking ${P.total} mods</span></div>
      <p class="p-say later"><b>${P.works} work as they are and ${P.ported} have a Horizon version.</b> Horizon already does what the other ${P.builtin} did.</p>
      <div class="p-acts later">
        <button class="primary" id="add"><i class="fillbar" id="fill"></i>${icon.plus}<span id="addtxt">Add the ${P.works + P.ported} to Horizon</span></button>
        <span class="p-done"><i>${icon.check}</i>${P.works + P.ported} added. Start Horizon and they’re there.</span>
        <button class="quiet" id="pmore">Show details ${icon.chev}</button>
      </div>
      <div class="p-list" id="plist">${PACK_LIST.map(([h, t]) => `<p><b>${h}</b>${t}</p>`).join('')}</div>`;
  }

  function wantedView() {
    const top = WANTED[0][3];
    return `
    <section class="page" id="page">
      <div class="page-head"><div><h1>Most wanted</h1><p>The mods Horizon players miss most. Your votes are marked.</p></div>
        <a class="quiet" href="https://github.com/UpperSupervisor/smelt">Port one with Smelt ${icon.out}</a></div>
      <div class="board">${WANTED.map(([name, slug, what, votes, tag], i) => {
        const act = tag === 'here' ? '<span class="pill ported">On Horizon now</span>'
          : tag === 'mine' ? `<button class="soft mine">${icon.check}You want it</button>`
          : '<button class="ghost want">Want it too</button>';
        return `<div class="row ${tag || ''}" style="--share:${votes / top}"><span class="rank">${i + 1}</span>${tile(name, '', slug)}<div class="who"><b>${name}</b><small>${what}</small></div>
          <div class="votes"><b>${fmt(votes)}</b><small>players</small></div><div class="act">${act}</div></div>`;
      }).join('')}</div>
      <div style="height:48px"></div>
    </section>
    <div class="fade"></div>`;
  }

  function watchingView() {
    return `
    <section class="page">
      <div class="page-head"><div><h1>Watching</h1><p>Hopper looks for these each time it starts, and tells you when one arrives.</p></div></div>
      <div class="arrived">${tile('Waystones', 'big', 'waystones')}<div class="who"><span class="new">Arrived this week</span><b>Waystones is on Horizon now</b><span>Someone made a Horizon version. It installs in one click.</span></div>
        <button class="primary" id="install">${icon.install}<span>Install</span></button></div>
      <p class="sub">Still waiting</p>
      <div class="board small">${WATCHING.map(([name, slug, votes, since]) => `
        <div class="row">${tile(name, '', slug)}<div class="who"><b>${name}</b><small>${fmt(votes)} players want it</small></div>
          <span class="pill notyet">Not yet</span><span class="since">Watching since ${since}</span></div>`).join('')}</div>
    </section>`;
  }

  // State -------------------------------------------------------------------
  const root = document.getElementById('app');
  root.innerHTML = shell();
  let current = null;

  function tab(name) {
    document.querySelectorAll('#tabs button').forEach((b) => b.classList.toggle('on', b.dataset.tab === name));
  }

  function show(name) {
    tab(name);
    $('view').innerHTML = { check: checkView, wanted: wantedView, watching: watchingView }[name]();
    light($('view'));
    current = null;
    if (name === 'check') wireCheck();
  }

  function find(text) {
    const q = text.trim().toLowerCase().replace(/[’]/g, "'");
    if (q.length < 3) return null;
    const key = Object.keys(MODS).find((k) => k.startsWith(q) || k.replace(/'/g, '').startsWith(q));
    return key ? MODS[key] : null;
  }

  async function answer(m) {
    const card = $('answer');
    const check = $('check');
    if (!m) {
      card.classList.remove('on');
      check.classList.remove('answered');
      $('smelt').classList.remove('on');
      current = null;
      return;
    }
    if (current === m) return;
    if (current) {
      card.classList.remove('on');
      $('smelt').classList.remove('on');
      await sleep(180);
    }
    current = m;
    card.innerHTML = answerCard(m);
    light(card);
    check.classList.add('answered');
    card.getBoundingClientRect();
    card.classList.add('on');
    if (m.kind === 'notyet') $('smelt').classList.add('on');
    wireAnswer(m);
  }

  function wireAnswer(m) {
    $('more')?.addEventListener('click', () => $('a-more').classList.toggle('on'));
    $('again')?.addEventListener('click', () => hopper.idle());
    $('want')?.addEventListener('click', () => hopper.want());
    $('watch')?.addEventListener('click', () => hopper.watch());
    if (m.button) $('act')?.addEventListener('click', () => $('a-more').classList.add('on'));
  }

  function wireCheck() {
    const q = $('q');
    let t;
    q.addEventListener('focus', () => $('search').classList.add('focus'));
    q.addEventListener('blur', () => $('search').classList.remove('focus'));
    q.addEventListener('input', () => {
      clearTimeout(t);
      t = setTimeout(() => answer(find(q.value)), 220);
    });
  }

  async function countUp(el, to, ms) {
    const t0 = performance.now();
    await new Promise((done) => {
      (function step(now) {
        const t = Math.min(1, (now - t0) / ms);
        el.textContent = Math.round(ease(t) * to);
        if (t < 1) requestAnimationFrame(step); else done();
      })(t0);
    });
  }

  async function progress(ms, fn) {
    const t0 = performance.now();
    await new Promise((done) => {
      (function step(now) {
        const t = Math.min(1, (now - t0) / ms);
        fn(t);
        if (t < 1) requestAnimationFrame(step); else done();
      })(t0);
    });
  }

  function wirePack() {
    $('again2').addEventListener('click', () => hopper.idle());
    $('pmore').addEventListener('click', () => $('plist').classList.toggle('on'));
    $('add').addEventListener('click', () => hopper.add());
  }

  document.addEventListener('click', (e) => {
    const b = e.target.closest('#tabs button');
    if (b) show(b.dataset.tab);
    const w = e.target.closest('.row .want');
    if (w) {
      const row = w.closest('.row');
      const n = row.querySelector('.votes b');
      n.textContent = fmt(+n.textContent.replace(/,/g, '') + 1);
      row.classList.add('mine');
      w.outerHTML = `<button class="soft mine">${icon.check}You want it</button>`;
    }
  });
  window.addEventListener('dragover', (e) => { e.preventDefault(); hopper.dragOver(true); });
  window.addEventListener('dragleave', (e) => { if (!e.relatedTarget) hopper.dragOver(false); });
  window.addEventListener('drop', (e) => { e.preventDefault(); hopper.dragOver(false); hopper.pack({ animate: true }); });

  // Driven by the core in the app, and by the capture scripts for the README.
  const hopper = {
    idle() {
      show('check');
    },
    focus(on = true) {
      $('search').classList.toggle('focus', on);
    },
    // Types into the search box one key at a time.
    async type(text, ms = 80) {
      const q = $('q');
      this.focus(true);
      q.value = '';
      for (const ch of text) {
        q.value += ch;
        await sleep(ms);
      }
    },
    // Empties the box for the next mod, keeping the layout where it is.
    async clear() {
      $('q').value = '';
      $('answer').classList.remove('on');
      $('smelt').classList.remove('on');
      current = null;
      await sleep(200);
    },
    answer(key) {
      return answer(MODS[key]);
    },
    press(sel) {
      const b = document.querySelector(sel);
      b.classList.add('pressed');
      return sleep(150).then(() => b.classList.remove('pressed'));
    },
    // Want it too: the count ticks up by one and the vote is marked as yours.
    want() {
      const b = $('want');
      if (!b || b.classList.contains('mine')) return;
      b.className = 'soft mine';
      b.innerHTML = `${icon.check}You want it`;
      const c = $('count');
      c.textContent = fmt(current.votes + 1);
      c.classList.add('tick');
    },
    watch() {
      const b = $('watch');
      b.innerHTML = `${icon.check}<span>Watching</span>`;
    },
    details(on = true) {
      $('a-more')?.classList.toggle('on', on);
      $('plist')?.classList.toggle('on', on);
    },
    dragOver(on) {
      $('drop')?.classList.toggle('on', on);
    },
    async pack({ animate = false } = {}) {
      if (!$('check')) show('check');
      $('check').classList.add('packed');
      $('answer').classList.remove('on');
      $('smelt').classList.remove('on');
      current = null;
      const pack = $('pack');
      pack.innerHTML = packView();
      pack.classList.add('on');
      wirePack();
      const bars = [...pack.querySelectorAll('.bar i')];
      const nums = [...pack.querySelectorAll('.num b')];
      if (!animate) {
        bars.forEach((b) => { b.style.transition = 'none'; b.style.width = b.dataset.w; });
        nums.forEach((n) => { n.textContent = n.dataset.to; });
        pack.classList.add('ready');
        return;
      }
      await progress(1300, (t) => {
        $('pprog').style.width = `${Math.round(t * 100)}%`;
        $('pstat').textContent = `Checking ${Math.max(1, Math.round(t * PACK.total))} of ${PACK.total} mods`;
      });
      bars.forEach((b) => { b.style.width = b.dataset.w; });
      await Promise.all(nums.map((n) => countUp(n, +n.dataset.to, 900)));
      pack.classList.add('ready');
    },
    async add(ms = 1600) {
      const n = PACK.works + PACK.ported;
      await progress(ms, (t) => {
        $('fill').style.width = `${t * 100}%`;
        $('addtxt').textContent = `Adding ${Math.max(1, Math.round(t * n))} of ${n}…`;
      });
      await sleep(120);
      $('pack').classList.add('added');
    },
    wanted() {
      show('wanted');
    },
    watching() {
      show('watching');
    },
    toast(on = true) {
      $('toast').classList.toggle('on', on);
    },
  };
  window.hopper = hopper;
  hopper.idle();
})();
