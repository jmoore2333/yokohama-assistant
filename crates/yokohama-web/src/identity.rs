//! Who a request is from — one configured source, one header, one normalized key.
//!
//! The room's first seam (`docs/plans/2026-09-14-web-frontend-m6-room-plan.md` §1).
//! Every mode reads EXACTLY ONE header and ignores every other identity header
//! (spec R-1.2), so a forged header on a path that does not read it changes
//! nothing. There is no default mode (R-1.3): `main.rs` refuses to start
//! without one, the Worker's own fail-closed shape for its secrets.
//!
//! `claimed` is a development convenience and not a security boundary (R-1.4):
//! anyone past the door can claim any email. It distinguishes two callers who
//! send two different emails; what it cannot do is verify that either is who
//! they say. Which mode a given deployment runs under belongs in that
//! deployment's own record, beside the [`ENV_VAR`] its container sets.
//! Reserved and unbuilt: `cf-access-jwt` and `neon`, each a new arm here.

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
    Claimed,
    CfAccess,
    Worker,
}

impl IdentityMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Claimed => "claimed",
            Self::CfAccess => "cf-access",
            Self::Worker => "worker",
        }
    }

    pub fn header(self) -> &'static str {
        match self {
            Self::Claimed => CLAIMED_HEADER,
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
            // **`claimed` needs the build to have been asked for it**
            // (M6.5b2, R-6.5b2.3). The mode itself stays — the variant, the
            // header and every cell that drives one are unconditional, because
            // a mode that vanished from the type would take the browser host's
            // own tests with it. What the feature gates is the one place a
            // DEPLOYMENT can choose it: this parse. A public build drops the
            // feature and the refusal below names it, so the operator reads why
            // rather than guessing at a typo.
            #[cfg(feature = "dev-identity")]
            "claimed" => Ok(Self::Claimed),
            #[cfg(not(feature = "dev-identity"))]
            "claimed" => Err(format!(
                "{ENV_VAR}=claimed needs the `dev-identity` feature, which this build was \
                 compiled without: `claimed` trusts whatever email a caller sends and is a \
                 development door, not a security boundary. Use cf-access or worker, or build \
                 with the feature."
            )),
            "cf-access" => Ok(Self::CfAccess),
            "worker" => Ok(Self::Worker),
            other => Err(format!(
                "{ENV_VAR} must be one of claimed, cf-access, worker — got {other:?}. \
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
    /// The key grammar, for EVERY caller — `resolve`, a create's
    /// `SeatSpec.user`, `check_shape`'s watcher list,
    /// `archive::caller_segment`, the Worker module's `identity_of` and
    /// `init`, and `backup::check_pair` — none of which knows the mode.
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
            return Err(format!("{raw:?} is neither an email nor a name: {NAME_RULE}"));
        }
        if s == "anonymous" {
            return Err(format!("{raw:?} is reserved: `anonymous` is how an edge says nobody"));
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
            return Err(format!("{raw:?} contains whitespace or a control character"));
        }
        let mut parts = s.split('@');
        match (parts.next(), parts.next(), parts.next()) {
            (Some(l), Some(d), None) if !l.is_empty() && !d.is_empty() => Ok(Self(s)),
            _ => Err(format!("{raw:?} is not an email: exactly one @ with something on both sides")),
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

/// The mode's ONE header, parsed; absent or invalid is `Err` with the sentence
/// the 401 carries. The Worker's `anonymous` — what it sends with no Access
/// policy in front of it — counts as absent.
pub fn resolve(mode: IdentityMode, headers: &HeaderMap) -> Result<User, String> {
    let raw = headers
        .get(mode.header())
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty() && !(mode == IdentityMode::Worker && *s == "anonymous"));
    match raw {
        Some(s) => User::parse(s),
        None => Err(match mode {
            IdentityMode::Claimed => "no signed-in user: send X-Yokohama-Claimed-User".to_string(),
            IdentityMode::CfAccess => "no identity reached the server: this deployment expects \
                                       Cf-Access-Authenticated-User-Email from the Cloudflare tunnel"
                .to_string(),
            IdentityMode::Worker => "no identity reached the server: this deployment expects \
                                     X-Yokohama-User from the edge Worker"
                .to_string(),
        }),
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
                table_status: None,
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
        assert_eq!(resolve(IdentityMode::Claimed, &all).unwrap().as_str(), "claimed@t.test");
        assert_eq!(resolve(IdentityMode::CfAccess, &all).unwrap().as_str(), "access@t.test");
        assert_eq!(resolve(IdentityMode::Worker, &all).unwrap().as_str(), "worker@t.test");
    }

    #[test]
    fn a_missing_header_is_absent_and_the_workers_anonymous_counts_as_absent() {
        assert!(resolve(IdentityMode::Claimed, &headers(&[])).is_err());
        assert!(
            resolve(IdentityMode::CfAccess, &headers(&[(CLAIMED_HEADER, "x@y.z")])).is_err(),
            "a forged claimed header on the tunnel path changes nothing"
        );
        assert!(resolve(IdentityMode::Worker, &headers(&[(WORKER_HEADER, "anonymous")])).is_err());
    }

    #[test]
    fn the_mode_comes_from_the_environment_and_has_no_default() {
        // `claimed` is the one arm that depends on a feature since M6.5b2
        // (R-6.5b2.3); both of its shapes are asserted by the cell below.
        #[cfg(feature = "dev-identity")]
        assert!(matches!(IdentityMode::from_str("claimed"), Ok(IdentityMode::Claimed)));
        assert!(matches!(IdentityMode::from_str("cf-access"), Ok(IdentityMode::CfAccess)));
        assert!(matches!(IdentityMode::from_str("worker"), Ok(IdentityMode::Worker)));
        let err = IdentityMode::from_str("").unwrap_err();
        assert!(
            err.contains("claimed") && err.contains("cf-access") && err.contains("worker"),
            "{err}"
        );
        assert!(IdentityMode::from_str("neon").is_err(), "reserved, not built");
        // **The fail-closed path itself.** `from_env` is the only code that
        // turns an UNSET variable into a refusal, and until this line nothing
        // ran it: `main.rs` has no test content and the browser walk always
        // sets the variable, so `Err(_) => Ok(Self::Claimed)` would have
        // deleted R-1.3 with every suite green.
        assert!(IdentityMode::from_env_value(None).is_err(), "unset is a refusal, not a default");
    }

    /// R-6.5b2.3: `claimed` is a DEVELOPMENT door and a build can refuse it.
    /// The feature is in `default`, so this cell runs in the accepting shape and
    /// asserts the mode parses; the refusing shape is asserted by its negation
    /// below, which compiles in both and is the one that would silently vanish if
    /// the `cfg` were written the other way round.
    #[test]
    fn claimed_parses_only_where_the_development_feature_is_compiled_in() {
        let parsed = IdentityMode::from_env_value(Some("claimed"));
        if cfg!(feature = "dev-identity") {
            assert!(matches!(parsed, Ok(IdentityMode::Claimed)));
        } else {
            let err = parsed.expect_err("a build without dev-identity refuses claimed");
            assert!(err.contains("dev-identity"), "the refusal NAMES the feature: {err}");
            assert!(err.contains("cf-access") && err.contains("worker"), "and what to use instead: {err}");
        }
    }
}
