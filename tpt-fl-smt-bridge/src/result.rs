//! Solver results, errors, and the process runner behind `check_sat`.

use std::fmt;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::counter_example::CounterExample;
use crate::sexpr::{self, SExpr};

/// The outcome of a satisfiability check.
///
/// # Example
///
/// ```rust
/// use tpt_fl_smt_bridge::SatResult;
///
/// let r = SatResult::parse("unsat\n(error \"line 5: model is not available\")\n").unwrap();
/// assert_eq!(r, SatResult::Unsat);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum SatResult {
    /// The constraints are satisfiable. The model is the witness.
    Sat(CounterExample),
    /// The constraints are unsatisfiable.
    Unsat,
    /// The solver could not decide. The string gives the reason it reported,
    /// such as `"timeout"` or `"unsupported"`.
    Unknown(String),
}

impl SatResult {
    /// Interprets a solver's standard output.
    ///
    /// The first meaningful token decides the result: `sat`, `unsat`,
    /// `unknown`, or `unsupported`. Output after `unsat` is ignored, so the
    /// `(error ...)` that Z3 prints when `(get-model)` follows `unsat` does not
    /// count as a failure.
    ///
    /// # Errors
    ///
    /// Returns [`SolverError::Solver`] if the output starts with an
    /// `(error ...)` or is not a recognised result.
    pub fn parse(output: &str) -> Result<Self, SolverError> {
        let nodes = sexpr::parse_all(output);
        let first = nodes.first();
        match first.and_then(SExpr::atom) {
            Some("sat") => Ok(SatResult::Sat(CounterExample::parse(output))),
            Some("unsat") => Ok(SatResult::Unsat),
            Some("unknown") => Ok(SatResult::Unknown(reason_unknown(output))),
            Some("unsupported") => Ok(SatResult::Unknown("unsupported".into())),
            _ => match first {
                Some(SExpr::List(items))
                    if items.first().and_then(SExpr::atom) == Some("error") =>
                {
                    Err(SolverError::Solver(
                        first.map_or_else(String::new, SExpr::to_text),
                    ))
                }
                _ => Err(SolverError::Solver(format!(
                    "unexpected solver output: {}",
                    output.trim()
                ))),
            },
        }
    }
}

/// Finds the reason in a `(:reason-unknown ...)` line, or returns `"unknown"`.
fn reason_unknown(output: &str) -> String {
    output
        .lines()
        .find_map(|line| {
            let rest = line.trim().strip_prefix("(:reason-unknown")?;
            Some(
                rest.trim()
                    .trim_end_matches(')')
                    .trim()
                    .trim_matches('"')
                    .to_string(),
            )
        })
        .unwrap_or_else(|| "unknown".into())
}

/// Reads the pairs printed by `(get-value (...))`.
///
/// Each pair is the printed term and its value, both as SMT-LIB2 text, in the
/// order they were requested. Output with no list of pairs gives an empty
/// vector.
///
/// # Example
///
/// ```rust
/// use tpt_fl_smt_bridge::parse_get_value;
///
/// let out = "sat\n((x 3) (y (- 4)))\n";
/// assert_eq!(
///     parse_get_value(out),
///     vec![("x".to_string(), "3".to_string()), ("y".to_string(), "(- 4)".to_string())]
/// );
/// ```
pub fn parse_get_value(output: &str) -> Vec<(String, String)> {
    sexpr::parse_all(output)
        .iter()
        .find_map(|node| match node {
            SExpr::List(items)
                if !items.is_empty()
                    && items
                        .iter()
                        .all(|item| matches!(item, SExpr::List(pair) if pair.len() == 2)) =>
            {
                Some(
                    items
                        .iter()
                        .filter_map(|item| match item {
                            SExpr::List(pair) => Some((pair[0].to_text(), pair[1].to_text())),
                            _ => None,
                        })
                        .collect(),
                )
            }
            _ => None,
        })
        .unwrap_or_default()
}

/// Reads an unsat core from the output of `(get-unsat-core)`, which is a
/// parenthesised list of assertion names.
///
/// # Example
///
/// ```rust
/// use tpt_fl_smt_bridge::parse_unsat_core;
///
/// assert_eq!(parse_unsat_core("unsat\n(a b)\n"), vec!["a".to_string(), "b".to_string()]);
/// ```
pub fn parse_unsat_core(output: &str) -> Vec<String> {
    sexpr::parse_all(output)
        .iter()
        .find_map(|node| match node {
            SExpr::List(items) => Some(
                items
                    .iter()
                    .filter_map(|item| item.atom().map(str::to_string))
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

/// An error from running a solver or reading its output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolverError {
    /// The solver program could not be started. The string is the program name.
    NotFound(String),
    /// The solver did not finish within the time limit and was killed.
    Timeout(Duration),
    /// The solver reported an error, or its output was not a result.
    Solver(String),
    /// Reading or writing the solver's pipes failed.
    Io(String),
}

impl fmt::Display for SolverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SolverError::NotFound(program) => write!(f, "solver `{program}` was not found"),
            SolverError::Timeout(d) => write!(f, "solver timed out after {d:?}"),
            SolverError::Solver(msg) => write!(f, "solver error: {msg}"),
            SolverError::Io(msg) => write!(f, "I/O error talking to solver: {msg}"),
        }
    }
}

impl std::error::Error for SolverError {}

/// Which solver program to run.
///
/// # Example
///
/// ```rust
/// use tpt_fl_smt_bridge::Backend;
///
/// let custom = Backend::Command {
///     program: "/opt/solvers/yices-smt2".into(),
///     args: vec!["--timeout=5".into()],
/// };
/// # let _ = custom;
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Backend {
    /// Z3, run as `z3 -in`.
    Z3,
    /// CVC5, run as `cvc5 --lang smt2`.
    Cvc5,
    /// Any SMT-LIB2 solver that reads a problem on standard input.
    Command {
        /// The program to run.
        program: String,
        /// Arguments passed before the problem is read.
        args: Vec<String>,
    },
}

impl Backend {
    fn program_and_args(&self) -> (String, Vec<String>) {
        match self {
            Backend::Z3 => ("z3".into(), vec!["-in".into()]),
            Backend::Cvc5 => ("cvc5".into(), vec!["--lang".into(), "smt2".into()]),
            Backend::Command { program, args } => (program.clone(), args.clone()),
        }
    }
}

/// Runs `backend` on `problem` and returns its standard output.
///
/// Stdin is written from a separate thread, so a large problem cannot block on
/// a full pipe. If the solver is still running after `timeout`, it is killed.
pub(crate) fn run(
    backend: &Backend,
    problem: &str,
    timeout: Duration,
) -> Result<String, SolverError> {
    let (program, args) = backend.program_and_args();
    let mut child = Command::new(&program)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => SolverError::NotFound(program.clone()),
            _ => SolverError::Io(e.to_string()),
        })?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| SolverError::Io("no stdin pipe".into()))?;
    let input = problem.to_string();
    let writer = thread::spawn(move || {
        // A solver that exits early closes its stdin; the write error is expected then.
        let _ = stdin.write_all(input.as_bytes());
    });

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| SolverError::Io("no stdout pipe".into()))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| SolverError::Io("no stderr pipe".into()))?;
    let out_reader = thread::spawn(move || {
        let mut s = String::new();
        let _ = stdout.read_to_string(&mut s);
        s
    });
    let err_reader = thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr.read_to_string(&mut s);
        s
    });

    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = writer.join();
                    let _ = out_reader.join();
                    let _ = err_reader.join();
                    return Err(SolverError::Timeout(timeout));
                }
                thread::sleep(Duration::from_millis(2));
            }
            Err(e) => return Err(SolverError::Io(e.to_string())),
        }
    };

    let _ = writer.join();
    let stdout_text = out_reader.join().unwrap_or_default();
    let stderr_text = err_reader.join().unwrap_or_default();

    if stdout_text.trim().is_empty() && !status.success() {
        return Err(SolverError::Solver(stderr_text.trim().to_string()));
    }
    Ok(stdout_text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sat_with_model() {
        let out = "sat\n(\n  (define-fun x () Int 3)\n)\n";
        match SatResult::parse(out).unwrap() {
            SatResult::Sat(ce) => assert_eq!(ce.get_int("x"), Some(3)),
            other => panic!("expected sat, got {other:?}"),
        }
    }

    #[test]
    fn unsat_ignores_model_error() {
        let out = "unsat\n(error \"line 5 column 10: model is not available\")\n";
        assert_eq!(SatResult::parse(out).unwrap(), SatResult::Unsat);
    }

    #[test]
    fn unknown_reports_reason() {
        let out = "unknown\n(:reason-unknown \"timeout\")\n";
        assert_eq!(
            SatResult::parse(out).unwrap(),
            SatResult::Unknown("timeout".into())
        );
        assert_eq!(
            SatResult::parse("unknown").unwrap(),
            SatResult::Unknown("unknown".into())
        );
    }

    #[test]
    fn unsupported_is_unknown() {
        assert_eq!(
            SatResult::parse("unsupported\n").unwrap(),
            SatResult::Unknown("unsupported".into())
        );
    }

    #[test]
    fn error_before_result_is_an_error() {
        match SatResult::parse("(error \"bad input\")\nsat\n") {
            Err(SolverError::Solver(msg)) => assert!(msg.contains("bad input")),
            other => panic!("expected solver error, got {other:?}"),
        }
    }

    #[test]
    fn empty_output_is_an_error() {
        assert!(matches!(SatResult::parse(""), Err(SolverError::Solver(_))));
    }

    #[test]
    fn unsat_core_is_read_as_names() {
        assert_eq!(
            parse_unsat_core("unsat\n(a_1 |odd name| c)\n"),
            vec!["a_1".to_string(), "odd name".to_string(), "c".to_string()]
        );
        assert!(parse_unsat_core("unsat\n").is_empty());
    }

    #[test]
    fn missing_program_is_not_found() {
        let backend = Backend::Command {
            program: "definitely-not-a-real-smt-solver-xyz".into(),
            args: vec![],
        };
        assert_eq!(
            run(&backend, "(check-sat)\n", Duration::from_secs(5)),
            Err(SolverError::NotFound(
                "definitely-not-a-real-smt-solver-xyz".into()
            ))
        );
    }
}
