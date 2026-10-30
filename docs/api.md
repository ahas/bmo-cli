# The fixed API: how `bmo` finds a unit's command and hands it on

**Fixed at 0.1.0-rev.1.**

`bmo` holds no API and runs no service. It reads the words it is given, finds the unit command that carries
them, and hands them on; what it answers itself is the help, the version, a refusal and its one verb, `use`,
which reaches a hosted tool. This document is the whole of that contract: the command-finding rule, the
hand-over and the verb, and nothing else. `bmo` publishes no HTTP API, so it publishes no OpenAPI schema and no
generated client; its fixed API is the command line below.

## The unit command

- **A unit command is an executable file named `bmo-<name>`.** `bmo` looks for them first in the directory
  holding `bmo` itself, then in each directory of `PATH` in order. Where one name stands in several places, the
  first one found is the unit command of that name and the rest are not read.
- **Every such executable is taken for a unit command** and asked `nouns`, so a program that is not one, a
  service's own server binary among them, is not installed under a `bmo-` name in those directories.
- **How `nouns` is asked:** with standard input closed and standard error discarded, every unit command at
  once, each given two seconds. A command that ends with any code but 0, prints nothing, or does not end in
  time declares no nouns; one still running at its time is stopped.
- **The answers are cached** in `bmo-cli/nouns.json` under the user's cache directory (`XDG_CACHE_HOME`, else
  `.cache` in the home directory), each keyed by the unit command's path, size and modification time. A unit
  command that changes is asked again; one that no longer stands is dropped. A cache that cannot be read or
  written is passed over, and every unit command is asked afresh.

## The nouns a unit declares

- **A noun is one line of the answer:** the noun's words with one space between them, then, after a tab, its
  marks separated by spaces. A mark `bmo` does not know is passed over.
- **The mark `home`** says the noun's commands run on the operations machine. Where a run is asked on a work
  node, `bmo` hands such a command to the home bridge (「The home bridge」).
- **The word `nouns` is the protocol's own** and is never dispatched.
- **The word `use` is `bmo`'s own** (「The tool verb」): a unit declaring it as a noun, or a unit command
  `bmo-use`, is never reached by it.
- **Every unit's command list is its own.** A unit's document names the nouns its command carries; `bmo` keeps
  no list of them, and a noun a unit does not declare is not served.

## Resolving a command

The words `bmo` is given are read in this order, and the first rule that matches decides.

1. **`bmo`'s own verb.** Where the first word is `use`, `bmo` answers the command itself (「The tool verb」).
2. **A declared noun.** The leading words are compared with every noun every unit command declares, and the
   longest noun matching them wins. `bmo` runs that unit command with all the words, the noun included:
   `bmo todo --add` runs `bmo-job todo --add`. Where two unit commands declare the longest matching noun, `bmo`
   refuses, naming the noun and both commands, and ends with 2.
3. **The git rule.** Where the first word is `<name>` and a unit command `bmo-<name>` stands, `bmo` runs it
   with the words after the first: `bmo graph nodes` runs `bmo-graph nodes`. The unit's own answer stands, its
   refusal of words it does not carry included.
4. **Nothing matched.** Where the leading words begin one or more declared nouns without being one, `bmo`
   names the longest run of them that does and lists every noun it begins, each with the unit command declaring
   it: `bmo deploy`, `bmo deploy --help` and `bmo deploy foo` all list the nouns beginning with `deploy`. Where
   no noun begins with the first word, the words are not served. Either way `bmo` ends with 2 (「What `bmo`
   answers itself」).

- **What is handed on unchanged:** the words, in their order; the environment; standard input, output and
  error; and the unit's exit code, which is `bmo`'s.
- **`--via <road>`**, the road a caller was told its turn is read on, may stand before the first word, as
  `--via <road>` or `--via=<road>`. `bmo` takes it out to read the words, and hands it on just after the noun's
  words: `bmo --via r room say --text t` runs a unit's `room say --via r --text t`. Under the git rule it is
  handed on first, before the remaining words. A `--via` after the first word is a word like any other and
  passes unchanged.
- **A word starting with `-` before the first word** other than `--via`, `--help`, `-h` and `--version` is
  refused with 2 (「What `bmo` answers itself」).

## What `bmo` answers itself

| Asked | Answer | Exit |
|---|---|---|
| `bmo --version` | `bmo <version>` on standard output | 0 |
| `bmo help`, `bmo --help`, `bmo -h` | the help, on standard output | 0 |
| `bmo help <words>` | `<words> --help`, resolved and handed on as any command is | the unit's |
| `bmo` with no words | the help, on standard error | 2 |
| words beginning declared nouns without being one | ``bmo: `<begun>` is not a command of its own; the nouns beginning with it are:`` then each such noun on a line of its own, with the unit command declaring it, on standard error | 2 |
| words nothing serves | ``bmo: `<words>` is not served yet: no installed unit carries it`` on standard error | 2 |
| a noun two unit commands declare | ``bmo: `<noun>` is declared by both `bmo-<one>` and `bmo-<other>` `` on standard error | 2 |
| a unit command that cannot be started | ``bmo: `bmo-<unit>` could not be started: <reason>`` on standard error | 1 |
| an option before the first word that `bmo` does not read | ``bmo: `<option>` is not an option of bmo; options of a command follow its words`` on standard error | 2 |
| `bmo use …` | the tool verb's own answer (「The tool verb」) | the verb's |

- **The help** lists the global options and the tool verb, then every noun found, each with the unit command declaring it, then
  every unit command the git rule reaches.
- **The words of a not-served answer** are the words as given, the leading options taken out.
- **The begun words** are the longest run of leading words that the start of a declared noun spells; the nouns
  beginning with them are listed in order, each indented, the unit commands aligned in a column after them, as
  the help lists nouns.

## The home bridge

Some commands read or write what only the operations machine holds. Run on a work node, such a command is
handed whole to the operations machine, and what comes back is the run's own.

- **When:** `BMO_HOME_BRIDGE` names the bridge's socket, a non-empty path, and the command resolves to a
  declared noun carrying the `home` mark. Any other command, or any command where no bridge is named, runs
  where it was asked.
- **Reaching the bridge:** the socket is tried until it answers, again every second or every
  `BMO_HOME_BRIDGE_WAIT_MS` where that is shorter, for at most `BMO_HOME_BRIDGE_WAIT_MS` milliseconds,
  15 seconds where it names no positive number. Out of reach, `bmo` says on standard error that the operations
  machine is out of reach from the node `BMO_NODE` names and that the command was not run, and ends with 2.
- **The request:** one line of JSON: `script`, the binary as the bridge names it under the deploy tree,
  `bin/bmo`; `args`, every word `bmo` was given, its own options included, in their order; `env`, every
  variable of the run whose name starts with `BMO_` or `CLAUDE`; and `cwd`, the directory it was asked from.
- **Frames:** after the request both ways speak in frames: one byte naming the channel, the length of the body
  as four bytes, most significant first, then the body.
- **Standing input** goes to the bridge as frames on channel 1, read as it comes, and an empty frame on
  channel 0 says it ended. The socket stays open both ways.
- **The answer** comes as frames: channel 1 is written to standard output, channel 2 to standard error, and
  channel 3 ends the answer with the exit code as a signed four-byte number, most significant first, which
  `bmo` ends with; a code outside 0 to 255 ends it with 1. Frames on any other channel are passed over. A
  bridge that drops the connection before channel 3 is answered on standard error, and `bmo` ends with 2.

## The tool verb

`bmo use` reaches one hosted tool: it asks the tool host's toolbook what the tool is, calls it, and answers what
it said, either at once or, run by the tool host, by waking the asking agent once it is done. It also lists
the tools it reaches and describes one, so a caller finds a tool and reads how to fill its arguments through `use`
alone. The tool host's own documents state the toolbook and the tools face; the receiver's state the instruction
route. `use` calls each across that published contract and reads nothing else of theirs.

| Asked | What it does |
|---|---|
| `bmo use <tool> [--sync] [--arg <key>=<value>]… [--json <object>]` | calls the tool and waits for its answer |
| `bmo use <tool> --async [--memory <words>] [--room <room>] [--agent <name>] [--max-seconds <n>] [--arg …]… [--json …]` | asks the tool host to run the call and answers its id |
| `bmo use --tools [--query <words>]` | the tools `use` reaches, one a line |
| `bmo use <tool> --describe` | the tool as the host describes it, as JSON |
| `bmo use --list` | every call kept, one a line |
| `bmo use --status <id>` | one call, as JSON |
| `bmo use --help` | the verb's usage |

### Reaching the tool host

- **`<tool>` is a full name,** `<namespace>/<name>`, as the toolbook's `list` names it.
- **The toolbook is asked over the tools face,** MCP over HTTP at `/mcp/toolbook` on the loopback address,
  every request one `tools/call` standing alone. The port is `BMO_MCP_HOST_PORT`, else the one the data root's
  `env/pocket.env` names, else 3290.
- **The token is the caller's own tool session's,** `BMO_MCP_TOKEN`, so the call is made under the caller's own
  record, its capabilities and grants. Where none is set, `use` says so and ends with 2.
- **The data root** is `BMO_DATA_DIR`, else `.local/share/bmo/data` in the home directory.
- **The tool is resolved first with `describe`,** read by its structured content alone: `reachedBy` names which of
  `call` and `act` reaches it and `input` is its input schema. A tool that only reads is called with `call`, one
  that acts with `act`.

### Finding a tool

- **`--tools` asks the toolbook's `list`,** with `query` where `--query` names words, and writes each item of its
  structured content one a line: the tool's full name, `call` or `act`, and the first sentence of its description,
  separated by two spaces. These are the tools the caller's session reaches through the toolbook rather than by
  their own names, as the tool host states it; a tool the session is shown by its own name is called through `use`
  as well, by its full name. Where none is listed, `use` says so on standard error and ends with 0. A refusal of
  the list is said as a call's is (「The answer」), the tool named `list`.
- **`<tool> --describe` asks the toolbook's `describe`** and writes its structured content as one JSON object: the
  tool in the shape the tool host answers a tool in, whose `description`, `manual`, `input`, `output`, `hints` and
  `reachedBy` say what it does, how its arguments are filled and which of `call` and `act` reaches it. An unknown
  tool is refused as a call of it is.

### The arguments

- **`--arg <key>=<value>`,** given any number of times, sets one argument. Where the tool's input schema types
  the key as a string, the value is that string as written; otherwise it is read as JSON where it reads, a number,
  a boolean, an array or an object, and as a string where it does not.
- **`--json <object>`** gives the arguments as one JSON object; each `--arg` is laid over it. No argument given
  is the empty object.

### The answer

- **Synchronous, the default:** the tool's answer on standard output. Where the tool answers structured content,
  that object as JSON; where every piece it answers is text, each piece's text as it stands; otherwise its
  content pieces as one JSON array.
- **A refusal** is a line on standard error, ``bmo: `<tool>` was refused: <code>: <why>``, and `use` ends with 1:
  - the host's refusal of the request, by its error body's `code` and `error`;
  - a tool answer marked failed, the toolbook's refusal of an unknown tool or of a `call` of a tool that acts
    among them, by the code `refused` and the answer's text;
  - a host out of reach, by the code `unreachable`;
  - a `describe` or `list` answer without the structured content stated above, by the code `unread`.
- **Exit codes:** 0 where the tool, or the toolbook asked by `--tools` or `--describe`, answered, 1 where the call
  was refused, 2 where the words or the token could not be read.

### Asynchronous: the tool host keeps the call

- **`--async` resolves the tool as a synchronous call does,** so an unknown tool is refused at once, then asks the
  tool host to keep and run the call: the host takes the request over its calls face (`POST /use` on the tools
  face, under the caller's own token, as the tool host's documents state it), keeps the record, runs the call
  under the caller's session and wakes the agent through the receiver. The caller's shell writes nothing outside
  its workspace, so a shell under a file sandbox asks as any other does.
- **The answer on standard output** is the host's: one JSON object `{id, state, record}`, the call's id, its state
  and where the host keeps the record. The caller's turn goes on; the answer comes as a wake.
- **A host that serves no calls face** — one older than the face, which answers no route at `GET /use` — leaves
  the caller to keep the call itself: the record stands in the caller's workspace, under `.bmo/use/` in the
  directory the command was run from, and the call is started as a unit of its own as 「The caller's own unit」
  states. `--status` and `--list` read those records where the host serves no face.
- **A call the host refuses** — an unknown tool, a body it does not read, a host out of reach — is said on
  standard error and `use` ends with 1, and nothing is kept.
- **The id** is the tool's namespace and name joined by `-`, every character but a letter, a digit, `_` and `-`
  made `-`, then `-` and eight hexadecimal digits drawn afresh. A call the caller keeps itself is `bmo-use-<id>`.
- **`--memory <words>`** is what the caller wants handed back with the answer: what it was waiting for and what it
  meant to do next. It is carried in the request and in the wake.
- **Whom the wake is for:** the room `--room` names, else `BMO_TURN_CHAT`; the agent `--agent` names, else
  `BMO_TURN_AGENT`; the road `--via` before the first word names, else `BMO_TURN_VIA`. Each is left out where none
  is named, and the receiver chooses as its route states.
- **A write the shell's sandbox refused** is said with what refused it: the error names the shell's file sandbox,
  the workspace it allows and the path the record was to stand in, so a caller tells a sandbox from a mode bit.

### The caller's own unit

- **The road of a shell whose host serves no calls face,** and of the records a caller keeps itself: the request is
  kept before the unit starts, the unit is `bmo-use-<id>`, and its answer and end are kept in the same record
  folder, which the wake names as the answer's path there.
- **The unit** is started with `systemd-run --user --unit=bmo-use-<id> --collect --quiet`, then its longest run
  as `--property=RuntimeMaxSec=<n>`, its last word as `--property=ExecStopPost=<bmo> use --ended <id>`, each
  variable below set with `--setenv`, and runs the same `bmo` binary as `use --run <id>`; `--run` and `--ended`
  are words for the unit alone. It is handed `BMO_DATA_DIR`, the data root resolved, `BMO_USE_RECORD_DIR`, the
  record folder, and, by name from the caller's environment where they are set, `BMO_MCP_TOKEN`,
  `BMO_MCP_HOST_PORT`, `RECEIVER_ADDRESS` and `RECEIVER_TOKEN`; no token stands on its command line.
- **Its longest run** is `--max-seconds <n>` seconds, a positive whole number, else 86400, one day. A unit still
  running then is stopped, and its last word wakes the agent with the failure.
- **The record is a folder,** `<record dir>/<id>/`, readable by the user alone, the record dir being the host's
  `use/` under its data root or the caller's own `.bmo/use/` in its workspace:
  - `request.json`, written before the unit starts: `id`, `tool`, `reachedBy` (`call` or `act`), `arguments`,
    `memory`, `room`, `agent`, `via`, `unit`, `maxSeconds` and `startedAt`;
  - `answer.json`, once the call ends: the tool's result as the host answered it, or the refusal or the failure as
    `{code, error}`;
  - `end.json`, last: `endedAt`, `state` (`done`, `refused` or `failed`), `summary`, and `wake`, the instruction's
    `id` where the receiver queued it or `error` where it did not.
- **The run calls the tool as a synchronous call does,** through the proxy the request names; a run with no
  token is refused by the code `unauthorized`. A refusal is its answer and ends it as `refused`.
- **The unit's last word, `use --ended <id>`,** runs whenever the unit stops, however it stopped. Where the run
  kept its end, it does nothing. Where it did not, the run died before it ended: stopped at its longest run, killed,
  or failed outside the call. The last word then keeps the failure as the answer, `{code: "failed", error}`, where
  no answer stands, keeps the end as `failed`, and wakes the agent with it as a run's end does, so no agent is left
  waiting on a unit that is gone. Its `error` names how the unit ended, by the service result, exit code and exit
  status systemd hands it.
- **The token a unit carries is the caller's, as it stood when `use --async` was asked.** `use` issues none and
  refreshes none, and keeps none on disk. The host checks it when the call reaches it, and what the record's tools
  keep for a call ends with the record, so the token must stand for the unit's whole run, which is at most its
  longest run. A tool session issued with an expiry lapses then; one issued without stands until its issuer
  revokes it, as an issuer does when the program carrying it is replaced. An issuer that hands a session `use`
  therefore issues its token to outlast the longest run, or stands it until it is revoked; a unit whose token
  lapsed or was revoked first is refused by the code `unauthorized`, or dies under its call and ends as `failed`,
  and the agent is woken with either.

### The wake

- **The host wakes the agent through the receiver's `POST /instructions`,** at `RECEIVER_ADDRESS` with
  `RECEIVER_TOKEN` as the bearer, else the two the data root's `env/receiver.env` names, once the answer is kept;
  the end is kept after it, with the wake's outcome. A call the caller keeps itself wakes as its unit's run once
  kept its answer, with the same body. The instruction is a fact, queued in the room for the agent as a job's end
  is:
  - `agentName`, `room` and `via` as above;
  - `said.fact`: `use`, the id; `tool`; `state`; `summary`; and `memory`, and, for a call the caller keeps,
    `answer`, the path of `answer.json` there;
  - `sort` `use-ended` and `summary`, the one-line summary;
  - no `kind`, so no wake replaces another, and no expiry, events, ride or run.
- **The summary** is one line: ``<tool> answered: <first line of its first text>``, ``<tool> was refused:
  <code>: <why>`` or ``<tool> failed: <why>``, cut to 160 characters.
- **A wake the receiver refuses or does not answer** is kept in the record's `end.json` as its `wake.error`, and a
  caller's own unit ends with 1; its end is kept, so the last word does not wake again.

### Reading the records

- **`bmo use --list`** asks the host's `GET /use` and writes each call, oldest first, one a line: its id, its state
  and its tool, separated by two spaces. Where the host serves no calls face it reads the records the caller keeps
  itself.
- **`bmo use --status <id>`** asks the host's `GET /use/{id}` and writes the record as a JSON object: the request's
  fields, `state`, and where it ended, `endedAt`, `summary` and `wake`; a caller's own record adds `answer`, the
  path of `answer.json`. An id the host does not hold, or a record that stands nowhere, ends with 2.
- **A record's state** is its end's where `end.json` stands; else `running` while the host runs it, or while its
  unit is active for a call the caller keeps; and `lost` once it runs neither, as where the machine restarted
  under it.

### Waiting is the caller's, not the tool's

- **Whether a turn waits for a tool is chosen once, by `--sync` or `--async`,** never by an argument of the tool:
  every tool runs the same either way, and the flag decides only whether the asking call or a later wake takes
  the answer.
- **A reminder** is an asynchronous `use` of a tool that waits for a moment and answers with a note, the
  receiver's `bmo/wait`; **a watch** is one of the same tool with its `check`, answering once the condition
  holds; **a long call** is any tool run asynchronously, a draw that announces its result in a later turn
  among them. Each wakes the agent once, as 「The wake」 states, with the tool's answer and the `--memory` it was
  given; a repeating reminder is the next one set by the turn its wake opens.
- **A wait longer than a day** needs `--max-seconds` past it, since the unit is stopped at its longest run.
- **Cancelling one** is stopping its unit, `bmo-use-<id>`; its last word keeps the end as `failed` and wakes the
  agent with it, so the agent learns the wait is gone.

## Where `bmo` stands

- **Every deployment links `bmo` into `.local/bin` under the home directory,** before the flip, making the folder
  where it does not stand and replacing whatever stood under that name. The link names the deploy tree's own link,
  `.local/share/bmo-cli/deploy/bin/bmo`, which stands across releases, so it names whichever release is live and
  never dangles once an older one is pruned.
- **A program finds `bmo` by its bare name where its `PATH` holds that folder,** a session's harness among them.
  The deployment guarantees the link, not any program's `PATH`: a service that starts programs puts the folder on
  the `PATH` it hands them, as its own documents state.
- **A unit command another unit's deployment links beside `bmo`** is found there first (「The unit command」).

## What `bmo` does not hold

- **No per-unit command list.** Each unit's own document names the words its command carries; `bmo` serves a
  noun only where the unit declares it, and the git rule only where the unit command stands.
- **No call to a unit's service but the tool verb's.** `bmo` calls no unit's route but the tool host's tools
  face and the receiver's instruction route, both for `use`, and reads no unit's code, tables or files; of the
  data root it reads only the env files those two units publish for their callers and writes only its own `use/`
  folder.
- **No settings of its own.** `bmo` keeps no settings file; a unit reads what its own settings name, and the tool
  verb reads the variables and env files 「The tool verb」 names.
- **No alias table.** An alias is served only where a unit declares it as a noun or the git rule reaches it.

## Versioning

- **The contract carries the repository's one version,** the version in `VERSION` and the Cargo manifest. It
  moves only at a release the owner declares, as bmo-chat's `docs/conventions.md` 「Versions」 states; a
  contract change moves none, and a deployment is told apart by its commit.
