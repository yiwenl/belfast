#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeightSettings {
    pub base: f32,
    pub spread: f32,
    pub outlier: f32,
    pub exponent: f32,
    pub limit: f32,
    pub noise_scale: f32,
    pub noise_amplitude: f32,
}

impl Default for HeightSettings {
    fn default() -> Self {
        Self {
            base: 1.25,
            spread: 0.5,
            outlier: 1.25,
            exponent: 8.,
            limit: 3.,
            noise_scale: 4.,
            noise_amplitude: 0.5,
        }
    }
}

impl HeightSettings {
    pub fn validate(self) -> Result<Self, String> {
        for (value, min, max) in [
            (self.base, 0.25, 3.),
            (self.spread, 0., 1.5),
            (self.outlier, 0., 3.),
            (self.exponent, 1., 16.),
            (self.limit, 0.25, 3.),
            (self.noise_scale, 0.5, 20.),
            (self.noise_amplitude, 0., 1.5),
        ] {
            if !value.is_finite() || !(min..=max).contains(&value) {
                return Err("Firefly height settings are outside their supported ranges".into());
            }
        }
        Ok(self)
    }

    pub fn base_ceiling(self, seed: f32) -> f32 {
        (self.base + self.spread * seed + self.outlier * seed.powf(self.exponent)).min(self.limit)
    }

    // Match ceiling_noise in WGSL so spawning and simulation share the same surface.
    pub fn ceiling(self, xz: [f32; 2], seed: f32) -> f32 {
        let p = xz.map(|v| v / self.noise_scale);
        let cell = p.map(|v| v.floor() as i32);
        let u: [f32; 2] = std::array::from_fn(|i| {
            let f = p[i] - cell[i] as f32;
            f * f * f * (f * (f * 6. - 15.) + 10.)
        });
        let sample = |x: i32, z: i32| {
            let mut h = (x as u32).wrapping_mul(1597334677)
                ^ (z as u32).wrapping_mul(2798796415)
                ^ 0x68bc21eb;
            h = (h ^ (h >> 16)).wrapping_mul(0x7feb352d);
            h = (h ^ (h >> 15)).wrapping_mul(0x846ca68b);
            h ^= h >> 16;
            (h >> 8) as f32 * (2. / 16_777_216.) - 1.
        };
        let [x, z] = cell;
        let a = sample(x, z);
        let b = sample(x + 1, z);
        let c = sample(x, z + 1);
        let d = sample(x + 1, z + 1);
        let near = a + (b - a) * u[0];
        let far = c + (d - c) * u[0];
        (self.base_ceiling(seed) + (near + (far - near) * u[1]) * self.noise_amplitude)
            .clamp(0.25, self.limit)
    }
}
