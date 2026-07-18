//! SMT-LIB2 sorts (types).

use std::fmt;

/// An SMT-LIB2 sort (type) for variables and expressions.
///
/// # Example
///
/// ```rust
/// use out_smt_bridge::Sort;
///
/// assert_eq!(Sort::Int.to_smtlib2(), "Int");
/// assert_eq!(Sort::BitVec(32).to_smtlib2(), "(_ BitVec 32)");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Sort {
    /// Boolean sort (`Bool`).
    Bool,
    /// Unbounded integer sort (`Int`).
    Int,
    /// Real number sort (`Real`).
    Real,
    /// Bit-vector sort of given width in bits (`(_ BitVec N)`).
    BitVec(u32),
    /// Array sort from a domain to a range (`(Array D R)`).
    Array(Box<Sort>, Box<Sort>),
}

impl Sort {
    /// Returns the SMT-LIB2 string representation of this sort.
    ///
    /// # Example
    ///
    /// ```rust
    /// use out_smt_bridge::Sort;
    ///
    /// assert_eq!(Sort::Bool.to_smtlib2(), "Bool");
    /// assert_eq!(Sort::Real.to_smtlib2(), "Real");
    /// assert_eq!(Sort::BitVec(64).to_smtlib2(), "(_ BitVec 64)");
    /// let arr = Sort::Array(Box::new(Sort::Int), Box::new(Sort::Int));
    /// assert_eq!(arr.to_smtlib2(), "(Array Int Int)");
    /// ```
    pub fn to_smtlib2(&self) -> String {
        match self {
            Sort::Bool => "Bool".into(),
            Sort::Int => "Int".into(),
            Sort::Real => "Real".into(),
            Sort::BitVec(w) => format!("(_ BitVec {w})"),
            Sort::Array(d, r) => format!("(Array {} {})", d.to_smtlib2(), r.to_smtlib2()),
        }
    }
}

impl fmt::Display for Sort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_smtlib2())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_display() {
        assert_eq!(Sort::Bool.to_smtlib2(), "Bool");
        assert_eq!(Sort::Int.to_smtlib2(), "Int");
        assert_eq!(Sort::Real.to_smtlib2(), "Real");
        assert_eq!(Sort::BitVec(8).to_smtlib2(), "(_ BitVec 8)");
        let arr = Sort::Array(Box::new(Sort::Int), Box::new(Sort::Bool));
        assert_eq!(arr.to_smtlib2(), "(Array Int Bool)");
    }
}
