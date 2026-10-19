# bmo-cli

The platform's thin entry point, `bmo`. It holds no API and runs no service. Each unit ships its own
command, named `bmo-<unit>`, and `bmo` finds the one that carries a command and hands the command to it,
the way git finds `git-<command>`. A unit added later needs no new build of `bmo`.

## Layout

| Folder | Holds |
|---|---|
| `src/` | The binary `bmo` |
| `docs/` | The canonical statements of what `bmo` does |

At the root stand the Cargo manifest, the version, `testbed.toml` (bmo-testbed's declaration of how this
repository is tested, checked, built and deployed), the Korean ledger the testbed's `korean` check reads,
`rustfmt.toml` and `CLAUDE.md`.

## Documents

| Document | States |
|---|---|
| `docs/boundary.md` | The contract: how unit commands are found and declare their nouns, how a command is resolved and handed on, what `bmo` answers itself, its settings and the home bridge, and where every command of the platform's shell stands |
| `docs/inventory.md` | What this repository takes from the single repository it was split out of, and what it leaves to the units |
