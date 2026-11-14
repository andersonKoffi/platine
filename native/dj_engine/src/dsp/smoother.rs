//! Linear parameter ramps that remove zipper noise and clicks.

/// A value that glides linearly towards its target over a fixed number of frames.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Smoother {
    current: f32,
    target: f32,
    step: f32,
    remaining: u32,
    ramp_frames: u32,
}

impl Smoother {
    pub fn new(value: f32, ramp_frames: u32) -> Self {
        Self {
            current: value,
            target: value,
            step: 0.0,
            remaining: 0,
            ramp_frames: ramp_frames.max(1),
        }
    }

    /// Starts a ramp from the current value to `target`.
    pub fn set_target(&mut self, target: f32) {
        if (target - self.target).abs() <= f32::EPSILON && self.remaining == 0 {
            return;
        }
        self.target = target;
        self.remaining = self.ramp_frames;
        self.step = (target - self.current) / self.ramp_frames as f32;
    }

    /// Jumps to `value` without ramping.
    pub fn set_immediate(&mut self, value: f32) {
        self.current = value;
        self.target = value;
        self.remaining = 0;
        self.step = 0.0;
    }

    /// Advances by one frame and returns the new value.
    #[inline]
    pub fn next_value(&mut self) -> f32 {
        if self.remaining > 0 {
            self.remaining -= 1;
            self.current = if self.remaining == 0 {
                self.target
            } else {
                self.current + self.step
            };
        }
        self.current
    }

    pub fn current(&self) -> f32 {
        self.current
    }

    pub fn target(&self) -> f32 {
        self.target
    }

    pub fn is_settled(&self) -> bool {
        self.remaining == 0
    }
}

#[cfg(test)]
mod tests {
    use super::Smoother;

    #[test]
    fn ramps_linearly_to_the_target() {
        let mut s = Smoother::new(0.0, 4);
        s.set_target(1.0);
        let values: Vec<f32> = (0..6).map(|_| s.next_value()).collect();
        assert_eq!(values, vec![0.25, 0.5, 0.75, 1.0, 1.0, 1.0]);
        assert!(s.is_settled());
        assert_eq!(s.target(), 1.0);
    }

    #[test]
    fn retargeting_mid_ramp_starts_from_the_current_value() {
        let mut s = Smoother::new(0.0, 4);
        s.set_target(1.0);
        s.next_value();
        s.next_value();
        s.set_target(0.0);
        assert_eq!(s.next_value(), 0.375);
    }

    #[test]
    fn immediate_set_skips_the_ramp() {
        let mut s = Smoother::new(0.0, 100);
        s.set_target(1.0);
        s.set_immediate(0.5);
        assert_eq!(s.next_value(), 0.5);
        assert_eq!(s.current(), 0.5);
    }

    #[test]
    fn setting_the_same_target_does_not_restart_the_ramp() {
        let mut s = Smoother::new(1.0, 10);
        s.set_target(1.0);
        assert!(s.is_settled());
        let zero_frames = Smoother::new(0.0, 0);
        assert_eq!(zero_frames.ramp_frames, 1);
    }
}
