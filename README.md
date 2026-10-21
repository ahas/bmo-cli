# bmo-cli

The platform's thin entry point, `bmo`. It holds no API and runs no service. Each unit ships its own
command, named `bmo-<unit>`, and `bmo` finds the one that carries a command and hands the command to it,
the way git finds `git-<command>`. A unit added later needs no new build of `bmo`.

## Layout

| Folder | Holds |
|---|---|
| `src/` | The library that reads the words, finds the unit commands and their nouns, resolves a command and hands it on, and the binary `bmo` |
| `tests/` | The contract's tests, each running the built `bmo` against stand-in unit commands in a directory of its own, and the check over every tracked file for personal data |
| `docs/` | The canonical statements of what `bmo` does, and the personal-data rule |
| `.graph/` | The code dependency graph: one node per feature, where a change starts reading. Its area files are generated in the checkout and untracked, so a fresh checkout builds them again, while `settings.toml` is the repository's configuration and stays tracked |

At the root stand the Cargo manifest, the version, `testbed.toml` (bmo-testbed's declaration of how this
repository is tested, checked, built and deployed), the Korean ledger the testbed's `korean` check reads,
`rustfmt.toml`, `CLAUDE.md` and the MIT licence in `LICENSE`.

## Documents

| Document | States |
|---|---|
| `docs/api.md` | The fixed API: how unit commands are found and declare their nouns, how a command is resolved and handed on, what `bmo` answers itself, and the home bridge |
| `docs/inventory.md` | What this repository takes from the single repository it was split out of, and what it leaves to the units |
| `docs/PERSONAL-DATA.md` | What personal data never goes into this repository, what to write instead, and the check that reads every tracked file for it |
