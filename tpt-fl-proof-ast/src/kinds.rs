//! Kind markers — zero-size types that tag AST nodes at the type level.

use core::fmt;

/// Sealed marker trait for AST node kinds.
///
/// Implemented by [`PropKind`] and [`TermKind`].
/// Cannot be implemented outside this crate (sealed).
pub trait Kind: private::Sealed + fmt::Debug + Clone + PartialEq {}

/// Kind marker for logical **propositions** (things that are true or false).
///
/// A `Formula` is an `Expr<PropKind>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PropKind;

/// Kind marker for mathematical **terms** (things that have a value).
///
/// A `Term` is an `Expr<TermKind>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TermKind;

impl Kind for PropKind {}
impl Kind for TermKind {}

mod private {
    pub trait Sealed {}
    impl Sealed for super::PropKind {}
    impl Sealed for super::TermKind {}
}
