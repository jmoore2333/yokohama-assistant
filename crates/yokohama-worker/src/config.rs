//! **`init`'s configuration** (spec §2.2, §3.1): the JSON the shell builds from
//! the Worker's variables, parsed and refused HERE — so a bad VALUE is refused
//! at `init` naming its variable, the Worker's form of the NAS refusing to
//! start. A configuration of the wrong SHAPE names no variable: JSON that is
//! not an OBJECT is refused in this crate's own sentence, which names the
//! nine keys ([`parse`]); text that is not JSON, an object with a key it does
//! not have or MISSING the door, and a value of the wrong JSON type, such as
//! `{"chat_keep": 500}` for the string `"500"`, in serde's words. The shell
//! always builds the object, with every value a string or `null` but the two
//! TTLs, which are the shell's own reading and so NUMBERS or `null`, and the
//! diagnostic-seeds switch, the shell's own reading too and so a BOOLEAN
//! (`worker/src/config.ts`'s `moduleConfig`, whose `shellConfig` refuses a
//! variable that is not a string, and a door that is not named, first: the
//! orchestrator's ruling E3; M6.5c2, spec §2.1; R-6.5c2.42), so no deployment
//! reaches those.

use serde::Deserialize;
use yokohama_web::chat::ChatConfig;
use yokohama_web::identity::{IdentityMode, User};

/// The door's variable (M6.5c2, spec §2.1; R-6.5c2.5): `worker` or `open`,
/// with no default.
pub(crate) const IDENTITY_VAR: &str = "YOKOHAMA_WEB_IDENTITY";

/// The archive switch's variable (spec §3.1): `r2` or `off`.
pub(crate) const ARCHIVE_VAR: &str = "YOKOHAMA_WEB_ARCHIVE";

/// The nine keys the shell sends, and no others: an unknown key is a
/// misspelling in the shell, refused rather than ignored, and a missing
/// `identity` is refused too — the door has no default. Read ONLY from the
/// object form ([`parse`]).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InitConfig {
    /// The door (M6.5c2, spec §2.1), as `shellConfig` read it.
    identity: String,
    archive: String,
    /// The Archive object's address (spec §2.6), and `null` on every other
    /// object. Read only under `r2`, where it is the receipt archive's
    /// caller.
    caller: Option<String>,
    // The three chat variables, raw — `null` for a variable the Worker does not
    // set, which the layer's parser reads as its default.
    lobby_chat: Option<String>,
    chat_keep: Option<String>,
    chat_text: Option<String>,
    // The two TTLs in whole days (R-6.5c2.42), as the shell's own readers
    // decided them — `null` while no table expires — for `/api/health` to
    // report ([`Ttls`]).
    table_ttl_days: Option<u64>,
    open_table_ttl_days: Option<u64>,
    /// `YOKOHAMA_DIAGNOSTIC_SEEDS` as the shell read it — `true` in the pool's
    /// test configurations alone; absent or `false` is the production rule,
    /// `SeedPolicy::SoloOnly`.
    diagnostic_seeds: Option<bool>,
}

/// What `init` builds the layer with.
pub(crate) struct Config {
    pub(crate) door: Door,
    pub(crate) chat: ChatConfig,
    pub(crate) archive: Switch,
    pub(crate) ttls: Ttls,
    pub(crate) diagnostic_seeds: bool,
}

/// The Worker's two table TTLs in whole days (R-6.5c2.42), which the host
/// reports on `/api/health` and nothing in this module applies: a table's
/// expiry is the shell's alarm. `table` is `YOKOHAMA_WEB_TABLE_TTL_DAYS` — how
/// long an idle table stands — and `open_table` is
/// `YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS`, how long one still waiting for players
/// does; both `None` while no table expires.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Ttls {
    pub(crate) table: Option<u64>,
    pub(crate) open_table: Option<u64>,
}

/// Which door this Worker has (M6.5c2, spec §2.1; R-6.5c2.5), fixed at `init`
/// with the rest of the configuration: every object builds its module from its
/// OWN configuration, so one deployment is one door everywhere. Two names move
/// with it (review R2.2-m2): the layer's mode a caller is resolved under
/// ([`Door::mode`]), and the door's name on the wire ([`Door::wire_name`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Door {
    /// Cloudflare Access, verified by the shell, which hands the objects the
    /// caller it verified — a person's assertion, or the dev door's claim under
    /// the credential. This repository's configuration.
    Worker,
    /// No identity service (R-6.5c2.3): the name a caller claims is who they
    /// are. The shell holds a claim only to what a header can carry; the
    /// layer's key grammar judges the rest.
    Open,
}

impl Door {
    /// The layer's identity mode for this door.
    pub(crate) fn mode(self) -> IdentityMode {
        match self {
            Self::Worker => IdentityMode::Worker,
            Self::Open => IdentityMode::Open,
        }
    }

    /// The door's name on the wire — `/api/me`'s `mode`, `/api/health`'s
    /// `identity` — in the layer's own words: `cf-access` under `worker`,
    /// because the door a client is behind is Cloudflare Access (the name
    /// `web/src/types.ts` and the client's `useIdentityMode` have read since
    /// M6.5c), and `open` under `open`.
    pub(crate) fn wire_name(self) -> &'static str {
        match self {
            Self::Worker => IdentityMode::CfAccess.name(),
            Self::Open => IdentityMode::Open.name(),
        }
    }
}

/// The archive switch (spec §2.6).
pub(crate) enum Switch {
    /// `off`: `NoArchive`, and every path takes the layer's own off-branch.
    Off,
    /// `r2`: the receipt archive, with the Archive object's address — the
    /// caller whose saves this instance files — or `None` on any other object.
    R2 { caller: Option<String> },
}

/// The configuration `init` builds the layer with.
///
/// **The door first** (M6.5c2, spec §2.1): exactly `worker` or `open`, refused
/// naming its variable otherwise.
///
/// The chat values go through the layer's own `ChatConfig::from_env_values`,
/// whose refusals already name their variables — the NAS's names, parser and
/// defaults, never a copy.
///
/// **Under `r2` the caller is NORMALIZED by the layer's own `User::parse`**,
/// the parse that normalizes every sender, and refused if it is no key — an
/// email, or (M6.5c2) a plain name.
/// The receipt archive reads it whole for spec §2.6's `test/` root, which must
/// read `Red@T.TEST` as the `red@t.test` a sender of that name normalizes to.
/// Under `off` nothing reads it, and it is not checked.
///
/// **The object form only** (R1's L-3): serde's derived `Deserialize` also
/// reads a struct from a positional ARRAY, a form `deny_unknown_fields` does
/// not govern, so anything but an object is refused before serde reads it —
/// the rule this crate keeps for a create body, the picker row and a
/// receipt's outcome. The refusal echoes nothing, since the configuration
/// carries the Archive object's address.
pub(crate) fn parse(config: &str) -> Result<Config, String> {
    let value: serde_json::Value = serde_json::from_str(config)
        .map_err(|e| format!("the module's init configuration does not parse: {e}"))?;
    if !value.is_object() {
        return Err("the module's init configuration is the object of its nine keys — \
                    identity, archive, caller, lobby_chat, chat_keep, chat_text, \
                    table_ttl_days, open_table_ttl_days and diagnostic_seeds"
            .to_string());
    }
    let c: InitConfig = serde_json::from_value(value)
        .map_err(|e| format!("the module's init configuration does not parse: {e}"))?;
    let door = match c.identity.as_str() {
        "worker" => Door::Worker,
        "open" => Door::Open,
        v => return Err(format!("{IDENTITY_VAR} must be worker or open — got {v:?}")),
    };
    let archive = match c.archive.as_str() {
        "off" => Switch::Off,
        "r2" => Switch::R2 { caller: c.caller.as_deref().map(normalized).transpose()? },
        v => return Err(format!("{ARCHIVE_VAR} must be r2 or off — got {v:?}")),
    };
    let chat =
        ChatConfig::from_env_values(c.lobby_chat.as_deref(), c.chat_keep.as_deref(), c.chat_text.as_deref())?;
    let ttls = Ttls { table: c.table_ttl_days, open_table: c.open_table_ttl_days };
    Ok(Config { door, chat, archive, ttls, diagnostic_seeds: c.diagnostic_seeds.unwrap_or(false) })
}

/// The archive caller as the layer normalizes an address.
fn normalized(raw: &str) -> Result<String, String> {
    User::parse(raw)
        .map(|user| user.as_str().to_string())
        .map_err(|e| format!("the archive's caller must be an email or a name: {e}"))
}
