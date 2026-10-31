//! Verified sorting with debug-mode postcondition checks.

use core::cmp::Ordering;

use alloc::vec::Vec;

use crate::predicates::{is_permutation, is_sorted, is_sorted_by};

/// Slices up to this length are sorted by insertion sort.
const INSERTION_THRESHOLD: usize = 16;

/// Sorts `slice` in non-decreasing order.
///
/// The sort is a stable merge sort, which takes O(n log n) comparisons. In
/// **debug builds** (`debug_assertions` on), the function asserts that the
/// resulting slice is sorted **and** a permutation of the original input. In
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
    let original: Vec<T> = slice.to_vec();

    merge_sort_by(slice, &mut T::cmp);

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

/// Sorts `slice` with a comparison function. The sort is stable and runs in
/// O(n log n) comparisons.
///
/// Debug builds assert that the result is sorted under `compare`. Permutation
/// is not checked, because the element type need not implement `PartialEq`.
///
/// # Example
///
/// ```rust
/// use tpt_verified_algorithms::verified_sort_by;
///
/// let mut words = ["pear", "fig", "apple"];
/// verified_sort_by(&mut words, |a, b| a.len().cmp(&b.len()));
/// assert_eq!(words, ["fig", "pear", "apple"]);
/// ```
pub fn verified_sort_by<T: Clone, F: FnMut(&T, &T) -> Ordering>(slice: &mut [T], mut compare: F) {
    merge_sort_by(slice, &mut compare);

    #[cfg(debug_assertions)]
    debug_assert!(
        is_sorted_by(slice, &mut compare),
        "verified_sort_by postcondition violated: not sorted under the comparator"
    );
}

/// Sorts `slice` by a key. The sort is stable and runs in O(n log n) comparisons.
///
/// The key is computed for each comparison. It should be cheap, or cache it
/// yourself by sorting `(key, value)` pairs.
///
/// # Example
///
/// ```rust
/// use tpt_verified_algorithms::verified_sort_by_key;
///
/// let mut people = [("ann", 31), ("bob", 25), ("cy", 31)];
/// verified_sort_by_key(&mut people, |p| p.1);
/// assert_eq!(people, [("bob", 25), ("ann", 31), ("cy", 31)]);
/// ```
pub fn verified_sort_by_key<T: Clone, K: Ord, F: FnMut(&T) -> K>(slice: &mut [T], mut key: F) {
    verified_sort_by(slice, |a, b| key(a).cmp(&key(b)));
}

/// Stable top-down merge sort. Slices of up to [`INSERTION_THRESHOLD`] elements
/// use insertion sort.
fn merge_sort_by<T: Clone, F: FnMut(&T, &T) -> Ordering>(slice: &mut [T], compare: &mut F) {
    let n = slice.len();
    if n <= INSERTION_THRESHOLD {
        insertion_sort_by(slice, compare);
        return;
    }
    let mid = n / 2;
    merge_sort_by(&mut slice[..mid], compare);
    merge_sort_by(&mut slice[mid..], compare);
    // Already in order across the seam: nothing to merge.
    if compare(&slice[mid - 1], &slice[mid]) != Ordering::Greater {
        return;
    }
    merge(slice, mid, compare);
}

/// Merges the sorted runs `slice[..mid]` and `slice[mid..]` in place, using a
/// copy of the left run. Equal elements keep their order, because the left
/// element is taken first on a tie.
fn merge<T: Clone, F: FnMut(&T, &T) -> Ordering>(slice: &mut [T], mid: usize, compare: &mut F) {
    let left: Vec<T> = slice[..mid].to_vec();
    let right_len = slice.len() - mid;
    let (mut i, mut j, mut k) = (0, mid, 0);
    while i < left.len() && j < mid + right_len {
        if compare(&left[i], &slice[j]) != Ordering::Greater {
            slice[k] = left[i].clone();
            i += 1;
        } else {
            slice.swap(k, j);
            j += 1;
        }
        k += 1;
    }
    while i < left.len() {
        slice[k] = left[i].clone();
        i += 1;
        k += 1;
    }
}

/// Stable insertion sort for short slices.
fn insertion_sort_by<T, F: FnMut(&T, &T) -> Ordering>(slice: &mut [T], compare: &mut F) {
    for i in 1..slice.len() {
        let mut j = i;
        while j > 0 && compare(&slice[j - 1], &slice[j]) == Ordering::Greater {
            slice.swap(j - 1, j);
            j -= 1;
        }
    }
}

#[cfg(kani)]
mod proofs {
    //! Kani model-checking harnesses. Run with `cargo kani`. They are compiled
    //! only under `cfg(kani)`, so they have no effect on normal builds.

    use super::verified_sort;

    #[kani::proof]
    #[kani::unwind(6)]
    fn sort_is_sorted_for_all_small_inputs() {
        let mut data: [u8; 4] = kani::any();
        let original = data;
        verified_sort(&mut data);
        assert!(data.windows(2).all(|w| w[0] <= w[1]));
        // Same multiset: every original element is still present.
        for x in original {
            assert!(data.contains(&x));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_sort_matches_std_on_pseudo_random_data() {
        let mut seed = 0x2545_F491_4F6C_DD1Du64;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for len in [0usize, 1, 2, 15, 16, 17, 100, 1000] {
            let data: Vec<u64> = (0..len).map(|_| next() % 50).collect();
            let mut ours = data.clone();
            verified_sort(&mut ours);
            let mut expected = data;
            expected.sort();
            assert_eq!(ours, expected, "len={len}");
        }
    }

    #[test]
    fn sort_by_is_stable() {
        let mut data: Vec<(u8, char)> = (0..200)
            .map(|i| ((i % 7) as u8, (b'a' + (i % 26) as u8) as char))
            .collect();
        let original = data.clone();
        verified_sort_by_key(&mut data, |p| p.0);
        // Within each key, the original order is kept.
        for key in 0..7u8 {
            let want: Vec<_> = original.iter().filter(|p| p.0 == key).copied().collect();
            let got: Vec<_> = data.iter().filter(|p| p.0 == key).copied().collect();
            assert_eq!(got, want, "key={key}");
        }
    }

    #[test]
    fn sort_by_descending_comparator() {
        let mut data = [3u32, 9, 1, 7];
        verified_sort_by(&mut data, |a, b| b.cmp(a));
        assert_eq!(data, [9, 7, 3, 1]);
    }

    #[test]
    fn large_inputs_sort_in_n_log_n_time() {
        // A quadratic sort would make this test visibly slow. Count comparisons
        // to check the bound directly: merge sort does about n*log2(n).
        let n = 4096usize;
        let mut data: Vec<usize> = (0..n).rev().collect();
        let mut comparisons = 0usize;
        verified_sort_by(&mut data, |a, b| {
            comparisons += 1;
            a.cmp(b)
        });
        assert!(data.windows(2).all(|w| w[0] <= w[1]));
        let bound = 2 * n * 12; // generous: n * log2(n) with slack for insertion sort
        assert!(
            comparisons < bound,
            "comparisons={comparisons} bound={bound}"
        );
    }
}
