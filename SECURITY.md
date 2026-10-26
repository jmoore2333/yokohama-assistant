# Security

Report a vulnerability privately through GitHub's private vulnerability reporting: open the repository's **Security**
tab and choose **Report a vulnerability**. Please do not open a public issue or a pull request for one. In scope:
- the table layer, `crates/yokohama-web`, as a self-hosted binary or its Docker image ([`deploy/`](deploy/README.md));
- the Cloudflare Worker host, `crates/yokohama-worker` and [`worker/`](worker/README.md);
- the browser client, `web/`, and the command-line driver, `crates/yokohama-cli`;
- the hosted sites, `yokohama.moore.nyc` and `yokohama-nas.moore.nyc`.

Expect an acknowledgement, a fix on its own branch, and a row in [`docs/security-ledger.md`](docs/security-ledger.md).

Out of scope are Cloudflare's and WorkOS's own services, which take reports of their own, and the open door's trust
model: under `YOKOHAMA_WEB_IDENTITY=open` a name is a label, never a credential, by design
([`deploy/README.md` §2](deploy/README.md#2-the-open-door-a-home-network-a-lan-or-a-vpn)).
