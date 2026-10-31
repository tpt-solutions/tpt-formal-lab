//! Strategies for generating test values.

use crate::rng::Xorshift64;

/// A deterministic seed for a property-test run.
///
/// Two runs with the same [`Seed`] produce the same generated sequence and the
/// same shrinking path. Every value is a valid seed, including `0`.
///
/// # Example
///
/// ```rust
/// use tpt_fl_det_proptest::{check, IntRange, Seed};
///
/// let r1 = check(50, Seed(1), IntRange::<i32>::new(0, 10), |x| *x >= 0);
/// let r2 = check(50, Seed(1), IntRange::<i32>::new(0, 10), |x| *x >= 0);
/// assert_eq!(r1.is_ok(), r2.is_ok());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seed(pub u64);

impl Default for Seed {
    fn default() -> Self {
        Seed(0x1234_5678_9ABC_DEF1)
    }
}

/// A value-generation strategy.
///
/// A strategy knows how to draw a value of type `T` from a [`Xorshift64`] RNG.
/// Implementations must be deterministic: the same RNG state yields the same
/// value.
///
/// Strategies that support shrinking (such as [`IntRange`]) also expose an
/// ordered index space. The runner uses it to bisect toward a smaller
/// counterexample.
pub trait Strategy {
    /// The type of value produced by this strategy.
    type Value;

    /// Draws the next value from `rng`.
    fn generate(&self, rng: &mut Xorshift64) -> Self::Value;

    /// Returns the inclusive `(first, last)` bounds of the strategy's index
    /// space, if it supports bisection shrinking. `None` otherwise.
    fn shrink_bounds(&self) -> Option<(u128, u128)> {
        None
    }

    /// Returns the index of `value` within the index space, if it has one.
    /// Only meaningful when [`shrink_bounds`](Strategy::shrink_bounds) is `Some`.
    fn index_of(&self, _value: &Self::Value) -> Option<u128> {
        None
    }

    /// Materialises the value at index `idx` within the strategy's index space.
    /// Only meaningful when [`shrink_bounds`](Strategy::shrink_bounds) is `Some`.
    fn at_index(&self, _idx: u128) -> Option<Self::Value> {
        None
    }
}

/// Generates integers uniformly over a range.
///
/// Supports every primitive integer type: `i8`–`i128`, `u8`–`u128`, `isize`
/// and `usize`. The `usize` and `isize` ranges depend on the target's pointer
/// width, so they are reproducible only across platforms of the same width.
///
/// Use [`IntRange::new`] for a half-open range `[low, high)` and
/// [`IntRange::inclusive`] for a closed range `[low, high]`.
///
/// # Example
///
/// ```rust
/// use tpt_fl_det_proptest::{IntRange, Seed, Strategy, Xorshift64};
///
/// let strat = IntRange::<i32>::new(0, 5);
/// let mut rng = Xorshift64::new(Seed::default().0);
/// let v = strat.generate(&mut rng);
/// assert!((0..5).contains(&v));
///
/// // Closed range that includes the maximum value.
/// let full = IntRange::<u8>::inclusive(0, u8::MAX);
/// assert_eq!(full.at_index(255), Some(u8::MAX));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct IntRange<T> {
    /// Smallest index, that is `low` under the order-preserving mapping.
    first: u128,
    /// Largest index, inclusive.
    last: u128,
    _marker: core::marker::PhantomData<T>,
}

impl<T: IntConvert + PartialOrd> IntRange<T> {
    /// Creates a strategy generating values in the half-open range `[low, high)`.
    ///
    /// # Panics
    ///
    /// Panics if `low >= high`.
    pub fn new(low: T, high: T) -> Self {
        assert!(low < high, "IntRange requires low < high");
        Self::from_indices(low.to_u128(), high.to_u128() - 1)
    }

    /// Creates a strategy generating values in the closed range `[low, high]`.
    ///
    /// # Panics
    ///
    /// Panics if `low > high`.
    pub fn inclusive(low: T, high: T) -> Self {
        assert!(low <= high, "IntRange requires low <= high");
        Self::from_indices(low.to_u128(), high.to_u128())
    }

    fn from_indices(first: u128, last: u128) -> Self {
        Self {
            first,
            last,
            _marker: core::marker::PhantomData,
        }
    }
}

/// Maps an integer type to and from `u128` for indexing.
///
/// The mapping is an **order-preserving bijection**: `a < b` implies
/// `a.to_u128() < b.to_u128()`. Implemented for every primitive integer type.
/// Not part of the public surface beyond [`IntRange`].
pub trait IntConvert: Copy {
    /// Converts `self` to `u128` under the order-preserving bijection.
    fn to_u128(self) -> u128;
    /// Inverse of [`to_u128`](IntConvert::to_u128).
    fn from_u128(v: u128) -> Self;
}

/// Flipping this bit maps two's-complement order onto unsigned order.
const SIGN_FLIP: u128 = 1 << 127;

macro_rules! impl_int_convert {
    ($t:ty, signed) => {
        impl IntConvert for $t {
            fn to_u128(self) -> u128 {
                (self as i128 as u128) ^ SIGN_FLIP
            }
            fn from_u128(v: u128) -> Self {
                ((v ^ SIGN_FLIP) as i128) as $t
            }
        }
    };
    ($t:ty, unsigned) => {
        impl IntConvert for $t {
            fn to_u128(self) -> u128 {
                self as u128
            }
            fn from_u128(v: u128) -> Self {
                v as $t
            }
        }
    };
}
impl_int_convert!(i8, signed);
impl_int_convert!(i16, signed);
impl_int_convert!(i32, signed);
impl_int_convert!(i64, signed);
impl_int_convert!(i128, signed);
impl_int_convert!(isize, signed);
impl_int_convert!(u8, unsigned);
impl_int_convert!(u16, unsigned);
impl_int_convert!(u32, unsigned);
impl_int_convert!(u64, unsigned);
impl_int_convert!(u128, unsigned);
impl_int_convert!(usize, unsigned);

macro_rules! impl_int_range {
    ($t:ty) => {
        impl Strategy for IntRange<$t> {
            type Value = $t;

            fn generate(&self, rng: &mut Xorshift64) -> $t {
                // `last - first` is the span minus one. Only the full u128 range
                // has span 2^128, which does not fit in u128 and needs a raw draw.
                let offset = match self.last - self.first {
                    u128::MAX => rng.next_u128(),
                    span => rng.next_below(span + 1),
                };
                <$t as IntConvert>::from_u128(self.first + offset)
            }

            fn shrink_bounds(&self) -> Option<(u128, u128)> {
                Some((self.first, self.last))
            }

            fn index_of(&self, value: &$t) -> Option<u128> {
                let idx = <$t as IntConvert>::to_u128(*value);
                (self.first..=self.last).contains(&idx).then_some(idx)
            }

            fn at_index(&self, idx: u128) -> Option<$t> {
                if (self.first..=self.last).contains(&idx) {
                    Some(<$t as IntConvert>::from_u128(idx))
                } else {
                    None
                }
            }
        }
    };
}

impl_int_range!(i8);
impl_int_range!(i16);
impl_int_range!(i32);
impl_int_range!(i64);
impl_int_range!(i128);
impl_int_range!(isize);
impl_int_range!(u8);
impl_int_range!(u16);
impl_int_range!(u32);
impl_int_range!(u64);
impl_int_range!(u128);
impl_int_range!(usize);

/// Generates boolean values (`true`/`false`), each equally likely.
///
/// Uses the top bit of the RNG output, which is the highest-quality bit of
/// xorshift output.
///
/// # Example
///
/// ```rust
/// use tpt_fl_det_proptest::{AnyBool, Seed, Strategy, Xorshift64};
///
/// let strat = AnyBool;
/// let mut rng = Xorshift64::new(Seed::default().0);
/// let _ = strat.generate(&mut rng);
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct AnyBool;

impl Strategy for AnyBool {
    type Value = bool;

    fn generate(&self, rng: &mut Xorshift64) -> bool {
        (rng.next_u64() >> 63) == 1
    }
}
