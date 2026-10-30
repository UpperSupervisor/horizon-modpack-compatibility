// Hopper's vote board: a Cloudflare Worker on D1.
//
//   POST   /v1/votes/:slug             want it too       body: { "install": "<32 hex>" }
//   DELETE /v1/votes/:slug             take the vote back
//   GET    /v1/top                     the 10 most wanted mods
//   GET    /v1/votes?slugs=create,ae2  counts for up to 50 mods
//
// One vote per mod per install, plus a per-IP limit. Counts are approximate
// on purpose: an install ID is only a random number, and nobody signs in.

const HOURLY_LIMIT = 30;
const SLUG = /^[a-z0-9][a-z0-9_.+-]{1,63}$/;
const INSTALL = /^[0-9a-f]{32}$/;

const json = (body, status = 200, headers = {}) =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json', ...headers } });

export default {
  async fetch(req, env, ctx) {
    const url = new URL(req.url);
    try {
      if (req.method === 'GET' && url.pathname === '/v1/top') return await top(req, env, ctx);
      if (req.method === 'GET' && url.pathname === '/v1/votes') return await counts(url, env);
      const m = url.pathname.match(/^\/v1\/votes\/([^/]+)$/);
      if (m && (req.method === 'POST' || req.method === 'DELETE')) return await vote(req, env, decodeURIComponent(m[1]));
      return json({ error: 'not found' }, 404);
    } catch (e) {
      console.error(e);
      return json({ error: 'something went wrong, try again soon' }, 500);
    }
  },

  async scheduled(_event, env) {
    await env.DB.prepare('DELETE FROM hits WHERE hour < ?').bind(hourNow() - 24).run();
  },
};

async function vote(req, env, slug) {
  if (!SLUG.test(slug)) return json({ error: 'unknown mod' }, 400);
  const { install } = await req.json().catch(() => ({}));
  if (!INSTALL.test(install || '')) return json({ error: 'bad install id' }, 400);
  if (!(await allowed(req, env))) return json({ error: 'too many votes from here, try again in an hour' }, 429);

  if (req.method === 'DELETE') {
    await env.DB.prepare('DELETE FROM votes WHERE slug = ? AND install = ?').bind(slug, install).run();
    return json({ slug, votes: await votesFor(env, slug), mine: false });
  }

  // The first vote for a mod names it from Modrinth, so the board only lists
  // real mods, under their real names.
  if (!(await env.DB.prepare('SELECT 1 FROM mods WHERE slug = ?').bind(slug).first())) {
    const res = await fetch(`https://api.modrinth.com/v2/project/${encodeURIComponent(slug)}`, {
      headers: { 'user-agent': env.USER_AGENT },
    });
    if (!res.ok) return json({ error: 'not a Modrinth project' }, 404);
    const p = await res.json();
    if (p.project_type !== 'mod' || p.slug !== slug) return json({ error: 'use the mod\'s Modrinth slug' }, 400);
    await env.DB.prepare('INSERT OR IGNORE INTO mods (slug, name) VALUES (?, ?)').bind(slug, String(p.title).slice(0, 80)).run();
  }
  // A second vote from the same install is ignored, so the trigger doesn't count it.
  await env.DB.prepare('INSERT OR IGNORE INTO votes (slug, install, at) VALUES (?, ?, ?)').bind(slug, install, Date.now()).run();
  return json({ slug, votes: await votesFor(env, slug), mine: true });
}

async function top(req, env, ctx) {
  // Cached at the edge for a minute, so opening the board costs almost no reads.
  const cache = caches.default;
  const hit = await cache.match(req);
  if (hit) return hit;
  const { results } = await env.DB.prepare(
    'SELECT slug, name, votes FROM mods WHERE votes > 0 ORDER BY votes DESC, name LIMIT 10',
  ).all();
  const res = json({ updated: new Date().toISOString(), mods: results }, 200, { 'cache-control': 'public, max-age=60' });
  ctx.waitUntil(cache.put(req, res.clone()));
  return res;
}

async function counts(url, env) {
  const slugs = [...new Set((url.searchParams.get('slugs') || '').split(','))].filter((s) => SLUG.test(s)).slice(0, 50);
  if (!slugs.length) return json({ mods: [] });
  const { results } = await env.DB.prepare(`SELECT slug, name, votes FROM mods WHERE slug IN (${slugs.map(() => '?').join(',')})`)
    .bind(...slugs)
    .all();
  return json({ mods: results });
}

async function votesFor(env, slug) {
  const row = await env.DB.prepare('SELECT votes FROM mods WHERE slug = ?').bind(slug).first();
  return row ? row.votes : 0;
}

// At most HOURLY_LIMIT votes an hour from one address. Only a salted hash of the address is kept.
async function allowed(req, env) {
  const ip = req.headers.get('cf-connecting-ip') || '';
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(`${env.SALT}|${ip}`));
  const key = [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, '0')).join('');
  const row = await env.DB.prepare(
    'INSERT INTO hits (ip, hour, n) VALUES (?, ?, 1) ON CONFLICT (ip, hour) DO UPDATE SET n = n + 1 RETURNING n',
  ).bind(key, hourNow()).first();
  return row.n <= HOURLY_LIMIT;
}

function hourNow() {
  return Math.floor(Date.now() / 3_600_000);
}
