//! The home bridge (docs/api.md, 「The home bridge」): a home-bound command asked on a work node is
//! handed whole to a stand-in bridge on a socket of the bench's, and its answer is the run's.

mod support;

use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::thread::JoinHandle;

use support::{Bench, errors, lines};

// What the stand-in bridge was asked: the request line, and the standing input its frames carried.
struct Asked {
  request: serde_json::Value,
  input: Vec<u8>,
}

// A stand-in bridge answering one run: it reads the request and the input frames to their end, then answers
// with `out` on channel 1, `err` on channel 2 and `code` on channel 3.
fn bridge(socket: &Path, out: &'static str, err: &'static str, code: i32) -> JoinHandle<Asked> {
  let listener = UnixListener::bind(socket).unwrap();

  std::thread::spawn(move || {
    let (stream, _) = listener.accept().unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let mut input = Vec::new();

    loop {
      let mut head = [0u8; 5];
      reader.read_exact(&mut head).unwrap();
      let mut body = vec![0; u32::from_be_bytes([head[1], head[2], head[3], head[4]]) as usize];
      reader.read_exact(&mut body).unwrap();

      if head[0] == 0 {
        break;
      }

      input.extend(body);
    }

    let mut writer = stream;

    for (channel, body) in [
      (1u8, out.as_bytes().to_vec()),
      (2, err.as_bytes().to_vec()),
      (3, code.to_be_bytes().to_vec()),
    ] {
      writer.write_all(&[channel]).unwrap();
      writer.write_all(&(body.len() as u32).to_be_bytes()).unwrap();
      writer.write_all(&body).unwrap();
    }

    Asked {
      request: serde_json::from_str(&line).unwrap(),
      input,
    }
  })
}

fn home_unit(bench: &Bench) {
  bench.command(
    "bmo-job",
    "if [ \"$1\" = nouns ]; then printf 'job\\njob ask\\thome\\ntodo\\thome\\n'; exit 0; fi\necho local\necho \"$@\"",
  );
}

#[test]
fn a_home_bound_command_on_a_node_is_handed_whole_to_the_bridge() {
  let bench = Bench::new();
  home_unit(&bench);
  let socket = bench.path().join("bridge.sock");
  let answered = bridge(&socket, "from home\n", "warned at home\n", 5);

  let output = bench.bmo_given(
    &["--via", "r", "todo", "--add", "x"],
    "the standing input",
    &[
      ("BMO_HOME_BRIDGE", socket.to_str().unwrap()),
      ("BMO_NODE", "n1"),
      ("UNRELATED", "kept here"),
    ],
  );
  let asked = answered.join().unwrap();

  assert_eq!(output.status.code(), Some(5));
  assert_eq!(lines(&output), ["from home"]);
  assert_eq!(errors(&output), "warned at home\n");
  assert_eq!(asked.request["script"], "bin/bmo");
  assert_eq!(asked.request["args"], serde_json::json!(["--via", "r", "todo", "--add", "x"]));
  assert_eq!(asked.request["env"]["BMO_NODE"], "n1");
  assert!(asked.request["env"].get("UNRELATED").is_none());
  assert!(asked.request["cwd"].is_string());
  assert_eq!(asked.input, b"the standing input");
}

#[test]
fn a_command_without_the_home_mark_runs_where_it_was_asked() {
  let bench = Bench::new();
  home_unit(&bench);
  let socket = bench.path().join("bridge.sock");

  let output = bench.bmo_given(&["job", "close"], "", &[("BMO_HOME_BRIDGE", socket.to_str().unwrap())]);

  assert!(output.status.success(), "{}", errors(&output));
  assert_eq!(lines(&output), ["local", "job close"]);
}

#[test]
fn a_home_bound_command_runs_here_where_no_bridge_is_named() {
  let bench = Bench::new();
  home_unit(&bench);

  let output = bench.bmo(&["job", "ask", "--talk", "1"]);

  assert_eq!(lines(&output), ["local", "job ask --talk 1"]);
}

#[test]
fn a_bridge_out_of_reach_is_refused_and_the_command_not_run() {
  let bench = Bench::new();
  home_unit(&bench);
  let socket = bench.path().join("absent.sock");

  let output = bench.bmo_given(
    &["todo"],
    "",
    &[
      ("BMO_HOME_BRIDGE", socket.to_str().unwrap()),
      ("BMO_HOME_BRIDGE_WAIT_MS", "1"),
      ("BMO_NODE", "n1"),
    ],
  );

  assert_eq!(output.status.code(), Some(2));
  assert!(output.stdout.is_empty());
  assert!(
    errors(&output).starts_with("home bridge: the operations machine is out of reach from n1 ("),
    "{}",
    errors(&output)
  );
  assert!(errors(&output).ends_with("; bin/bmo was not run\n"), "{}", errors(&output));
}

#[test]
fn a_bridge_that_drops_the_run_is_refused() {
  let bench = Bench::new();
  home_unit(&bench);
  let socket = bench.path().join("bridge.sock");
  let listener = UnixListener::bind(&socket).unwrap();
  let dropping = std::thread::spawn(move || {
    let (stream, _) = listener.accept().unwrap();
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).unwrap();
  });

  let output = bench.bmo_given(&["todo"], "", &[("BMO_HOME_BRIDGE", socket.to_str().unwrap())]);
  dropping.join().unwrap();

  assert_eq!(output.status.code(), Some(2));
  assert_eq!(errors(&output), "home bridge: the operations machine dropped bin/bmo before it ended\n");
}
