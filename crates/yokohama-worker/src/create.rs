//! **`create`** (spec §2.2; §2.3's create row): `POST /api/games` on this host,
//! where the Worker NAMES the object before the layer runs — by the body's `id`
//! when the body is a JSON object whose `id` is a string, else by a name it
//! draws — and the ORIGINAL bytes reach this module with that name. So every
//! 201 must name the object that holds the table: this module makes the body
//! carry that name, or refuses it.
//!
//! **Nothing is re-serialized** (spec §0.4): a round trip through a JSON value
//! turned a duplicate-key 400 into a 201 and moved an unknown field's column.
//! The only bytes this module writes into a body are the inserted `,"id":…`
//! member. Any other body it forwards reaches the layer exactly as it arrived,
//! so one the layer's parser refuses is answered in the layer's words, byte for
//! byte: axum's for every shape but the positional array, which since M6.5c2 is
//! `api::NOT_AN_OBJECT`.

use yokohama_web::api;
use yokohama_web::tables::NewGame;
use yokohama_web::ApiError;

use crate::{dispatch_inner, host_refusal, identity_of, ReplyData};

/// The two sentences this route writes itself — spec §4's host-written list,
/// less the one the layer took over in M6.5c2.
const NULL_ID: &str = "a game id, when given, must be a string";
const MISROUTED: &str = "the create's id does not name the table it was routed to";

/// `POST /api/games` for the object named `id`, in this order:
///
/// - **The caller first** (the orchestrator's ruling at Task 3). With no caller
///   the body goes to the layer UNCHANGED, and the layer's `dispatch` reads the
///   caller before the body — so a caller-less create is the layer's own 401
///   whatever the body, as it is on the NAS, and never one of the host-written
///   sentences below.
/// - **The layer's own `parse_json` into its own `NewGame` decides.** A body it
///   refuses — a duplicate key, an unknown field, a missing field, a non-string
///   `id`, a missing content type, an unparseable body, a non-object it cannot
///   read — goes to the layer UNCHANGED, and the layer's 400 answers it.
/// - **A body that is not a JSON OBJECT is the layer's 400 since M6.5c2** (spec
///   §2.6). serde's derived `Deserialize` reads a struct from a positional ARRAY
///   too, and the layer used to answer such a body 201, a quirk of the NAS as
///   well. The layer's `parse_json` now refuses it in its own words
///   (`api::NOT_AN_OBJECT`), so it reaches the layer unchanged, as every body
///   the layer refuses does. The Worker names an object from an object's `id`
///   alone, and that holds by the layer's rule.
/// - **A string `id` is the name the Worker routed by**, and the body goes on
///   unchanged. One that is NOT this object's name cannot arrive through the
///   Worker once the array arm holds — spec §2.2: an object is read the same way
///   by both parsers, and one they read differently fails the layer's parse. It
///   is still refused 500, so a routing defect is loud rather than a 201 for an
///   object that holds nothing.
/// - **An `id` key the layer reads as absent — `"id": null` — is refused 400.**
///   Forwarded, it would have the layer draw an id of its own after the Worker
///   had already named the object: the re-review measured a 201 naming a table
///   no object held (spec §0.4).
/// - **No `id` key**: the object's name is inserted as the body's last member
///   (`with_id`), and the body goes on.
pub fn create_inner(user: Option<&str>, body: &[u8], id: &str, content_type: Option<&str>) -> ReplyData {
    let forward = |body: &[u8]| dispatch_inner(user, "POST", "/api/games", "", content_type, body);
    // Caller first: a create with no caller gets the layer's 401, never a host sentence (spec §4).
    if identity_of(user).is_err() {
        return forward(body);
    }
    let Ok(game) = api::parse_json::<NewGame>(content_type, body) else {
        return forward(body);
    };
    // The second look, which only DECIDES: nothing it reads reaches the layer.
    // On an object it says whether an `id` key is there at all, which `game.id`
    // cannot: `"id": null` reads as `None`. **It meets nothing but an object
    // since M6.5c2** (spec §2.6): the layer's `parse_json` refuses any other
    // body before this line, and this module forwards that body like every
    // other the layer refuses. The arm stays as the invariant's guard, in the
    // layer's own words.
    let Ok(object) = serde_json::from_slice::<serde_json::Map<String, serde_json::Value>>(body) else {
        return host_refusal(ApiError::bad_request(api::NOT_AN_OBJECT));
    };
    match game.id {
        Some(named) if named == id => forward(body),
        Some(_) => host_refusal(ApiError::server_error(MISROUTED)),
        None if object.contains_key("id") => host_refusal(ApiError::bad_request(NULL_ID)),
        None => forward(&with_id(body, id)),
    }
}

/// `body` with `,"id":<name>` inserted immediately before the object's FINAL `}`.
///
/// **That brace is the last byte that is not JSON whitespace** — space, tab,
/// `\n` or `\r`, serde_json's set — and NOT the body's last byte: a file most
/// editors save ends in a newline, and `curl --data-binary @file` sends it as
/// it is (Review Focus 1). The caller has already read the body as an object
/// with nothing but whitespace after it, which is what makes that byte the
/// object's closing brace; and `seats` is a required field, so a member always
/// precedes the brace and the leading comma is always right. The name is
/// written by `serde_json::to_string`, so it is a JSON string whatever it holds.
fn with_id(body: &[u8], name: &str) -> Vec<u8> {
    let close = body
        .iter()
        .rposition(|b| !matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
        .expect("a body that parsed as an object is not all whitespace");
    let member = format!(",\"id\":{}", serde_json::to_string(name).expect("a string serializes"));
    [&body[..close], member.as_bytes(), &body[close..]].concat()
}
