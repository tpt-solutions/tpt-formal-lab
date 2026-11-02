//! Stateful, model-based testing with deterministic command sequences.
//!
//! A [`StateMachine`] describes an implementation under test (the SUT), a simple
//! reference model, and the commands that drive both. Each run generates a
//! sequence of commands from a per-run seed, applies it to both, and checks
//! that they agree after every command. A failing sequence is shrunk by removing
//! commands while it still fails. The failure carries the run seed, so
//! [`replay_run`] regenerates it exactly.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use crate::rng::Xorshift64;
use crate::strategy::Seed;

/// A system under test, a reference model, and the commands that drive them.
pub trait StateMachine {
    /// The implementation under test.
    type Sut;
    /// A simple reference model of the expected behaviour.
    type Model;
    /// One operation, applied to both the SUT and the model.
    type Command: Clone + fmt::Debug;

    /// Creates a fresh SUT for one run.
    fn new_sut(&self) -> Self::Sut;

    /// Creates a fresh model for one run.
    fn new_model(&self) -> Self::Model;

    /// Chooses the next command. It may read the model, so generated commands can
    /// respect preconditions.
    fn gen_command(&self, model: &Self::Model, rng: &mut Xorshift64) -> Self::Command;

    /// Applies `cmd` to the SUT and the model, then checks that they agree.
    ///
    /// Shrinking removes commands, so a command may later run in a state its
    /// generator never saw. Such a command should do nothing on both sides,
    /// rather than panic.
    ///
    /// # Errors
    ///
    /// Returns a message describing the disagreement.
    fn apply(
        &self,
        sut: &mut Self::Sut,
        model: &mut Self::Model,
        cmd: &Self::Command,
    ) -> Result<(), String>;
}

/// A failing command sequence, after shrinking.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SequenceFailure<C> {
    /// The seed of the failing run. Pass it to [`replay_run`] to regenerate the
    /// failure.
    pub run_seed: u64,
    /// The length of the generated sequence, up to and including the failing
    /// command, before shrinking.
    pub original_len: usize,
    /// The shrunk sequence. It still fails, and no single command can be removed
    /// from it while it still fails.
    pub commands: Vec<C>,
    /// The index in `commands` of the command that fails.
    pub failing_index: usize,
    /// The message from [`StateMachine::apply`].
    pub message: String,
}

impl<C: fmt::Debug> fmt::Display for SequenceFailure<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "stateful check failed (run seed {:#x}, shrunk {} -> {} commands): {} at command {} of {:?}",
            self.run_seed,
            self.original_len,
            self.commands.len(),
            self.message,
            self.failing_index,
            self.commands
        )
    }
}

/// Runs `runs` sequences of at most `max_len` commands each.
///
/// The per-run seeds come from `seed`, so the same arguments always test the
/// same sequences.
///
/// # Errors
///
/// Returns the first failing run, shrunk to a minimal sequence.
///
/// # Example
///
/// ```rust
/// use tpt_fl_det_proptest::{check_stateful, Seed, StateMachine, Xorshift64};
///
/// // A counter whose SUT saturates at 10, while the model does not.
/// struct Capped;
/// impl StateMachine for Capped {
///     type Sut = u32;
///     type Model = u32;
///     type Command = u32;
///     fn new_sut(&self) -> u32 { 0 }
///     fn new_model(&self) -> u32 { 0 }
///     fn gen_command(&self, _m: &u32, rng: &mut Xorshift64) -> u32 {
///         1 + rng.next_u32() % 4
///     }
///     fn apply(&self, sut: &mut u32, model: &mut u32, cmd: &u32) -> Result<(), String> {
///         *model += cmd;
///         *sut = (*sut + cmd).min(10);
///         if sut == model { Ok(()) } else { Err(format!("sut {sut} != model {model}")) }
///     }
/// }
///
/// let failure = check_stateful(&Capped, 100, 20, Seed(1)).unwrap_err();
/// assert!(failure.commands.len() <= 4);
/// ```
pub fn check_stateful<M: StateMachine>(
    machine: &M,
    runs: usize,
    max_len: usize,
    seed: Seed,
) -> Result<(), SequenceFailure<M::Command>> {
    let mut rng = Xorshift64::new(seed.0);
    for _ in 0..runs {
        let run_seed = rng.next_u64();
        run_one(machine, run_seed, max_len)?;
    }
    Ok(())
}

/// Regenerates and runs the sequence for one run seed, as reported in a
/// [`SequenceFailure`].
///
/// # Errors
///
/// Returns the failure of that run, shrunk, if the run fails.
pub fn replay_run<M: StateMachine>(
    machine: &M,
    run_seed: u64,
    max_len: usize,
) -> Result<(), SequenceFailure<M::Command>> {
    run_one(machine, run_seed, max_len)
}

fn run_one<M: StateMachine>(
    machine: &M,
    run_seed: u64,
    max_len: usize,
) -> Result<(), SequenceFailure<M::Command>> {
    let mut rng = Xorshift64::new(run_seed);
    let len = (rng.next_u64() % (max_len as u64 + 1)) as usize;
    let mut sut = machine.new_sut();
    let mut model = machine.new_model();
    let mut commands = Vec::with_capacity(len);
    for _ in 0..len {
        let cmd = machine.gen_command(&model, &mut rng);
        commands.push(cmd.clone());
        if let Err(message) = machine.apply(&mut sut, &mut model, &cmd) {
            // The sequence fails at its last command, so shrinking starts there.
            let original_len = commands.len();
            let minimal = shrink(machine, commands);
            let (failing_index, message) = match run_commands(machine, &minimal) {
                Err(failure) => failure,
                // Unreachable: `shrink` only returns failing sequences. Keep the
                // generated failure if it ever happens.
                Ok(()) => (original_len - 1, message),
            };
            return Err(SequenceFailure {
                run_seed,
                original_len,
                commands: minimal,
                failing_index,
                message,
            });
        }
    }
    Ok(())
}

/// Applies a fixed sequence to fresh SUT and model. Returns the index and message
/// of the first failing command.
fn run_commands<M: StateMachine>(
    machine: &M,
    commands: &[M::Command],
) -> Result<(), (usize, String)> {
    let mut sut = machine.new_sut();
    let mut model = machine.new_model();
    for (i, cmd) in commands.iter().enumerate() {
        machine
            .apply(&mut sut, &mut model, cmd)
            .map_err(|message| (i, message))?;
    }
    Ok(())
}

/// Greedily removes commands while the sequence still fails.
///
/// Each accepted removal makes the sequence shorter, so this terminates. The
/// result is 1-minimal: removing any one command makes it pass.
fn shrink<M: StateMachine>(machine: &M, mut commands: Vec<M::Command>) -> Vec<M::Command> {
    loop {
        let mut improved = false;
        for i in 0..commands.len() {
            let mut candidate = commands.clone();
            candidate.remove(i);
            if run_commands(machine, &candidate).is_err() {
                commands = candidate;
                improved = true;
                break;
            }
        }
        if !improved {
            return commands;
        }
    }
}
