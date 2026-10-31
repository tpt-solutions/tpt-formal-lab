//! `FixedPoint<DENOM>` — cross-platform bitwise-deterministic fixed-point numbers.

use core::fmt;
use core::ops::{Add, Div, Mul, Neg, Sub};

#[cfg(feature = "exact-math")]
use tpt_fl_exact_math::Rational;

/// A fixed-point rational number `raw / DENOM` where `DENOM` is a compile-time
/// constant denominator.
///
/// All arithmetic uses integer operations only — no floating-point at any step —
/// guaranteeing that results are bitwise identical across all CPU architectures.
///
/// The type parameter `DENOM` sets the precision and must be positive. Common
/// choices:
/// - `1_000` — 3 decimal places
/// - `1_000_000` — 6 decimal places (microsecond-level timesteps)
/// - `1_000_000_000` — 9 decimal places
///
/// Multiplication and division truncate toward zero. Overflow panics in the
/// operators; use the `checked_*` or `saturating_*` methods to handle it.
///
/// # Example
///
/// ```rust
/// use tpt_fl_deterministic_sim::FixedPoint;
///
/// type Fp = FixedPoint<1_000>; // 3 decimal places
///
/// let a = Fp::from_int(1);          //  1.000
/// let b = Fp::from_raw(500);        //  0.500
/// let c = a + b;
/// assert_eq!(c.raw(), 1_500);       //  1.500
/// assert_eq!(c.to_string(), "1.500");
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FixedPoint<const DENOM: i64> {
    raw: i64,
}

impl<const DENOM: i64> FixedPoint<DENOM> {
    /// Evaluated whenever a constructor is used, so a non-positive `DENOM` fails
    /// to compile at the use site.
    const DENOM_IS_POSITIVE: () = assert!(DENOM > 0, "FixedPoint DENOM must be positive");

    /// Creates a `FixedPoint` from a raw integer numerator.
    ///
    /// The value represented is `raw / DENOM`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_fl_deterministic_sim::FixedPoint;
    /// type Fp = FixedPoint<1_000>;
    /// let half = Fp::from_raw(500); // 0.500
    /// assert_eq!(half.raw(), 500);
    /// ```
    pub const fn from_raw(raw: i64) -> Self {
        let () = Self::DENOM_IS_POSITIVE;
        Self { raw }
    }

    /// Creates a `FixedPoint` from an integer value (multiplied by `DENOM`).
    ///
    /// # Panics
    ///
    /// Panics if `n * DENOM` overflows `i64`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_fl_deterministic_sim::FixedPoint;
    /// type Fp = FixedPoint<1_000>;
    /// let three = Fp::from_int(3);
    /// assert_eq!(three.raw(), 3_000);
    /// ```
    pub const fn from_int(n: i64) -> Self {
        match n.checked_mul(DENOM) {
            Some(raw) => Self::from_raw(raw),
            None => panic!("FixedPoint::from_int overflow"),
        }
    }

    /// Returns zero.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_fl_deterministic_sim::FixedPoint;
    /// assert_eq!(FixedPoint::<1_000>::zero().raw(), 0);
    /// ```
    pub const fn zero() -> Self {
        Self::from_raw(0)
    }

    /// Returns one (= `DENOM / DENOM`).
    pub const fn one() -> Self {
        Self::from_raw(DENOM)
    }

    /// Returns the raw integer numerator.
    pub const fn raw(self) -> i64 {
        self.raw
    }

    /// Returns `true` if this value is zero.
    pub const fn is_zero(self) -> bool {
        self.raw == 0
    }

    /// Returns `true` if this value is strictly negative.
    pub const fn is_negative(self) -> bool {
        self.raw < 0
    }

    /// Returns `true` if this value is strictly positive.
    pub const fn is_positive(self) -> bool {
        self.raw > 0
    }

    /// Returns the absolute value.
    ///
    /// # Panics
    ///
    /// Panics if the value is `i64::MIN` in raw form, whose absolute value does
    /// not fit in `i64`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_fl_deterministic_sim::FixedPoint;
    /// type Fp = FixedPoint<1_000>;
    /// assert_eq!(Fp::from_raw(-500).abs(), Fp::from_raw(500));
    /// ```
    pub const fn abs(self) -> Self {
        match self.raw.checked_abs() {
            Some(raw) => Self::from_raw(raw),
            None => panic!("FixedPoint::abs overflow"),
        }
    }

    /// Returns the absolute value, or `None` if the raw value is `i64::MIN`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_fl_deterministic_sim::FixedPoint;
    /// type Fp = FixedPoint<1_000>;
    /// assert_eq!(Fp::from_raw(i64::MIN).checked_abs(), None);
    /// ```
    pub fn checked_abs(self) -> Option<Self> {
        self.raw.checked_abs().map(Self::from_raw)
    }

    /// Checked addition. Returns `None` on overflow.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_fl_deterministic_sim::FixedPoint;
    /// type Fp = FixedPoint<1_000>;
    /// assert_eq!(Fp::from_raw(i64::MAX).checked_add(Fp::from_raw(1)), None);
    /// ```
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.raw.checked_add(rhs.raw).map(Self::from_raw)
    }

    /// Checked subtraction. Returns `None` on overflow.
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.raw.checked_sub(rhs.raw).map(Self::from_raw)
    }

    /// Checked negation. Returns `None` for `i64::MIN` in raw form.
    pub fn checked_neg(self) -> Option<Self> {
        self.raw.checked_neg().map(Self::from_raw)
    }

    /// Checked multiplication, truncating toward zero. The product is formed in
    /// `i128`, so it cannot overflow before rescaling. Returns `None` if the
    /// rescaled result does not fit in `i64`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_fl_deterministic_sim::FixedPoint;
    /// type Fp = FixedPoint<1_000>;
    /// let big = Fp::from_raw(i64::MAX / 2);
    /// assert_eq!(big.checked_mul(Fp::from_int(3)), None);
    /// assert_eq!(Fp::from_int(2).checked_mul(Fp::from_int(3)), Some(Fp::from_int(6)));
    /// ```
    pub fn checked_mul(self, rhs: Self) -> Option<Self> {
        let prod = (self.raw as i128 * rhs.raw as i128) / DENOM as i128;
        i64::try_from(prod).ok().map(Self::from_raw)
    }

    /// Checked division, truncating toward zero. Returns `None` if `rhs` is zero
    /// or the result does not fit in `i64`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_fl_deterministic_sim::FixedPoint;
    /// type Fp = FixedPoint<1_000>;
    /// assert_eq!(Fp::from_int(1).checked_div(Fp::zero()), None);
    /// assert_eq!(Fp::from_int(1).checked_div(Fp::from_int(4)), Some(Fp::from_raw(250)));
    /// ```
    pub fn checked_div(self, rhs: Self) -> Option<Self> {
        if rhs.raw == 0 {
            return None;
        }
        let quot = (self.raw as i128 * DENOM as i128) / rhs.raw as i128;
        i64::try_from(quot).ok().map(Self::from_raw)
    }

    /// Addition that clamps at the bounds of `i64` instead of overflowing.
    pub fn saturating_add(self, rhs: Self) -> Self {
        Self::from_raw(self.raw.saturating_add(rhs.raw))
    }

    /// Subtraction that clamps at the bounds of `i64` instead of overflowing.
    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self::from_raw(self.raw.saturating_sub(rhs.raw))
    }

    /// Multiplication that clamps at the bounds of `i64` instead of overflowing.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_fl_deterministic_sim::FixedPoint;
    /// type Fp = FixedPoint<1_000>;
    /// let max = Fp::from_raw(i64::MAX);
    /// assert_eq!(max.saturating_mul(Fp::from_int(2)), max);
    /// assert_eq!(max.saturating_mul(-Fp::from_int(2)), Fp::from_raw(i64::MIN));
    /// ```
    pub fn saturating_mul(self, rhs: Self) -> Self {
        let prod = (self.raw as i128 * rhs.raw as i128) / DENOM as i128;
        Self::from_raw(prod.clamp(i64::MIN as i128, i64::MAX as i128) as i64)
    }

    /// Converts to `f64` for display or interoperability only.
    ///
    /// Do not use the result in further simulation computations.
    pub fn to_f64(self) -> f64 {
        self.raw as f64 / DENOM as f64
    }

    /// Converts from an `f64` by truncating toward zero.
    ///
    /// Lossy, and not part of the deterministic core: the result depends on
    /// floating-point rounding of `v * DENOM`. Prefer [`from_raw`](Self::from_raw)
    /// or [`from_int`](Self::from_int) for deterministic construction. Values
    /// outside the `i64` range saturate, and NaN becomes zero.
    pub fn from_f64_truncating(v: f64) -> Self {
        Self::from_raw((v * DENOM as f64) as i64)
    }
}

#[cfg(feature = "exact-math")]
impl<const DENOM: i64> FixedPoint<DENOM> {
    /// Converts to the exact rational `raw / DENOM`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[cfg(feature = "exact-math")] {
    /// use tpt_fl_deterministic_sim::FixedPoint;
    /// use tpt_fl_exact_math::Rational;
    /// type Fp = FixedPoint<1_000>;
    /// assert_eq!(Fp::from_raw(250).to_rational(), Rational::from_frac(1, 4));
    /// # }
    /// ```
    pub fn to_rational(self) -> Rational {
        Rational::from_frac(self.raw, DENOM)
    }

    /// Converts an exact rational to fixed point. Returns `None` unless
    /// `r · DENOM` is an integer that fits in `i64`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[cfg(feature = "exact-math")] {
    /// use tpt_fl_deterministic_sim::FixedPoint;
    /// use tpt_fl_exact_math::Rational;
    /// type Fp = FixedPoint<1_000>;
    /// assert_eq!(Fp::from_rational(&Rational::from_frac(1, 4)), Some(Fp::from_raw(250)));
    /// assert_eq!(Fp::from_rational(&Rational::from_frac(1, 3)), None);
    /// # }
    /// ```
    pub fn from_rational(r: &Rational) -> Option<Self> {
        let scaled = r.clone() * Rational::from_frac(DENOM, 1);
        if scaled.floor() != scaled {
            return None;
        }
        scaled.to_i64().map(Self::from_raw)
    }
}

// ── Arithmetic ────────────────────────────────────────────────────────────────

impl<const DENOM: i64> Add for FixedPoint<DENOM> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        self.checked_add(rhs).expect("FixedPoint addition overflow")
    }
}

impl<const DENOM: i64> Sub for FixedPoint<DENOM> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        self.checked_sub(rhs)
            .expect("FixedPoint subtraction overflow")
    }
}

impl<const DENOM: i64> Neg for FixedPoint<DENOM> {
    type Output = Self;
    fn neg(self) -> Self {
        self.checked_neg().expect("FixedPoint negation overflow")
    }
}

impl<const DENOM: i64> Mul for FixedPoint<DENOM> {
    type Output = Self;
    /// Multiplies two fixed-point values: `(a/D) * (b/D) = (a*b) / D²`,
    /// then rescales back to `/ D` by dividing the raw product by `DENOM`.
    ///
    /// # Panics
    ///
    /// Panics if the result overflows `i64`.
    #[allow(clippy::suspicious_arithmetic_impl)] // rescaling division is correct fixed-point math, not a bug
    fn mul(self, rhs: Self) -> Self {
        self.checked_mul(rhs)
            .expect("FixedPoint multiplication overflow")
    }
}

impl<const DENOM: i64> Div for FixedPoint<DENOM> {
    type Output = Self;
    /// Divides: `(a/D) / (b/D) = (a * D) / b`.
    ///
    /// # Panics
    ///
    /// Panics if `rhs` is zero or the result overflows `i64`.
    fn div(self, rhs: Self) -> Self {
        assert!(!rhs.is_zero(), "FixedPoint division by zero");
        self.checked_div(rhs).expect("FixedPoint division overflow")
    }
}

// ── Display / Debug ───────────────────────────────────────────────────────────

/// Number of decimal digits printed after the point: one less than the number
/// of digits in `DENOM`. This is exact for powers of ten and truncates otherwise.
const fn fraction_digits(denom: i64) -> u32 {
    let mut digits = 0;
    let mut n = denom;
    while n >= 10 {
        n /= 10;
        digits += 1;
    }
    digits
}

impl<const DENOM: i64> fmt::Display for FixedPoint<DENOM> {
    /// Formats as a decimal with `fraction_digits(DENOM)` digits after the point.
    /// The fractional part is truncated, so for a `DENOM` that is not a power of
    /// ten the printed value can be slightly below the exact one.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let width = fraction_digits(DENOM);
        let sign = if self.raw < 0 { "-" } else { "" };
        let abs_raw = self.raw.unsigned_abs();
        let denom_u = DENOM as u64;
        let int_part = abs_raw / denom_u;
        if width == 0 {
            return write!(f, "{sign}{int_part}");
        }
        // Remainder is below DENOM, so this product fits comfortably in u128.
        let rem = (abs_raw % denom_u) as u128;
        let frac = rem * 10u128.pow(width) / denom_u as u128;
        write!(
            f,
            "{sign}{int_part}.{frac:0>width$}",
            width = width as usize
        )
    }
}

impl<const DENOM: i64> fmt::Debug for FixedPoint<DENOM> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FixedPoint<{DENOM}>({self})")
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    type Fp = FixedPoint<1_000>;

    #[test]
    fn addition() {
        let a = Fp::from_raw(750);
        let b = Fp::from_raw(250);
        assert_eq!((a + b).raw(), 1_000);
    }

    #[test]
    fn subtraction() {
        let a = Fp::from_int(2);
        let b = Fp::from_raw(500);
        assert_eq!((a - b).raw(), 1_500);
    }

    #[test]
    fn multiplication() {
        // 2.0 * 3.0 = 6.0
        let a = Fp::from_int(2);
        let b = Fp::from_int(3);
        assert_eq!((a * b).raw(), 6_000);
    }

    #[test]
    fn multiplication_fractional() {
        // 0.5 * 0.5 = 0.25
        let a = Fp::from_raw(500);
        let b = Fp::from_raw(500);
        assert_eq!((a * b).raw(), 250);
    }

    #[test]
    fn multiplication_uses_wide_intermediate() {
        // The raw product 5e9 * 5e9 = 2.5e19 overflows i64; only after the
        // rescale by DENOM does the result (2.5e16) fit.
        let a = Fp::from_raw(5_000_000_000);
        let b = Fp::from_raw(5_000_000_000);
        assert_eq!((a * b).raw(), 25_000_000_000_000_000);
    }

    #[test]
    fn division() {
        // 1.0 / 4.0 = 0.25
        let a = Fp::from_int(1);
        let b = Fp::from_int(4);
        assert_eq!((a / b).raw(), 250);
    }

    #[test]
    fn division_uses_wide_intermediate() {
        // 1e16 * 1000 = 1e19 overflows i64; the i128 path gives 1e19 / 2000 = 5e15.
        let a = Fp::from_raw(10_000_000_000_000_000);
        let b = Fp::from_raw(2_000);
        assert_eq!((a / b).raw(), 5_000_000_000_000_000);
    }

    #[test]
    fn negation() {
        let a = Fp::from_raw(500);
        assert_eq!((-a).raw(), -500);
    }

    #[test]
    fn negative_truncation_toward_zero() {
        // -1.001 * 0.5 = -0.5005 → raw -500.5, truncated toward zero to -500.
        let a = Fp::from_raw(-1_001);
        let half = Fp::from_raw(500);
        assert_eq!((a * half).raw(), -500);
        // -1.001 / 2.0 = -0.5005 → raw -500.5 → truncated to -500.
        assert_eq!((a / Fp::from_int(2)).raw(), -500);
    }

    #[test]
    #[should_panic(expected = "addition overflow")]
    fn add_overflow_panics() {
        let _ = Fp::from_raw(i64::MAX) + Fp::from_raw(1);
    }

    #[test]
    #[should_panic(expected = "subtraction overflow")]
    fn sub_overflow_panics() {
        let _ = Fp::from_raw(i64::MIN) - Fp::from_raw(1);
    }

    #[test]
    #[should_panic(expected = "negation overflow")]
    fn neg_min_panics() {
        let _ = -Fp::from_raw(i64::MIN);
    }

    #[test]
    #[should_panic(expected = "multiplication overflow")]
    fn mul_overflow_panics() {
        let _ = Fp::from_raw(i64::MAX) * Fp::from_int(2);
    }

    #[test]
    #[should_panic(expected = "division by zero")]
    fn div_by_zero_panics() {
        let _ = Fp::from_int(1) / Fp::zero();
    }

    #[test]
    #[should_panic(expected = "from_int overflow")]
    fn from_int_overflow_panics() {
        let _ = Fp::from_int(i64::MAX);
    }

    #[test]
    fn checked_ops_return_none_on_overflow() {
        assert_eq!(Fp::from_raw(i64::MAX).checked_add(Fp::from_raw(1)), None);
        assert_eq!(Fp::from_raw(i64::MIN).checked_sub(Fp::from_raw(1)), None);
        assert_eq!(Fp::from_raw(i64::MIN).checked_neg(), None);
        assert_eq!(Fp::from_raw(i64::MIN).checked_abs(), None);
        assert_eq!(Fp::from_raw(i64::MAX).checked_mul(Fp::from_int(2)), None);
        assert_eq!(Fp::from_int(1).checked_div(Fp::zero()), None);
        // Division result too large for i64: MAX * 1000 / 1 raw.
        assert_eq!(Fp::from_raw(i64::MAX).checked_div(Fp::from_raw(1)), None);
    }

    #[test]
    fn saturating_ops_clamp() {
        let max = Fp::from_raw(i64::MAX);
        let min = Fp::from_raw(i64::MIN);
        assert_eq!(max.saturating_add(Fp::from_raw(1)), max);
        assert_eq!(min.saturating_sub(Fp::from_raw(1)), min);
        assert_eq!(max.saturating_mul(Fp::from_int(2)), max);
        assert_eq!(max.saturating_mul(-Fp::from_int(2)), min);
    }

    #[test]
    fn ordering() {
        assert!(Fp::from_raw(500) < Fp::from_raw(750));
        assert!(Fp::from_int(1) > Fp::from_raw(999));
        assert_eq!(Fp::from_raw(500), Fp::from_raw(500));
    }

    #[test]
    fn display_integer() {
        assert_eq!(Fp::from_int(3).to_string(), "3.000");
    }

    #[test]
    fn display_fractional() {
        assert_eq!(Fp::from_raw(1_500).to_string(), "1.500");
    }

    #[test]
    fn display_negative() {
        assert_eq!(Fp::from_raw(-750).to_string(), "-0.750");
    }

    #[test]
    fn display_other_power_of_ten_denominators() {
        assert_eq!(FixedPoint::<10>::from_raw(15).to_string(), "1.5");
        assert_eq!(FixedPoint::<1_000_000>::from_raw(1).to_string(), "0.000001");
        assert_eq!(
            FixedPoint::<1_000_000_000>::from_raw(-1).to_string(),
            "-0.000000001"
        );
        assert_eq!(FixedPoint::<1>::from_raw(7).to_string(), "7");
    }

    #[test]
    fn display_non_power_of_ten_denominator_truncates() {
        // 500/1024 = 0.48828125 exactly; three digits are printed, truncated.
        assert_eq!(FixedPoint::<1_024>::from_raw(500).to_string(), "0.488");
        // 3/1024 truncated to three digits = 0.002.
        assert_eq!(FixedPoint::<1_024>::from_raw(3).to_string(), "0.002");
    }

    #[test]
    fn display_extreme_values() {
        assert_eq!(Fp::from_raw(i64::MIN).to_string(), "-9223372036854775.808");
        assert_eq!(Fp::from_raw(i64::MAX).to_string(), "9223372036854775.807");
    }

    #[test]
    fn deterministic_across_operations() {
        // Prove that the same sequence of operations always gives the same raw bits
        let a = Fp::from_raw(314_159);
        let b = Fp::from_raw(271_828);
        let result = (a + b) * Fp::from_int(2) - Fp::from_raw(100_000);
        assert_eq!(result.raw(), 1_071_974);
    }

    #[cfg(feature = "exact-math")]
    #[test]
    fn rational_round_trip() {
        let x = Fp::from_raw(-1_250);
        let r = x.to_rational();
        assert_eq!(r, Rational::from_frac(-5, 4));
        assert_eq!(Fp::from_rational(&r), Some(x));
        assert_eq!(Fp::from_rational(&Rational::from_frac(1, 3)), None);
    }
}
