//! Repeatable chance: the same seed always gives the same numbers, on
//! every computer.

/// SplitMix64: small, fast and good enough for shapes.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut r = Rng(seed ^ 0x5DEE_CE66_D1A4_F87B);
        r.next();
        r
    }

    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// From 0 (included) to 1 (excluded).
    pub fn f(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// From `a` to `b`.
    pub fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.f()
    }

    /// From `-1` to `1`.
    pub fn signed(&mut self) -> f64 {
        self.f() * 2.0 - 1.0
    }

    /// A new generator for a part, so that changing one part does not
    /// move the numbers of the others.
    pub fn fork(&mut self, salt: u64) -> Rng {
        let mut h = Hash::new();
        h.add(self.next());
        h.add(salt);
        Rng::new(h.finish())
    }
}

/// A hash of numbers (FNV-1a over their bytes, mixed at the end).
pub struct Hash(u64);

impl Hash {
    pub fn new() -> Self {
        Hash(0xCBF2_9CE4_8422_2325)
    }

    pub fn add(&mut self, v: u64) {
        for b in v.to_le_bytes() {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(0x0100_0000_01B3);
        }
    }

    pub fn finish(&self) -> u64 {
        let mut z = self.0;
        z = (z ^ (z >> 33)).wrapping_mul(0xFF51_AFD7_ED55_8CCD);
        z = (z ^ (z >> 33)).wrapping_mul(0xC4CE_B9FE_1A85_EC53);
        z ^ (z >> 33)
    }
}
