/// The `SplitMix64` generator of Steele, Lea and Flood (2014).
///
/// The state is one `u64` advanced by the golden-ratio increment
/// `0x9E37_79B9_7F4A_7C15`; each output is the new state passed through a
/// fixed mixing function. It has period `2^64`, passes `BigCrush`, and is
/// fully specified by these few lines, so a run is reproducible from the
/// seed alone, without depending on the stream stability of an external
/// crate. It is not cryptographically secure, which sampling does not need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// A generator whose stream is determined by `seed`.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// The next 64-bit output, uniform over all `u64` values.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}
