//! Shared reproducible random streams for manifold integration properties.

/// xorshift64* — small, deterministic, portable.
pub(crate) struct Rng {
    state: u64,
    property_seed: u64,
    master_seed: Option<u64>,
}

impl Rng {
    pub(crate) fn for_property(stream: u64) -> Self {
        let master_seed = match std::env::var("MANIFOLD_PROPERTY_SEED") {
            Err(std::env::VarError::NotPresent) => None,
            Err(std::env::VarError::NotUnicode(_)) => {
                panic!("MANIFOLD_PROPERTY_SEED must be valid UTF-8")
            }
            Ok(value) => {
                let parsed = value
                    .strip_prefix("0x")
                    .map_or_else(|| value.parse::<u64>(), |hex| u64::from_str_radix(hex, 16))
                    .unwrap_or_else(|_| {
                        panic!(
                            "MANIFOLD_PROPERTY_SEED must be a decimal or 0x-prefixed u64: {value:?}"
                        )
                    });
                Some(parsed)
            }
        };
        let property_seed = master_seed.map_or(stream, |master| {
            // SplitMix64's finalizer derives independent property streams from
            // one run seed while keeping every failing CI case reproducible.
            let mut z = master
                .wrapping_add(stream)
                .wrapping_add(0x9E37_79B9_7F4A_7C15);
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            let mixed = z ^ (z >> 31);
            if mixed == 0 { stream } else { mixed }
        });
        Self {
            state: property_seed,
            property_seed,
            master_seed,
        }
    }

    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    pub(crate) fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    #[allow(dead_code)] // Shared by test crates with different generators.
    pub(crate) fn coord(&mut self) -> f64 {
        // Coordinates on a coarse grid with occasional duplicates and NaN-free
        // jitter: welding must find real duplicates, noise must stay finite.
        let base = (self.below(8)) as f64;
        if self.below(4) == 0 {
            base + self.below(1000) as f64 * 1e-9
        } else {
            base
        }
    }
}

impl Drop for Rng {
    fn drop(&mut self) {
        if std::thread::panicking() {
            match self.master_seed {
                Some(master) => eprintln!(
                    "manifold property failed; reproduce with MANIFOLD_PROPERTY_SEED={master} (property seed {})",
                    self.property_seed
                ),
                None => eprintln!(
                    "manifold property failed with fixed property seed {} (no MANIFOLD_PROPERTY_SEED override)",
                    self.property_seed
                ),
            }
        }
    }
}
