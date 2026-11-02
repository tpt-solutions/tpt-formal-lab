//! Tests for record and replay in `tpt-fl-deterministic-sim`.

use tpt_fl_deterministic_sim::{DeterministicSim, StepLog};

fn counter_sim(bump_at: Option<u64>) -> DeterministicSim<u64> {
    let mut sim = DeterministicSim::<u64>::new();
    sim.spawn(0);
    sim.add_system("count", move |entities, ctx| {
        // Adds 1 each step, plus 100 once at `bump_at` for a tampered run.
        for v in entities.values_mut() {
            *v += 1;
            if Some(ctx.step()) == bump_at {
                *v += 100;
            }
        }
    });
    sim
}

#[test]
fn identical_runs_produce_identical_logs() {
    let mut a = counter_sim(None);
    let mut log_a = StepLog::new();
    a.step_n_logged(20, &mut log_a);

    let mut b = counter_sim(None);
    let mut log_b = StepLog::new();
    b.step_n_logged(20, &mut log_b);

    assert_eq!(log_a, log_b);
    assert_eq!(log_a.first_divergence(&log_b), None);
    assert_eq!(log_a.head(), log_b.head());
}

#[test]
fn replay_matches_a_recorded_log() {
    let mut a = counter_sim(None);
    let mut log = StepLog::new();
    a.step_n_logged(15, &mut log);

    let mut replay = counter_sim(None);
    assert_eq!(replay.verify_log(&log), Ok(()));
}

#[test]
fn tampered_run_is_located_at_the_first_bad_step() {
    let mut good = counter_sim(None);
    let mut log = StepLog::new();
    good.step_n_logged(30, &mut log);

    // `ctx.step()` is the count before the step runs, so this changes the state
    // recorded at log index 12.
    let mut bad = counter_sim(Some(12));
    let mut bad_log = StepLog::new();
    bad.step_n_logged(30, &mut bad_log);

    assert_eq!(log.first_divergence(&bad_log), Some(12));

    let mut replay = counter_sim(None);
    let err = replay.verify_log(&bad_log).unwrap_err();
    assert_eq!(err.index, 12);
    assert_ne!(err.expected, err.actual);
}

#[test]
fn verify_log_reports_the_divergence_point() {
    let mut good = counter_sim(None);
    let mut log = StepLog::new();
    good.step_n_logged(10, &mut log);

    let mut tampered = counter_sim(Some(5));
    let mut bad_log = StepLog::new();
    tampered.step_n_logged(10, &mut bad_log);

    let mut replay = counter_sim(None);
    let err = replay.verify_log(&bad_log).unwrap_err();
    assert_eq!(err.index, 5);
    assert!(err.to_string().contains("entry 5"));
}

#[test]
fn prefix_logs_diverge_at_the_shorter_length() {
    let mut a = counter_sim(None);
    let mut long = StepLog::new();
    a.step_n_logged(8, &mut long);

    let mut b = counter_sim(None);
    let mut short = StepLog::new();
    b.step_n_logged(5, &mut short);

    assert_eq!(short.first_divergence(&long), Some(5));
    assert_eq!(long.first_divergence(&short), Some(5));
}

#[test]
fn empty_logs_agree() {
    let empty = StepLog::new();
    assert!(empty.is_empty());
    assert_eq!(empty.first_divergence(&StepLog::new()), None);
    assert_eq!(empty.head(), StepLog::new().head());
}
