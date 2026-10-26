/// Frequency, pulse length, randomness, participation, traveling wave.
pub(super) fn sample(seconds: f32) -> [f32; 5] {
    const QUIET: [f32; 5] = [0.22, 0.4, 1.3, 0.10, 0.];
    const DARK: [f32; 5] = [0.12, 0.28, 1.5, 0.008, 0.];
    const KEYS: [(f32, [f32; 5]); 7] = [
        (0., QUIET),
        (8., QUIET),
        (12., [0.5, 0.65, 0.4, 0.55, 1.]),
        (16., [0.8, 0.8, 0.2, 0.8, 1.]),
        (22., DARK),
        (27., DARK),
        (30., QUIET),
    ];
    let time = seconds.rem_euclid(30.);
    let pair = KEYS.windows(2).find(|pair| time < pair[1].0).unwrap();
    let t = (time - pair[0].0) / (pair[1].0 - pair[0].0);
    let blend = t * t * (3. - 2. * t);
    std::array::from_fn(|i| pair[0].1[i] + (pair[1].1[i] - pair[0].1[i]) * blend)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rhythm_waves_disperse_gradually_then_leave_only_a_few_fireflies() {
        let quiet = sample(4.);
        let wave = sample(14.);
        let dispersing = sample(19.);
        let dark = sample(24.);
        let returning = sample(29.);
        assert!(quiet[3] < 0.15 && quiet[4] == 0.);
        assert!(wave[3] > quiet[3] * 4. && wave[4] > 0.9);
        assert!(dispersing[3] < wave[3] && dispersing[4] > 0. && dispersing[4] < wave[4]);
        assert!(
            dispersing[2] > wave[2],
            "individual variation returns while waves disperse"
        );
        assert!(dark[3] < 0.01 && dark[4] == 0.);
        assert!(returning[3] > dark[3] * 4. && returning[3] < quiet[3]);
    }

    #[test]
    fn rhythm_loops_continuously_without_jumps_at_stage_boundaries() {
        for time in [0., 8., 12., 16., 18., 20.5, 21.5, 22., 27., 30.] {
            let before = sample(time - 0.0001);
            let after = sample(time + 0.0001);
            for i in 0..before.len() {
                assert!((before[i] - after[i]).abs() < 0.001, "jump at {time}");
                assert!((sample(time)[i] - sample(time + 30.)[i]).abs() < 0.00001);
            }
        }
    }
}
