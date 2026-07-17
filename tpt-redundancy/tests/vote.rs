//! Tests for `tpt-redundancy`.

use tpt_redundancy::{Replicated, VoteResult};

#[test]
fn unanimous_vote() {
    let r = Replicated::new([5u8, 5u8, 5u8]);
    assert!(matches!(r.majority_vote(), VoteResult::Unanimous(5)));
    assert_eq!(r.majority_vote().value(), Some(&5u8));
}

#[test]
fn majority_vote_detects_plurality() {
    let r = Replicated::new([5u8, 5u8, 9u8]);
    assert!(matches!(r.majority_vote(), VoteResult::Majority(5)));
    assert!(r.majority_vote().is_majority());
}

#[test]
fn no_majority_on_tie() {
    let r = Replicated::new([1u8, 2u8]);
    assert!(matches!(r.majority_vote(), VoteResult::NoMajority));
    assert!(!r.majority_vote().is_majority());
    assert_eq!(r.majority_vote().value(), None);
}

#[test]
fn even_n_exact_tie_is_no_majority() {
    // Two groups of two — neither reaches N/2 + something.
    let r = Replicated::new([1u32, 1u32, 2u32, 2u32]);
    assert!(matches!(r.majority_vote(), VoteResult::NoMajority));
}

#[test]
fn even_n_with_majority() {
    let r = Replicated::new([1u32, 1u32, 1u32, 2u32]);
    assert!(matches!(r.majority_vote(), VoteResult::Majority(1)));
}

#[test]
fn empty_replication_is_no_majority() {
    let r: Replicated<u8, 0> = Replicated::new([]);
    assert!(matches!(r.majority_vote(), VoteResult::NoMajority));
    assert!(r.is_empty());
}

#[test]
fn median_vote_odd_n() {
    let r = Replicated::new([3u32, 1u32, 2u32, 4u32, 9u32]);
    // sorted: [1, 2, 3, 4, 9]; middle index 2 → 3
    assert_eq!(r.median_vote(), 3u32);
}

#[test]
fn median_rejects_outlier() {
    let r = Replicated::new([10u32, 10u32, 10u32, 10u32, 999u32]);
    assert_eq!(r.median_vote(), 10u32);
}

#[test]
fn median_vote_known_array() {
    let r = Replicated::new([5u32, 1u32, 3u32]);
    assert_eq!(r.median_vote(), 3u32);
}

#[test]
fn order_statistic_matches() {
    let r = Replicated::new([3u32, 1u32, 2u32]);
    assert_eq!(r.order_statistic(0), 1u32);
    assert_eq!(r.order_statistic(1), 2u32);
    assert_eq!(r.order_statistic(2), 3u32);
}

#[test]
fn bitwise_vote_majority_bits() {
    // Two channels set bits 1 and 3; one clears bit 3.
    let r = Replicated::new([0b1010u8, 0b1010u8, 0b0010u8]);
    assert_eq!(r.bitwise_vote(), 0b1010u8);
}

#[test]
fn bitwise_vote_unanimous_passthrough() {
    let r = Replicated::new([0b1100u8, 0b1100u8, 0b1100u8]);
    assert_eq!(r.bitwise_vote(), 0b1100u8);
}

#[test]
fn bitwise_vote_hand_computed_pattern() {
    // Bits layout:        ch0 = 0b1011_0010  (178)
    //                     ch1 = 0b1001_0011  (147)
    //                     ch2 = 0b1010_0010  (162)
    // Per-bit majority:   bit7=1 bit6=0 bit5=1 bit4=1 bit3=0 bit2=0 bit1=1 bit0=0
    //                   = 0b1011_0010  (178)
    let r = Replicated::new([0b1011_0010u8, 0b1001_0011u8, 0b1010_0010u8]);
    assert_eq!(r.bitwise_vote(), 0b1011_0010u8);
}

#[test]
fn bitwise_vote_even_n_tie_favors_zeros() {
    let r = Replicated::new([0b1111_1111u8, 0b0000_0000u8]);
    assert_eq!(r.bitwise_vote(), 0b0000_0000u8);
}

#[test]
fn bitwise_vote_wider_type() {
    let r = Replicated::new([0xFFFFu16, 0xFFFFu16, 0x0u16]);
    assert_eq!(r.bitwise_vote(), 0xFFFFu16);
}
