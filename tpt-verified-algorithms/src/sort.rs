//! Verified sorting with debug-mode postcondition checks.

use crate::predicates::{is_permutation, is_sorted};

/// Sorts `slice` in non-decreasing order.
///
/// The sort is a stable insertion sort, chosen for its simplicity and clear
/// correctness argument. In **debug builds** (`debug_assertions` on), the
/// function asserts that the resulting slice is sorted **and** a permutation of
/// the original input — a runtime proof of the two key postconditions. In
/// release builds these checks are elided.
///
/// # Example
///
/// ```rust
/// use tpt_verified_algorithms::verified_sort;
///
/// let mut data = [3, 1, 2];
/// verified_sort(&mut data);
/// assert_eq!(data, [1, 2, 3]);
/// ```
pub fn verified_sort<T: Ord + Clone>(slice: &mut [T]) {
    #[cfg(debug_assertions)]
    let original: alloc::vec::Vec<T> = slice.to_vec();

    // Insertion sort.
    for i in 1..slice.len() {
        let mut j = i;
        while j > 0 && slice[j - 1] > slice[j] {
            slice.swap(j - 1, j);
            j -= 1;
        }
    }

    #[cfg(debug_assertions)]
    {
        debug_assert!(
            is_sorted(slice),
            "verified_sort postcondition violated: not sorted"
        );
        debug_assert!(
            is_permutation(&original, slice),
            "verified_sort postcondition violated: not a permutation of the input"
        );
    }
}
