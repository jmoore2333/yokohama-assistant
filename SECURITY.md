# Security

## Reporting a vulnerability

Report a vulnerability privately, through GitHub's private vulnerability reporting: open the repository's
**Security** tab and choose **Report a vulnerability**. Please do not open a public issue or a pull request for one.

## In scope

- The table layer, `crates/yokohama-web`, as the self-hosted binary and its Docker image
  ([`deploy/`](deploy/README.md)).
- The Cloudflare Worker host, `crates/yokohama-worker` and [`worker/`](worker/README.md).
- The browser client, `web/`.
- The command-line driver, `crates/yokohama-cli`.
- The hosted sites, `yokohama.moore.nyc` and `yokohama-nas.moore.nyc`.

## What to expect

- An acknowledgement within a week.
- A fix on a branch of its own, reviewed before it merges.
- A row in [`docs/security-ledger.md`](docs/security-ledger.md).
- Credit for the finding, if you want it.

## Out of scope

- Cloudflare's and WorkOS's own services. Report those to them.
- The open door's trust model. Under `YOKOHAMA_WEB_IDENTITY=open` a name is a label and a key, never a credential:
  anyone who can reach the server may use any name. That is by design, and
  [`deploy/README.md` §2](deploy/README.md#2-the-open-door-a-home-network-a-lan-or-a-vpn) says where such a server
  belongs.
