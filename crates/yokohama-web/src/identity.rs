//! Who a request is from — one configured source, one header, one normalized key.
//!
//! The room's first seam (`docs/plans/2026-09-14-web-frontend-m6-room-plan.md` §1).
//! Every mode reads EXACTLY ONE header and ignores every other identity header
//! (spec R-1.2), so a forged header on a path that does not read it changes
//! nothing. There is no default mode (R-1.3): `main.rs` refuses to start
//! without one, the Worker's own fail-closed shape for its secrets.
//!
//! **`open` is the self-host door** (M6.5c2, R-6.5c2.3, superseding R-1.4's
//! framing of `claimed` as a development convenience): players choose a name,
//! and no identity provider is needed. Its trust model is stated rather than
//! hidden — anyone who can reach the server may use any name — and nothing
//! here verifies that a caller is who they say. It is also this project's dev
//! door. `cf-access` and `worker` read an address an edge in front has
//! verified, and the mode rule ([`admit`]) holds their callers to that shape.
//! Which mode a deployment runs under belongs in that deployment's own record,
//! beside the [`ENV_VAR`] its container sets. Reserved and unbuilt:
//! `cf-access-jwt` and `neon`, each a new arm here.

use std::fmt;
use std::str::FromStr;

use http::HeaderMap;

#[cfg(feature = "server")]
use std::sync::Arc;
#[cfg(feature = "server")]
use axum::extract::FromRequestParts;
#[cfg(feature = "server")]
use axum::http::request::Parts;
#[cfg(feature = "server")]
use http::StatusCode;
#[cfg(feature = "server")]
use crate::{ApiError, AppState};
// ---- end server ----

pub const CLAIMED_HEADER: &str = "x-yokohama-claimed-user";
pub const CF_ACCESS_HEADER: &str = "cf-access-authenticated-user-email";
pub const WORKER_HEADER: &str = "x-yokohama-user";
pub const ENV_VAR: &str = "YOKOHAMA_WEB_IDENTITY";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityMode {
    Open,
    CfAccess,
    Worker,
}

impl IdentityMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::CfAccess => "cf-access",
            Self::Worker => "worker",
        }
    }

    pub fn header(self) -> &'static str {
        match self {
            Self::Open => CLAIMED_HEADER,
            Self::CfAccess => CF_ACCESS_HEADER,
            Self::Worker => WORKER_HEADER,
        }
    }

    /// The whole of [`Self::from_env`] except the read, so the UNSET case is
    /// testable without mutating the process environment under a threaded test
    /// runner. An absent variable is the same refusal an unknown value is —
    /// that equality is the whole of R-1.3, and it lived in a line no test
    /// reached until this split.
    pub fn from_env_value(v: Option<&str>) -> Result<Self, String> {
        Self::from_str(v.unwrap_or(""))
    }
}

#[cfg(feature = "server")]
impl IdentityMode {
    /// The read, split from [`Self::from_env_value`] so the parse is testable
    /// without the process environment — and `server`-only since M6.5a, because
    /// a host with no environment (the browser, a Worker) names its mode itself.
    pub fn from_env() -> Result<Self, String> {
        Self::from_env_value(std::env::var(ENV_VAR).ok().as_deref())
    }
}
// ---- end server ----

impl FromStr for IdentityMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s.trim() {
            // **`open` needs the build to have been asked for it** (M6.5b2's
            // R-6.5b2.3; the mode and its feature renamed by M6.5c2's
            // R-6.5c2.3). The mode itself stays — the variant, the header and
            // every cell that drives one are unconditional, because a mode that
            // vanished from the type would take the browser host's own tests
            // with it. What the feature gates is the one place a DEPLOYMENT can
            // choose it: this parse. A build without the feature refuses it
            // here and says why, so the operator reads the trust model rather
            // than guessing at a typo. There is no alias for the old name.
            #[cfg(feature = "open-door")]
            "open" => Ok(Self::Open),
            #[cfg(not(feature = "open-door"))]
            "open" => Err(format!(
                "{ENV_VAR}=open needs the `open-door` feature, which this build was compiled \
                 without. The open door trusts whatever name a caller sends: anyone who can \
                 reach this server may play as anyone, and nothing checks who they are. Use \
                 cf-access or worker behind an identity provider, or build with the feature."
            )),
            "cf-access" => Ok(Self::CfAccess),
            "worker" => Ok(Self::Worker),
            other => Err(format!(
                "{ENV_VAR} must be one of open, cf-access, worker — got {other:?}. \
                 There is no default: an identity source is a deployment decision."
            )),
        }
    }
}

/// A KEY (M6.5c2, spec §2.1 rule 1): what every seat binding, watcher entry
/// and archive directory is written under — an email, or a plain name.
/// Normalized HERE and nowhere else: [`User::parse`] is the one grammar, and
/// it knows no deployment mode; which keys a MODE admits as a caller is
/// [`resolve`]'s rule, not this one.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct User(String);

/// The longest plain name, in characters (every one of them ASCII).
const NAME_MAX: usize = 32;

/// A plain name's rule, as the refusal words it.
const NAME_RULE: &str = "a name is 1 to 32 letters, digits, spaces, dots, dashes or \
                         underscores, and begins with a letter or a digit";

impl User {
    /// The key grammar, for every caller — among them `resolve`, a create's
    /// `SeatSpec.user`, `tables::check_shape`'s watcher list (and through it
    /// `backup::check_pair`), `chat::check_shape`'s authors, the browser host's
    /// `dispatch_inner`, and the Worker module's `identity_of`, `init`,
    /// `ReceiptArchive::root` and picker row. It knows no mode: which keys a
    /// mode admits is [`admit`]'s rule, which `resolve` and `identity_of` apply
    /// after it.
    ///
    /// The value's [`Self::typed_form`] (trimmed, each run of internal spaces
    /// collapsed to one), lowercased by ASCII rules, is then one of two shapes:
    /// - **an email**, exactly today's shape: one `@` with something on both
    ///   sides, no whitespace or control character, at most 254 bytes. A shape
    ///   check, not RFC validation.
    /// - **a plain name**: 1 to 32 ASCII letters, digits, spaces, `.`, `-` and
    ///   `_`, **beginning with a letter or a digit**, and never the literal
    ///   `anonymous`. The leading character is what keeps a name's archive
    ///   segment from being `.` or `..` (review H1, measured); `anonymous` is
    ///   refused so it never becomes a key (the Worker's HEADER `anonymous`
    ///   reads as absent — a separate rule, in [`resolve`]).
    ///
    /// The `@` decides which shape is being checked, because a name's alphabet
    /// has none: an email is never a name, and a name never an email.
    pub fn parse(raw: &str) -> Result<Self, String> {
        let s = Self::typed_form(raw).to_ascii_lowercase();
        if s.is_empty() {
            return Err("no name or email given".to_string());
        }
        if s.contains('@') {
            return Self::email(raw, s);
        }
        let alphabet = s
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b' ' | b'.' | b'-' | b'_'));
        if s.len() > NAME_MAX || !alphabet || !s.as_bytes()[0].is_ascii_alphanumeric() {
            return Err(format!("\"{raw}\" is neither an email nor a name: {NAME_RULE}", raw = crate::echo::echo(raw)));
        }
        if s == "anonymous" {
            return Err(format!("\"{raw}\" is reserved: `anonymous` is how an edge says nobody", raw = crate::echo::echo(raw)));
        }
        Ok(Self(s))
    }

    /// Today's email rule, over the already-normalized `s`; `raw` is quoted in
    /// the refusal as the caller sent it.
    fn email(raw: &str, s: String) -> Result<Self, String> {
        if s.len() > 254 {
            return Err("an email is at most 254 bytes".to_string());
        }
        if s.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(format!("\"{raw}\" contains whitespace or a control character", raw = crate::echo::echo(raw)));
        }
        let mut parts = s.split('@');
        match (parts.next(), parts.next(), parts.next()) {
            (Some(l), Some(d), None) if !l.is_empty() && !d.is_empty() => Ok(Self(s)),
            _ => Err(format!("\"{raw}\" is not an email: exactly one @ with something on both sides", raw = crate::echo::echo(raw))),
        }
    }

    /// What a person typed, tidied: `raw` trimmed (Rust's `str::trim`, Unicode
    /// white space), each run of internal SPACES collapsed to one, case KEPT.
    /// Everything [`Self::parse`] does to a value before it lowercases and
    /// checks it, so an accepted key is exactly this, ASCII-lowercased — which
    /// is why the client may show it as the player's name (`"  Ben   Castle "`
    /// shows as `Ben Castle`, keyed `ben castle`). Emitted beside each key in
    /// `web/src/wire/identity_cases.json`, which the client's name step is
    /// pinned to.
    pub fn typed_form(raw: &str) -> String {
        raw.trim().split(' ').filter(|part| !part.is_empty()).collect::<Vec<_>>().join(" ")
    }

    /// Whether this key has the email shape — the only shape a key with an `@`
    /// can have. The mode rule's test: under `cf-access` and `worker` a caller
    /// must be one.
    pub fn is_email(&self) -> bool {
        self.0.contains('@')
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The mode's ONE header, parsed by the key grammar and then held to the
/// mode's rule ([`admit`]); absent or invalid is `Err` with the sentence the
/// 401 carries. The Worker's `anonymous` — what it sends with no Access policy
/// in front of it — counts as absent.
pub fn resolve(mode: IdentityMode, headers: &HeaderMap) -> Result<User, String> {
    let raw = headers
        .get(mode.header())
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty() && !(mode == IdentityMode::Worker && *s == "anonymous"));
    match raw {
        Some(s) => User::parse(s).and_then(|user| admit(mode, user)),
        None => Err(absent(mode)),
    }
}

/// **The mode rule** (M6.5c2, spec §2.1 rule 2), in ONE place: under `open`
/// every key the grammar accepts is a caller; under `cf-access` and `worker` a
/// caller must also have the email shape, and a plain name reads as ABSENT —
/// the sentence no header at all gets, so `/api/me` answers `user: null` (the
/// client's state B) rather than the client drawing a refusal of the name. An
/// edge that asserts an identity asserts an address; anything else means the
/// edge in front is not the one this deployment expects.
///
/// A value the GRAMMAR refuses is refused before this, in the grammar's own
/// words, under every mode — as it was before this rule. `pub` for the Worker
/// module, whose `identity_of` takes these same two steps for a value no
/// header can carry (review R2-m1).
pub fn admit(mode: IdentityMode, user: User) -> Result<User, String> {
    match mode {
        IdentityMode::Open => Ok(user),
        IdentityMode::CfAccess | IdentityMode::Worker if user.is_email() => Ok(user),
        IdentityMode::CfAccess | IdentityMode::Worker => Err(absent(mode)),
    }
}

/// The 401's sentence when no caller reached the server, per mode.
fn absent(mode: IdentityMode) -> String {
    match mode {
        IdentityMode::Open => "no signed-in user: send X-Yokohama-Claimed-User".to_string(),
        IdentityMode::CfAccess => "no identity reached the server: this deployment expects \
                                   Cf-Access-Authenticated-User-Email from the Cloudflare tunnel"
            .to_string(),
        IdentityMode::Worker => "no identity reached the server: this deployment expects \
                                 X-Yokohama-User from the edge Worker"
            .to_string(),
    }
}

/// The extractor every `/api/games` route takes. A missing identity is a 401 in
/// this API's `{"error": …}` shape.
#[cfg(feature = "server")]
pub struct Identity(pub User);

#[cfg(feature = "server")]
impl FromRequestParts<Arc<AppState>> for Identity {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &Arc<AppState>) -> Result<Self, ApiError> {
        resolve(state.identity, &parts.headers)
            .map(Identity)
            .map_err(|message| ApiError {
                status: StatusCode::UNAUTHORIZED,
                message,
                table_status: None, foreign_bot: None,
            })
    }
}

// ---- end server ----

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(
                axum::http::HeaderName::from_bytes(k.as_bytes()).expect("a header name"),
                v.parse().expect("a header value"),
            );
        }
        h
    }

    /// **The key grammar** (M6.5c2, spec §2.1 rule 1): a key is an email OR a
    /// plain name, and one rule normalizes both — trim, collapse each run of
    /// internal spaces to one, lowercase by ASCII rules. Both families' bounds
    /// are asserted from both sides, so a grammar that refused everything, or
    /// took everything, fails here rather than in a lobby.
    #[test]
    fn a_key_is_an_email_or_a_plain_name_normalized_by_one_rule() {
        for (raw, key) in [
            ("  A@Example.COM ", "a@example.com"),
            ("a+b@x.test", "a+b@x.test"),
            ("  Ben   Castle ", "ben castle"),
            ("BEN", "ben"),
            ("x", "x"),
            ("7 wonders", "7 wonders"),
            ("r2-d2_x.y", "r2-d2_x.y"),
            ("a.test", "a.test"),
            ("ben\t", "ben"),
        ] {
            assert_eq!(User::parse(raw).map(|u| u.as_str().to_string()), Ok(key.to_string()), "{raw:?}");
        }
        for bad in [
            "", "   ", "two@@signs", "a@b c", "@example.com", "a@", "a\u{7}@b.c",
            "Bén", "ben\tcastle", "a/b", "ben!",
        ] {
            assert!(User::parse(bad).is_err(), "{bad:?} must be refused");
        }
        // The two length bounds, each from both sides.
        let email = |n: usize| format!("{}@x.y", "a".repeat(n - 4));
        assert!(User::parse(&email(254)).is_ok(), "254 bytes is an email");
        assert!(User::parse(&email(255)).is_err(), "255 bytes is one too many");
        assert!(User::parse(&"n".repeat(32)).is_ok(), "32 characters is a name");
        assert!(User::parse(&"n".repeat(33)).is_err(), "33 is one too many");
    }

    /// **The names that would name a path step, or nobody, are never keys**
    /// (M6.5c2, spec §2.1; review H1, MEASURED: an unconstrained `..` became
    /// the archive segment `..` and landed a browser's save in the server's
    /// own `rev7/over/`). A plain name must BEGIN with a letter or a digit, and
    /// the literal `anonymous` — how an unprotected edge says "nobody" — is
    /// refused in any case. The accepted half is the anti-vacuity: a dot, a
    /// dash or the word itself later in a name is fine.
    #[test]
    fn a_name_that_could_be_a_path_step_or_nobody_is_never_a_key() {
        for bad in [".", "..", ". .", "-", ".x", "_x", "-x", " .. ", "anonymous", "Anonymous", " ANONYMOUS "] {
            assert!(User::parse(bad).is_err(), "{bad:?} must never be a key");
        }
        for good in ["x.", "a..b", "z-", "anonymous1", "anon ymous", "anonymous@x.test"] {
            assert!(User::parse(good).is_ok(), "{good:?} is a key");
        }
    }

    /// **(Review Focus 1)** A name typed with capitals and extra spaces is ONE
    /// key, and the same person typing it plainly later is the same key — so
    /// they reach the same seats. The typed form keeps the case, for the screen
    /// to show; the key is that form lowercased and nothing else.
    /// [`User::is_email`] says which of the two shapes a key has.
    #[test]
    fn a_name_typed_with_capitals_and_extra_spaces_is_one_key_shown_as_typed() {
        let typed = User::parse("  Ben   Castle ").unwrap();
        assert_eq!(typed.as_str(), "ben castle");
        assert_eq!(User::parse("ben castle").unwrap(), typed, "the same person, typed plainly");
        assert_eq!(User::parse("BEN CASTLE").unwrap(), typed);
        assert_eq!(User::typed_form("  Ben   Castle "), "Ben Castle", "shown as typed, tidied");
        assert_eq!(User::typed_form("  Red@Wire.TEST "), "Red@Wire.TEST");
        assert!(!typed.is_email());
        assert!(User::parse("Red@Wire.TEST").unwrap().is_email());
        assert!(!User::parse("a.test").unwrap().is_email(), "a dot is not an address");
    }

    /// R-1.2: a request carrying all three headers with three different values
    /// resolves to the MODE's own, under each mode. A mutation that reads any
    /// header regardless of mode fails all three arms at once.
    #[test]
    fn every_mode_reads_its_own_header_and_only_that_one() {
        let all = headers(&[
            (CLAIMED_HEADER, "claimed@t.test"),
            (CF_ACCESS_HEADER, "access@t.test"),
            (WORKER_HEADER, "worker@t.test"),
        ]);
        assert_eq!(resolve(IdentityMode::Open, &all).unwrap().as_str(), "claimed@t.test");
        assert_eq!(resolve(IdentityMode::CfAccess, &all).unwrap().as_str(), "access@t.test");
        assert_eq!(resolve(IdentityMode::Worker, &all).unwrap().as_str(), "worker@t.test");
    }

    #[test]
    fn a_missing_header_is_absent_and_the_workers_anonymous_counts_as_absent() {
        assert!(resolve(IdentityMode::Open, &headers(&[])).is_err());
        assert!(
            resolve(IdentityMode::CfAccess, &headers(&[(CLAIMED_HEADER, "x@y.z")])).is_err(),
            "a forged claimed header on the tunnel path changes nothing"
        );
        assert!(resolve(IdentityMode::Worker, &headers(&[(WORKER_HEADER, "anonymous")])).is_err());
    }

    #[test]
    fn the_mode_comes_from_the_environment_and_has_no_default() {
        // `open` is the one arm that depends on a feature (R-6.5b2.3, renamed
        // by R-6.5c2.3); both of its shapes are asserted by the cell below.
        #[cfg(feature = "open-door")]
        assert!(matches!(IdentityMode::from_str("open"), Ok(IdentityMode::Open)));
        assert!(matches!(IdentityMode::from_str("cf-access"), Ok(IdentityMode::CfAccess)));
        assert!(matches!(IdentityMode::from_str("worker"), Ok(IdentityMode::Worker)));
        let err = IdentityMode::from_str("").unwrap_err();
        assert!(err.contains("open") && err.contains("cf-access") && err.contains("worker"), "{err}");
        assert!(IdentityMode::from_str("neon").is_err(), "reserved, not built");
        // **No alias** (spec §2.1, "The rename"): a configuration still naming
        // the old mode is refused at start-up, and the refusal lists the three
        // it may say instead — `open` among them.
        let old = IdentityMode::from_str("claimed").unwrap_err();
        assert!(old.contains("\"claimed\"") && old.contains("open"), "{old}");
        // **The fail-closed path itself.** `from_env` is the only code that
        // turns an UNSET variable into a refusal, and until this line nothing
        // ran it: `main.rs` has no test content and the browser walk always
        // sets the variable, so `Err(_) => Ok(Self::Open)` would have
        // deleted R-1.3 with every suite green.
        assert!(IdentityMode::from_env_value(None).is_err(), "unset is a refusal, not a default");
    }

    /// R-6.5c2.3 (superseding R-1.4's framing, and renaming R-6.5b2.3's
    /// feature): `open` is a supported self-host door, and a build can still
    /// refuse it. The feature is in `default`, so this cell runs in the
    /// accepting shape and asserts the mode parses; the refusing shape runs
    /// only in the public build CI compiles, and there the refusal must NAME
    /// the feature, the two modes to use instead, and the trust model.
    #[test]
    fn open_parses_only_where_the_open_door_feature_is_compiled_in() {
        let parsed = IdentityMode::from_env_value(Some("open"));
        if cfg!(feature = "open-door") {
            assert!(matches!(parsed, Ok(IdentityMode::Open)));
        } else {
            let err = parsed.expect_err("a build without open-door refuses open");
            assert!(err.contains("open-door"), "the refusal NAMES the feature: {err}");
            assert!(err.contains("cf-access") && err.contains("worker"), "and what to use instead: {err}");
            assert!(err.contains("anyone who can reach"), "and the trust model it declines: {err}");
        }
    }

    /// **The mode rule** (M6.5c2, spec §2.1 rule 2), at `resolve`: under `open`
    /// every key the grammar accepts is a caller, a plain name included; under
    /// `cf-access` and `worker` a caller must also have the email shape, and a
    /// plain name reads as ABSENT — byte for byte the sentence no header gets,
    /// which is what makes `/api/me` answer `user: null` (state B) rather than
    /// the client drawing a refusal of the name. An address still resolves
    /// under every mode, so the rule is the shape and not the header.
    #[test]
    fn the_mode_rule_admits_a_plain_name_only_under_open() {
        let named = |mode: IdentityMode, value: &str| resolve(mode, &headers(&[(mode.header(), value)]));
        assert_eq!(named(IdentityMode::Open, "  Ben   Castle ").unwrap().as_str(), "ben castle");
        for mode in [IdentityMode::CfAccess, IdentityMode::Worker] {
            let absent = resolve(mode, &headers(&[])).unwrap_err();
            assert_eq!(named(mode, "ben").unwrap_err(), absent, "{}: a name is nobody here", mode.name());
            assert_eq!(named(mode, "Red@Wire.TEST").unwrap().as_str(), "red@wire.test", "{}", mode.name());
        }
        // The grammar still speaks first: what is no key at all is refused in
        // its own words under every mode, never mistaken for a name.
        assert_eq!(named(IdentityMode::CfAccess, "..").unwrap_err(), User::parse("..").unwrap_err());
    }
}
