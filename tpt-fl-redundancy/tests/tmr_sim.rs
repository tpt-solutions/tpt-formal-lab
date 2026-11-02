//! Triple modular redundancy inside a deterministic simulation.
//!
//! Three replica entities compute the same value each step. A fault system
//! corrupts some replicas according to a schedule drawn from a seeded RNG, and a
//! voter system takes the majority of the three. Because the schedule and the
//! simulation are both deterministic, the same seed always gives the same run.

use std::cell::RefCell;
use std::rc::Rc;

use tpt_fl_det_proptest::Xorshift64;
use tpt_fl_deterministic_sim::{DeterministicSim, EntityId, StepLog};
use tpt_fl_redundancy::{Replicated, VoteResult};

/// `(step, output)` for every step, recorded by the voter.
type Outputs = Rc<RefCell<Vec<(u64, i64)>>>;
/// Steps at which the voter found no majority.
type Detections = Rc<RefCell<Vec<u64>>>;

/// The value every correct replica computes at `step`.
fn truth(step: u64) -> i64 {
    step as i64 * 7 + 3
}

/// Builds a schedule with at most one corrupted replica per step, drawn from
/// `seed`. If `double_at` is given, replicas 0 and 1 are both corrupted at that
/// step.
fn schedule(seed: u64, steps: usize, double_at: Option<usize>) -> Vec<Vec<usize>> {
    let mut rng = Xorshift64::new(seed);
    (0..steps)
        .map(|step| {
            if Some(step) == double_at {
                vec![0, 1]
            } else if rng.next_u32() % 2 == 0 {
                vec![(rng.next_u32() % 3) as usize]
            } else {
                Vec::new()
            }
        })
        .collect()
}

/// Builds the TMR simulation. The returned ids are the three replicas and the
/// output entity.
fn build(
    schedule: Vec<Vec<usize>>,
    outputs: Outputs,
    detections: Detections,
) -> (DeterministicSim<i64>, Vec<EntityId>, EntityId) {
    let mut sim = DeterministicSim::<i64>::new();
    let replicas: Vec<EntityId> = (0..3).map(|_| sim.spawn(0)).collect();
    let output = sim.spawn(0);

    // Systems run in name order: compute, then fault, then vote.
    let compute_ids = replicas.clone();
    sim.add_system("a_compute", move |entities, ctx| {
        for id in &compute_ids {
            *entities.get_mut(id).unwrap() = truth(ctx.step());
        }
    });

    let fault_ids = replicas.clone();
    sim.add_system("b_fault", move |entities, ctx| {
        let hits = schedule
            .get(ctx.step() as usize)
            .cloned()
            .unwrap_or_default();
        for &i in &hits {
            // Each replica gets its own wrong value, so two corrupted replicas
            // can never agree with each other.
            let wrong = truth(ctx.step()) + 1000 * (i as i64 + 1);
            *entities.get_mut(&fault_ids[i]).unwrap() = wrong;
        }
    });

    let vote_ids = replicas.clone();
    sim.add_system("c_vote", move |entities, ctx| {
        let [a, b, c] = [
            *entities.get(&vote_ids[0]).unwrap(),
            *entities.get(&vote_ids[1]).unwrap(),
            *entities.get(&vote_ids[2]).unwrap(),
        ];
        let out = entities.get_mut(&output).unwrap();
        match Replicated::new([a, b, c]).majority_vote() {
            VoteResult::Unanimous(v) | VoteResult::Majority(v) => *out = v,
            VoteResult::NoMajority => {
                // Keep the previous output, and record that the voter gave up.
                detections.borrow_mut().push(ctx.step());
            }
        }
        outputs.borrow_mut().push((ctx.step(), *out));
    });

    (sim, replicas, output)
}

#[test]
fn single_faults_never_change_the_output() {
    for seed in 1..=20 {
        let outputs = Rc::new(RefCell::new(Vec::new()));
        let detections = Rc::new(RefCell::new(Vec::new()));
        let (mut sim, _, _) = build(
            schedule(seed, 200, None),
            outputs.clone(),
            detections.clone(),
        );
        sim.step_n(200);

        assert!(detections.borrow().is_empty(), "seed {seed}");
        for &(step, out) in outputs.borrow().iter() {
            assert_eq!(out, truth(step), "seed {seed}, step {step}");
        }
    }
}

#[test]
fn double_fault_is_detected_and_no_wrong_value_is_emitted() {
    let outputs = Rc::new(RefCell::new(Vec::new()));
    let detections = Rc::new(RefCell::new(Vec::new()));
    let (mut sim, _, _) = build(
        schedule(7, 100, Some(50)),
        outputs.clone(),
        detections.clone(),
    );
    sim.step_n(100);

    assert_eq!(*detections.borrow(), vec![50]);
    for &(step, out) in outputs.borrow().iter() {
        if step == 50 {
            // The voter keeps the previous output rather than emit a wrong value.
            assert_eq!(out, truth(49));
        } else {
            assert_eq!(out, truth(step), "step {step}");
        }
    }
}

#[test]
fn tmr_run_replays_from_its_log() {
    let steps = 60;
    let outputs = Rc::new(RefCell::new(Vec::new()));
    let detections = Rc::new(RefCell::new(Vec::new()));
    let (mut sim, _, _) = build(
        schedule(11, steps, None),
        outputs.clone(),
        detections.clone(),
    );
    let mut log = StepLog::new();
    sim.step_n_logged(steps as u64, &mut log);

    let (mut replay, _, _) = build(
        schedule(11, steps, None),
        Rc::new(RefCell::new(Vec::new())),
        Rc::new(RefCell::new(Vec::new())),
    );
    assert_eq!(replay.verify_log(&log), Ok(()));
}
