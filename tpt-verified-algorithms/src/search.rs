//! Verified binary search with precondition contracts.

use out_verify_macros::{ensures, requires};

/// Returns the index of an element equal to `key` in the sorted slice `slice`,
/// or `None` if `key` is absent.
///
/// The slice must be sorted in non-decreasing order; this is a **precondition**.
/// In debug builds, violating it panics. In release builds the precondition is
/// unchecked (zero cost) and the result is unspecified for unsorted input.
///
/// **Postcondition**: when `key` is present, the returned index satisfies
/// `slice[index] == key`. When `key` is absent, `None` is returned, and no
/// element of `slice` equals `key`.
///
/// **Duplicate keys**: when `key` occurs more than once, the returned index is
/// that of *some* matching element, not necessarily the first or the last. Use
/// [`lower_bound`] or [`upper_bound`] to locate the boundaries of a run of
/// equal elements.
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
/// Violating the precondition is covered by the integration tests, which are
/// gated on `debug_assertions`. Doctests do not reliably match the profile
/// the library was built with, so they are not used for it.
#[requires(is_sorted_slice(slice))]
#[ensures(match result {
    Some(i) => slice[i] == *key,
    None => !slice.contains(key),
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

/// Returns the index of the first element that is **not less than** `key`.
///
/// This is the insertion point that keeps `slice` sorted. It is `slice.len()`
/// when every element is less than `key`.
///
/// # Precondition
///
/// `slice` is sorted in non-decreasing order. In debug builds, violating this
/// panics.
///
/// # Example
///
/// ```rust
/// use tpt_verified_algorithms::lower_bound;
///
/// let data = [1, 2, 2, 2, 5];
/// assert_eq!(lower_bound(&data, &2), 1);
/// assert_eq!(lower_bound(&data, &3), 4);
/// assert_eq!(lower_bound(&data, &9), 5);
/// ```
#[requires(is_sorted_slice(slice))]
#[ensures(result == 0 || slice[result - 1] < *key)]
#[ensures(result == slice.len() || slice[result] >= *key)]
pub fn lower_bound<T: Ord>(slice: &[T], key: &T) -> usize {
    partition(slice, |x| x < key)
}

/// Returns the index of the first element that is **greater than** `key`.
///
/// It is `slice.len()` when no element is greater than `key`. The elements equal
/// to `key` occupy `lower_bound(slice, key)..upper_bound(slice, key)`.
///
/// # Precondition
///
/// `slice` is sorted in non-decreasing order. In debug builds, violating this
/// panics.
///
/// # Example
///
/// ```rust
/// use tpt_verified_algorithms::upper_bound;
///
/// let data = [1, 2, 2, 2, 5];
/// assert_eq!(upper_bound(&data, &2), 4);
/// assert_eq!(upper_bound(&data, &0), 0);
/// ```
#[requires(is_sorted_slice(slice))]
#[ensures(result == 0 || slice[result - 1] <= *key)]
#[ensures(result == slice.len() || slice[result] > *key)]
pub fn upper_bound<T: Ord>(slice: &[T], key: &T) -> usize {
    partition(slice, |x| x <= key)
}

/// Returns the length of the longest prefix of `slice` for which `pred` holds.
/// `pred` must be monotone: once it is false, it stays false.
fn partition<T>(slice: &[T], pred: impl Fn(&T) -> bool) -> usize {
    let mut lo = 0usize;
    let mut hi = slice.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if pred(&slice[mid]) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

/// Helper predicate used by the `#[requires]` preconditions above.
fn is_sorted_slice<T: PartialOrd>(slice: &[T]) -> bool {
    crate::predicates::is_sorted(slice)
}
