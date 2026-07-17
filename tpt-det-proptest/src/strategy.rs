//! Strategies for generating test values.

use crate::rng::Xorshift64;

/// A deterministic seed for a property-test run.
///
/// Two runs with the same [`Seed`] produce the same generated sequence and the
/// same shrinking path.
///
/// # Example
///
/// ```rust
/// use tpt_det_proptest::{check, IntRange, Seed};
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
/// Strategies that support shrinking (such as [`IntRange`]) additionally expose
/// an ordered index space so the runner can bisect toward a minimal
/// counterexample.
pub trait Strategy {
    /// The type of value produced by this strategy.
    type Value;

    /// Draws the next value from `rng`.
    fn generate(&self, rng: &mut Xorshift64) -> Self::Value;

    /// Returns the inclusive-lower / exclusive-upper bounds of the strategy's
    /// index space, if it supports bisection shrinking. `None` otherwise.
    fn shrink_bounds(&self) -> Option<(u64, u64)> {
        None
    }

    /// Materialises the value at index `idx` within the strategy's index space.
    /// Only meaningful when [`shrink_bounds`](Strategy::shrink_bounds) is `Some`.
    fn at_index(&self, _idx: u64) -> Option<Self::Value> {
        None
    }
}

/// Generates integers uniformly in the half-open range `[low, high)`.
///
/// Works for any integer type that can be converted to/from `u64`. For ranges
/// wider than `u64` a panic panic occurs; the provided tests use small ranges.
///
/// # Example
///
/// ```rust
/// use tpt_det_proptest::{IntRange, Seed, Strategy, Xorshift64};
///
/// let strat = IntRange::<i32>::new(0, 5);
/// let mut rng = Xorshift64::new(Seed::default().0);
/// let v = strat.generate(&mut rng);
/// assert!((0..5).contains(&v));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct IntRange<T> {
    low_u64: u64,
    high_u64: u64,
    _marker: core::marker::PhantomData<T>,
}

impl<T> IntRange<T> {
    /// Creates a strategy generating values in `[low, high)`.
    ///
    /// # Panics
    ///
    /// Panics if `low >= high` or if the range spans more than `u64::MAX`
    /// representable values.
    pub fn new(low: T, high: T) -> Self
    where
        T: Copy + PartialOrd,
        T: IntConvert,
    {
        assert!(low < high, "IntRange requires low < high");
        let low_u64 = low.to_u64();
        let high_u64 = high.to_u64();
        assert!(
            high_u64 > low_u64,
            "IntRange span too wide for u64 index space"
        );
        Self {
            low_u64,
            high_u64,
            _marker: core::marker::PhantomData,
        }
    }
}

/// Trait mapping an integer type to/from `u64` for indexing. Implemented for all
/// primitive integer types. Not part of the public surface beyond `IntRange`.
pub trait IntConvert {
    /// Converts `self` to `u64` under an **order-preserving** bijection, so that
    /// `a < b` implies `a.to_u64() < b.to_u64()`.
    fn to_u64(self) -> u64;
    /// Inverse of [`to_u64`](IntConvert::to_u64).
    fn from_u64(v: u64) -> Self;
}

const SIGN_FLIP: u64 = 0x8000_0000_0000_0000;

macro_rules! impl_int_convert {
    ($t:ty, signed) => {
        impl IntConvert for $t {
            fn to_u64(self) -> u64 {
                // Two's-complement order-preserving bijection to unsigned.
                (self as i64 as u64) ^ SIGN_FLIP
            }
            fn from_u64(v: u64) -> Self {
                ((v ^ SIGN_FLIP) as i64) as $t
            }
        }
    };
    ($t:ty, unsigned) => {
        impl IntConvert for $t {
            fn to_u64(self) -> u64 {
                self as u64
            }
            fn from_u64(v: u64) -> Self {
                v as $t
            }
        }
    };
}
impl_int_convert!(i8, signed);
impl_int_convert!(i16, signed);
impl_int_convert!(i32, signed);
impl_int_convert!(i64, signed);
impl_int_convert!(u8, unsigned);
impl_int_convert!(u16, unsigned);
impl_int_convert!(u32, unsigned);
impl_int_convert!(u64, unsigned);

macro_rules! impl_int_range {
    ($t:ty) => {
        impl Strategy for IntRange<$t> {
            type Value = $t;

            fn generate(&self, rng: &mut Xorshift64) -> $t {
                let span = self.high_u64 - self.low_u64;
                let idx = rng.next_u64() % span;
                <$t as IntConvert>::from_u64(self.low_u64 + idx)
            }

            fn shrink_bounds(&self) -> Option<(u64, u64)> {
                Some((self.low_u64, self.high_u64))
            }

            fn at_index(&self, idx: u64) -> Option<Self::Value> {
                if idx >= self.high_u64 {
                    None
                } else {
                    Some(<$t as IntConvert>::from_u64(idx))
                }
            }
        }
    };
}

impl_int_range!(i8);
impl_int_range!(i16);
impl_int_range!(i32);
impl_int_range!(i64);
impl_int_range!(u8);
impl_int_range!(u16);
impl_int_range!(u32);
impl_int_range!(u64);

/// Generates boolean values (`true`/`false`), each equally likely.
///
/// # Example
///
/// ```rust
/// use tpt_det_proptest::{AnyBool, Seed, Strategy, Xorshift64};
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
        (rng.next_u64() & 1) == 1
    }
}
