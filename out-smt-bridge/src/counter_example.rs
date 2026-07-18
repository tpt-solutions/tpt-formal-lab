//! Parse solver model output into concrete witness values.

use std::collections::HashMap;

/// A concrete witness produced by an SMT solver for a `sat` result.
///
/// `CounterExample` parses the `(model ...)` output from a solver and lets you
/// look up the concrete values assigned to each declared constant.
///
/// # Example
///
/// ```rust
/// use out_smt_bridge::CounterExample;
///
/// let model = "(model\n  (define-fun x () Int 3)\n  (define-fun y () Int 7)\n)";
/// let ce = CounterExample::parse(model);
/// assert_eq!(ce.get_int("x"), Some(3));
/// assert_eq!(ce.get_int("y"), Some(7));
/// ```
#[derive(Debug, Clone, Default)]
pub struct CounterExample {
    values: HashMap<String, CeValue>,
}

/// A concrete value from a solver model.
#[derive(Debug, Clone, PartialEq)]
pub enum CeValue {
    /// A boolean value.
    Bool(bool),
    /// An integer value (may be arbitrarily large; stored as `i64` for convenience).
    Int(i64),
    /// A real value as a decimal string.
    Real(String),
    /// An opaque value that could not be parsed into a known type.
    Opaque(String),
}

impl CounterExample {
    /// Creates an empty `CounterExample`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses a solver model string into a `CounterExample`.
    ///
    /// Supports the common `(define-fun name () Sort value)` format emitted by
    /// Z3, CVC5, and most SMT-LIB2 compliant solvers.
    ///
    /// # Example
    ///
    /// ```rust
    /// use out_smt_bridge::CounterExample;
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
        // Simple line-by-line parser for `(define-fun name () Sort value)`
        for line in model.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with("(define-fun") {
                continue;
            }
            // Format: (define-fun name () Sort value)
            // Do NOT pre-strip parens — the inner value may itself contain parens (e.g. "(- 5)").
            let tokens: Vec<&str> = trimmed.split_whitespace().collect();
            // tokens: ["(define-fun", "name", "()", "Sort", "value...", ")"]
            if tokens.len() < 5 {
                continue;
            }
            let name = tokens[1];
            let sort = tokens[3];
            // Everything from index 4 onwards is the value followed by the closing ")" of define-fun.
            // Strip exactly one trailing ")" to remove the define-fun closer.
            let value_raw = tokens[4..].join(" ");
            let value_str = if value_raw.ends_with(')') {
                value_raw[..value_raw.len() - 1].trim().to_string()
            } else {
                value_raw
            };

            let value = match sort {
                "Bool" => match value_str.as_str() {
                    "true" => CeValue::Bool(true),
                    "false" => CeValue::Bool(false),
                    _ => CeValue::Opaque(value_str),
                },
                "Int" => parse_int_value(&value_str),
                "Real" => CeValue::Real(value_str),
                _ => CeValue::Opaque(value_str),
            };

            ce.values.insert(name.to_string(), value);
        }
        ce
    }

    /// Returns all parsed variable names.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.values.keys().map(|s| s.as_str())
    }

    /// Returns the raw [`CeValue`] for a variable, if present.
    pub fn get(&self, name: &str) -> Option<&CeValue> {
        self.values.get(name)
    }

    /// Returns the integer value of a variable, if present and parseable as `i64`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use out_smt_bridge::CounterExample;
    ///
    /// let ce = CounterExample::parse("(model\n  (define-fun x () Int 5)\n)");
    /// assert_eq!(ce.get_int("x"), Some(5));
    /// ```
    pub fn get_int(&self, name: &str) -> Option<i64> {
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

    /// Returns the real value as a string, if present.
    pub fn get_real(&self, name: &str) -> Option<&str> {
        match self.values.get(name)? {
            CeValue::Real(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

fn parse_int_value(s: &str) -> CeValue {
    // Handle "(- N)" negation form from solvers
    let s = s.trim();
    if let Some(inner) = s.strip_prefix("(- ").and_then(|t| t.strip_suffix(')')) {
        if let Ok(n) = inner.trim().parse::<i64>() {
            return CeValue::Int(-n);
        }
    }
    if let Ok(n) = s.parse::<i64>() {
        CeValue::Int(n)
    } else {
        CeValue::Opaque(s.to_string())
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
    }

    #[test]
    fn names_iterator() {
        let model = "(model\n  (define-fun a () Int 1)\n  (define-fun b () Int 2)\n)";
        let ce = CounterExample::parse(model);
        let mut names: Vec<&str> = ce.names().collect();
        names.sort();
        assert_eq!(names, vec!["a", "b"]);
    }
}
