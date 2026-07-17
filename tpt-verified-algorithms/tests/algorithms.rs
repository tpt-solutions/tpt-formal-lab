//! Tests for `tpt-verified-algorithms`.

use tpt_verified_algorithms::{is_permutation, is_sorted, verified_binary_search, verified_sort};

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
#[should_panic]
fn binary_search_panics_on_unsorted_in_debug() {
    let bad = [3, 1, 2];
    verified_binary_search(&bad, &2);
}
