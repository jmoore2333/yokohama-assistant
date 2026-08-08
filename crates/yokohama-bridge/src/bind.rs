//! The loopback guard.
//!
//! The relay listens while the owner is logged into their own BGA session and
//! writes whatever the page hands it to disk. That is fine on `127.0.0.1` and
//! is not fine anywhere else, so "reachable from the network" is not a
//! configuration mistake this binary can make: every bind address goes through
//! [`resolve`], and anything that is not loopback comes back as an error the
//! binary prints and exits on.

use std::fmt;
use std::net::{SocketAddr, ToSocketAddrs};

/// Why a requested bind address was rejected.
#[derive(Debug)]
pub enum BindError {
    /// The host string could not be resolved at all.
    Resolve { host: String, source: std::io::Error },
    /// It resolved, but to nothing.
    NoAddresses { host: String },
    /// It resolved to an address that is not loopback. This is the refusal the
    /// module exists for.
    NotLoopback { host: String, addr: SocketAddr },
}

impl fmt::Display for BindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BindError::Resolve { host, source } => {
                write!(f, "could not resolve bind host {host:?}: {source}")
            }
            BindError::NoAddresses { host } => {
                write!(f, "bind host {host:?} resolved to no addresses")
            }
            BindError::NotLoopback { host, addr } => write!(
                f,
                "refusing to bind {host:?} ({addr}): the bridge relay listens on loopback only. \
                 It captures your live game session to disk and has no authentication; making it \
                 reachable from the network is not a supported configuration."
            ),
        }
    }
}

impl std::error::Error for BindError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BindError::Resolve { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Resolve `host:port` to a single socket address, refusing anything that is
/// not loopback.
///
/// A host that resolves to several addresses (`localhost` usually gives both
/// `127.0.0.1` and `::1`) must have EVERY one of them be loopback — a host
/// with one loopback and one routable address is exactly the trick this guard
/// is meant to catch, so the first non-loopback answer refuses the whole set.
pub fn resolve(host: &str, port: u16) -> Result<SocketAddr, BindError> {
    let resolved: Vec<SocketAddr> = (host, port)
        .to_socket_addrs()
        .map_err(|source| BindError::Resolve { host: host.to_string(), source })?
        .collect();

    let mut chosen = None;
    for addr in resolved {
        if !addr.ip().is_loopback() {
            return Err(BindError::NotLoopback { host: host.to_string(), addr });
        }
        // Prefer the first, which on a normal machine is IPv4 for `localhost`.
        chosen.get_or_insert(addr);
    }

    chosen.ok_or_else(|| BindError::NoAddresses { host: host.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_hosts_resolve() {
        assert_eq!(resolve("127.0.0.1", 8787).expect("v4 loopback").port(), 8787);
        assert!(resolve("127.0.0.1", 1).expect("v4 loopback").ip().is_loopback());
        assert!(resolve("::1", 1).expect("v6 loopback").ip().is_loopback());
        assert!(resolve("localhost", 1).expect("localhost").ip().is_loopback());
    }

    #[test]
    fn the_wildcard_binds_are_refused_by_name() {
        // `0.0.0.0` and `::` are the two an operator reaches for when they want
        // "listen everywhere", which is the one thing this relay must not do.
        for wildcard in ["0.0.0.0", "::"] {
            let err = resolve(wildcard, 8787).expect_err("wildcard must be refused");
            assert!(matches!(err, BindError::NotLoopback { .. }), "{err}");
            assert!(
                err.to_string().contains("loopback only"),
                "the refusal has to explain itself: {err}"
            );
        }
    }

    #[test]
    fn routable_addresses_are_refused() {
        for routable in ["192.168.1.50", "10.0.0.7", "8.8.8.8"] {
            assert!(
                matches!(resolve(routable, 8787), Err(BindError::NotLoopback { .. })),
                "{routable} must be refused"
            );
        }
    }

    #[test]
    fn an_unresolvable_host_is_an_error_not_a_panic() {
        let err = resolve("this-host-does-not-exist.invalid", 8787)
            .expect_err(".invalid is reserved and never resolves");
        assert!(matches!(err, BindError::Resolve { .. } | BindError::NoAddresses { .. }), "{err}");
    }
}
