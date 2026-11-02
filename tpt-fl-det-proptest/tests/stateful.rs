//! Tests for stateful, model-based testing in `tpt-fl-det-proptest`.

use tpt_fl_det_proptest::{check_stateful, replay_run, Seed, StateMachine, Xorshift64};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Op {
    Add(u32),
    Reset,
}

/// A counter with a planted bug: the SUT saturates at `cap`, the model does not.
struct Counter {
    cap: Option<u32>,
}

impl StateMachine for Counter {
    type Sut = u32;
    type Model = u32;
    type Command = Op;

    fn new_sut(&self) -> u32 {
        0
    }

    fn new_model(&self) -> u32 {
        0
    }

    fn gen_command(&self, _model: &u32, rng: &mut Xorshift64) -> Op {
        if rng.next_u32() % 8 == 0 {
            Op::Reset
        } else {
            Op::Add(1 + rng.next_u32() % 4)
        }
    }

    fn apply(&self, sut: &mut u32, model: &mut u32, cmd: &Op) -> Result<(), String> {
        match cmd {
            Op::Add(n) => {
                *model += n;
                *sut = match self.cap {
                    Some(cap) => (*sut + n).min(cap),
                    None => *sut + n,
                };
            }
            Op::Reset => {
                *model = 0;
                *sut = 0;
            }
        }
        if sut == model {
            Ok(())
        } else {
            Err(format!("sut {sut} != model {model}"))
        }
    }
}

#[test]
fn correct_machine_passes_all_runs() {
    let machine = Counter { cap: None };
    assert_eq!(check_stateful(&machine, 200, 40, Seed(3)), Ok(()));
}

#[test]
fn planted_bug_is_found_and_shrunk() {
    let machine = Counter { cap: Some(10) };
    let failure = check_stateful(&machine, 200, 40, Seed(3)).unwrap_err();

    // The shrunk sequence still fails, at its last command.
    let total: u32 = failure
        .commands
        .iter()
        .map(|c| match c {
            Op::Add(n) => *n,
            Op::Reset => 0,
        })
        .sum();
    assert!(total > 10);
    assert_eq!(failure.failing_index, failure.commands.len() - 1);
    assert!(failure.commands.len() < failure.original_len);

    // 1-minimality: dropping any single command makes the sequence pass.
    for i in 0..failure.commands.len() {
        let mut shorter = failure.commands.clone();
        shorter.remove(i);
        let mut sut = 0;
        let mut model = 0;
        let failed = shorter
            .iter()
            .any(|cmd| machine.apply(&mut sut, &mut model, cmd).is_err());
        assert!(!failed, "removing command {i} should make the run pass");
    }
}

#[test]
fn run_seed_replays_the_same_failure() {
    let machine = Counter { cap: Some(10) };
    let failure = check_stateful(&machine, 200, 40, Seed(3)).unwrap_err();
    let replayed = replay_run(&machine, failure.run_seed, 40).unwrap_err();
    assert_eq!(replayed, failure);
}

#[test]
fn same_seed_gives_the_same_result() {
    let machine = Counter { cap: Some(10) };
    let a = check_stateful(&machine, 200, 40, Seed(9));
    let b = check_stateful(&machine, 200, 40, Seed(9));
    assert_eq!(a, b);
}

#[test]
fn failure_message_names_the_run_seed() {
    let machine = Counter { cap: Some(10) };
    let failure = check_stateful(&machine, 200, 40, Seed(3)).unwrap_err();
    assert!(failure
        .to_string()
        .contains(&format!("{:#x}", failure.run_seed)));
}
