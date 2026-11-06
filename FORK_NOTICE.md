# v8-legacy-runner fork notice

This repository, `v8-legacy-runner`, is the maintained fork of
[`IngvarConsulting/v8-runner-rust`](https://github.com/IngvarConsulting/v8-runner-rust)
that keeps 1C:Enterprise 8.3.12–8.3.23 working.

- Fork established: 2026-11-06.
- Upstream baseline: `9b6c5d8` (upstream `master`, minimum platform 8.3.24).
- Maintained source: <https://github.com/Asmody/v8-legacy-runner>.
- License: GNU Affero General Public License v3.0 only (`AGPL-3.0-only`).

The fork carries a delta over the upstream, laid out as knowledge in `src/legacy/`
plus thin seams in upstream files (the table of seams lives in `src/legacy/mod.rs`).
`master` mirrors the upstream and is only fast-forwarded; all work happens on
`legacy/8.3.12`. The update procedure from the upstream is described in `AGENTS.md`,
section "Legacy delta over the upstream".
