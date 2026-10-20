# The fixed API: how `bmo` finds a unit's command and hands it on

**고정됨 1.0.0.**

`bmo` holds no API and runs no service. It reads the words it is given, finds the unit command that carries
them, and hands them on; what it answers itself is the help, the version and a refusal. This document is the
whole of that contract: the command-finding rule and the hand-over, and nothing else. `bmo` publishes no HTTP
API, so it publishes no OpenAPI schema and no generated client; its fixed API is the command line below.

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
- **Every unit's command list is its own.** A unit's document names the nouns its command carries; `bmo` keeps
  no list of them, and a noun a unit does not declare is not served.

## Resolving a command

The words `bmo` is given are read in this order, and the first rule that matches decides.

1. **A declared noun.** The leading words are compared with every noun every unit command declares, and the
   longest noun matching them wins. `bmo` runs that unit command with all the words, the noun included:
   `bmo todo --add` runs `bmo-job todo --add`. Where two unit commands declare the longest matching noun, `bmo`
   refuses, naming the noun and both commands, and ends with 2.
2. **The git rule.** Where the first word is `<name>` and a unit command `bmo-<name>` stands, `bmo` runs it
   with the words after the first: `bmo graph nodes` runs `bmo-graph nodes`. The unit's own answer stands, its
   refusal of words it does not carry included.
3. **Not served.** Nothing above matched (「What `bmo` answers itself」).

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
| words nothing serves | ``bmo: `<words>` is not served yet: no installed unit carries it`` on standard error | 2 |
| a noun two unit commands declare | ``bmo: `<noun>` is declared by both `bmo-<one>` and `bmo-<other>` `` on standard error | 2 |
| a unit command that cannot be started | ``bmo: `bmo-<unit>` could not be started: <reason>`` on standard error | 1 |
| an option before the first word that `bmo` does not read | ``bmo: `<option>` is not an option of bmo; options of a command follow its words`` on standard error | 2 |

- **The help** lists the global options, then every noun found, each with the unit command declaring it, then
  every unit command the git rule reaches.
- **The words of a not-served answer** are the words as given, the leading options taken out.

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

## What `bmo` does not hold

- **No per-unit command list.** Each unit's own document names the words its command carries; `bmo` serves a
  noun only where the unit declares it, and the git rule only where the unit command stands.
- **No call to a unit's service.** `bmo` calls no unit's route and reads no unit's code, tables or files.
- **No settings.** `bmo` reads no settings file; a unit reads what its own settings name.
- **No alias table.** An alias is served only where a unit declares it as a noun or the git rule reaches it.

## Versioning

- **The contract carries the repository's one version, in semver,** the version in `VERSION` and the Cargo
  manifest.
- **The major moves on a change a caller or a unit reads:** the resolution order, the reading of `nouns`, the
  placing of `--via`, an exit code, or an answer's words. The minor moves on an addition: a mark `bmo`
  understands, an option of `bmo`. The patch moves on anything else.
