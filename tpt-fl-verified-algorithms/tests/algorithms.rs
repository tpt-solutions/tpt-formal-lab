//! Tests for `tpt-fl-verified-algorithms`.

use tpt_fl_verified_algorithms::{
    is_permutation, is_sorted, lower_bound, upper_bound, verified_binary_search, verified_sort,
};

#[test]
fn is_sorted_basics() {
    assert!(is_sorted(&[1usize, 2, 3]));
    assert!(is_sorted(&[2usize, 2, 2]));
    assert!(is_sorted(&[] as &[i32; 0]));
    assert!(is_sorted(&[1i32]));
    assert!(!is_sorted(&[3i32, 1, 2]));
}

#[test]
fn is_permutation_basics() {
    assert!(is_permutation(&[1, 2, 3], &[3, 1, 2]));
    assert!(is_permutation(&[1, 1, 2], &[2, 1, 1]));
    assert!(!is_permutation(&[1, 2, 3], &[1, 2, 4]));
    assert!(!is_permutation(&[1, 2], &[1, 2, 3]));
}

#[test]
fn sort_empty() {
    let mut data: [i32; 0] = [];
    verified_sort(&mut data);
    assert!(is_sorted(&data));
}

#[test]
fn sort_single() {
    let mut data = [42];
    verified_sort(&mut data);
    assert_eq!(data, [42]);
}

#[test]
fn sort_duplicates() {
    let mut data = [5, 2, 5, 1, 2];
    let original = data;
    verified_sort(&mut data);
    assert!(is_sorted(&data));
    assert!(is_permutation(&original, &data));
}

#[test]
fn sort_reverse() {
    let mut data = [5, 4, 3, 2, 1];
    let original = data;
    verified_sort(&mut data);
    assert_eq!(data, [1, 2, 3, 4, 5]);
    assert!(is_permutation(&original, &data));
}

#[test]
fn sort_randomish() {
    let mut data = [9, 3, 7, 1, 8, 2, 6, 4, 5, 0];
    let original = data;
    verified_sort(&mut data);
    assert!(is_sorted(&data));
    assert!(is_permutation(&original, &data));
}

#[test]
fn binary_search_hit() {
    let data = [1, 2, 3, 4, 5];
    for (expected, key) in data.iter().enumerate() {
        assert_eq!(verified_binary_search(&data, key), Some(expected));
    }
}

#[test]
fn binary_search_miss() {
    let data = [2, 4, 6, 8, 10];
    assert_eq!(verified_binary_search(&data, &1), None);
    assert_eq!(verified_binary_search(&data, &5), None);
    assert_eq!(verified_binary_search(&data, &11), None);
    assert_eq!(verified_binary_search(&data, &7), None);
}

#[test]
fn binary_search_empty() {
    let data: [i32; 0] = [];
    assert_eq!(verified_binary_search(&data, &1), None);
}

#[test]
#[cfg(debug_assertions)]
#[should_panic]
fn binary_search_panics_on_unsorted_in_debug() {
    let bad = [3, 1, 2];
    verified_binary_search(&bad, &2);
}

#[test]
fn is_permutation_detects_multiplicity_mismatch() {
    assert!(!is_permutation(&[1, 1, 2], &[1, 2, 2]));
    assert!(!is_permutation(&[1, 2, 2], &[1, 1, 2]));
    assert!(is_permutation(&[2, 2, 1], &[1, 2, 2]));
}

/// Element whose `Ord` looks only at `key`, so two values with the same key
/// are equal for sorting. Equality still compares every field, which lets the
/// tests tell the elements apart.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Keyed {
    key: u8,
    tag: char,
}

impl PartialOrd for Keyed {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Keyed {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.key.cmp(&other.key)
    }
}

#[test]
fn sort_is_stable_for_equal_keys() {
    let mut data = [
        Keyed { key: 2, tag: 'a' },
        Keyed { key: 1, tag: 'b' },
        Keyed { key: 2, tag: 'c' },
        Keyed { key: 1, tag: 'd' },
    ];
    verified_sort(&mut data);
    let tags: Vec<char> = data.iter().map(|k| k.tag).collect();
    assert_eq!(tags, ['b', 'd', 'a', 'c']);
}

#[test]
fn sort_large_all_equal_and_descending() {
    let mut equal = vec![7u32; 500];
    verified_sort(&mut equal);
    assert!(equal.iter().all(|&v| v == 7));

    let original: Vec<u32> = (0..500).rev().collect();
    let mut data = original.clone();
    verified_sort(&mut data);
    assert!(is_sorted(&data));
    assert!(is_permutation(&original, &data));
    assert_eq!(data.first(), Some(&0));
    assert_eq!(data.last(), Some(&499));
}

#[test]
fn binary_search_hit_on_all_equal_slice() {
    let data = [4u8; 10];
    let idx = verified_binary_search(&data, &4).expect("present");
    assert_eq!(data[idx], 4);
}

#[test]
fn lower_and_upper_bound_with_duplicates() {
    let data = [1, 2, 2, 2, 5];
    assert_eq!(lower_bound(&data, &2), 1);
    assert_eq!(upper_bound(&data, &2), 4);
    assert_eq!(lower_bound(&data, &0), 0);
    assert_eq!(upper_bound(&data, &9), 5);
    assert_eq!(lower_bound(&data, &3), 4);
    assert_eq!(upper_bound(&data, &3), 4);
}

#[test]
fn bounds_on_empty_slice() {
    let data: [i32; 0] = [];
    assert_eq!(lower_bound(&data, &1), 0);
    assert_eq!(upper_bound(&data, &1), 0);
}

#[test]
fn binary_search_miss_below_and_above_duplicates() {
    let data = [2, 2, 4, 4];
    assert_eq!(verified_binary_search(&data, &3), None);
    assert_eq!(verified_binary_search(&data, &1), None);
    assert_eq!(verified_binary_search(&data, &5), None);
}
