# Working in bmo-cli

These rules hold every change to this repository.

## The documents come first

- `docs/` is the canonical statement of what `bmo` does; the code is one realisation of it. Before
  changing code, read the document covering that area.
- A change that would make a document wrong or incomplete repairs the document first, stating what is
  true now, and the code is written to match.
- A document states what is true now: no change history, no rationale told through an earlier
  design, no pasted source, signature or config fragment, and no file-and-line pins.
- A document names no file outside this repository, but for `docs/inventory.md`, which accounts for the
  single repository's files. Another unit is named by its name, and what it keeps is its own documents'
  to state.

## Boundaries

- **`bmo` holds no API and runs no service.** It finds the unit that carries a command and hands the
  command to it. It never reads another unit's code, tables or files.
- **Dependencies:** crates from the registry, and among other repositories only the base libraries
  `bmo-chat`, `bmo-chat-client` and `bmo-ui`. The testbed's `dependencies` check holds this.
- **No lookup table standing in for polymorphism.** What differs per unit belongs to that unit: its
  command declares its own nouns, and a module carrying a unit's commands stands on its own.
- **No hand-written parser for a general format.** TOML, JSON, command lines and their kin are read by
  a maintained library.

## Language

- Everything in this repository is English: code, comments, strings, tests, documents, commit messages
  and branch names. No Korean in code or tests; the testbed's `korean` check holds this.
- What `bmo` answers is an English sentence for a person and an exit code for a machine.
- No personal data: no user names, home directories, machine names or names of any agent.
  `docs/PERSONAL-DATA.md` states the rule whole, and the repository's test reads every tracked file for it.

## Layout

- Only the Cargo manifest at the root, and no directory named after its parent; the testbed's `layout`
  check holds both.
- Tests live in `tests/`, never beside the code.

## The stages of a change

1. **Writing:** the only check is `cargo check --all-targets`.
2. **Review:** read the whole diff against the document governing it, then `cargo fmt --check` and
   `cargo clippy --all-targets -- -D warnings`.
3. **Verification:** the tests touched, then `cargo test` once, output written to a file and the file
   read.

A change is not done before stage 3 has passed.

## Releasing

- The version stands in `VERSION` and in the Cargo manifest, the same in both, in semver. The major
  moves on a change a caller reads: the resolution order, the `nouns` reading or the exit codes. The
  minor moves on an addition; the patch on anything else.
- bmo-testbed builds and deploys this repository by `testbed.toml`.
