const FULL_SCALE: f32 = 32768.0;

pub fn peak(samples: &[i16]) -> f32 {
    samples
        .iter()
        .map(|s| (*s as f32).abs() / FULL_SCALE)
        .fold(0.0, f32::max)
}

pub fn rms(samples: &[i16]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();
    (sum_sq / samples.len() as f64).sqrt() as f32 / FULL_SCALE
}

pub fn to_db(level: f32) -> f32 {
    if level <= 1e-6 {
        -90.0
    } else {
        20.0 * level.log10()
    }
}

pub struct SmoothedMeter {
    level: f32,
    release_per_frame: f32,
}

impl SmoothedMeter {
    pub fn new(release_per_frame: f32) -> Self {
        Self {
            level: 0.0,
            release_per_frame,
        }
    }

    pub fn update(&mut self, target: f32) -> f32 {
        if target >= self.level {
            self.level = target;
        } else {
            self.level = (self.level - self.release_per_frame).max(target);
        }
        self.level
    }

    pub fn level(&self) -> f32 {
        self.level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peak_of_silence_is_zero() {
        assert_eq!(peak(&[0, 0, 0]), 0.0);
    }

    #[test]
    fn peak_is_absolute_max_normalized() {
        let samples = [0, 1000, -2000, 500];
        let expected = 2000.0 / 32768.0;
        assert!((peak(&samples) - expected).abs() < 1e-6);
    }

    #[test]
    fn peak_of_full_scale_is_one() {
        assert!((peak(&[i16::MIN, i16::MAX]) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn rms_of_empty_is_zero() {
        assert_eq!(rms(&[]), 0.0);
    }

    #[test]
    fn rms_of_full_scale_constant_is_one() {
        let samples = [i16::MAX; 64];
        assert!((rms(&samples) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn rms_of_silence_is_zero() {
        assert_eq!(rms(&[0; 32]), 0.0);
    }

    #[test]
    fn rms_halves_for_quarter_amplitude() {
        let samples = [i16::MAX / 4; 32];
        assert!((rms(&samples) - 0.25).abs() < 1e-3);
    }

    #[test]
    fn to_db_of_full_scale_is_zero() {
        assert!(to_db(1.0).abs() < 1e-6);
    }

    #[test]
    fn to_db_of_half_is_about_minus_six() {
        assert!((to_db(0.5) - (-6.0206)).abs() < 1e-3);
    }

    #[test]
    fn to_db_of_silence_is_floor() {
        assert_eq!(to_db(0.0), -90.0);
    }

    #[test]
    fn smoothed_meter_attacks_immediately() {
        let mut meter = SmoothedMeter::new(0.1);
        assert_eq!(meter.update(0.5), 0.5);
    }

    #[test]
    fn smoothed_meter_releases_gradually() {
        let mut meter = SmoothedMeter::new(0.1);
        meter.update(0.5);
        let released = meter.update(0.0);
        assert!((released - 0.4).abs() < 1e-6);
    }

    #[test]
    fn smoothed_meter_release_never_goes_below_target() {
        let mut meter = SmoothedMeter::new(0.1);
        meter.update(0.5);
        let released = meter.update(0.45);
        assert!((released - 0.45).abs() < 1e-6);
    }

    #[test]
    fn smoothed_meter_level_persists() {
        let mut meter = SmoothedMeter::new(0.1);
        meter.update(0.7);
        assert_eq!(meter.level(), 0.7);
    }
}
