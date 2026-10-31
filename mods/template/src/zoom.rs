//! Zoom state machine, ported from Zoomify's `Interpolator.kt`, `ZoomHelper.kt` and
//! `DefaultZoomHelpers.kt`.
//!
//! Zoomify ticks at 20 Hz and lerps between ticks. Cinnabar calls `frame` once per rendered
//! frame with the elapsed seconds, so here every frame is one step of that many seconds and
//! no tick lerp is needed.

use crate::transition::Transition;

/// Cinnabar's camera API limits the FOV multiplier to 0.1..=1, so zoom is 1x..=10x.
pub const MIN_DIVISOR: f64 = 1.0;
pub const MAX_DIVISOR: f64 = 10.0;
/// Zoomify's scroll smoothing moves a fixed fraction of the remaining distance per 50 ms tick.
const TICK_SECONDS: f64 = 0.05;

#[derive(Clone, Copy, Debug)]
pub struct ZoomParams {
    /// Zoom divisor reached once the key is held (`initialZoom`).
    pub initial_zoom: f64,
    pub transition_in: Transition,
    pub transition_out: Transition,
    pub time_in: f64,
    pub time_out: f64,
    /// Percent; 150 means every step multiplies the zoom by 1.5.
    pub zoom_per_step: f64,
    pub max_steps: u32,
    /// Fraction of the remaining step distance covered per 50 ms; 1.0 is instant.
    pub step_smoothness: f64,
}

#[derive(Debug)]
struct ZoomHelper {
    initial: f64,
    active: Transition,
    prev_target: f64,
    zooming_last_tick: bool,
    scroll: f64,
    last_step: u32,
    resetting: bool,
    reset_multiplier: f64,
}

impl ZoomHelper {
    fn new() -> Self {
        Self {
            initial: 0.0,
            active: Transition::Linear,
            prev_target: 0.0,
            zooming_last_tick: false,
            scroll: 0.0,
            last_step: 0,
            resetting: false,
            reset_multiplier: 1.0,
        }
    }

    fn tick(&mut self, zooming: bool, steps: u32, dt: f64, p: &ZoomParams) {
        // Initial zoom (TransitionInterpolator).
        if zooming && !self.zooming_last_tick {
            self.resetting = false;
        }
        let target = if zooming { 1.0 } else { 0.0 };
        self.initial = self.tick_transition(target, self.initial, dt, p);
        self.zooming_last_tick = zooming;

        // Step zoom (SmoothInterpolator).
        if steps > self.last_step {
            self.resetting = false;
        }
        let target = if p.max_steps > 0 {
            f64::from(steps) / f64::from(p.max_steps)
        } else {
            0.0
        };
        self.scroll = if p.step_smoothness >= 1.0 {
            target
        } else {
            let k = 1.0 - (1.0 - p.step_smoothness).powf(dt / TICK_SECONDS);
            let next = self.scroll + (target - self.scroll) * k;
            if (target - next).abs() < 1e-4 { target } else { next }
        };
        self.last_step = steps;
    }

    fn tick_transition(&mut self, target: f64, mut current: f64, dt: f64, p: &ZoomParams) -> f64 {
        let t_in = p.transition_in;
        let t_out = p.transition_out.opposite();
        if target > current {
            self.active = t_in;
            // The target just flipped upwards: keep the visible zoom continuous when the
            // curve changes, as Zoomify does.
            if self.prev_target < target {
                if let Some(t) = t_in.inverse(t_out.apply(current)) {
                    current = t;
                }
            }
        } else if target < current {
            self.active = t_out;
            if self.prev_target > target {
                if let Some(t) = t_out.inverse(t_in.apply(current)) {
                    current = t;
                }
            }
        }
        self.prev_target = target;

        if self.active == Transition::Instant {
            return target;
        }
        if target > current {
            if p.time_in <= 0.0 {
                target
            } else {
                (current + dt / p.time_in).min(target)
            }
        } else if target < current {
            if p.time_out <= 0.0 {
                target
            } else {
                (current - dt / p.time_out).max(target)
            }
        } else {
            target
        }
    }

    /// Zoom divisor for this frame, clamped to what Cinnabar can display.
    fn divisor(&mut self, p: &ZoomParams) -> f64 {
        let eased = self.active.apply(self.initial).clamp(0.0, 1.0);
        let target_multiplier = if self.resetting {
            self.reset_multiplier
        } else {
            1.0 / p.initial_zoom.max(1.0)
        };
        let initial_multiplier = 1.0 + eased * (target_multiplier - 1.0);
        let base_divisor = 1.0 / initial_multiplier;

        let scroll = if self.resetting { 0.0 } else { self.scroll };
        let current_step = scroll * f64::from(p.max_steps);
        let step_multiplier = p.zoom_per_step / 100.0;
        let divisor = (base_divisor * step_multiplier.powf(current_step))
            .clamp(MIN_DIVISOR, MAX_DIVISOR);

        if self.initial == 0.0 && self.scroll == 0.0 {
            self.resetting = false;
        }
        if !self.resetting {
            self.reset_multiplier = 1.0 / divisor;
        }
        divisor
    }

    /// Releasing the key drops the step zoom at once and animates the whole zoom out
    /// from where it currently is.
    fn reset(&mut self) {
        if !self.resetting && self.scroll > 0.0 {
            self.resetting = true;
            self.scroll = 0.0;
        }
    }
}

/// What the camera should do this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    /// Total zoom divisor (1.0 = no zoom).
    pub divisor: f64,
    /// FOV multiplier for `camera.set-view-scale`.
    pub fov_scale: f32,
    /// Look-sensitivity multiplier for `camera.set-view-scale`.
    pub look_scale: f32,
}

impl View {
    pub fn is_neutral(&self) -> bool {
        self.fov_scale >= 0.9995 && self.look_scale >= 0.9995
    }
}

pub struct FrameInput {
    pub zooming: bool,
    pub secondary_zooming: bool,
    /// Zoom-in presses minus zoom-out presses this frame.
    pub step_delta: i32,
    pub seconds: f64,
    pub retain_steps: bool,
    /// `relativeSensitivity` as a fraction, 0.0..=1.0.
    pub relative_sensitivity: f64,
}

pub struct Zoomer {
    primary: ZoomHelper,
    secondary: ZoomHelper,
    steps: u32,
}

impl Default for Zoomer {
    fn default() -> Self {
        Self::new()
    }
}

impl Zoomer {
    pub fn new() -> Self {
        Self {
            primary: ZoomHelper::new(),
            secondary: ZoomHelper::new(),
            steps: 0,
        }
    }

    #[cfg(test)]
    pub fn steps(&self) -> u32 {
        self.steps
    }

    pub fn update(
        &mut self,
        input: &FrameInput,
        primary: &ZoomParams,
        secondary: &ZoomParams,
    ) -> View {
        let max = i64::from(primary.max_steps);
        self.steps = (i64::from(self.steps) + i64::from(input.step_delta)).clamp(0, max) as u32;

        self.primary.tick(input.zooming, self.steps, input.seconds, primary);
        self.secondary
            .tick(input.secondary_zooming, 0, input.seconds, secondary);

        if !input.zooming {
            if !input.retain_steps {
                self.steps = 0;
            }
            self.primary.reset();
        }

        let primary_divisor = self.primary.divisor(primary);
        let secondary_divisor = self.secondary.divisor(secondary);
        let divisor = (primary_divisor * secondary_divisor).clamp(MIN_DIVISOR, MAX_DIVISOR);

        // Mouse sensitivity divided by lerp(relativeSensitivity, 1, divisor), as in MouseMixin.
        let look = 1.0 / (1.0 + input.relative_sensitivity.clamp(0.0, 1.0) * (primary_divisor - 1.0));
        View {
            divisor,
            fov_scale: (1.0 / divisor).clamp(0.1, 1.0) as f32,
            look_scale: look.clamp(0.05, 1.0) as f32,
        }
    }
}

/// The parameters Zoomify's `SecondaryZoomHelper` uses: a plain timed toggle, no steps.
pub fn secondary_params(amount: f64, time_in: f64, time_out: f64) -> ZoomParams {
    ZoomParams {
        initial_zoom: amount,
        transition_in: Transition::Linear,
        transition_out: Transition::Linear,
        time_in,
        time_out,
        zoom_per_step: 100.0,
        max_steps: 0,
        step_smoothness: 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> ZoomParams {
        ZoomParams {
            initial_zoom: 4.0,
            transition_in: Transition::EaseOutExp,
            transition_out: Transition::EaseOutExp,
            time_in: 1.0,
            time_out: 0.5,
            zoom_per_step: 150.0,
            max_steps: 10,
            step_smoothness: 0.37,
        }
    }

    fn input(zooming: bool, step_delta: i32) -> FrameInput {
        FrameInput {
            zooming,
            secondary_zooming: false,
            step_delta,
            seconds: 1.0 / 60.0,
            retain_steps: false,
            relative_sensitivity: 1.0,
        }
    }

    fn run(z: &mut Zoomer, frames: usize, zooming: bool) -> View {
        let sec = secondary_params(4.0, 10.0, 1.0);
        let mut view = None;
        for _ in 0..frames {
            view = Some(z.update(&input(zooming, 0), &params(), &sec));
        }
        view.unwrap()
    }

    #[test]
    fn idle_is_neutral() {
        let mut z = Zoomer::new();
        assert!(run(&mut z, 10, false).is_neutral());
    }

    #[test]
    fn zooms_in_to_initial_zoom_then_out() {
        let mut z = Zoomer::new();
        let v = run(&mut z, 60 * 2, true);
        assert!((v.divisor - 4.0).abs() < 1e-6, "got {}", v.divisor);
        assert!((v.fov_scale - 0.25).abs() < 1e-6);
        assert!((v.look_scale - 0.25).abs() < 1e-6);
        let v = run(&mut z, 60, false);
        assert!(v.is_neutral(), "got {v:?}");
    }

    #[test]
    fn zoom_is_monotonic_while_zooming_in() {
        let mut z = Zoomer::new();
        let sec = secondary_params(4.0, 10.0, 1.0);
        let mut prev = 1.0;
        for _ in 0..120 {
            let v = z.update(&input(true, 0), &params(), &sec);
            assert!(v.divisor + 1e-9 >= prev);
            prev = v.divisor;
        }
    }

    #[test]
    fn reversing_mid_zoom_keeps_the_visible_zoom_continuous() {
        // Reversing swaps curves; Zoomify remaps progress so the eased value is unchanged.
        let p = params();
        for (zoom_in, zoom_out) in [
            (Transition::EaseOutExp, Transition::EaseOutExp),
            (Transition::EaseOutSine, Transition::EaseOutQuad),
            (Transition::EaseOutCubic, Transition::EaseOutSine),
        ] {
            let p = ZoomParams { transition_in: zoom_in, transition_out: zoom_out, ..p };
            let mut helper = ZoomHelper::new();
            for _ in 0..15 {
                helper.tick(true, 0, 1.0 / 60.0, &p);
            }
            let before = helper.active.apply(helper.initial);
            assert!(before > 0.05 && before < 0.95, "{zoom_in:?}: {before}");
            // A zero-length step changes direction without moving.
            helper.tick(false, 0, 0.0, &p);
            let after = helper.active.apply(helper.initial);
            assert!((after - before).abs() < 1e-6, "{zoom_in:?}/{zoom_out:?}: {before} -> {after}");
        }
    }

    #[test]
    fn zoom_out_after_partial_zoom_in_is_monotonic() {
        let mut z = Zoomer::new();
        let sec = secondary_params(4.0, 10.0, 1.0);
        for _ in 0..15 {
            z.update(&input(true, 0), &params(), &sec);
        }
        let mut prev = f64::MAX;
        for _ in 0..120 {
            let v = z.update(&input(false, 0), &params(), &sec);
            assert!(v.divisor <= prev + 1e-9);
            prev = v.divisor;
        }
        assert_eq!(prev, 1.0);
    }

    #[test]
    fn steps_zoom_further_and_are_clamped() {
        let mut z = Zoomer::new();
        let sec = secondary_params(4.0, 10.0, 1.0);
        run(&mut z, 120, true);
        for _ in 0..3 {
            z.update(&input(true, 1), &params(), &sec);
        }
        let v = run(&mut z, 240, true);
        assert!(v.divisor > 9.99 && v.divisor <= MAX_DIVISOR + 1e-9, "got {}", v.divisor);
        assert_eq!(z.steps(), 3);
        for _ in 0..50 {
            z.update(&input(true, 1), &params(), &sec);
        }
        assert_eq!(z.steps(), 10);
    }

    #[test]
    fn releasing_after_steps_animates_out_from_current_zoom() {
        let mut z = Zoomer::new();
        let sec = secondary_params(4.0, 10.0, 1.0);
        run(&mut z, 120, true);
        for _ in 0..2 {
            z.update(&input(true, 1), &params(), &sec);
        }
        let zoomed = run(&mut z, 240, true).divisor;
        assert!(zoomed > 8.0);
        let first = z.update(&input(false, 0), &params(), &sec);
        // It starts from the zoomed-in view and eases out; it does not jump to 1x.
        assert!(first.divisor < zoomed && first.divisor > 2.0, "went to {}", first.divisor);
        let mut prev = first.divisor;
        for _ in 0..120 {
            let v = z.update(&input(false, 0), &params(), &sec);
            assert!(v.divisor <= prev + 1e-9);
            prev = v.divisor;
        }
        assert_eq!(prev, 1.0);
        assert_eq!(z.steps(), 0);
    }

    #[test]
    fn instant_transitions_snap() {
        let mut p = params();
        p.transition_in = Transition::Instant;
        p.transition_out = Transition::Instant;
        let sec = secondary_params(4.0, 10.0, 1.0);
        let mut z = Zoomer::new();
        let v = z.update(&input(true, 0), &p, &sec);
        assert!((v.divisor - 4.0).abs() < 1e-9);
        let v = z.update(&input(false, 0), &p, &sec);
        assert!(v.is_neutral());
    }

    #[test]
    fn zero_time_does_not_divide_by_zero() {
        let mut p = params();
        p.time_in = 0.0;
        p.time_out = 0.0;
        let sec = secondary_params(4.0, 10.0, 1.0);
        let mut z = Zoomer::new();
        let v = z.update(&input(true, 0), &p, &sec);
        assert!(v.divisor.is_finite() && (v.divisor - 4.0).abs() < 1e-9);
    }

    #[test]
    fn secondary_zoom_multiplies_and_total_is_capped() {
        let sec = secondary_params(4.0, 10.0, 1.0);
        let mut z = Zoomer::new();
        let mut frame = input(true, 0);
        frame.secondary_zooming = true;
        let mut v = z.update(&frame, &params(), &sec);
        for _ in 0..60 * 12 {
            v = z.update(&frame, &params(), &sec);
        }
        assert!((v.divisor - MAX_DIVISOR).abs() < 1e-9, "got {}", v.divisor);
        assert!((v.fov_scale - 0.1).abs() < 1e-6);
    }

    #[test]
    fn relative_sensitivity_zero_keeps_look_unscaled() {
        let sec = secondary_params(4.0, 10.0, 1.0);
        let mut z = Zoomer::new();
        let mut frame = input(true, 0);
        frame.relative_sensitivity = 0.0;
        let mut v = z.update(&frame, &params(), &sec);
        for _ in 0..200 {
            v = z.update(&frame, &params(), &sec);
        }
        assert_eq!(v.look_scale, 1.0);
        assert!(v.fov_scale < 1.0);
    }
}
