//! Who a request is from — one configured source, one header, one normalized email.
//!
//! The room's first seam (`docs/plans/2026-09-14-web-frontend-m6-room-plan.md` §1).
//! Every mode reads EXACTLY ONE header and ignores every other identity header
//! (spec R-1.2), so a forged header on a path that does not read it changes
//! nothing. There is no default mode (R-1.3): `main.rs` refuses to start
//! without one, the Worker's own fail-closed shape for its secrets.
//!
//! `claimed` is a development convenience and not a security boundary (R-1.4):
//! anyone past the door can claim any email. The runbook says so beside the
//! mode. Reserved and unbuilt: `cf-access-jwt` and `neon`, each a new arm here.

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{HeaderMap, StatusCode};

use crate::{ApiError, AppState};

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

    pub fn from_env() -> Result<Self, String> {
        Self::from_env_value(std::env::var(ENV_VAR).ok().as_deref())
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

impl FromStr for IdentityMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s.trim() {
            "claimed" => Ok(Self::Claimed),
            "cf-access" => Ok(Self::CfAccess),
            "worker" => Ok(Self::Worker),
            other => Err(format!(
                "{ENV_VAR} must be one of claimed, cf-access, worker — got {other:?}. \
                 There is no default: an identity source is a deployment decision."
            )),
        }
    }
}

/// A normalized email: trimmed, lowercased, exactly one `@` with both sides
/// non-empty, no whitespace or control characters, at most 254 bytes (R-1.1).
/// A shape check, not RFC validation. Normalized HERE and nowhere else — this
/// is the key every seat binding is written under.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct User(String);

impl User {
    pub fn parse(raw: &str) -> Result<Self, String> {
        let s = raw.trim().to_ascii_lowercase();
        if s.is_empty() {
            return Err("no email given".to_string());
        }
        if s.len() > 254 {
            return Err("an email is at most 254 bytes".to_string());
        }
        if s.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(format!("{raw:?} contains whitespace or a control character"));
        }
        let mut parts = s.split('@');
        let (local, domain, rest) = (parts.next(), parts.next(), parts.next());
        match (local, domain, rest) {
            (Some(l), Some(d), None) if !l.is_empty() && !d.is_empty() => Ok(Self(s)),
            _ => {
                Err(format!("{raw:?} is not an email: exactly one @ with something on both sides"))
            }
        }
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
pub struct Identity(pub User);

impl FromRequestParts<Arc<AppState>> for Identity {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &Arc<AppState>) -> Result<Self, ApiError> {
        resolve(state.identity, &parts.headers)
            .map(Identity)
            .map_err(|message| ApiError { status: StatusCode::UNAUTHORIZED, message })
    }
}

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

    #[test]
    fn an_email_is_normalized_once_and_the_shapes_that_are_not_one_are_refused() {
        assert_eq!(User::parse("  A@Example.COM ").expect("an email").as_str(), "a@example.com");
        for bad in ["", "nobody", "two@@signs", "a@b c", "@example.com", "a@", "a\u{7}@b.c"] {
            assert!(User::parse(bad).is_err(), "{bad:?} must be refused");
        }
        let long = format!("{}@x.y", "a".repeat(252));
        assert!(User::parse(&long).is_err(), "255 bytes is one too many");
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
}
