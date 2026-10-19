# The inventory: what this repository takes from the single repository

The platform's shell command used to be one binary of the single repository, which read every command
itself and ran it in its own process. This repository takes from it only what a thin entry point is:
reading the words, finding who carries them, and handing them on. Every command's own work stays with
the unit that owns it (docs/boundary.md, 「Where every command stands」). This is the one document that
names the single repository's files, since accounting for them is what it is for.

## How the sets were read

- **The commands,** 136: the deployed binary's help walked command by command, the hidden `runner`
  commands read from its `runner` help, and the six commands that are also nouns of their own
  (`agent status`, `dev-server list`, `dev-server pane`, `search branch`, `song gasazip`,
  `todo korean-records`). A command's own subcommands, such as `draw verify` or `node work setup`, are
  words of that command and pass through with it.
- **The aliases,** 96: every `alias` attribute on a command in `bot/bmo-server/src/bin/bmo.rs`, the
  `verbs/*.rs` files and the `verbs/*/mod.rs` files, attributes spread over several lines included. Each
  belongs to one of the 136 commands.
- **The files:** `git ls-files` over the paths named below, and a search of the tests for the binary's
  name.

## Taken (4 files, in part)

| File | Lines | What is taken | Where it stands here |
|---|---|---|---|
| `bot/bmo-server/src/bin/bmo.rs` | 2058 | The entry's shape: the global `--via` option and its meaning, the help and the version, and clap's exit code 2 for words it does not read. Its commands, its startup reading of the bot's environment file and its per-command code stay with their owners | the binary's reading of the words and its own answers (docs/boundary.md, 「Resolving a command」, 「What `bmo` answers itself」) |
| `bot/bmo-server/src/bin/bmo/tests.rs` | 94 | Of its three cases, the one that the help lists the nouns and no command of two words. The other two, a flat name read as its command and a noun given alone read as the command it was the alias of, are the aliases' cases, which their units carry | the help's test in `tests/` |
| `bot/bmo-server/src/verbs/nouns.rs` and its `nouns/tests.rs` | 128 and 104 | How the words are read: a noun of two words, the flags before the first word passed over, and the global `--via`'s value passed over with it. The clap tree it builds out of the declared commands is not taken: here the units declare their nouns | the matching of declared nouns (docs/boundary.md, 「Unit commands」) |
| `bot/bmo-server/src/verbs/home_bridge.rs` | 141 | The client half: the bridge named by `BMO_HOME_BRIDGE`, the wait `BMO_HOME_BRIDGE_WAIT_MS` names (15 seconds where none is named), the framed exchange over the socket and the relayed exit code. Which commands run at home is each unit's, declared by the `home` mark | docs/boundary.md, 「The home bridge」 |

- **Personal data and agent names** in what is taken: none. The one agent name in the entry file stands
  on a command that is not taken.
- **Korean** in what is taken: none.

## Not taken

| Set | Count | Why |
|---|---|---|
| `scripts/`, every file | 36 | 21 are superseded by commands that now belong to their owners, 3 more look superseded and are no entry's either, and 12 belong to other units: the Android build and the chat platforms' logins and hand tools, with the package's own manifests |
| Tests that name the binary | 27 | 3 under `bot/bmo-server-tests/` and 23 under `bot/bmo-server/tests/integration/`, plus 1 fixture: each tests a command's own work, which its owner carries with its tests |
| Policy files read by the entry | 0 | The entry reads no policy file; every policy file is read by a command's own code |
| Service units | 0 | The entry runs no service |
| The documents describing the commands | 2 | `docs/repository-layout.md`, 「The commands」, and `docs/rust-migration.md` state the single binary's shape; this repository's own documents state the entry point's |

- **The scripts by class,** as the investigation classed them:
  - superseded, each by the command named: `brain-hold.ts` (`brain hold`), `cargo.ts` (`cargo`),
    `check.ts` (`check`), `dashes.ts` (`check dashes`), `deploy-web.ts` (`deploy web`), `digest-pause.ts`
    (`digest pause`), `edit.ts` (`edit`), `graph.ts` (`graph`), `html-read.ts` (`archive html-read`),
    `imports.ts` (`imports`), `job-close.ts` (`job close`), `korean-in-code.ts` (`check korean`),
    `query.ts` (`query`), `quick-check.ts` (`check quick`), `running.ts` (`run list`), `test-log.ts`
    (`test log`), `test-suite.ts` (`test suite`), `test-template.ts` (`test template`), `wording.ts`
    (`check wording`), `work-order.ts` (`work-order`), `worktree-setup.ts` (`checkout setup`);
  - looking superseded, not confirmed: `host-stand-in.ts` (`machine stand-in`), `process-memory.ts`
    (`machine memory`), `setup/archive-roles.ts` (no command named);
  - other units: `build/android.ts`, `build/android.test.ts`, `platforms/backfill.ts`,
    `platforms/kakao-chatserver.ts`, `platforms/kakao.ts`, `platforms/redroid.sh`,
    `platforms/teams-signin.ts`, `platforms/tg-login.ts`, `platforms/windows-openssh.ps1`,
    `package.json`, `bunfig.toml`, `tsconfig.json`.
