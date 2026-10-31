//! Deterministic PRNG: xorshift64, seeded through splitmix64.

/// A deterministic 64-bit xorshift pseudo-random number generator.
///
/// Given the same seed it produces the same sequence of values on every
/// platform and every run, with no reliance on system entropy. This is what
/// makes the property tests in this crate reproducible.
///
/// The seed is first mixed through splitmix64, so nearby seeds (such as `0`
/// and `1`) give unrelated sequences.
///
/// # Example
///
/// ```rust
/// use tpt_fl_det_proptest::Xorshift64;
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

/// Used in place of a seed whose mixed value is zero, because xorshift is
/// stuck at zero forever.
const FALLBACK_STATE: u64 = 0x9E37_79B9_7F4A_7C15;

/// The splitmix64 finaliser. It is a bijection on `u64`, so distinct inputs
/// give distinct outputs.
const fn splitmix64(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl Xorshift64 {
    /// Creates a new generator from a seed.
    ///
    /// Every seed, including `0`, is valid. The seed is mixed through
    /// splitmix64 before use. The only seed whose mixed value would be zero is
    /// mapped to a fixed non-zero state.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_fl_det_proptest::Xorshift64;
    ///
    /// let mut rng = Xorshift64::new(0);
    /// assert_ne!(rng.next_u64(), 0);
    /// ```
    pub fn new(seed: u64) -> Self {
        let mixed = splitmix64(seed);
        Self {
            state: if mixed == 0 { FALLBACK_STATE } else { mixed },
        }
    }

    /// Returns the next pseudo-random `u64`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_fl_det_proptest::Xorshift64;
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

    /// Returns a pseudo-random `u128` built from two `u64` draws.
    pub(crate) fn next_u128(&mut self) -> u128 {
        let hi = self.next_u64() as u128;
        let lo = self.next_u64() as u128;
        (hi << 64) | lo
    }

    /// Returns a value uniformly distributed in `[0, bound)`.
    ///
    /// Draws at or above the largest multiple of `bound` are rejected, so no
    /// residue is favoured. Plain `draw % bound` favours small residues whenever
    /// `bound` does not divide the draw range. The rejection rate is below
    /// `bound / 2^128`, which is negligible.
    ///
    /// # Panics
    ///
    /// Panics if `bound` is zero.
    pub(crate) fn next_below(&mut self, bound: u128) -> u128 {
        assert!(bound > 0, "next_below requires a positive bound");
        let zone = (u128::MAX / bound) * bound;
        loop {
            let x = self.next_u128();
            if x < zone {
                return x % bound;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_seed_is_not_degenerate() {
        let mut rng = Xorshift64::new(0);
        let values: [u64; 4] = [
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
        ];
        assert!(values.iter().all(|&v| v != 0));
        assert_ne!(values[0], values[1]);
    }

    #[test]
    fn zero_and_golden_ratio_seeds_do_not_collide() {
        // The old implementation mapped seed 0 to the golden-ratio constant.
        let mut a = Xorshift64::new(0);
        let mut b = Xorshift64::new(0x9E37_79B9_7F4A_7C15);
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn splitmix_is_injective_on_a_sample() {
        let mut seen = [0u64; 256];
        for (i, slot) in seen.iter_mut().enumerate() {
            *slot = splitmix64(i as u64);
        }
        for i in 0..seen.len() {
            for j in (i + 1)..seen.len() {
                assert_ne!(seen[i], seen[j]);
            }
        }
    }

    #[test]
    fn next_below_has_no_modulo_bias() {
        // With bound = 2^63 + 1, a plain `draw % bound` over u64 puts about two
        // thirds of the mass into the lower half. Rejection sampling should give
        // about one half.
        let bound: u128 = (1u128 << 63) + 1;
        let half = bound / 2;
        let mut rng = Xorshift64::new(2024);
        let samples = 20_000;
        let lower = (0..samples)
            .filter(|_| rng.next_below(bound) < half)
            .count();
        let fraction = lower as f64 / samples as f64;
        assert!(
            (0.46..0.54).contains(&fraction),
            "lower-half fraction {fraction} is biased"
        );
    }

    #[test]
    fn next_below_stays_in_range() {
        let mut rng = Xorshift64::new(5);
        for bound in [1u128, 2, 3, 7, 1000, u64::MAX as u128 + 5] {
            for _ in 0..200 {
                assert!(rng.next_below(bound) < bound);
            }
        }
    }
}
