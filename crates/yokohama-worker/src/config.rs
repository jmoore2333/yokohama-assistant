//! **`init`'s configuration** (spec §2.2, §3.1): the JSON the shell builds from
//! the Worker's variables, parsed and refused HERE — so a bad value is refused
//! at `init` naming its variable, the Worker's form of the NAS refusing to
//! start.

use serde::Deserialize;
use yokohama_web::chat::ChatConfig;

/// The archive switch's variable (spec §3.1): `r2` or `off`.
const ARCHIVE_VAR: &str = "YOKOHAMA_WEB_ARCHIVE";

/// The five keys the shell sends, and no others: an unknown key is a
/// misspelling in the shell, refused rather than ignored.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InitConfig {
    archive: String,
    /// The Archive object's address, for the receipt archive (spec §2.6) —
    /// which this build refuses (see [`parse`]), so nothing here reads it.
    /// Declared so the shell's five keys all parse; the `expect` fails the lint
    /// gate the moment a reader arrives, so this sentence cannot outlive its
    /// truth.
    #[expect(dead_code, reason = "read only by the receipt archive, which this build refuses")]
    caller: Option<String>,
    // The three chat variables, raw — `null` for a variable the Worker does not
    // set, which the layer's parser reads as its default.
    lobby_chat: Option<String>,
    chat_keep: Option<String>,
    chat_text: Option<String>,
}

/// The chat configuration `init` builds the layer with.
///
/// The archive switch is checked here and not returned: `off` is the only
/// value this build accepts, and it means `NoArchive`, which `init` builds.
/// The chat values go through the layer's own `ChatConfig::from_env_values`,
/// whose refusals already name their variables — the NAS's names, parser and
/// defaults, never a copy.
pub(crate) fn parse(config: &str) -> Result<ChatConfig, String> {
    let c: InitConfig = serde_json::from_str(config)
        .map_err(|e| format!("the module's init configuration does not parse: {e}"))?;
    match c.archive.as_str() {
        "off" => {}
        "r2" => {
            return Err(format!(
                "{ARCHIVE_VAR}=r2 needs the receipt archive, which this build does not have yet"
            ))
        }
        v => return Err(format!("{ARCHIVE_VAR} must be r2 or off — got {v:?}")),
    }
    ChatConfig::from_env_values(c.lobby_chat.as_deref(), c.chat_keep.as_deref(), c.chat_text.as_deref())
}
