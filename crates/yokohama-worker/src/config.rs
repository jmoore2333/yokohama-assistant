//! **`init`'s configuration** (spec §2.2, §3.1): the JSON the shell builds from
//! the Worker's variables, parsed and refused HERE — so a bad VALUE is refused
//! at `init` naming its variable, the Worker's form of the NAS refusing to
//! start. A configuration of the wrong SHAPE names no variable: JSON that is
//! not an OBJECT is refused in this crate's own sentence, which names the five
//! keys ([`parse`]); text that is not JSON, and an object with a key it does not
//! have or a value of the wrong JSON type, such as `{"chat_keep": 500}` for the
//! string `"500"`, in serde's words. The shell always builds the object, with
//! every value a string or `null` (`worker/src/config.ts`'s `moduleConfig`, whose
//! `shellConfig` refuses a variable that is not a string first: the
//! orchestrator's ruling E3), so no deployment reaches those.

use serde::Deserialize;
use yokohama_web::chat::ChatConfig;
use yokohama_web::identity::User;

/// The archive switch's variable (spec §3.1): `r2` or `off`.
pub(crate) const ARCHIVE_VAR: &str = "YOKOHAMA_WEB_ARCHIVE";

/// The five keys the shell sends, and no others: an unknown key is a
/// misspelling in the shell, refused rather than ignored. Read ONLY from the
/// object form ([`parse`]).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InitConfig {
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
}

/// What `init` builds the layer with.
pub(crate) struct Config {
    pub(crate) chat: ChatConfig,
    pub(crate) archive: Switch,
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
        return Err("the module's init configuration is the object of its five keys — \
                    archive, caller, lobby_chat, chat_keep and chat_text"
            .to_string());
    }
    let c: InitConfig = serde_json::from_value(value)
        .map_err(|e| format!("the module's init configuration does not parse: {e}"))?;
    let archive = match c.archive.as_str() {
        "off" => Switch::Off,
        "r2" => Switch::R2 { caller: c.caller.as_deref().map(normalized).transpose()? },
        v => return Err(format!("{ARCHIVE_VAR} must be r2 or off — got {v:?}")),
    };
    let chat =
        ChatConfig::from_env_values(c.lobby_chat.as_deref(), c.chat_keep.as_deref(), c.chat_text.as_deref())?;
    Ok(Config { chat, archive })
}

/// The archive caller as the layer normalizes an address.
fn normalized(raw: &str) -> Result<String, String> {
    User::parse(raw)
        .map(|user| user.as_str().to_string())
        .map_err(|e| format!("the archive's caller must be an email or a name: {e}"))
}
