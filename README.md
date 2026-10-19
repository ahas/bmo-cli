# bmo-cli

The platform's thin entry point, `bmo`. It holds no API and runs no service. Each unit ships its own
command, named `bmo-<unit>`, and `bmo` finds the one that carries a command and hands the command to it,
the way git finds `git-<command>`. A unit added later needs no new build of `bmo`.

## Layout

| Folder | Holds |
|---|---|
| `src/` | The binary `bmo` |

At the root stand the Cargo manifest, the version, `testbed.toml` (bmo-testbed's declaration of how this
repository is tested, checked, built and deployed), the Korean ledger the testbed's `korean` check reads,
`rustfmt.toml` and `CLAUDE.md`.
