//! Predicate building blocks: sortedness and permutation checks.

/// Returns `true` if the slice is sorted in non-decreasing order.
///
/// An empty slice or a single-element slice is always sorted.
///
/// # Example
///
/// ```rust
/// use tpt_verified_algorithms::is_sorted;
///
/// assert!(is_sorted(&[1, 2, 2, 3]));
/// assert!(!is_sorted(&[3, 1, 2]));
/// ```
pub fn is_sorted<T: PartialOrd>(slice: &[T]) -> bool {
    slice.windows(2).all(|w| w[0] <= w[1])
}

/// Returns `true` if `b` is a permutation of `a` — that is, `a` and `b` contain
/// exactly the same multiset of elements.
///
/// This is a quadratic-time check suitable for verification and small inputs.
///
/// # Example
///
/// ```rust
/// use tpt_verified_algorithms::is_permutation;
///
/// assert!(is_permutation(&[1, 2, 3], &[3, 1, 2]));
/// assert!(!is_permutation(&[1, 2, 3], &[1, 2, 4]));
/// ```
pub fn is_permutation<T: PartialEq>(a: &[T], b: &[T]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut used = alloc::vec![false; b.len()];
    for x in a {
        let mut found = false;
        for (j, y) in b.iter().enumerate() {
            if !used[j] && x == y {
                used[j] = true;
                found = true;
                break;
            }
        }
        if !found {
            return false;
        }
    }
    true
}
