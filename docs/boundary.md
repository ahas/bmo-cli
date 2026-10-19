# The boundary: how `bmo` finds the unit that carries a command

`bmo` holds no API and runs no service. A person, a job or a session runs `bmo <words>`; `bmo` finds the
unit that carries those words and hands them to it, or calls the route that unit documents for them. This
document is the whole of that contract: what `bmo` reads from a unit's command, what it hands on, what
it answers itself, and, command by command, where every command of the platform's shell stands today.

## Versioning

- **The contract carries the repository's one version, in semver,** the version in `VERSION` and the
  Cargo manifest.
- **The major moves on a change a caller or a unit reads:** the resolution order, the reading of
  `nouns`, the placing of `--via`, an exit code, an answer's words, or the settings. The minor moves on an
  addition: a carried command, a mark read on a `nouns` line, a setting. The patch moves on anything
  else.

## Unit commands

- **A unit command is an executable file named `bmo-<name>`.** `bmo` looks for them first in the
  directory holding `bmo` itself, then in each directory of `PATH` in order. Where one name stands in
  several places, the first one found is the unit command of that name and the rest are not read.
- **A unit command declares its nouns** when asked with the one word `nouns`: it prints every noun it
  carries, one a line. A noun of two words is the two words with one space between them. A line may
  carry marks after a tab, separated by spaces; a mark `bmo` does not know is passed over.
- **The mark `home`** says the noun's commands run on the operations machine. Where a run is asked on a
  work node, `bmo` hands such a command to the home bridge (「The home bridge」).
- **How `nouns` is asked:** with standard input closed and a short timeout. A command that ends with any
  code but 0, prints nothing, or does not end in time declares no nouns. A noun spelled `nouns` is the
  protocol's own word and is never dispatched.
- **The answers are cached** under the user's cache directory (`XDG_CACHE_HOME`, else `.cache` in the
  home directory), each keyed by the unit command's path, size and modification time. A unit command
  that changes is asked again; one that no longer stands is dropped. A cache that cannot be read or
  written is passed over, and every unit command is asked afresh.

## Resolving a command

The words `bmo` is given are read in this order, and the first rule that matches decides.

1. **A declared noun.** The leading words are compared with every noun every unit command declares, and
   the longest noun matching them wins. `bmo` runs that unit command with all the words, the noun
   included: `bmo todo --add` runs `bmo-job todo --add`. Where two unit commands declare the longest
   matching noun, `bmo` refuses, naming the noun and both commands, and ends with 2.
2. **A carried command.** Where a unit carries a command under words other than its declared nouns,
   or documents a route for it and carries no command at all, a module of this repository carries those
   words on the unit's behalf: it runs the unit's command with the words the unit documents, or calls the
   route the unit documents. Each module stands for one unit and names the commands it carries; the
   tables below say which. A carried command whose unit command does not stand on this machine is not
   served (「What `bmo` answers itself」), naming the command it looked for.
3. **The git rule.** Where the first word is `<name>` and a unit command `bmo-<name>` stands, `bmo`
   runs it with the words after the first: `bmo pocket put` runs `bmo-pocket put`. The unit's own answer
   stands, its refusal of words it does not carry included.
4. **Not served.** Nothing above matched (「What `bmo` answers itself」).

- **What is handed on unchanged:** the words, in their order; the environment; standard input, output
  and error; and the unit's exit code, which is `bmo`'s.
- **`--via <road>`**, the road a caller was told its turn is read on, may stand before the first word,
  as `--via <road>` or `--via=<road>`. `bmo` takes it out to read the words, and hands it on just after
  the noun's words: `bmo --via r room say --text t` runs a unit's `room say --via r --text t`. Under the
  git rule it is handed on first, before the remaining words. A `--via` after the first word is a word
  like any other and passes unchanged.
- **A word starting with `-` before the first word** other than `--via`, `--help`, `-h` and `--version`
  is refused with 2.

## What `bmo` answers itself

| Asked | Answer | Exit |
|---|---|---|
| `bmo --version` | `bmo <version>` on standard output | 0 |
| `bmo help`, `bmo --help`, `bmo -h` | the help, on standard output | 0 |
| `bmo` with no words | the help, on standard error | 2 |
| words nothing serves | ``bmo: `<words>` is not served yet: no installed unit carries it`` on standard error | 2 |
| a carried command whose unit command does not stand | ``bmo: `<words>` is not served yet: `bmo-<unit>` is not installed`` on standard error | 2 |
| a noun two unit commands declare | ``bmo: `<noun>` is declared by both `bmo-<one>` and `bmo-<other>` `` on standard error | 2 |
| a unit command that cannot be started | ``bmo: `bmo-<unit>` could not be started: <reason>`` on standard error | 1 |

- **The help** lists the global options, then every noun found, each with the unit command declaring
  it, then every carried command with its unit, then every unit command the git rule reaches.
- **The words of a not-served answer** are the words as given, the leading options taken out.

## The settings

- **One TOML file,** named by `BMO_CLI_SETTINGS`, else `settings.toml` in the release `bmo` stands in:
  the parent of the directory holding `bmo`. A file that does not stand is no settings.
- **`[unit.<name>]`** is one unit's table, read by the module carrying that unit's commands alone;
  `address` is the base address of the unit's HTTP service. A carried call whose unit has no address is
  refused with 2, naming the setting it needs.

## The home bridge

- **On a work node**, `BMO_HOME_BRIDGE` names the socket of the bridge to the operations machine, and
  `BMO_HOME_BRIDGE_WAIT_MS` how long a bridge that does not answer is tried before the run is refused.
- **A command whose noun carries the `home` mark** is handed to the bridge with its words, and the
  answer the bridge relays is `bmo`'s, its exit code included.

## Where every command stands

These are the platform's shell commands, 136 of them, each under the unit that owns it. A status says
what `bmo` does with the command today; each command's alias is listed under the same unit, for that
unit to carry. `bmo` holds no alias table: an alias is served only where a unit declares it as a noun or
the git rule reaches it.

- **Not served yet** means no unit command or route serves the words today; the unit named above the
  table is the one to carry them.
- **The git rule still reaches** a command not served whose first word names a unit command that stands,
  and that unit's own answer stands (「Resolving a command」): `node land` reaches `bmo-node` as `land`.

### bmo-job (19)

| Command | Alias | Status |
|---|---|---|
| `job ask` | `ask` | bmo-job, by its noun `job` |
| `job borrow` | `borrow` | bmo-job, by its noun `job` |
| `job branches` | `branch-split` | bmo-job, by its noun `job` |
| `job close` | `job-close` | bmo-job, by its noun `job` |
| `job cowork-timing` | `cowork-timing` | bmo-job, by its noun `job` |
| `job figures` | `job-figures` | bmo-job, by its noun `job` |
| `job guard` | `job-guard` | bmo-job, by its noun `job` |
| `job land` | `land` | bmo-job, by its noun `job` |
| `job live-claude` | `live-claude` | bmo-job, by its noun `job` |
| `job start` | `task` | bmo-job, by its noun `job` |
| `job tool-calls` | `tool-calls` | bmo-job, by its noun `job` |
| `job wait` | `waiting` | bmo-job, by its noun `job` |
| `todo` | | bmo-job, by its noun `todo` |
| `todo korean-records` | `korean-records` | bmo-job, by its noun `todo` |
| `work-order` | | bmo-job, by its noun `work-order` |
| `overtime` | | bmo-job, by its noun `overtime` |
| `checkout setup` | | bmo-job, by its noun `checkout setup` |
| `room jobs` | `jobs` | bmo-job, by its noun `room jobs` |
| `node land` | `node-land` | not served yet under these words; the alias `node-land` is bmo-job's noun |

### bmo-testbed (21)

| Command | Alias | Status |
|---|---|---|
| `test suite` | `test-suite` | not served yet |
| `test count` | `test-count` | not served yet |
| `test log` | | not served yet |
| `test template` | | not served yet |
| `test full` | `full-suite` | not served yet |
| `check` | | not served yet |
| `imports` | | not served yet |
| `cargo` | | not served yet |
| `edit` | | not served yet |
| `testbed run` | `testbed` | bmo-testbed, by the git rule |
| `testbed status` | | bmo-testbed, by the git rule |
| `deploy commit` | `deploy` | not served yet |
| `deploy tool` | | not served yet |
| `deploy web` | | not served yet |
| `deploy backlog` | `deploy-backlog` | retired |
| `deploy main` | `suite-deploy` | retired |
| `deploy backups` | `deploy-backups` | not served yet |
| `deploy down-alert` | `bot-down` | not served yet |
| `machine stand-in` | | not served yet |
| `backup snapshot` | `snapshot-backup` | not served yet |
| `backup drill` | `restore-drill` | not served yet |

- **Retired:** the platform deploys each repository on its own through bmo-testbed, so no command
  deploys several repositories at once. `deploy backlog` and `deploy main` answer as not served.

### bmo-artifact (8)

| Command | Alias | Status |
|---|---|---|
| `archive doc` | `archive-doc` | not served yet |
| `archive share` | `archive-share` | not served yet |
| `archive html-read` | | not served yet |
| `search` | | not served yet |
| `search branch` | `research-branch` | not served yet |
| `runner search` | `search-runner` | not served yet |
| `news-brief` | | not served yet |
| `runner news-brief` | `news-brief-runner` | not served yet |

### bmo-receiver (25)

| Command | Alias | Status |
|---|---|---|
| `room say` | `say` | not served yet |
| `room pin` | `pin` | not served yet |
| `room react` | `react` | not served yet |
| `room sticker` | `sticker` | not served yet |
| `room photo` | `photo` | not served yet |
| `room photos` | `photos` | not served yet |
| `room file` | `file` | not served yet |
| `room upload` | `upload` | not served yet |
| `room mp3` | `mp3` | not served yet |
| `room mute` | `mute` | not served yet |
| `room notice` | `notice` | not served yet |
| `room rules` | `room-rules` | not served yet |
| `runner say` | `say-runner` | not served yet |
| `runner mp3` | `mp3-runner` | not served yet |
| `owner tell` | `tell-owner` | not served yet |
| `reminder set` | `remind` | not served yet |
| `reminder list` | `reminders` | not served yet |
| `consult` | | not served yet |
| `draw` | `random-draw` | not served yet |
| `archive scan` | `archive-scan` | not served yet |
| `archive photo-repair` | `photo-repair` | not served yet |
| `watch set` | `watch` | not served yet |
| `watch list` | `watches` | not served yet |
| `runner watch` | `watch-runner` | not served yet |
| `procedure show` | `procedure` | not served yet |

### bmo-agency (20)

| Command | Alias | Status |
|---|---|---|
| `agent` | `agents` | not served yet |
| `agent status` | `agent-status` | not served yet |
| `model` | `models` | not served yet |
| `coworker pool` | `coworker-pool` | not served yet |
| `coworker hook` | `coworker-pool-hook` | not served yet |
| `worker fold` | `fold-worker` | not served yet |
| `run list` | | not served yet |
| `run stop` | `stop` | not served yet |
| `machine memory` | | not served yet |
| `reload` | | not served yet |
| `node work` | `work-node` | not served yet |
| `node claude-token` | `claude-token` | not served yet |
| `node pane` | `node-pane` | not served yet |
| `dev-server` | | not served yet |
| `dev-server list` | `dev-servers` | not served yet |
| `dev-server pane` | `dev-pane` | not served yet |
| `companion` | | not served yet |
| `claude batch` | `claude-batch` | not served yet |
| `usage` | | not served yet |
| `fasttrack` | | not served yet |

### bmo-pocket (7)

| Command | Alias | Status |
|---|---|---|
| `tool list` | | not served yet |
| `tool describe` | | not served yet |
| `tool call` | | not served yet |
| `tool probe` | `tool-probe` | not served yet |
| `tool traffic` | `tool-traffic` | not served yet |
| `tool sites` | `site-tools` | not served yet |
| `tool unread-pages` | `unread-pages` | not served yet |

### bmo-brain (9)

| Command | Alias | Status |
|---|---|---|
| `memory` | | not served yet |
| `note` | | not served yet |
| `fragment` | | not served yet |
| `digest run` | `digest` | not served yet |
| `digest pause` | | not served yet |
| `digest people` | `people-digest` | not served yet |
| `digest now` | `digest-now` | not served yet |
| `brain hold` | | not served yet |
| `brain wait` | | not served yet |

### bmo-node (2)

| Command | Alias | Status |
|---|---|---|
| `node list` | `nodes` | not served yet |
| `node hardware` | `hardware-setup` | not served yet |

### bmo-word-chain (1)

| Command | Alias | Status |
|---|---|---|
| `word-chain` | | bmo-word-chain, by the git rule |

### bmo-gate (10)

| Command | Alias | Status |
|---|---|---|
| `gate eval` | `gate-eval` | not served yet |
| `gate bench` | `gate-bench` | not served yet |
| `gate laya-agreement` | `laya-agreement` | not served yet |
| `gate format-cases` | `format-cases` | not served yet |
| `reference compare` | `reference-compare` | not served yet |
| `reference inputs` | `reference-inputs` | not served yet |
| `reference narrowing` | `reference-narrowing` | not served yet |
| `reference replay` | `reference-replay` | not served yet |
| `reference roads` | `reference-roads` | not served yet |
| `reference waste` | `reference-waste` | not served yet |

### bmo-explorer (4)

| Command | Alias | Status |
|---|---|---|
| `browse` | | not served yet |
| `runner browse` | `browse-runner` | not served yet |
| `remote-browser` | | not served yet |
| `runner remote-browser` | `remote-browser-runner` | not served yet |

### bmo-voice (3)

| Command | Alias | Status |
|---|---|---|
| `voice say` | `voice` | not served yet |
| `voice brief` | `voice-brief` | not served yet |
| `runner voice` | `voice-runner` | not served yet |

### bmo-song (3)

| Command | Alias | Status |
|---|---|---|
| `song` | | not served yet |
| `song gasazip` | `gasazip` | not served yet |
| `runner song` | `song-runner` | not served yet |

### bmo-graph (1)

| Command | Alias | Status |
|---|---|---|
| `graph` | | not served yet |

### bmo-drive (1)

| Command | Alias | Status |
|---|---|---|
| `file share` | `file-share` | not served yet |

### bmo-remote (1)

| Command | Alias | Status |
|---|---|---|
| `node remote-desktop` | `remote-desktop` | not served yet |

### No owner (1)

| Command | Alias | Status |
|---|---|---|
| `query` | | not served yet |

- **`query`** put one SQL statement to one database that held every table. Each unit now keeps its own
  tables in its own database, so no single unit owns the command; a unit that wants a read of its own
  tables offers it under its own words.

## What the units are asked to match

- **bmo-job and bmo-agency both claim** `job live-claude`, `job borrow`, `job wait` and `overtime`:
  bmo-job declares the nouns `job` and `overtime`, and bmo-agency's documents give `bmo-agency
  live-claude`, `bmo-agency borrow`, `bmo-agency wait` and `bmo-agency overtime`. The declared noun
  wins (「Resolving a command」), so bmo-job answers these until the two settle which one carries them.
- **bmo-job declares `node-land`,** while the platform's words are `node land`. Declaring the noun
  `node land` as well serves both.
- **bmo-job declares the noun `worker`,** so `worker fold`, bmo-agency's command, reaches bmo-job. Either
  bmo-job declares the narrower nouns it carries, or bmo-agency declares `worker fold`, which as the
  longer noun wins.
- **The agent sessions' shell hooks run** `job-guard` and `coworker-pool-hook`, aliases no unit declares.
  bmo-job is asked to declare `job-guard`, and bmo-agency `coworker-pool-hook`, before the hooks run on
  this `bmo`.
- **Every unit asked to carry commands** declares its nouns: bmo-job alone answers `nouns` today.
