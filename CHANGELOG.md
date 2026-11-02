# Changelog


## 1.3.7 — 3 November 2026

A maintenance release.

### Fixes

- Mod scan handles a missing mods folder without an error dialog.

## 1.3.6 · 2026-11-01

- Mrpack imports finish even when a file entry has no hash.
- Instance picker remembers the last choice.
- The three numbers update live while a pack is scanning.
- Neoforge metadata parses again after a format tweak.

## 1.3.4 · 2026-10-09

- Watched mods are rechecked on every start, and arrivals say which version landed.
- Prism and MultiMC instances answer in one drop, with the three numbers up front.

## 1.3.5 · 2026-10-30

- **Is my mod on Horizon?** Type a mod's name or drop its jar and get one answer: built in, ported, works or not yet, with one button.
- **Bring your whole pack.** Mods folders, Prism and MultiMC instances, Modrinth `.mrpack` files and CurseForge zips. Three numbers, and one button that adds everything that comes with you.
- **Most wanted.** The 10 mods players miss most, with vote counts. One vote per mod per install, no account.
- **Watching.** On start, Hopper tells you when a watched mod has arrived in Horizon's registry.
- Mod icons from Modrinth's CDN, cached on your PC, with the mod's initials when there's no icon.
- Built-in rules for 13 groups of Java mods, each linked to the Horizon doc that backs it.
- Registry files are checked against their SHA-256 and, when signed, their minisign signature. Revoked files are turned off with the reason.
- Portable builds for Windows (`.exe`) and Linux (AppImage).
