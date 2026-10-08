//! Answers "Is my mod on Horizon?" for one mod or a whole pack.
//!
//! Every answer comes from one of four places, checked in this order:
//!
//! 1. **Built in.** `rules/builtin.toml`: Horizon already does what the mod did.
//! 2. **Ported.** A Rust version is in Horizon's registry ([`INDEX_URL`]), found
//!    through `rules/ports.toml`. A revoked file never counts.
//! 3. **Works.** Horizon's launcher checked the jar
//!    (`horizon-launcher mods check <slug> --json`) and its content carries over.
//! 4. **Not yet.** Anything else.
//!
//! Each mod's tile shows its icon from Modrinth's CDN, cached on the PC, and
//! falls back to a two-letter monogram ([`Tile`]).

use serde::Deserialize;
use std::fmt;

/// Horizon's registry index, built by horizon-rust/horizon-mods.
pub const INDEX_URL: &str = "https://horizon-rust.github.io/horizon-mods/index.json";

/// The game version Hopper answers for.
pub const GAME: &str = "26.3";

/// Sent with every Modrinth request, as Modrinth asks.
pub const USER_AGENT: &str = concat!(
    "UpperSupervisor/hopper/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/UpperSupervisor/horizon-modpack-compatibility)"
);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A jar's `fabric.mod.json` or `mods.toml` couldn't be read.
    Manifest(String),
    /// The file isn't a mod manifest Hopper knows.
    NotAManifest,
    Rules(String),
    Index(String),
    /// The registry uses a newer index format than this Hopper reads.
    NewerIndex(u32),
    Pack(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Manifest(e) => write!(f, "couldn't read the mod's manifest: {e}"),
            Error::NotAManifest => write!(f, "not a Fabric, NeoForge or Forge manifest"),
            Error::Rules(e) => write!(f, "rules: {e}"),
            Error::Index(e) => write!(f, "registry index: {e}"),
            Error::NewerIndex(n) => write!(f, "the registry index is schema {n}: update Hopper"),
            Error::Pack(e) => write!(f, "couldn't read the pack: {e}"),
        }
    }
}

impl std::error::Error for Error {}

// Mods ------------------------------------------------------------------------

/// Turns a mod id, slug or name into the form the rules use: lowercase, with
/// `-`, `.` and spaces as `_`. `fabric-api` becomes `fabric_api`.
pub fn normalize_id(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for c in raw.trim().chars() {
        match c {
            'a'..='z' | '0'..='9' | '_' => out.push(c),
            'A'..='Z' => out.push(c.to_ascii_lowercase()),
            '-' | '.' | ' ' => out.push('_'),
            _ => {}
        }
    }
    out.trim_matches('_').to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Loader {
    Fabric,
    NeoForge,
    Forge,
}

/// A mod as its jar describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JarMod {
    /// Normalized with [`normalize_id`].
    pub id: String,
    pub name: String,
    /// `None` when the jar leaves it to the build, as in `${file.jarVersion}`.
    pub version: Option<String>,
    pub loader: Loader,
}

/// The manifests Hopper reads, by their path inside the jar.
pub const MANIFESTS: [(&str, Loader); 3] = [
    ("fabric.mod.json", Loader::Fabric),
    ("META-INF/neoforge.mods.toml", Loader::NeoForge),
    ("META-INF/mods.toml", Loader::Forge),
];

/// Reads a mod from the text of its manifest. `path` is where the file sits in
/// the jar; the app unzips, so this works on plain strings.
pub fn read_manifest(path: &str, text: &str) -> Result<JarMod, Error> {
    let text = text.trim_start_matches('\u{feff}');
    match MANIFESTS.iter().find(|(p, _)| *p == path) {
        Some((_, Loader::Fabric)) => read_fabric(text),
        Some((_, loader)) => read_mods_toml(text, *loader),
        None => Err(Error::NotAManifest),
    }
}

fn known_version(v: Option<String>) -> Option<String> {
    v.filter(|v| !v.trim().is_empty() && !v.contains("${"))
}

fn read_fabric(text: &str) -> Result<JarMod, Error> {
    #[derive(Deserialize)]
    struct Fabric {
        id: String,
        name: Option<String>,
        version: Option<String>,
    }
    let f: Fabric = serde_json::from_str(text).map_err(|e| Error::Manifest(e.to_string()))?;
    Ok(JarMod {
        id: normalize_id(&f.id),
        name: f.name.unwrap_or_else(|| f.id.clone()),
        version: known_version(f.version),
        loader: Loader::Fabric,
    })
}

fn read_mods_toml(text: &str, loader: Loader) -> Result<JarMod, Error> {
    #[derive(Deserialize)]
    struct ModsToml {
        #[serde(default)]
        mods: Vec<Entry>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Entry {
        mod_id: String,
        version: Option<String>,
        display_name: Option<String>,
    }
    let t: ModsToml = toml::from_str(text).map_err(|e| Error::Manifest(e.to_string()))?;
    // A jar may hold several mods; the first is the one it's named after.
    let first = t
        .mods
        .into_iter()
        .next()
        .ok_or(Error::Manifest("no [[mods]] entry".into()))?;
    Ok(JarMod {
        id: normalize_id(&first.mod_id),
        name: first.display_name.unwrap_or_else(|| first.mod_id.clone()),
        version: known_version(first.version),
        loader,
    })
}

// Rules -----------------------------------------------------------------------

/// How a player gets Horizon's own version of a built-in mod.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// Part of the engine. Nothing to turn on.
    Engine,
    /// Off by default, one switch in Horizon's settings.
    Setting,
    /// Installs from the launcher's Mods page.
    OfficialMod,
}

/// A row of `rules/builtin.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Builtin {
    pub mods: Vec<String>,
    #[serde(default)]
    pub words: Vec<String>,
    pub horizon: String,
    pub kind: Kind,
    pub how: String,
    pub source: String,
}

/// A row of `rules/ports.toml`: the registry project that ports a Java mod.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Port {
    pub java: String,
    pub horizon: String,
    pub by: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rules {
    pub builtin: Vec<Builtin>,
    pub ports: Vec<Port>,
}

impl Rules {
    pub fn parse(builtin_toml: &str, ports_toml: &str) -> Result<Rules, Error> {
        #[derive(Deserialize)]
        struct B {
            #[serde(default)]
            builtin: Vec<Builtin>,
        }
        #[derive(Deserialize)]
        struct P {
            #[serde(default)]
            port: Vec<Port>,
        }
        let b: B =
            toml::from_str(builtin_toml).map_err(|e| Error::Rules(format!("builtin.toml: {e}")))?;
        let p: P =
            toml::from_str(ports_toml).map_err(|e| Error::Rules(format!("ports.toml: {e}")))?;
        let ids = b
            .builtin
            .iter()
            .flat_map(|r| &r.mods)
            .chain(p.port.iter().map(|r| &r.java));
        for id in ids {
            if *id != normalize_id(id) {
                return Err(Error::Rules(format!(
                    "write \"{id}\" as \"{}\"",
                    normalize_id(id)
                )));
            }
        }
        if let Some(row) = b.builtin.iter().find(|r| !r.source.starts_with("https://")) {
            return Err(Error::Rules(format!("{} needs a source link", row.horizon)));
        }
        Ok(Rules {
            builtin: b.builtin,
            ports: p.port,
        })
    }

    /// What Horizon has in place of this mod: by id first, then by words in its name.
    pub fn builtin_for(&self, id: &str, name: &str) -> Option<&Builtin> {
        let id = normalize_id(id);
        let name = name.to_lowercase();
        self.builtin
            .iter()
            .find(|b| b.mods.contains(&id))
            .or_else(|| {
                self.builtin
                    .iter()
                    .find(|b| b.words.iter().any(|w| name.contains(w.as_str())))
            })
    }

    pub fn port_for(&self, id: &str) -> Option<&Port> {
        let id = normalize_id(id);
        self.ports.iter().find(|p| p.java == id)
    }
}

// Horizon's registry -------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Index {
    pub schema: u32,
    pub projects: Vec<Project>,
    #[serde(default)]
    pub revoked: Vec<Revoked>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Project {
    pub id: String,
    pub kind: String,
    pub name: String,
    /// The author's minisign public key. When set, every file must be signed with it.
    pub key: Option<String>,
    /// Newest first.
    pub versions: Vec<Version>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Version {
    pub version: String,
    pub game: String,
    pub file: File,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct File {
    pub name: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
    pub signature: Option<String>,
}

/// A file taken down: one file by its hash, or every version when there's no hash.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Revoked {
    pub id: String,
    pub sha256: Option<String>,
    pub reason: String,
}

impl Index {
    pub fn parse(json: &str) -> Result<Index, Error> {
        // Read the schema first, so a newer index is refused instead of misread.
        #[derive(Deserialize)]
        struct Head {
            schema: u32,
        }
        let head: Head = serde_json::from_str(json).map_err(|e| Error::Index(e.to_string()))?;
        if head.schema > 1 {
            return Err(Error::NewerIndex(head.schema));
        }
        serde_json::from_str(json).map_err(|e| Error::Index(e.to_string()))
    }

    /// Why a file was taken down, if it was. Installed copies are turned off with this reason.
    pub fn revoked(&self, id: &str, sha256: &str) -> Option<&str> {
        self.revoked
            .iter()
            .find(|r| {
                r.id == id
                    && r.sha256
                        .as_deref()
                        .is_none_or(|h| h.eq_ignore_ascii_case(sha256))
            })
            .map(|r| r.reason.as_str())
    }

    /// The newest version of a mod that may be installed: a content or native
    /// mod, made for `game`, and not revoked.
    pub fn installable(&self, id: &str, game: &str) -> Option<(&Project, &Version)> {
        let project = self
            .projects
            .iter()
            .find(|p| p.id == id && install_folder(&p.kind).is_some())?;
        let version = project
            .versions
            .iter()
            .find(|v| v.game == game && self.revoked(&project.id, &v.file.sha256).is_none())?;
        Some((project, version))
    }
}

/// Where a registry file goes in the game folder.
pub fn install_folder(kind: &str) -> Option<&'static str> {
    match kind {
        "mod" => Some("horizon-mods"),
        "native-mod" => Some("horizon-native-mods"),
        _ => None,
    }
}

// Answers -----------------------------------------------------------------------

/// What Horizon's launcher said about a Java mod. It's Horizon's own check, not Hopper's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Check {
    /// Its blocks, items, recipes, loot and worldgen carry over.
    Works,
    /// The content carries over; these parts don't.
    Partly(Vec<String>),
    /// Nothing to carry over, or the jar is too old.
    DoesntRun(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer<'a> {
    BuiltIn(&'a Builtin),
    Ported {
        project: &'a Project,
        version: &'a Version,
    },
    /// `left_out` is empty when everything carries over.
    Works {
        left_out: Vec<String>,
    },
    NotYet,
    /// Nothing could check it, such as a CurseForge-only jar.
    NotSure,
}

/// Where a mod lands in a pack's answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bucket {
    Works,
    Ported,
    BuiltIn,
    NotYet,
}

impl Answer<'_> {
    pub fn bucket(&self) -> Bucket {
        match self {
            Answer::BuiltIn(_) => Bucket::BuiltIn,
            Answer::Ported { .. } => Bucket::Ported,
            Answer::Works { .. } => Bucket::Works,
            Answer::NotYet | Answer::NotSure => Bucket::NotYet,
        }
    }

    /// The word on the status pill.
    pub fn label(&self) -> &'static str {
        match self {
            Answer::BuiltIn(_) => "Built in",
            Answer::Ported { .. } => "Ported",
            Answer::Works { .. } => "Works",
            Answer::NotYet => "Not yet",
            Answer::NotSure => "Not sure yet",
        }
    }

    /// The one button under the answer. Engine features have nothing to turn on.
    pub fn button(&self) -> Option<&'static str> {
        match self {
            Answer::BuiltIn(b) if b.kind == Kind::Engine => None,
            Answer::BuiltIn(_) => Some("How to turn it on"),
            Answer::Ported { .. } => Some("Install"),
            Answer::Works { .. } => Some("Add to Horizon"),
            Answer::NotYet | Answer::NotSure => Some("Want it too"),
        }
    }
}

/// Answers one mod: built in, then ported, then Horizon's launcher check, then not yet.
pub fn answer<'a>(
    id: &str,
    name: &str,
    rules: &'a Rules,
    index: &'a Index,
    check: Option<&Check>,
) -> Answer<'a> {
    if let Some(b) = rules.builtin_for(id, name) {
        return Answer::BuiltIn(b);
    }
    if let Some((project, version)) = rules
        .port_for(id)
        .and_then(|p| index.installable(&p.horizon, GAME))
    {
        return Answer::Ported { project, version };
    }
    match check {
        Some(Check::Works) => Answer::Works {
            left_out: Vec::new(),
        },
        Some(Check::Partly(parts)) => Answer::Works {
            left_out: parts.clone(),
        },
        Some(Check::DoesntRun(_)) => Answer::NotYet,
        None => Answer::NotSure,
    }
}

// Packs -------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PackSummary {
    pub works: u32,
    pub ported: u32,
    pub built_in: u32,
    pub not_yet: u32,
}

impl PackSummary {
    pub fn of(buckets: impl IntoIterator<Item = Bucket>) -> Self {
        let mut s = PackSummary::default();
        for b in buckets {
            match b {
                Bucket::Works => s.works += 1,
                Bucket::Ported => s.ported += 1,
                Bucket::BuiltIn => s.built_in += 1,
                Bucket::NotYet => s.not_yet += 1,
            }
        }
        s
    }

    pub fn total(&self) -> u32 {
        self.works + self.ported + self.built_in + self.not_yet
    }

    /// Mods that work as they are, or have a Horizon version.
    pub fn come_with_you(&self) -> u32 {
        self.works + self.ported
    }

    /// The three big numbers: come with you, don't need anymore, not yet.
    pub fn big_numbers(&self) -> [u32; 3] {
        [self.come_with_you(), self.built_in, self.not_yet]
    }

    pub fn headline(&self) -> String {
        let come = self.come_with_you();
        let verb = if come == 1 { "comes" } else { "come" };
        format!(
            "{come} {verb} with you. {} you don't need anymore. {} not yet.",
            self.built_in, self.not_yet
        )
    }

    /// The primary button, or `None` when there's nothing to add.
    pub fn button(&self) -> Option<String> {
        match self.come_with_you() {
            0 => None,
            1 => Some("Add it to Horizon".into()),
            n => Some(format!("Add the {n} to Horizon")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackKind {
    /// A `mods` folder full of jars.
    ModsFolder,
    /// A Prism Launcher or MultiMC instance.
    Instance,
    /// A Modrinth `.mrpack`.
    Mrpack,
    /// A CurseForge modpack zip.
    CurseForge,
}

/// Works out what was dropped, from its name and the paths inside it
/// (relative, `/`-separated).
pub fn pack_kind(name: &str, entries: &[&str]) -> Option<PackKind> {
    let has = |p: &str| entries.contains(&p);
    let name = name.to_ascii_lowercase();
    if name.ends_with(".mrpack") && has("modrinth.index.json") {
        Some(PackKind::Mrpack)
    } else if name.ends_with(".zip") && has("manifest.json") {
        Some(PackKind::CurseForge)
    } else if has("instance.cfg") || has("mmc-pack.json") {
        Some(PackKind::Instance)
    } else if entries
        .iter()
        .any(|e| !e.contains('/') && e.to_ascii_lowercase().ends_with(".jar"))
    {
        Some(PackKind::ModsFolder)
    } else {
        None
    }
}

/// The mods in a Modrinth pack, as (path, SHA-1). Modrinth resolves the hashes
/// with `POST /v2/version_files`, so nothing is downloaded to answer.
pub fn mrpack_mods(index_json: &str) -> Result<Vec<(String, String)>, Error> {
    #[derive(Deserialize)]
    struct Mrpack {
        files: Vec<FileEntry>,
    }
    #[derive(Deserialize)]
    struct FileEntry {
        path: String,
        hashes: Hashes,
    }
    #[derive(Deserialize)]
    struct Hashes {
        sha1: String,
    }
    let m: Mrpack = serde_json::from_str(index_json).map_err(|e| Error::Pack(e.to_string()))?;
    Ok(m.files
        .into_iter()
        .filter(|f| f.path.starts_with("mods/") && f.path.ends_with(".jar"))
        .map(|f| (f.path, f.hashes.sha1))
        .collect())
}

/// Mod names from a CurseForge pack's `modlist.html`. CurseForge's API needs a
/// key, so Hopper looks these names up on Modrinth instead.
pub fn curseforge_names(modlist_html: &str) -> Vec<String> {
    modlist_html
        .split("<li>")
        .skip(1)
        .filter_map(|item| {
            let start = item.find('>')? + 1;
            let end = start + item[start..].find("</a>")?;
            let text = item[start..end].trim();
            let name = text.rfind(" (by ").map_or(text, |i| &text[..i]);
            let name = name
                .replace("&#39;", "'")
                .replace("&quot;", "\"")
                .replace("&amp;", "&");
            (!name.is_empty()).then_some(name)
        })
        .collect()
}

// Icons -------------------------------------------------------------------------

/// The only place icons are fetched from: Modrinth's CDN, where a project's
/// `icon_url` points. Anything else keeps the monogram.
pub const ICON_HOST: &str = "https://cdn.modrinth.com/";

/// The file name an icon is cached under, or `None` when the URL isn't an
/// image on Modrinth's CDN. `data/<project>/<hash>_96.webp` becomes
/// `<project>-<hash>_96.webp`, so one icon is fetched once.
pub fn icon_cache_name(url: &str) -> Option<String> {
    let rest = url.strip_prefix(ICON_HOST)?.strip_prefix("data/")?;
    let ext = rest.rsplit_once('.')?.1.to_ascii_lowercase();
    if !matches!(ext.as_str(), "png" | "webp" | "gif" | "jpg" | "jpeg") {
        return None;
    }
    let name = rest.replace('/', "-");
    let safe = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    (safe && !name.contains("..")).then_some(name)
}

/// What a mod's tile shows. The monogram is what every tile starts as, and
/// stays when there's no icon, Hopper is offline, the mod is CurseForge-only,
/// or the icon is still loading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tile {
    Icon { cached: String, scaling: Scaling },
    Monogram(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scaling {
    Smooth,
    /// Pixel art stays crisp.
    Nearest,
}

/// Pixel art is scaled with nearest-neighbour: a lossless icon with few
/// colours, or a tiny one. Renders, photos and lossy thumbnails are smoothed.
pub fn icon_scaling(lossless: bool, colours: usize, size: u32) -> Scaling {
    if (lossless && colours <= 1024) || size <= 64 {
        Scaling::Nearest
    } else {
        Scaling::Smooth
    }
}

/// Two letters for a mod's tile: "Create" is "Cr", "Applied Energistics 2" is "AE".
pub fn monogram(name: &str) -> String {
    let words: Vec<String> = name
        .split_whitespace()
        .map(|w| {
            w.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
        })
        .filter(|w| {
            w.starts_with(|c: char| c.is_alphabetic())
                && !matches!(w.to_lowercase().as_str(), "and" | "of" | "the")
        })
        .collect();
    let letters: String = match words.as_slice() {
        [] => "?".into(),
        [one] => one.chars().take(2).collect(),
        [a, b, ..] => [a, b].iter().filter_map(|w| w.chars().next()).collect(),
    };
    let mut chars = letters.chars();
    chars
        .next()
        .map_or_else(String::new, |f| f.to_uppercase().chain(chars).collect())
}

// Watching ----------------------------------------------------------------------

/// A watched mod that has a Horizon version now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arrival<'a> {
    pub java: String,
    pub project: &'a Project,
    pub version: &'a Version,
}

impl Arrival<'_> {
    pub fn message(&self) -> String {
        format!("{} is on Horizon now", self.project.name)
    }
}

/// Compares the watchlist with the registry. Runs on start; the watchlist never leaves the PC.
pub fn arrivals<'a>(watched: &[String], rules: &Rules, index: &'a Index) -> Vec<Arrival<'a>> {
    watched
        .iter()
        .filter_map(|id| {
            let port = rules.port_for(id)?;
            let (project, version) = index.installable(&port.horizon, GAME)?;
            Some(Arrival {
                java: normalize_id(id),
                project,
                version,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUILTIN: &str = include_str!("../../../rules/builtin.toml");
    const PORTS: &str = include_str!("../../../rules/ports.toml");

    const INDEX: &str = r#"{
      "schema": 1,
      "generated": "2026-10-31T09:00:00Z",
      "projects": [
        { "id": "veinminer", "kind": "native-mod", "name": "Veinminer", "license": "MIT",
          "versions": [
            { "version": "2.1.0", "game": "26.3", "api": "native-1/a1/b2",
              "file": { "name": "veinminer-2.1.0.hnmod", "url": "https://example.org/v210", "size": 51200, "sha256": "b210" } },
            { "version": "2.0.0", "game": "26.3",
              "file": { "name": "veinminer-2.0.0.hnmod", "url": "https://example.org/v200", "size": 50176, "sha256": "b200" } }
          ] },
        { "id": "waystones", "kind": "mod", "name": "Waystones", "key": "RWQ...",
          "versions": [
            { "version": "1.0.0", "game": "26.3",
              "file": { "name": "waystones-1.0.0.hmod", "url": "https://example.org/w", "size": 90112, "sha256": "c100", "signature": "untrusted comment: ..." } }
          ] },
        { "id": "appleskin", "kind": "resourcepack", "name": "Not a mod",
          "versions": [
            { "version": "1.0.0", "game": "26.3",
              "file": { "name": "a.zip", "url": "https://example.org/a", "size": 10, "sha256": "d100" } }
          ] }
      ],
      "revoked": []
    }"#;

    fn rules() -> Rules {
        Rules::parse(BUILTIN, PORTS).expect("the shipped rules parse")
    }

    fn index() -> Index {
        Index::parse(INDEX).unwrap()
    }

    #[test]
    fn ids_are_normalized() {
        assert_eq!(normalize_id("fabric-api"), "fabric_api");
        assert_eq!(normalize_id("  Sodium "), "sodium");
        assert_eq!(normalize_id("OK Zoomer"), "ok_zoomer");
        assert_eq!(normalize_id("Xaero's Minimap"), "xaeros_minimap");
        assert_eq!(normalize_id("farmers-delight"), "farmers_delight");
    }

    #[test]
    fn fabric_manifest() {
        let json = r#"{ "schemaVersion": 1, "id": "sodium", "version": "0.6.13+mc26.3", "name": "Sodium",
                        "environment": "client", "entrypoints": {} }"#;
        let m = read_manifest("fabric.mod.json", json).unwrap();
        assert_eq!(
            (m.id.as_str(), m.name.as_str(), m.loader),
            ("sodium", "Sodium", Loader::Fabric)
        );
        assert_eq!(m.version.as_deref(), Some("0.6.13+mc26.3"));
        let bare = read_manifest("fabric.mod.json", "\u{feff}{ \"id\": \"fabric-api\" }").unwrap();
        assert_eq!(
            (bare.id.as_str(), bare.name.as_str(), bare.version),
            ("fabric_api", "fabric-api", None)
        );
    }

    #[test]
    fn neoforge_and_forge_manifests() {
        let toml = r#"
            modLoader = "javafml"
            loaderVersion = "[4,)"
            license = "MIT"
            [[mods]]
            modId = "create"
            version = "${file.jarVersion}"
            displayName = "Create"
            [[dependencies.create]]
            modId = "neoforge"
            type = "required"
        "#;
        let m = read_manifest("META-INF/neoforge.mods.toml", toml).unwrap();
        assert_eq!(
            (m.id.as_str(), m.name.as_str(), m.version, m.loader),
            ("create", "Create", None, Loader::NeoForge)
        );
        let forge = read_manifest(
            "META-INF/mods.toml",
            "[[mods]]\nmodId=\"waystones\"\nversion=\"21.1.4\"",
        )
        .unwrap();
        assert_eq!(
            (forge.name.as_str(), forge.version.as_deref(), forge.loader),
            ("waystones", Some("21.1.4"), Loader::Forge)
        );
        assert_eq!(read_manifest("pack.mcmeta", "{}"), Err(Error::NotAManifest));
        assert!(matches!(
            read_manifest("META-INF/mods.toml", "modLoader = \"javafml\""),
            Err(Error::Manifest(_))
        ));
    }

    #[test]
    fn the_shipped_rules_are_valid() {
        let r = rules();
        assert_eq!(r.builtin.len(), 13);
        assert!(r.builtin.iter().all(|b| {
            b.source
                .starts_with("https://github.com/horizon-rust/horizon")
        }));
        assert_eq!(
            r.builtin_for("sodium", "Sodium").unwrap().kind,
            Kind::Engine
        );
        assert_eq!(r.builtin_for("emi", "EMI").unwrap().horizon, "Recipes");
        assert_eq!(
            r.builtin_for("ok-zoomer", "OK Zoomer").unwrap().horizon,
            "Zoom"
        );
        assert_eq!(
            r.builtin_for("brightness_toggle", "Fullbright Toggle")
                .unwrap()
                .horizon,
            "Fullbright"
        );
        assert_eq!(
            r.builtin_for("simple_fps", "Simple FPS Display")
                .unwrap()
                .horizon,
            "FPS Counter"
        );
        assert!(r.builtin_for("create", "Create").is_none());
        assert_eq!(r.port_for("Veinminer").unwrap().horizon, "veinminer");
    }

    #[test]
    fn rules_refuse_ids_that_arent_normalized() {
        let bad = "[[builtin]]\nmods = [\"Fabric-API\"]\nhorizon = \"x\"\nkind = \"engine\"\nhow = \"x\"\nsource = \"https://x\"";
        assert!(matches!(Rules::parse(bad, ""), Err(Error::Rules(_))));
    }

    #[test]
    fn the_four_sample_answers() {
        let (r, i) = (rules(), index());
        let sodium = answer("sodium", "Sodium", &r, &i, None);
        assert_eq!((sodium.label(), sodium.button()), ("Built in", None));
        let create = answer(
            "create",
            "Create",
            &r,
            &i,
            Some(&Check::DoesntRun("needs Java code".into())),
        );
        assert_eq!(
            (create.label(), create.button()),
            ("Not yet", Some("Want it too"))
        );
        let vein = answer("veinminer", "Veinminer", &r, &i, None);
        assert_eq!((vein.label(), vein.button()), ("Ported", Some("Install")));
        let Answer::Ported { version, .. } = vein else {
            unreachable!()
        };
        assert_eq!(version.version, "2.1.0");
        let fences = answer(
            "mcwfences",
            "Macaw's Fences and Walls",
            &r,
            &i,
            Some(&Check::Works),
        );
        assert_eq!(
            (fences.label(), fences.button()),
            ("Works", Some("Add to Horizon"))
        );
        assert_eq!(
            answer("x", "Some Mod", &r, &i, None).label(),
            "Not sure yet"
        );
        let lambs = answer("lambdynlights", "LambDynamicLights", &r, &i, None);
        assert_eq!(lambs.button(), Some("How to turn it on"));
    }

    #[test]
    fn revoked_files_never_count() {
        let r = rules();
        let mut i = index();
        i.revoked.push(Revoked {
            id: "veinminer".into(),
            sha256: Some("B210".into()),
            reason: "Bad build.".into(),
        });
        assert_eq!(i.revoked("veinminer", "b210"), Some("Bad build."));
        let Answer::Ported { version, .. } = answer("veinminer", "Veinminer", &r, &i, None) else {
            panic!()
        };
        assert_eq!(version.version, "2.0.0");
        i.revoked.push(Revoked {
            id: "veinminer".into(),
            sha256: None,
            reason: "Taken down.".into(),
        });
        assert_eq!(
            answer("veinminer", "Veinminer", &r, &i, Some(&Check::Works)).bucket(),
            Bucket::Works
        );
        assert_eq!(
            answer("veinminer", "Veinminer", &r, &i, None),
            Answer::NotSure
        );
    }

    #[test]
    fn only_mods_install_and_newer_indexes_are_refused() {
        let i = index();
        assert!(i.installable("appleskin", GAME).is_none());
        assert!(i.installable("waystones", "26.2").is_none());
        assert_eq!(install_folder("mod"), Some("horizon-mods"));
        assert_eq!(install_folder("native-mod"), Some("horizon-native-mods"));
        assert_eq!(
            Index::parse(r#"{ "schema": 2, "projects": [] }"#),
            Err(Error::NewerIndex(2))
        );
    }

    #[test]
    fn the_sample_pack_adds_up() {
        let buckets = [
            (Bucket::Works, 24),
            (Bucket::Ported, 6),
            (Bucket::BuiltIn, 14),
            (Bucket::NotYet, 16),
        ]
        .into_iter()
        .flat_map(|(b, n)| std::iter::repeat_n(b, n));
        let s = PackSummary::of(buckets);
        assert_eq!(s.total(), 60);
        assert_eq!(s.big_numbers(), [30, 14, 16]);
        assert_eq!(
            s.headline(),
            "30 come with you. 14 you don't need anymore. 16 not yet."
        );
        assert_eq!(s.button().as_deref(), Some("Add the 30 to Horizon"));
        let one = PackSummary {
            works: 1,
            ..Default::default()
        };
        assert_eq!(
            one.headline(),
            "1 comes with you. 0 you don't need anymore. 0 not yet."
        );
        assert_eq!(one.button().as_deref(), Some("Add it to Horizon"));
        assert_eq!(PackSummary::default().button(), None);
    }

    #[test]
    fn packs_are_recognised() {
        assert_eq!(
            pack_kind("mods", &["sodium-0.6.jar", "create-6.0.jar"]),
            Some(PackKind::ModsFolder)
        );
        assert_eq!(
            pack_kind(
                "Our SMP",
                &["instance.cfg", "mmc-pack.json", "minecraft/mods/a.jar"]
            ),
            Some(PackKind::Instance)
        );
        assert_eq!(
            pack_kind(
                "smp.mrpack",
                &["modrinth.index.json", "overrides/options.txt"]
            ),
            Some(PackKind::Mrpack)
        );
        assert_eq!(
            pack_kind("SMP.zip", &["manifest.json", "modlist.html"]),
            Some(PackKind::CurseForge)
        );
        assert_eq!(pack_kind("photos", &["a.png"]), None);
    }

    #[test]
    fn modrinth_and_curseforge_lists() {
        let mr = r#"{ "formatVersion": 1, "game": "minecraft", "name": "Our SMP pack", "files": [
            { "path": "mods/create-6.0.6.jar", "hashes": { "sha1": "aa11", "sha512": "bb22" }, "downloads": [], "fileSize": 1 },
            { "path": "resourcepacks/soft.zip", "hashes": { "sha1": "cc33", "sha512": "dd44" }, "downloads": [], "fileSize": 1 } ] }"#;
        assert_eq!(
            mrpack_mods(mr).unwrap(),
            vec![("mods/create-6.0.6.jar".to_string(), "aa11".to_string())]
        );
        let html = "<ul>\n<li><a href=\"https://www.curseforge.com/minecraft/mc-mods/jei\">Just Enough Items (JEI) (by mezz)</a></li>\n\
                    <li><a href=\"https://www.curseforge.com/minecraft/mc-mods/farmers-delight\">Farmer&#39;s Delight (by vectorwing)</a></li>\n</ul>";
        assert_eq!(
            curseforge_names(html),
            ["Just Enough Items (JEI)", "Farmer's Delight"]
        );
    }

    #[test]
    fn icons_come_only_from_modrinths_cdn() {
        let url = "https://cdn.modrinth.com/data/LNytGWDc/61d716699bcf1ec42ed4926a9e1c7311be6087e2_96.webp";
        assert_eq!(
            icon_cache_name(url).as_deref(),
            Some("LNytGWDc-61d716699bcf1ec42ed4926a9e1c7311be6087e2_96.webp")
        );
        for bad in [
            "http://cdn.modrinth.com/data/a/b.png",
            "https://cdn.modrinth.com.example.org/data/a/b.png",
            "https://example.org/data/a/b.png",
            "https://cdn.modrinth.com/data/a/b.svg",
            "https://cdn.modrinth.com/data/../../b.png",
            "https://cdn.modrinth.com/data/a/b.png?x=1",
        ] {
            assert_eq!(icon_cache_name(bad), None, "{bad}");
        }
    }

    #[test]
    fn monograms_and_scaling() {
        assert_eq!(monogram("Create"), "Cr");
        assert_eq!(monogram("Applied Energistics 2"), "AE");
        assert_eq!(monogram("Macaw's Fences and Walls"), "MF");
        assert_eq!(monogram("Iron's Spells 'n Spellbooks"), "IS");
        assert_eq!(monogram("sodium"), "So");
        assert_eq!(icon_scaling(true, 74, 96), Scaling::Nearest);
        assert_eq!(icon_scaling(true, 2849, 400), Scaling::Smooth);
        assert_eq!(icon_scaling(false, 2219, 64), Scaling::Nearest);
        assert_eq!(icon_scaling(false, 4506, 96), Scaling::Smooth);
        let t = Tile::Monogram(monogram("Create"));
        assert_eq!(t, Tile::Monogram("Cr".into()));
    }

    #[test]
    fn watched_mods_arrive() {
        let (r, i) = (rules(), index());
        let watched = [
            "create".to_string(),
            "waystones".to_string(),
            "farmers_delight".to_string(),
        ];
        let got = arrivals(&watched, &r, &i);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].message(), "Waystones is on Horizon now");
        assert!(got[0].project.key.is_some());
    }
}
