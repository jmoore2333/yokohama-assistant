//! **The `Host` allow-list** (security S3, finding D4; decision D-8; R-S3-14).
//!
//! The NAS binary answers only the host names it is told to.
//! A DNS-rebinding page reaches the binary under the attacker's name, and the only thing that name cannot forge is the `Host` the browser sends.
//! So the comparison is on the host part of the request's authority, never the port: a rebinding attack rebinds a NAME.
//!
//! Two variables, both read once by `main`:
//! - [`HOSTNAMES_ENABLED_VAR`] is the switch.
//!   Absent or `on` turns the check on, `off` answers any `Host` as the binary did before, and any other value refuses to start naming it.
//! - [`HOSTNAMES_VAR`] is a comma-separated list of names, added to the default set.
//!   The default set is the loopback names plus the bind address when it is a specific one.
//!
//! This file is free of server-only imports on purpose: it needs `std` and [`crate::echo`], so it builds for every target the crate does.
//! The middleware that applies it is `host_allowed` in `lib.rs`, and the state field is `AppState::hostnames`.

use std::net::{IpAddr, Ipv6Addr};

use crate::echo::echo;

/// The list's variable, held here so the read, the refusal sentence and the cells name one spelling.
pub const HOSTNAMES_VAR: &str = "YOKOHAMA_WEB_HOSTNAMES";

/// The switch's variable.
pub const HOSTNAMES_ENABLED_VAR: &str = "YOKOHAMA_WEB_HOSTNAMES_ENABLED";

/// The fixed part of the 421 sentence; the offered host follows it, quoted and echoed through [`echo`].
/// It names no listed host, so a refusal teaches a prober nothing about what the server answers to.
pub const NOT_FOR_THIS_HOST: &str = "This server does not answer to the host name";

/// The names every on-state answers to before the operator lists any.
const LOOPBACK: [&str; 3] = ["localhost", "127.0.0.1", "::1"];

/// Which host names the binary answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hostnames {
    /// The switch is `off`: any `Host` is answered, and so is none.
    Off,
    /// The check is on.
    /// The names are lower-cased, carry no trailing dot, and write an IPv6 literal in its canonical form without brackets.
    On { names: Vec<String> },
}

impl Hostnames {
    /// The on-state with the default set for `bind` and nothing listed: what every constructor of the state builds.
    pub fn on_for_bind(bind: &str) -> Self {
        Self::On { names: default_names(bind) }
    }

    /// The two variables' values and the bind address, parsed.
    /// The switch is read first, exactly: `None` and `on` are on, `off` is off, and every other value — a different case and a stray space included — is an `Err` naming the variable and the value.
    /// Off ignores the list.
    /// On with no list is the default set; on with a list is the default set plus each entry, trimmed and checked.
    pub fn from_env_values(enabled: Option<&str>, raw: Option<&str>, bind: &str) -> Result<Self, String> {
        match enabled {
            None | Some("on") => {}
            Some("off") => return Ok(Self::Off),
            Some(other) => {
                return Err(format!("{HOSTNAMES_ENABLED_VAR} must be on or off — got \"{}\"", echo(other)));
            }
        }
        let mut names = default_names(bind);
        if let Some(raw) = raw {
            for entry in raw.split(',') {
                push_new(&mut names, checked_entry(entry.trim())?);
            }
        }
        Ok(Self::On { names })
    }

    /// Whether the check is running.
    pub fn is_on(&self) -> bool {
        matches!(self, Self::On { .. })
    }

    /// Whether a request whose `Host` (or HTTP/2 `:authority`) is `authority` is answered.
    /// Always true when off.
    /// When on: the host part is compared, case-insensitively, with one trailing dot and the port ignored.
    /// An authority that is not a host and an optional port — empty, a userinfo, an unclosed bracket, a port that is not digits — is refused.
    pub fn admits(&self, authority: &str) -> bool {
        match self {
            Self::Off => true,
            Self::On { names } => host_of(authority).is_some_and(|host| names.contains(&host)),
        }
    }

    /// The names answered, in the order the start line prints them: the defaults, then the listed.
    /// Empty when off.
    pub fn listed(&self) -> Vec<String> {
        match self {
            Self::Off => Vec::new(),
            Self::On { names } => names.clone(),
        }
    }

    /// The line `main` prints at start, so an operator reads the state in the log.
    pub fn start_line(&self) -> String {
        match self {
            Self::Off => format!("yokohama-web answers any host name — {HOSTNAMES_ENABLED_VAR} is off"),
            Self::On { names } => format!("yokohama-web answers host names: {}", names.join(", ")),
        }
    }
}

/// The loopback names, then the bind address when it is a specific one: a wildcard bind is no name.
fn default_names(bind: &str) -> Vec<String> {
    let mut names: Vec<String> = LOOPBACK.iter().map(|n| (*n).to_string()).collect();
    let bind = bind.trim();
    let wildcard = bind.parse::<IpAddr>().is_ok_and(|ip| ip.is_unspecified());
    if !bind.is_empty() && !wildcard {
        push_new(&mut names, normalize(bind));
    }
    names
}

fn push_new(names: &mut Vec<String>, name: String) {
    if !names.contains(&name) {
        names.push(name);
    }
}

/// A host in the one spelling the set and the comparison share: an IPv6 literal in its canonical form, anything else lower-cased.
fn normalize(host: &str) -> String {
    match host.parse::<Ipv6Addr>() {
        Ok(v6) => v6.to_string(),
        Err(_) => host.to_ascii_lowercase(),
    }
}

/// One list entry, trimmed, or the sentence that refuses it.
/// A host name has no whitespace, scheme, path, user or port, so the entry is refused when it is empty or holds whitespace, `/`, `@` or `#`, or a `:` that is not an IPv6 literal.
fn checked_entry(entry: &str) -> Result<String, String> {
    let bad = entry.is_empty()
        || entry.chars().any(|c| c.is_whitespace() || matches!(c, '/' | '@' | '#'))
        || (entry.contains(':') && entry.parse::<Ipv6Addr>().is_err());
    if bad {
        return Err(format!(
            "{HOSTNAMES_VAR}: \"{}\" is not a host name — a name has no space, scheme, path, user or port, and an IPv6 literal is written without brackets",
            echo(entry)
        ));
    }
    Ok(normalize(entry.strip_suffix('.').unwrap_or(entry)))
}

/// The host part of a request's authority in [`normalize`]'s spelling, or `None` when the authority is not a host with an optional port.
fn host_of(authority: &str) -> Option<String> {
    let (host, port) = if let Some(rest) = authority.strip_prefix('[') {
        // A bracketed literal ends its host at `]`, and only `:port` may follow.
        let (inner, after) = rest.split_once(']')?;
        let port = if after.is_empty() { "" } else { after.strip_prefix(':')? };
        if inner.parse::<Ipv6Addr>().is_err() {
            return None;
        }
        (inner, port)
    } else if authority.matches(':').count() > 1 {
        // Unbracketed with several colons: only a whole IPv6 literal with no port is a host.
        (authority, "")
    } else {
        authority.split_once(':').unwrap_or((authority, ""))
    };
    if !port.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let host = host.strip_suffix('.').unwrap_or(host);
    if host.is_empty() {
        return None;
    }
    Some(normalize(host))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on(raw: Option<&str>, bind: &str) -> Hostnames {
        Hostnames::from_env_values(None, raw, bind).expect("a well-formed list")
    }

    #[test]
    fn the_default_set_is_loopback_plus_a_specific_bind_address() {
        assert_eq!(on(None, "0.0.0.0").listed(), ["localhost", "127.0.0.1", "::1"]);
        assert_eq!(on(None, "10.0.0.5").listed(), ["localhost", "127.0.0.1", "::1", "10.0.0.5"]);
        assert_eq!(on(None, "::").listed(), ["localhost", "127.0.0.1", "::1"]);
        // A bind that is already a default is not listed twice, and a bound IPv6 address is the header's spelling.
        assert_eq!(on(None, "127.0.0.1").listed(), ["localhost", "127.0.0.1", "::1"]);
        assert_eq!(on(None, "fd00::5").listed(), ["localhost", "127.0.0.1", "::1", "fd00::5"]);
        assert!(on(None, "fd00::5").admits("[fd00::5]:8080"));
    }

    #[test]
    fn a_listed_name_passes_in_every_spelling_a_client_sends() {
        let h = on(Some("yokohama-nas.moore.nyc"), "0.0.0.0");
        for spelling in [
            "yokohama-nas.moore.nyc",
            "YOKOHAMA-NAS.moore.nyc",
            "yokohama-nas.moore.nyc.",
            "yokohama-nas.moore.nyc:8080",
        ] {
            assert!(h.admits(spelling), "{spelling}");
        }
        for spelling in ["localhost:9999", "[::1]:8080", "127.0.0.1", "LOCALHOST.", "[::1]", "[0:0:0:0:0:0:0:1]:80", "localhost:"] {
            assert!(h.admits(spelling), "{spelling}");
        }
        // The list is printed after the defaults, in the order given.
        assert_eq!(h.listed(), ["localhost", "127.0.0.1", "::1", "yokohama-nas.moore.nyc"]);
    }

    #[test]
    fn an_unlisted_name_an_empty_authority_and_a_different_name_with_a_listed_port_are_refused() {
        let h = on(Some("yokohama-nas.moore.nyc"), "0.0.0.0");
        for refused in [
            "attacker.example",
            "attacker.example:8080",
            "",
            ":8080",
            "localhost:80x",
            "localhost@attacker.example",
            "yokohama-nas.moore.nyc..",
            "[localhost]",
            "[::1]x",
            "[::1",
            "::1:8080",
            "localhost.attacker.example",
        ] {
            assert!(!h.admits(refused), "{refused:?}");
        }
    }

    #[test]
    fn a_malformed_list_is_refused_naming_the_entry() {
        let err = |raw: &str| Hostnames::from_env_values(None, Some(raw), "0.0.0.0").expect_err(raw);
        let e = err("a,,b");
        assert!(e.contains(HOSTNAMES_VAR) && e.contains("\"\""), "the empty entry: {e}");
        assert!(err("good.example,bad name").contains("bad name"));
        assert!(err("http://x").contains("http://x"));
        assert!(err("user@x").contains("user@x"));
        assert!(err("x#y").contains("x#y"));
        assert!(err("x:8080").contains("x:8080"), "a port is not a name");
        assert!(err("[::1]").contains("[::1]"), "an IPv6 literal is written without brackets");
        assert!(err("a,").contains(HOSTNAMES_VAR), "a trailing comma is an empty entry");
        assert!(err("").contains(HOSTNAMES_VAR), "an empty value is an empty entry");
        let long = "x".repeat(200) + " y";
        let e = err(&long);
        assert!(e.chars().count() < 400 && e.contains('…'), "the entry is echoed through the cap: {e}");
        // A well-formed entry is trimmed, lower-cased and de-dotted; an IPv6 literal is itself.
        let ok = Hostnames::from_env_values(None, Some(" ::1 , A.Example. ,b.example"), "0.0.0.0").expect("well-formed");
        assert_eq!(ok.listed(), ["localhost", "127.0.0.1", "::1", "a.example", "b.example"]);
        assert!(Hostnames::from_env_values(None, Some("fe80::1"), "0.0.0.0").expect("an IPv6 literal").admits("[fe80::1]"));
    }

    #[test]
    fn the_switch_absent_or_on_is_on_and_off_admits_any_host() {
        for enabled in [None, Some("on")] {
            let h = Hostnames::from_env_values(enabled, None, "0.0.0.0").expect("on");
            assert!(h.is_on(), "{enabled:?}");
            assert!(!h.admits("attacker.example"), "{enabled:?}");
            assert!(h.admits("localhost"), "{enabled:?}");
            assert!(h.start_line().starts_with("yokohama-web answers host names: localhost, 127.0.0.1, ::1"));
        }
        let off = Hostnames::from_env_values(Some("off"), None, "0.0.0.0").expect("off");
        assert!(!off.is_on());
        assert!(off.admits("attacker.example") && off.admits(""));
        assert!(off.listed().is_empty());
        assert!(off.start_line().contains("answers any host name"), "{}", off.start_line());
        assert!(off.start_line().contains(HOSTNAMES_ENABLED_VAR));
        // Off ignores the list, even a malformed one: the check that would read it is not running.
        assert!(Hostnames::from_env_values(Some("off"), Some("bad name"), "0.0.0.0").is_ok());
    }

    #[test]
    fn a_malformed_switch_value_is_refused_naming_it() {
        for bad in ["yes", "ON ", "", "On", "true"] {
            let e = Hostnames::from_env_values(Some(bad), None, "0.0.0.0").expect_err(bad);
            assert!(e.contains(HOSTNAMES_ENABLED_VAR) && e.contains(&format!("\"{bad}\"")), "{bad:?}: {e}");
        }
    }
}
