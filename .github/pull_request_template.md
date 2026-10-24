### What changes

<!-- In prose: what this pull request changes, and why. Name the issue it answers, if there is one. -->

### The suites run

- [ ] The per-edit suites and the lint gate (`CONTRIBUTING.md`, "How a change lands"), green with zero warnings.
- [ ] The web lane's suites (`docs/web-lane.md` §2), if the change reaches what the layer compiles.
- [ ] A `SEMANTICS_REV` bump in the same commit, with its decision-log row, if which moves are allowed, how a move applies, scoring or setup moved.

### The documents updated

- [ ] The CLI guide pair, `docs/cli-guide.md` and `docs/cli-guide.html`, if the CLI's behaviour or text moved.
- [ ] The lane document's cells, `crates/yokohama-web/tests/lane_document.rs` and `worker/scripts/lane-variables.test.mjs`, green if `docs/web-lane.md` moved.

### The ask

- [ ] I am comfortable with what this pull request sends and comfortable discussing it.
