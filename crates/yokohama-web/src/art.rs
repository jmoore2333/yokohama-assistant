//! The card art, and the join between an engine identifier and a picture.
//!
//! The art is cropped out of the TTS sprite sheets by `docs/tools/mkcardart.py`
//! and COMMITTED under `assets/art/`, manifest included. This module does two
//! things with it: serves the files, and — the part that matters — proves the
//! join is total. Nothing here decides what a card IS; the manifest is keyed on
//! identifiers the wire already carries, so a client looks a card up with the
//! same value it renders.
//!
//! **The keys are the engine's, and each was chosen rather than defaulted.**
//! Orders key on `OrderCardId`, which is the number printed in the card's
//! corner. Sites key on the 0-based index `screen_model::printed_site_index`
//! returns — the same number the CLI's tile prints as `Site n`. Achievements
//! and area tiles key on their serde names. Technologies key on the printed
//! TITLE and not on the registry's `codes` slug: both copies of a doubled title
//! print one face, and `TechCard.name` rides inside `Rules::content_hash` while
//! `codes` is hashed nowhere — a slug could change without moving `rules_hash`,
//! and the art join would break with no test able to see it.
//!
//! **Art is optional and its absence is not an error.** A checkout with no
//! `assets/art` answers `/api/art` with a 404 and the client draws what it
//! drew before there was any art. What is NOT tolerated is a manifest that
//! disagrees with the engine: [`tests`] fails if any identifier the engine can
//! produce has no entry, if any entry names a file that is not on disk, or if
//! any entry names an identifier the engine does not have. A half-covered
//! manifest would show as a few silently missing pictures, which is exactly the
//! failure a human never reports.

use std::collections::BTreeMap;
use std::path::PathBuf;

use axum::body::Body;
use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

/// One picture: where it is, and how big it is.
///
/// The dimensions ride along so a client can reserve the box before the image
/// arrives. They are the EMITTED size, not the sheet's cell — the generator
/// computes the height from the source aspect rather than typing it, so a
/// 530:799 card cannot silently become a 4:3 one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtEntry {
    /// Relative to the art root; the client prefixes `/art/`.
    pub file: String,
    pub w: u32,
    pub h: u32,
}

/// What `mkcardart.py` wrote.
///
/// `#[serde(default)]` on every map so an older manifest missing a section
/// loads and fails the COVERAGE test with a list of what it lacks, rather than
/// failing to parse with a message about a missing field. The first tells you
/// what to regenerate; the second tells you the file is broken.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArtManifest {
    #[serde(default)]
    pub orders: BTreeMap<String, ArtEntry>,
    #[serde(default)]
    pub techs: BTreeMap<String, ArtEntry>,
    #[serde(default)]
    pub sites: BTreeMap<String, ArtEntry>,
    #[serde(default)]
    pub achievements: BTreeMap<String, ArtEntry>,
    #[serde(default)]
    pub areas: BTreeMap<String, ArtEntry>,
    /// Keyed `"<variant>_<payload>"` — `"Copper_2"`, `"ImportedGoods_1"`. A
    /// 5-Power face is a (variant, count) PAIR and the key spells both, because
    /// `Copper(2)` and a hypothetical `Copper(3)` would be different pictures.
    #[serde(default)]
    pub five_power: BTreeMap<String, ArtEntry>,
    /// Keyed by the `Country` serde name.
    #[serde(default)]
    pub mercenaries: BTreeMap<String, ArtEntry>,
    #[serde(default)]
    pub backs: BTreeMap<String, ArtEntry>,
    #[serde(default)]
    pub tokens: BTreeMap<String, ArtEntry>,
}

/// Where the cropped art lives.
///
/// `YOKOHAMA_WEB_ART` overrides it; the default is repo-root-relative, the same
/// shape (and the same reasoning) as [`crate::assets::dist_dir`] — a checkout
/// run from its root finds it, and a container sets the variable. No build-time
/// absolute path is baked in, so the binary is movable.
pub fn art_dir() -> PathBuf {
    std::env::var_os("YOKOHAMA_WEB_ART")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("crates/yokohama-web/assets/art"))
}

/// Read and parse `manifest.json` under `dir`.
pub fn load(dir: &std::path::Path) -> Result<ArtManifest, String> {
    let path = dir.join("manifest.json");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("no art manifest at {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// `GET /art/**` — one cropped card.
///
/// The traversal guard is [`crate::assets::resolve`]'s, reused rather than
/// restated: it refuses every path component that is not a plain name, which is
/// a property of the walk and not of a denylist. A miss is a 404 and never the
/// SPA fallback — a missing picture must not answer with HTML that the browser
/// then reports as a broken image for an unrelated reason.
///
/// One hour of caching, deliberately short of `immutable`: the filenames are
/// stable rather than content-hashed, so a regenerated card keeps its name and
/// an `immutable` promise would be a lie for as long as the max-age.
pub async fn serve(uri: Uri) -> Response {
    let root = art_dir();
    let rel = uri.path().strip_prefix("/art").unwrap_or("");
    let Some(path) = crate::assets::resolve(&root, rel) else {
        return (StatusCode::BAD_REQUEST, "bad path").into_response();
    };
    match std::fs::read(&path) {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, crate::assets::content_type(&path)),
                (header::CACHE_CONTROL, "public, max-age=3600"),
            ],
            Body::from(bytes),
        )
            .into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "no such art").into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use yokohama_cli::screen_model::achievement_registry;
    use yokohama_core::{full_tile_set, Rules};

    /// The committed art, addressed from the crate rather than from the CWD so
    /// `cargo test` finds it from anywhere.
    fn committed() -> (PathBuf, ArtManifest) {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets").join("art");
        let manifest = load(&dir).expect("the committed manifest parses");
        (dir, manifest)
    }

    /// `{"Copper": 2}` -> `"Copper_2"`. Derived from the SERIALIZED form so it
    /// cannot drift from the wire the client reads.
    fn five_power_key(effect: &yokohama_core::FivePowerEffect) -> String {
        match serde_json::to_value(effect) {
            Ok(serde_json::Value::Object(map)) if map.len() == 1 => {
                let (k, v) = map.into_iter().next().expect("one entry");
                format!("{k}_{v}")
            }
            other => panic!("expected a single-key newtype variant, got {other:?}"),
        }
    }

    fn key_of<T: Serialize>(v: &T) -> String {
        match serde_json::to_value(v) {
            Ok(serde_json::Value::String(s)) => s,
            other => panic!("expected a unit-variant serde name, got {other:?}"),
        }
    }

    /// EVERY identifier the engine can hand a client resolves to a picture.
    ///
    /// Derived from the engine's own registries rather than from a count: a
    /// 37th order card or a 13th achievement would fail here without anyone
    /// remembering that this test exists. The four families are asserted in one
    /// test so a regeneration that dropped a whole section reports all of it at
    /// once instead of one section per run.
    #[test]
    fn every_engine_identifier_has_art() {
        let (_, m) = committed();
        let rules = Rules::standard();

        let mut missing: Vec<String> = Vec::new();
        for card in rules.orders.iter_sorted_by_id() {
            let key = card.id.0.to_string();
            if !m.orders.contains_key(&key) {
                missing.push(format!("order {key}"));
            }
        }
        for name in rules.techs.iter().map(|c| c.name.clone()).collect::<BTreeSet<_>>() {
            if !m.techs.contains_key(&name) {
                missing.push(format!("tech {name:?}"));
            }
        }
        for i in 0..rules.building_sites.len() {
            if !m.sites.contains_key(&i.to_string()) {
                missing.push(format!("site {i}"));
            }
        }
        for a in achievement_registry() {
            let key = key_of(&a.achievement_type);
            if !m.achievements.contains_key(&key) {
                missing.push(format!("achievement {key}"));
            }
        }
        for key in full_tile_set().iter().map(key_of).collect::<BTreeSet<String>>() {
            if !m.areas.contains_key(&key) {
                missing.push(format!("area {key}"));
            }
        }
        // The 5-Power pool is the engine's own bag, so a re-priced face (a
        // `Tea(3)` that became a `Tea(4)`) shows up here as a missing picture
        // rather than as a tile that quietly draws the wrong number.
        for key in
            yokohama_core::canonical_five_power_tokens().iter().map(five_power_key).collect::<BTreeSet<String>>()
        {
            if !m.five_power.contains_key(&key) {
                missing.push(format!("five-power {key}"));
            }
        }
        for c in yokohama_core::Country::ALL {
            let key = key_of(&c);
            if !m.mercenaries.contains_key(&key) {
                missing.push(format!("mercenary {key}"));
            }
        }
        assert!(missing.is_empty(), "art is missing for: {missing:?}");
    }

    /// Nothing in the manifest names an identifier the engine does not have.
    ///
    /// The other direction of the same join, and not redundant with it: a
    /// stale key (a renamed achievement, a dropped site) leaves a picture
    /// nothing will ever ask for, and the coverage test above passes happily
    /// while the client shows one wrong card.
    #[test]
    fn the_manifest_names_no_identifier_the_engine_lacks() {
        let (_, m) = committed();
        let rules = Rules::standard();

        let orders: BTreeSet<String> =
            rules.orders.iter().map(|c| c.id.0.to_string()).collect();
        let techs: BTreeSet<String> = rules.techs.iter().map(|c| c.name.clone()).collect();
        let sites: BTreeSet<String> =
            (0..rules.building_sites.len()).map(|i| i.to_string()).collect();
        let achievements: BTreeSet<String> =
            achievement_registry().iter().map(|a| key_of(&a.achievement_type)).collect();
        let areas: BTreeSet<String> = full_tile_set().iter().map(key_of).collect();
        let five_power: BTreeSet<String> =
            yokohama_core::canonical_five_power_tokens().iter().map(five_power_key).collect();
        let mercenaries: BTreeSet<String> =
            yokohama_core::Country::ALL.iter().map(key_of).collect();

        let mut extra: Vec<String> = Vec::new();
        for (family, have, want) in [
            ("order", &m.orders, &orders),
            ("tech", &m.techs, &techs),
            ("site", &m.sites, &sites),
            ("achievement", &m.achievements, &achievements),
            ("area", &m.areas, &areas),
            ("five-power", &m.five_power, &five_power),
            ("mercenary", &m.mercenaries, &mercenaries),
        ] {
            for key in have.keys() {
                if !want.contains(key) {
                    extra.push(format!("{family} {key:?}"));
                }
            }
        }
        assert!(extra.is_empty(), "the manifest names identifiers the engine does not have: {extra:?}");
    }

    /// Every entry names a file that is on disk and is not empty.
    ///
    /// A manifest is a promise about the filesystem, and the promise is exactly
    /// what a browser discovers as a broken image at the worst moment. The
    /// zero-length check is not paranoia: an interrupted generator run leaves
    /// empty files behind, and `Path::exists` is happy with those.
    #[test]
    fn every_entry_resolves_to_a_file_on_disk() {
        let (dir, m) = committed();
        let mut bad: Vec<String> = Vec::new();
        let families: [&BTreeMap<String, ArtEntry>; 9] = [
            &m.orders,
            &m.techs,
            &m.sites,
            &m.achievements,
            &m.areas,
            &m.five_power,
            &m.mercenaries,
            &m.backs,
            &m.tokens,
        ];
        let mut seen = 0usize;
        for family in families {
            for entry in family.values() {
                seen += 1;
                match std::fs::metadata(dir.join(&entry.file)) {
                    Ok(md) if md.len() > 0 => {}
                    Ok(_) => bad.push(format!("{} is empty", entry.file)),
                    Err(e) => bad.push(format!("{}: {e}", entry.file)),
                }
                assert!(entry.w > 0 && entry.h > 0, "{} has no dimensions", entry.file);
            }
        }
        assert!(bad.is_empty(), "unresolvable art: {bad:?}");
        assert!(seen >= 110, "only {seen} entries — a truncated manifest would pass every other check");
    }

    /// A key names the file its own name derives, and no two keys name one file.
    ///
    /// **This is the test that catches a PERMUTATION**, and it exists because
    /// the other four demonstrably do not. Measured against the committed
    /// manifest with three deliberate mutations: swapping `orders."1"` and
    /// `orders."2"`, rotating all 36 order files by one, and pointing every
    /// order/tech/site/achievement entry at `orders/O01.webp` — **all three pass
    /// the other four tests**, because those compare KEY SETS and ask only
    /// whether each named file exists and is non-empty. A player would see
    /// order 2's requirements drawn on order 1's card and nothing would go red.
    ///
    /// It is cheap on purpose: a string comparison per entry and one set, no
    /// image decoded, no dependency. It cannot prove the PICTURE is right — only
    /// looking does that, and two mapping errors in this milestone needed
    /// exactly that — but it does prove the manifest is a bijection onto the
    /// names the generator builds, which is the half a machine can check.
    ///
    /// The injectivity half is not redundant with the naming half: a `slug()`
    /// collision between two tech titles would overwrite one file, and both
    /// entries would then name a file that legitimately derives from one of
    /// them.
    #[test]
    fn every_key_names_the_file_it_derives_and_no_file_is_named_twice() {
        let (_, m) = committed();

        /// `mkcardart.py`'s `slug`: lowercase, every run of non-alphanumerics
        /// to one dash, trimmed. Restated rather than imported because the
        /// generator is Python — and restating it is the point, since the two
        /// disagreeing is itself the failure this catches.
        fn slug(title: &str) -> String {
            let mut out = String::new();
            let mut dash = false;
            for c in title.chars() {
                if c.is_ascii_alphanumeric() {
                    out.extend(c.to_lowercase());
                    dash = false;
                } else if !dash {
                    out.push('-');
                    dash = true;
                }
            }
            out.trim_matches('-').to_string()
        }

        let mut wrong: Vec<String> = Vec::new();
        let mut check = |dir: &str, entries: &BTreeMap<String, ArtEntry>, name: &dyn Fn(&str) -> String| {
            for (key, entry) in entries {
                let want = format!("{dir}/{}.webp", name(key));
                if entry.file != want {
                    wrong.push(format!("{dir} key {key:?} names {:?}, not {want:?}", entry.file));
                }
            }
        };
        let two_digit = |k: &str| k.parse::<u32>().map_or_else(|_| k.to_string(), |n| format!("{n:02}"));
        check("orders", &m.orders, &|k| format!("O{}", two_digit(k)));
        check("sites", &m.sites, &|k| format!("S{}", two_digit(k)));
        check("techs", &m.techs, &|k| slug(k));
        check("achievements", &m.achievements, &|k| k.to_string());
        check("areas", &m.areas, &|k| k.to_string());
        check("five-power", &m.five_power, &|k| k.to_string());
        check("mercenaries", &m.mercenaries, &|k| k.to_string());
        check("backs", &m.backs, &|k| k.to_string());
        check("tokens", &m.tokens, &|k| k.to_string());
        assert!(wrong.is_empty(), "keys and filenames disagree: {wrong:#?}");

        let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
        for family in [
            &m.orders,
            &m.techs,
            &m.sites,
            &m.achievements,
            &m.areas,
            &m.five_power,
            &m.mercenaries,
            &m.backs,
            &m.tokens,
        ] {
            for entry in family.values() {
                *seen.entry(entry.file.as_str()).or_default() += 1;
            }
        }
        let shared: Vec<_> = seen.iter().filter(|(_, n)| **n > 1).collect();
        assert!(shared.is_empty(), "these files are named by more than one key: {shared:?}");
    }

    /// Art keeps its source aspect ratio.
    ///
    /// Every card cell on every sheet is 530x799 and every area cell 709x531;
    /// the generator scales to a width and COMPUTES the height, so a wrong
    /// aspect here means the crop geometry moved, not that a constant was
    /// mistyped. Half a pixel of rounding is allowed and one whole one is not.
    ///
    /// The ACHIEVEMENTS are the one family stored sideways on their sheets and
    /// rotated by the generator (M5, `docs/plans/2026-09-01-web-frontend-m5-spec.md`
    /// §2), so they keep the cell's aspect turned on its side: 799x530.
    #[test]
    fn emitted_art_keeps_the_sheet_aspect() {
        let (_, m) = committed();
        for (family, entries, (cw, ch)) in [
            ("orders", &m.orders, (530.0, 799.0)),
            ("techs", &m.techs, (530.0, 799.0)),
            ("sites", &m.sites, (530.0, 799.0)),
            ("achievements", &m.achievements, (799.0, 530.0)),
            ("areas", &m.areas, (709.0, 531.0)),
        ] {
            for (key, e) in entries {
                let want = f64::from(e.w) * ch / cw;
                assert!(
                    (want - f64::from(e.h)).abs() <= 0.5,
                    "{family}/{key}: {}x{} is not the {cw}x{ch} aspect (expected h {want:.1})",
                    e.w,
                    e.h,
                );
            }
        }
    }
}
