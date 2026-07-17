//! Verified binary search with precondition contracts.

use tpt_verify_macros::{ensures, requires};

/// Returns the index of `key` in the sorted slice `slice`, or `None` if absent.
///
/// The slice must be sorted in non-decreasing order; this is a **precondition**.
/// In debug builds, violating it panics. In release builds the precondition is
/// unchecked (zero cost) and the result is unspecified for unsorted input.
///
/// When `key` is present, the returned index satisfies
/// `slice[index] == key` (**postcondition**). When absent, `None` is returned.
///
/// # Example
///
/// ```rust
/// use tpt_verified_algorithms::verified_binary_search;
///
/// let data = [1, 2, 3, 4, 5];
/// assert_eq!(verified_binary_search(&data, &3), Some(2));
/// assert_eq!(verified_binary_search(&data, &6), None);
/// ```
///
/// ```should_panic
/// use tpt_verified_algorithms::verified_binary_search;
///
/// // Precondition violation: slice is not sorted.
/// let bad = [3, 1, 2];
/// verified_binary_search(&bad, &2);
/// ```
#[requires(is_sorted_slice(slice))]
#[ensures(match result {
    Some(i) => slice[i] == *key,
    None => true,
})]
pub fn verified_binary_search<T: Ord>(slice: &[T], key: &T) -> Option<usize> {
    let mut lo = 0usize;
    let mut hi = slice.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        match slice[mid].cmp(key) {
            core::cmp::Ordering::Equal => return Some(mid),
            core::cmp::Ordering::Greater => hi = mid,
            core::cmp::Ordering::Less => lo = mid + 1,
        }
    }
    None
}

/// Helper predicate used by the `#[requires]` precondition of [`verified_binary_search`].
///
/// Exposed so callers and tooling can reuse the exact sortedness condition.
pub fn is_sorted_slice<T: PartialOrd>(slice: &[T]) -> bool {
    crate::predicates::is_sorted(slice)
}
