//! Deterministic multi-click counting, ported from SDL 3.4.14 `src/events/SDL_mouse.c`.
//! `press` mirrors the `down` branch of `SDL_PrivateSendMouseButton` (timeout and
//! per-axis radius check, then saturating `++`); `release` mirrors the same function
//! with `down == false`, where SDL only reads `clickstate->click_count` and leaves the
//! state untouched; per-`source` state mirrors SDL's per-`SDL_MouseInputSource`
//! clickstates, so touch / pen / mouse ids count independently.

use std::collections::HashMap;

/// SDL's Linux default (`SDL_MOUSE_DOUBLE_CLICK_TIME` unset).
pub const DOUBLE_CLICK_TIME_MS: u64 = 500;
/// The editor sets `SDL_HINT_MOUSE_DOUBLE_CLICK_RADIUS=4`; SDL compares the hint
/// directly against `fabs(dx)` / `fabs(dy)`, it is not squared.
pub const DOUBLE_CLICK_RADIUS: f32 = 4.0;

const MAX_CLICKS: u8 = 255;

#[derive(Default)]
struct ClickState {
    last_ms: u64,
    x: f32,
    y: f32,
    count: u8,
}

#[derive(Default)]
pub struct Clicks {
    states: HashMap<(u32, u8), ClickState>,
}

impl Clicks {
    /// Clicks for this press, as SDL would report in the button-down event.
    pub fn press(&mut self, now_ms: u64, source: u32, button: u8, x: f32, y: f32) -> u8 {
        let s = self.states.entry((source, button)).or_default();
        if now_ms >= s.last_ms.saturating_add(DOUBLE_CLICK_TIME_MS)
            || (x - s.x).abs() > DOUBLE_CLICK_RADIUS
            || (y - s.y).abs() > DOUBLE_CLICK_RADIUS
        {
            s.count = 0;
        }
        s.last_ms = now_ms;
        s.x = x;
        s.y = y;
        if s.count < MAX_CLICKS {
            s.count += 1;
        }
        s.count
    }

    /// Clicks for the button-up event. SDL does not update the click state here,
    /// so this is the count of the matching press; 0 means SDL would have ignored
    /// the release (no press was tracked for this source/button).
    pub fn release(&mut self, source: u32, button: u8) -> u8 {
        self.states.get(&(source, button)).map_or(0, |s| s.count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOUSE: u32 = 0;

    #[test]
    fn single_press_is_one() {
        let mut c = Clicks::default();
        assert_eq!(c.press(0, MOUSE, 1, 10.0, 10.0), 1);
    }

    #[test]
    fn fast_presses_in_radius_count_up() {
        let mut c = Clicks::default();
        assert_eq!(c.press(1000, MOUSE, 1, 10.0, 10.0), 1);
        assert_eq!(c.release(MOUSE, 1), 1);
        assert_eq!(c.press(1100, MOUSE, 1, 11.0, 10.0), 2);
        assert_eq!(c.release(MOUSE, 1), 2);
        assert_eq!(c.press(1200, MOUSE, 1, 14.0, 14.0), 3);
        assert_eq!(c.release(MOUSE, 1), 3);
    }

    #[test]
    fn slow_press_resets() {
        let mut c = Clicks::default();
        assert_eq!(c.press(1000, MOUSE, 1, 10.0, 10.0), 1);
        // SDL uses `now >= last + 500`, so exactly 500 ms elapsed already resets.
        assert_eq!(c.press(1500, MOUSE, 1, 10.0, 10.0), 1);
        assert_eq!(c.press(1510, MOUSE, 1, 10.0, 10.0), 2);
    }

    #[test]
    fn outside_radius_resets() {
        let mut c = Clicks::default();
        assert_eq!(c.press(1000, MOUSE, 1, 10.0, 10.0), 1);
        // 4 px is inside (SDL uses `>`), 4.5 px is outside on either axis.
        assert_eq!(c.press(1100, MOUSE, 1, 14.0, 6.0), 2);
        assert_eq!(c.press(1200, MOUSE, 1, 18.5, 6.0), 1);
        assert_eq!(c.press(1300, MOUSE, 1, 18.5, 10.5), 1);
    }

    #[test]
    fn buttons_count_independently() {
        let mut c = Clicks::default();
        assert_eq!(c.press(1000, MOUSE, 1, 10.0, 10.0), 1);
        assert_eq!(c.press(1050, MOUSE, 3, 10.0, 10.0), 1);
        assert_eq!(c.press(1100, MOUSE, 1, 10.0, 10.0), 2);
        assert_eq!(c.release(MOUSE, 1), 2);
        assert_eq!(c.release(MOUSE, 3), 1);
    }

    #[test]
    fn sources_count_independently() {
        let mut c = Clicks::default();
        assert_eq!(c.press(1000, MOUSE, 1, 10.0, 10.0), 1);
        assert_eq!(c.press(1050, MOUSE + 1, 1, 10.0, 10.0), 1);
        assert_eq!(c.press(1100, MOUSE, 1, 10.0, 10.0), 2);
    }

    #[test]
    fn release_without_press_is_zero() {
        let mut c = Clicks::default();
        assert_eq!(c.release(MOUSE, 1), 0);
        c.press(0, MOUSE, 1, 0.0, 0.0);
        assert_eq!(c.release(MOUSE, 2), 0);
    }

    #[test]
    fn saturates_at_255() {
        let mut c = Clicks::default();
        for i in 0..300u64 {
            assert_eq!(c.press(i, MOUSE, 1, 0.0, 0.0), (i + 1).min(255) as u8);
        }
        assert_eq!(c.press(400, MOUSE, 1, 0.0, 0.0), 255);
        assert_eq!(c.release(MOUSE, 1), 255);
        // A slow press still resets from saturation.
        assert_eq!(c.press(2000, MOUSE, 1, 0.0, 0.0), 1);
    }
}
