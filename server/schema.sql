-- Hopper's vote board on Cloudflare D1.
-- Apply with: npx wrangler d1 execute hopper-votes --remote --file schema.sql

CREATE TABLE IF NOT EXISTS mods (
  slug  TEXT PRIMARY KEY,            -- the mod's Modrinth slug
  name  TEXT NOT NULL,               -- its title, read from Modrinth by the Worker
  votes INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS mods_by_votes ON mods (votes DESC);

-- One vote per mod per install. The install ID is a random number made on
-- first start; nothing else about the player is stored.
CREATE TABLE IF NOT EXISTS votes (
  slug    TEXT NOT NULL REFERENCES mods (slug),
  install TEXT NOT NULL,
  at      INTEGER NOT NULL,
  PRIMARY KEY (slug, install)
);

CREATE TRIGGER IF NOT EXISTS vote_added AFTER INSERT ON votes
BEGIN UPDATE mods SET votes = votes + 1 WHERE slug = NEW.slug; END;

CREATE TRIGGER IF NOT EXISTS vote_removed AFTER DELETE ON votes
BEGIN UPDATE mods SET votes = votes - 1 WHERE slug = OLD.slug; END;

-- The per-IP rate limit. ip is a SHA-256 of the address and a secret salt,
-- never the address itself. Rows older than a day are deleted every night.
CREATE TABLE IF NOT EXISTS hits (
  ip   TEXT NOT NULL,
  hour INTEGER NOT NULL,
  n    INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (ip, hour)
);
