//! End-to-end tests that run a real solver. They are skipped when `z3` is not on
//! `PATH`, so the suite still passes on machines without it.

use std::process::Command;
use std::time::Duration;

use out_smt_bridge::{Backend, Expr, SatResult, SmtSolver, SolverError, Sort};

fn z3_available() -> bool {
    Command::new("z3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

macro_rules! require_z3 {
    () => {
        if !z3_available() {
            eprintln!("skipping: z3 not found on PATH");
            return;
        }
    };
}

const TIMEOUT: Duration = Duration::from_secs(30);

fn int(name: &str) -> Expr {
    Expr::var(name, Sort::Int)
}

#[test]
fn sat_problem_returns_a_checked_model() {
    require_z3!();
    let mut s = SmtSolver::new();
    s.set_logic("QF_LIA");
    s.declare_const("x", Sort::Int);
    s.declare_const("y", Sort::Int);
    s.assert(Expr::gt(int("x"), Expr::int(0)));
    s.assert(Expr::gt(int("y"), Expr::int(0)));
    s.assert(Expr::eq(Expr::add(int("x"), int("y")), Expr::int(10)));
    s.assert(Expr::gt(int("x"), int("y")));

    match s.check_sat(&Backend::Z3, TIMEOUT).expect("z3 runs") {
        SatResult::Sat(model) => {
            let x = model.get_int("x").expect("x in model");
            let y = model.get_int("y").expect("y in model");
            assert!(
                x > 0 && y > 0 && x + y == 10 && x > y,
                "model x={x} y={y} violates constraints"
            );
        }
        other => panic!("expected sat, got {other:?}"),
    }
}

#[test]
fn unsat_problem_is_reported() {
    require_z3!();
    let mut s = SmtSolver::new();
    s.declare_const("x", Sort::Int);
    s.assert(Expr::gt(int("x"), Expr::int(1)));
    s.assert(Expr::lt(int("x"), Expr::int(0)));
    assert_eq!(
        s.check_sat(&Backend::Z3, TIMEOUT).expect("z3 runs"),
        SatResult::Unsat
    );
}

#[test]
fn quoted_names_and_real_negative_values_round_trip() {
    require_z3!();
    let mut s = SmtSolver::new();
    s.declare_const("my var", Sort::Int);
    s.declare_const("r", Sort::Real);
    s.assert(Expr::eq(Expr::var("my var", Sort::Int), Expr::int(-3)));
    s.assert(Expr::eq(
        Expr::var("r", Sort::Real),
        Expr::rdiv(Expr::real(-1, 1), Expr::real(2, 1)),
    ));
    match s.check_sat(&Backend::Z3, TIMEOUT).expect("z3 runs") {
        SatResult::Sat(model) => {
            assert_eq!(model.get_int("my var"), Some(-3));
            assert_eq!(model.get_real("r"), Some("-1.0/2.0"));
        }
        other => panic!("expected sat, got {other:?}"),
    }
}

#[test]
fn bitvector_and_string_models_are_read() {
    require_z3!();
    let mut s = SmtSolver::new();
    s.declare_const("b", Sort::BitVec(8));
    s.declare_const("t", Sort::String);
    s.assert(Expr::eq(
        Expr::bvadd(Expr::var("b", Sort::BitVec(8)), Expr::bitvec(1, 8)),
        Expr::bitvec(0, 8),
    ));
    s.assert(Expr::eq(
        Expr::str_len(Expr::var("t", Sort::String)),
        Expr::int(3),
    ));
    match s.check_sat(&Backend::Z3, TIMEOUT).expect("z3 runs") {
        SatResult::Sat(model) => {
            assert_eq!(model.get_bitvec("b"), Some((255, 8)));
            assert_eq!(model.get_str("t").map(str::len), Some(3));
        }
        other => panic!("expected sat, got {other:?}"),
    }
}

#[test]
fn array_model_is_kept_as_text() {
    require_z3!();
    let arr = Sort::Array(Box::new(Sort::Int), Box::new(Sort::Int));
    let mut s = SmtSolver::new();
    s.declare_const("a", arr.clone());
    s.assert(Expr::eq(
        Expr::select(Expr::var("a", arr), Expr::int(1)),
        Expr::int(7),
    ));
    match s.check_sat(&Backend::Z3, TIMEOUT).expect("z3 runs") {
        SatResult::Sat(model) => {
            assert!(matches!(
                model.get("a"),
                Some(out_smt_bridge::CeValue::Opaque(_))
            ));
        }
        other => panic!("expected sat, got {other:?}"),
    }
}

#[test]
fn unsat_core_names_the_conflicting_assertions() {
    require_z3!();
    let mut s = SmtSolver::new();
    s.declare_const("x", Sort::Int);
    s.assert_named("positive", Expr::gt(int("x"), Expr::int(0)));
    s.assert_named("negative", Expr::lt(int("x"), Expr::int(0)));
    s.assert_named("unrelated", Expr::eq(int("x"), int("x")));

    let output = run_raw(&s.emit_check_core());
    let core = out_smt_bridge::parse_unsat_core(&output);
    assert!(core.contains(&"positive".to_string()), "core={core:?}");
    assert!(core.contains(&"negative".to_string()), "core={core:?}");
}

#[test]
fn push_and_pop_scope_assertions() {
    require_z3!();
    let mut s = SmtSolver::new();
    s.declare_const("x", Sort::Int);
    s.push();
    s.assert(Expr::lt(int("x"), Expr::int(0)));
    s.assert(Expr::gt(int("x"), Expr::int(0)));
    s.pop();
    // Only the popped scope had the contradiction, so the problem is satisfiable.
    assert!(matches!(
        s.check_sat(&Backend::Z3, TIMEOUT).expect("z3 runs"),
        SatResult::Sat(_)
    ));
}

#[test]
fn unknown_logic_is_reported_as_unknown() {
    require_z3!();
    let mut s = SmtSolver::new();
    s.set_logic("NOT_A_LOGIC");
    s.declare_const("x", Sort::Int);
    assert!(matches!(
        s.check_sat(&Backend::Z3, TIMEOUT).expect("z3 runs"),
        SatResult::Unknown(_)
    ));
}

#[test]
fn slow_solver_is_killed_at_the_timeout() {
    require_z3!();
    // Factoring a large semiprime with nonlinear integer arithmetic is far too
    // slow for a 1 ms budget, which is also shorter than process start-up.
    let mut s = SmtSolver::new();
    s.declare_const("a", Sort::Int);
    s.declare_const("b", Sort::Int);
    s.assert(Expr::eq(
        Expr::mul(int("a"), int("b")),
        Expr::int128(1_000_000_007_i128 * 1_000_000_009_i128),
    ));
    s.assert(Expr::gt(int("a"), Expr::int(1)));
    s.assert(Expr::gt(int("b"), Expr::int(1)));
    let started = std::time::Instant::now();
    let result = s.check_sat(&Backend::Z3, Duration::from_millis(1));
    assert!(
        matches!(result, Err(SolverError::Timeout(_))),
        "result={result:?}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "solver was not killed promptly"
    );
}

#[test]
fn missing_solver_is_reported() {
    let s = SmtSolver::new();
    let backend = Backend::Command {
        program: "no-such-smt-solver-binary".into(),
        args: vec![],
    };
    assert!(matches!(
        s.check_sat(&backend, TIMEOUT),
        Err(SolverError::NotFound(_))
    ));
}

/// Runs z3 on `problem` and returns its raw stdout, for checks that need text
/// the typed API does not expose.
fn run_raw(problem: &str) -> String {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = Command::new("z3")
        .arg("-in")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("z3 starts");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(problem.as_bytes())
        .expect("write problem");
    let out = child.wait_with_output().expect("z3 finishes");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn get_value_returns_requested_terms() {
    require_z3!();
    let mut s = SmtSolver::new();
    s.declare_const("x", Sort::Int);
    s.assert(Expr::eq(int("x"), Expr::int(-4)));
    let x = int("x");
    let (verdict, values) = s
        .check_values(
            &[x.clone(), Expr::add(x, Expr::int(1))],
            &Backend::Z3,
            TIMEOUT,
        )
        .expect("z3 runs");
    assert!(matches!(verdict, SatResult::Sat(_)), "verdict={verdict:?}");
    assert_eq!(values.len(), 2, "values={values:?}");
    assert_eq!(values[0].0, "x");
    assert_eq!(values[0].1, "(- 4)");
    assert_eq!(values[1].1, "(- 3)");
}
