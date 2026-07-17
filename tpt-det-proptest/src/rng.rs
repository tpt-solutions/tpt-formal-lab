//! Deterministic PRNG: xorshift64.

/// A deterministic 64-bit xorshift pseudo-random number generator.
///
/// Given the same [`seed`](Xorshift64::new) it produces the same sequence of
/// `u64` values on every platform and every run, with no reliance on system
/// entropy. This is what makes the property tests in this crate reproducible.
///
/// # Example
///
/// ```rust
/// use tpt_det_proptest::Xorshift64;
///
/// let mut rng = Xorshift64::new(12345);
/// let a = rng.next_u64();
/// let mut rng2 = Xorshift64::new(12345);
/// assert_eq!(a, rng2.next_u64()); // same seed ⇒ same first value
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    /// Creates a new generator from a non-zero seed.
    ///
    /// A zero seed would freeze the generator at zero, so it is mapped to a
    /// fixed non-zero default.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_det_proptest::Xorshift64;
    ///
    /// let mut rng = Xorshift64::new(1);
    /// assert_ne!(rng.next_u64(), 0);
    /// ```
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x9E3779B97F4A7C15 } else { seed },
        }
    }

    /// Returns the next pseudo-random `u64`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_det_proptest::Xorshift64;
    ///
    /// let mut rng = Xorshift64::new(99);
    /// let _ = rng.next_u64();
    /// ```
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Returns the next pseudo-random `u32`.
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
}
