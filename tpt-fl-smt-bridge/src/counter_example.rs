//! Parse solver model output into concrete witness values.

use std::collections::BTreeMap;

use crate::sexpr::{self, SExpr};

/// A concrete witness produced by an SMT solver for a `sat` result.
///
/// `CounterExample` reads the model that a solver prints after `(get-model)`
/// and lets you look up the value of each constant. It accepts both the
/// `(model ...)` wrapper and the bare list of `define-fun`s that Z3 prints.
/// Multi-line output, quoted symbols, and negative and rational numbers are
/// all supported.
///
/// # Example
///
/// ```rust
/// use tpt_fl_smt_bridge::CounterExample;
///
/// let model = "(model\n  (define-fun x () Int 3)\n  (define-fun y () Int 7)\n)";
/// let ce = CounterExample::parse(model);
/// assert_eq!(ce.get_int("x"), Some(3));
/// assert_eq!(ce.get_int("y"), Some(7));
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CounterExample {
    // A BTreeMap keeps `names()` in a fixed order, so output is deterministic.
    values: BTreeMap<String, CeValue>,
}

/// A concrete value from a solver model.
#[derive(Debug, Clone, PartialEq)]
pub enum CeValue {
    /// A boolean value.
    Bool(bool),
    /// An integer value. Values beyond `i128` are kept as [`CeValue::Opaque`].
    Int(i128),
    /// A real value, as normalised decimal text such as `"-1.0/2.0"`.
    Real(String),
    /// A bit-vector value and its width in bits.
    BitVec {
        /// The value, with no bits above `width`.
        value: u128,
        /// The width in bits.
        width: u32,
    },
    /// A string value, with SMT-LIB2 escapes already decoded.
    Str(String),
    /// A value that could not be read as one of the types above, such as an
    /// array or a function. It holds the value's SMT-LIB2 text.
    Opaque(String),
}

impl CounterExample {
    /// Creates an empty `CounterExample`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses a solver model string into a `CounterExample`.
    ///
    /// Each `(define-fun name () Sort value)` in the input becomes an entry.
    /// Text outside the `define-fun` forms, such as `sat` or `(model`, is
    /// ignored. A `define-fun` with parameters (a function interpretation) is
    /// stored as [`CeValue::Opaque`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_fl_smt_bridge::CounterExample;
    ///
    /// let model = r#"(model
    ///   (define-fun n () Int 42)
    ///   (define-fun flag () Bool true)
    /// )"#;
    /// let ce = CounterExample::parse(model);
    /// assert_eq!(ce.get_int("n"), Some(42));
    /// assert_eq!(ce.get_bool("flag"), Some(true));
    /// ```
    pub fn parse(model: &str) -> Self {
        let mut ce = Self::new();
        for node in sexpr::parse_all(model) {
            collect_define_funs(&node, &mut ce);
        }
        ce
    }

    /// Returns all parsed variable names, in sorted order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.values.keys().map(String::as_str)
    }

    /// Returns the number of parsed values.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns `true` if no values were parsed.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Returns the raw [`CeValue`] for a variable, if present.
    pub fn get(&self, name: &str) -> Option<&CeValue> {
        self.values.get(name)
    }

    /// Returns the integer value of a variable, if it is an integer that fits in `i64`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_fl_smt_bridge::CounterExample;
    ///
    /// let ce = CounterExample::parse("(model\n  (define-fun x () Int 5)\n)");
    /// assert_eq!(ce.get_int("x"), Some(5));
    /// ```
    pub fn get_int(&self, name: &str) -> Option<i64> {
        i64::try_from(self.get_int128(name)?).ok()
    }

    /// Returns the integer value of a variable as an `i128`.
    pub fn get_int128(&self, name: &str) -> Option<i128> {
        match self.values.get(name)? {
            CeValue::Int(n) => Some(*n),
            _ => None,
        }
    }

    /// Returns the boolean value of a variable, if present.
    pub fn get_bool(&self, name: &str) -> Option<bool> {
        match self.values.get(name)? {
            CeValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the real value as normalised text, if present.
    pub fn get_real(&self, name: &str) -> Option<&str> {
        match self.values.get(name)? {
            CeValue::Real(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Returns `(value, width)` for a bit-vector variable, if present.
    pub fn get_bitvec(&self, name: &str) -> Option<(u128, u32)> {
        match self.values.get(name)? {
            CeValue::BitVec { value, width } => Some((*value, *width)),
            _ => None,
        }
    }

    /// Returns the string value of a variable, if present.
    pub fn get_str(&self, name: &str) -> Option<&str> {
        match self.values.get(name)? {
            CeValue::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

/// Finds every `define-fun` in `node`, descending through `(model ...)` and
/// other wrappers.
fn collect_define_funs(node: &SExpr, ce: &mut CounterExample) {
    let SExpr::List(items) = node else {
        return;
    };
    if items.len() == 5 && items[0].atom() == Some("define-fun") {
        if let Some(name) = items[1].atom() {
            let value = if matches!(&items[2], SExpr::List(params) if !params.is_empty()) {
                // A function interpretation: keep its text, which is not a single value.
                CeValue::Opaque(node.to_text())
            } else {
                value_of(&items[3], &items[4])
            };
            ce.values.insert(name.to_string(), value);
        }
        return;
    }
    for child in items {
        collect_define_funs(child, ce);
    }
}

/// Reads a value from its sort and its body.
fn value_of(sort: &SExpr, body: &SExpr) -> CeValue {
    match sort.atom() {
        Some("Bool") => match body.atom() {
            Some("true") => CeValue::Bool(true),
            Some("false") => CeValue::Bool(false),
            _ => opaque(body),
        },
        Some("Int") => int_value(body).map_or_else(|| opaque(body), CeValue::Int),
        Some("Real") => real_text(body).map_or_else(|| opaque(body), CeValue::Real),
        Some("String") => match body {
            SExpr::Str(s) => CeValue::Str(s.clone()),
            _ => opaque(body),
        },
        _ => bitvec_sort_width(sort)
            .and_then(|width| bitvec_value(body).map(|value| CeValue::BitVec { value, width }))
            .unwrap_or_else(|| opaque(body)),
    }
}

fn opaque(node: &SExpr) -> CeValue {
    CeValue::Opaque(node.to_text())
}

/// Reads `N` or `(- N)` as an integer.
fn int_value(node: &SExpr) -> Option<i128> {
    match node {
        SExpr::Atom(n) => n.parse::<i128>().ok(),
        SExpr::List(items) => match items.as_slice() {
            [SExpr::Atom(op), SExpr::Atom(n)] if op == "-" => {
                n.parse::<i128>().ok().and_then(i128::checked_neg)
            }
            _ => None,
        },
        SExpr::Str(_) => None,
    }
}

/// Reads a real as normalised text: `3.0`, `-3.0`, `1.0/2.0`, or `-1.0/2.0`.
fn real_text(node: &SExpr) -> Option<String> {
    match node {
        SExpr::Atom(s) => {
            let numeral = s.strip_prefix('-').unwrap_or(s);
            let is_decimal = !numeral.is_empty()
                && numeral.chars().all(|c| c.is_ascii_digit() || c == '.')
                && numeral.chars().any(|c| c.is_ascii_digit());
            is_decimal.then(|| s.clone())
        }
        SExpr::List(items) => match items.as_slice() {
            [SExpr::Atom(op), inner] if op == "-" => Some(format!("-{}", real_text(inner)?)),
            [SExpr::Atom(op), num, den] if op == "/" => {
                Some(format!("{}/{}", real_text(num)?, real_text(den)?))
            }
            _ => None,
        },
        SExpr::Str(_) => None,
    }
}

/// Reads the width from a `(_ BitVec N)` sort.
fn bitvec_sort_width(sort: &SExpr) -> Option<u32> {
    match sort {
        SExpr::List(items) => match items.as_slice() {
            [SExpr::Atom(us), SExpr::Atom(bv), SExpr::Atom(w)] if us == "_" && bv == "BitVec" => {
                w.parse().ok()
            }
            _ => None,
        },
        SExpr::Atom(_) | SExpr::Str(_) => None,
    }
}

/// Reads a bit-vector body: `#x` hex, `#b` binary, or `(_ bvN w)`.
fn bitvec_value(node: &SExpr) -> Option<u128> {
    match node {
        SExpr::Atom(s) => {
            if let Some(hex) = s.strip_prefix("#x") {
                u128::from_str_radix(hex, 16).ok()
            } else if let Some(bin) = s.strip_prefix("#b") {
                u128::from_str_radix(bin, 2).ok()
            } else {
                None
            }
        }
        SExpr::List(items) => match items.as_slice() {
            [SExpr::Atom(us), SExpr::Atom(bv), SExpr::Atom(_)] if us == "_" => {
                bv.strip_prefix("bv")?.parse().ok()
            }
            _ => None,
        },
        SExpr::Str(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_ints() {
        let model = "(model\n  (define-fun x () Int 3)\n  (define-fun y () Int 7)\n)";
        let ce = CounterExample::parse(model);
        assert_eq!(ce.get_int("x"), Some(3));
        assert_eq!(ce.get_int("y"), Some(7));
    }

    #[test]
    fn parse_negative_int() {
        let model = "(model\n  (define-fun x () Int (- 5))\n)";
        let ce = CounterExample::parse(model);
        assert_eq!(ce.get_int("x"), Some(-5));
    }

    #[test]
    fn parse_bool() {
        let model = "(model\n  (define-fun flag () Bool true)\n)";
        let ce = CounterExample::parse(model);
        assert_eq!(ce.get_bool("flag"), Some(true));
    }

    #[test]
    fn missing_variable_returns_none() {
        let ce = CounterExample::new();
        assert_eq!(ce.get_int("nonexistent"), None);
        assert!(ce.is_empty());
    }

    #[test]
    fn names_iterator_is_sorted() {
        let model = "(model\n  (define-fun b () Int 2)\n  (define-fun a () Int 1)\n)";
        let ce = CounterExample::parse(model);
        let names: Vec<&str> = ce.names().collect();
        assert_eq!(names, vec!["a", "b"]);
        assert_eq!(ce.len(), 2);
    }

    #[test]
    fn real_z3_transcript_multi_line_model() {
        // Recorded from `z3 -in` 5.3.1 on a problem with ints, reals, bit-vectors,
        // strings and a quoted symbol. Z3 prints a bare list, not `(model ...)`.
        let model = r#"sat
(
  (define-fun x () Int
    (- 4))
  (define-fun s () String
    "a""b")
  (define-fun b () (_ BitVec 8)
    #xf0)
  (define-fun r () Real
    (- (/ 1.0 2.0)))
  (define-fun |my var| () Int
    (- 3))
)
"#;
        let ce = CounterExample::parse(model);
        assert_eq!(ce.get_int("x"), Some(-4));
        assert_eq!(ce.get_int("my var"), Some(-3));
        assert_eq!(ce.get_str("s"), Some("a\"b"));
        assert_eq!(ce.get_bitvec("b"), Some((0xf0, 8)));
        assert_eq!(ce.get_real("r"), Some("-1.0/2.0"));
    }

    #[test]
    fn real_z3_transcript_array_and_positive_values() {
        let model = r#"sat
(
  (define-fun x () Int
    7)
  (define-fun f () (Array Int Int)
    ((as const (Array Int Int)) 0))
)"#;
        let ce = CounterExample::parse(model);
        assert_eq!(ce.get_int("x"), Some(7));
        assert_eq!(
            ce.get("f"),
            Some(&CeValue::Opaque("((as const (Array Int Int)) 0)".into()))
        );
    }

    #[test]
    fn bignum_ints_are_kept_as_i128() {
        let model = "(model (define-fun big () Int 170141183460469231731687303715884105727))";
        let ce = CounterExample::parse(model);
        assert_eq!(ce.get_int128("big"), Some(i128::MAX));
        assert_eq!(ce.get_int("big"), None, "does not fit i64");
    }

    #[test]
    fn integers_beyond_i128_stay_opaque() {
        let model = "(model (define-fun huge () Int 999999999999999999999999999999999999999999))";
        let ce = CounterExample::parse(model);
        assert!(matches!(ce.get("huge"), Some(CeValue::Opaque(_))));
    }

    #[test]
    fn bitvec_literal_forms() {
        let model = r#"(model
          (define-fun a () (_ BitVec 4) #b1010)
          (define-fun b () (_ BitVec 8) (_ bv200 8))
        )"#;
        let ce = CounterExample::parse(model);
        assert_eq!(ce.get_bitvec("a"), Some((0b1010, 4)));
        assert_eq!(ce.get_bitvec("b"), Some((200, 8)));
    }

    #[test]
    fn functions_are_opaque_and_do_not_hide_other_values() {
        let model = "(model\n  (define-fun f ((x Int)) Int (+ x 1))\n  (define-fun y () Int 2)\n)";
        let ce = CounterExample::parse(model);
        assert!(matches!(ce.get("f"), Some(CeValue::Opaque(_))));
        assert_eq!(ce.get_int("y"), Some(2));
    }

    #[test]
    fn non_model_output_gives_empty_counterexample() {
        assert!(CounterExample::parse("unsat\n").is_empty());
        assert!(CounterExample::parse("(error \"line 5: model is not available\")").is_empty());
    }
}
