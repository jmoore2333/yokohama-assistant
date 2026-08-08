//! The BGA importer (Phase 3.5b H1, plan `docs/plans/2026-08-08-phase3.5b-bridge-plan.md` §2).
//!
//! Everything here translates BGA's captured wire data into the engine's own
//! vocabulary — and NOTHING here bypasses the engine: the importer's product
//! is the same `SetupObservations`/`StartingOrderChoice` answers the live
//! wizard collects, fed through the same `Session::new_live` path as typed
//! entry. The standing posture is HARD-ERROR-NEVER-GUESS (design §6, plan §4):
//! an identity this module cannot prove from evidence is a typed error or an
//! `Absent` answer-sheet field naming its reason, never a plausible default.
//!
//! Module map:
//! - [`orders`] — the vector card mapper (plan T3). BGA's card numbering is a
//!   permutation of ours, so the requirement+reward VECTOR is the key.
//! - [`names`] — the name-keyed identity tables (plan T4): BGA tile ids/names
//!   to `AreaType`, and the accumulating `tech_id → TechCardId` ledger.
//! - [`setup`] — setup-answer synthesis (plan T5): earliest `gamedatas`
//!   snapshot + collapsed feed → an evidence-carrying answer sheet and, where
//!   the evidence suffices, a live-mode `Session`.
//! - [`import`] — the `yokohama-cli import` entry point (plan T6).

pub mod import;
pub mod names;
pub mod orders;
pub mod setup;

use std::fmt;

/// Importer implementation version, stamped as `SeatProvenance.version` on
/// every record this importer emits (plan §1). Bumped when records written
/// under the `bga-human` controller name stop being comparable.
pub const BGA_IMPORT_VERSION: u32 = 1;

/// The controller name for a seat a human played ON BGA — distinct from
/// [`yokohama_core::CONTROLLER_HUMAN`] (a seat typed at THIS keyboard) so a
/// reader of the file can tell an imported game from a typed one. With the
/// `bga-import/<table_id>` run id this is the hook the map §1.5/§1.6
/// comparison-only fence rides: these records are comparison corpus, never a
/// tuning input.
pub const CONTROLLER_BGA_HUMAN: &str = "bga-human";

/// Typed mapping errors. Every variant names what was seen — these are the
/// bridge's schema-drift alarms, so a generic message would defeat them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BgaMapError {
    /// A BGA row missing a field the measured shape always carries.
    MissingField { context: String, field: String },
    /// A field that should be numeric was not ("a non-numeric string is a
    /// hard error, not a 0" — mapping doc §1).
    NonNumeric { context: String, field: String, value: String },
    /// A requirement+reward vector matching no engine order card.
    NoVectorMatch { context: String, vector: String },
    /// Two engine cards sharing one vector — the key's uniqueness is a
    /// property of card data a `gen-data` run could break, so it is asserted
    /// at construction rather than trusted.
    DuplicateVectorInRules { first: u8, second: u8 },
    /// A BGA country code outside the pinned 1..=5 table.
    UnknownCountryCode { code: u64 },
    /// The vector matched but the BGA country code disagrees with the matched
    /// card's country — one of the two data shapes moved (plan T3).
    CountryDisagreement {
        bga_order_id: u64,
        bga_country_code: u64,
        matched_card: u8,
        expected_code: u64,
    },
    /// A BGA `tile_id` outside the corpus-evidenced table (U11: 3P/4P areas
    /// are unobserved).
    UnknownTileId { tile_id: u64 },
    /// A BGA `tile_name` outside the corpus-evidenced table.
    UnknownTileName { name: String },
    /// The committed tech ledger failed its own load checks.
    LedgerInvalid { detail: String },
    /// A `techAcquisition` observation contradicting a ledger row.
    LedgerContradiction { tech_id: u64, detail: String },
    /// A tech name no engine card carries.
    UnknownTechTitle { title: String },
    /// A tech observation whose (country, gears) class disagrees with the
    /// engine card of that title.
    TechClassMismatch { tech_id: u64, title: String, detail: String },
    /// More BGA tech ids claiming a title than the engine has physical copies.
    TitleCapacityExceeded { title: String, capacity: usize },
}

impl fmt::Display for BgaMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BgaMapError::MissingField { context, field } => {
                write!(f, "{context}: missing required field `{field}`")
            }
            BgaMapError::NonNumeric { context, field, value } => {
                write!(f, "{context}: field `{field}` is `{value}`, not a number — refusing to guess")
            }
            BgaMapError::NoVectorMatch { context, vector } => {
                write!(f, "{context}: vector {vector} matches no engine order card")
            }
            BgaMapError::DuplicateVectorInRules { first, second } => {
                write!(
                    f,
                    "engine order cards O{first:02} and O{second:02} share one requirement+reward vector — \
                     the identification key is broken; card data changed"
                )
            }
            BgaMapError::UnknownCountryCode { code } => {
                write!(f, "BGA country code {code} is outside the pinned 1..=5 table")
            }
            BgaMapError::CountryDisagreement { bga_order_id, bga_country_code, matched_card, expected_code } => {
                write!(
                    f,
                    "BGA order {bga_order_id}: vector matched engine O{matched_card:02} but BGA country \
                     code {bga_country_code} != expected {expected_code} — one of the two data shapes moved"
                )
            }
            BgaMapError::UnknownTileId { tile_id } => {
                write!(f, "BGA tile_id {tile_id} is outside the evidenced table (only the 2P board's ten areas are proven)")
            }
            BgaMapError::UnknownTileName { name } => {
                write!(f, "BGA tile name `{name}` is outside the evidenced table")
            }
            BgaMapError::LedgerInvalid { detail } => write!(f, "tech ledger invalid: {detail}"),
            BgaMapError::LedgerContradiction { tech_id, detail } => {
                write!(f, "tech_id {tech_id}: observation contradicts the ledger: {detail}")
            }
            BgaMapError::UnknownTechTitle { title } => {
                write!(f, "tech title `{title}` matches no engine tech card")
            }
            BgaMapError::TechClassMismatch { tech_id, title, detail } => {
                write!(f, "tech_id {tech_id} (`{title}`): class disagreement: {detail}")
            }
            BgaMapError::TitleCapacityExceeded { title, capacity } => {
                write!(f, "more BGA tech ids claim `{title}` than the engine's {capacity} physical copies")
            }
        }
    }
}

impl std::error::Error for BgaMapError {}

/// Importer-level errors: everything `yokohama-cli import` can refuse on.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportError {
    /// The capture file could not be read.
    Io { path: String, detail: String },
    /// The feed reader/collapse refused (typed, from the bridge crate).
    Feed(yokohama_bridge::feed::FeedError),
    /// A card/name mapping refused.
    Map(BgaMapError),
    /// The capture carries no `gamedatas` snapshot — setup synthesis needs one.
    NoSnapshot,
    /// The `gamedatas` shape contradicted itself or the engine's board model —
    /// the plan T5 board STOP and its relatives. Carries the full story.
    Sheet { detail: String },
    /// The evidence does not cover everything `Session::new_live` needs.
    /// NOT a failure of the import — the answer sheet is the product and the
    /// wizard asks the rest — but a `Session` cannot be built yet.
    Incomplete { missing: Vec<String> },
    /// `Session::new_live` itself refused (engine-side).
    Session { detail: String },
    /// Bad command-line usage.
    Usage { detail: String },
    /// A documented flag whose implementation is a later phase (H2).
    NotImplemented { flag: String },
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportError::Io { path, detail } => write!(f, "cannot read {path}: {detail}"),
            ImportError::Feed(e) => write!(f, "capture feed: {e}"),
            ImportError::Map(e) => write!(f, "card mapping: {e}"),
            ImportError::NoSnapshot => write!(
                f,
                "the capture carries no gamedatas snapshot — setup synthesis needs one \
                 (take one at table open via the extension's snapshot action)"
            ),
            ImportError::Sheet { detail } => write!(f, "gamedatas contradiction: {detail}"),
            ImportError::Incomplete { missing } => {
                write!(f, "cannot build a Session yet — the wizard would still ask: {}", missing.join("; "))
            }
            ImportError::Session { detail } => write!(f, "session construction refused: {detail}"),
            ImportError::Usage { detail } => write!(f, "usage: {detail}"),
            ImportError::NotImplemented { flag } => {
                write!(f, "`{flag}` is the catch-up translator's flag (H2) — not implemented yet")
            }
        }
    }
}

impl std::error::Error for ImportError {}

impl From<yokohama_bridge::feed::FeedError> for ImportError {
    fn from(e: yokohama_bridge::feed::FeedError) -> Self {
        ImportError::Feed(e)
    }
}

impl From<BgaMapError> for ImportError {
    fn from(e: BgaMapError) -> Self {
        ImportError::Map(e)
    }
}

/// Fetch a required field off a BGA JSON object and parse it as u64
/// (accepting both spellings — BGA serves strings in history, ints on the
/// socket; `yokohama_bridge::feed::flex_u64` is the shared normalizer).
pub(crate) fn require_u64(
    obj: &serde_json::Value,
    field: &str,
    context: &str,
) -> Result<u64, BgaMapError> {
    let v = obj.get(field).ok_or_else(|| BgaMapError::MissingField {
        context: context.to_string(),
        field: field.to_string(),
    })?;
    yokohama_bridge::feed::flex_u64(v).ok_or_else(|| BgaMapError::NonNumeric {
        context: context.to_string(),
        field: field.to_string(),
        value: v.to_string(),
    })
}

/// Same, narrowed to u8 (card-data scale).
pub(crate) fn require_u8(
    obj: &serde_json::Value,
    field: &str,
    context: &str,
) -> Result<u8, BgaMapError> {
    let n = require_u64(obj, field, context)?;
    u8::try_from(n).map_err(|_| BgaMapError::NonNumeric {
        context: context.to_string(),
        field: field.to_string(),
        value: n.to_string(),
    })
}
