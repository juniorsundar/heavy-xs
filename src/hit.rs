//! `hit_test` from `src/api/system.c`, as pure arithmetic so it can be compared
//! against the C function over a grid (see `scripts/rust-port/hit-test-check.sh`).
//! No dependencies: this file also compiles on its own with `rustc --test`.
//!
//! The codes are `SDL_HitTestResult` values; `syswindow.rs` asserts that at compile time.

pub const NORMAL: i32 = 0;
pub const DRAGGABLE: i32 = 1;
pub const RESIZE_TOPLEFT: i32 = 2;
pub const RESIZE_TOPRIGHT: i32 = 4;
pub const RESIZE_BOTTOMRIGHT: i32 = 6;
pub const RESIZE_BOTTOM: i32 = 7;
pub const RESIZE_BOTTOMLEFT: i32 = 8;
pub const RESIZE_LEFT: i32 = 9;

/// `RESIZE_FROM_TOP` and `RESIZE_FROM_RIGHT` are both 0 in the C, so the top and
/// right edge arms (`RESIZE_TOP`, `RESIZE_RIGHT`) are compiled out there and omitted here.
pub fn classify(x: i32, y: i32, w: i32, h: i32, title_height: i32, controls_width: i32, resize_border: i32) -> i32 {
    let rb = resize_border;
    if y < title_height && x > rb && x < w - controls_width {
        DRAGGABLE
    } else if x < rb && y < rb {
        RESIZE_TOPLEFT
    } else if x > w - rb && y < rb {
        RESIZE_TOPRIGHT
    } else if x > w - rb && y > h - rb {
        RESIZE_BOTTOMRIGHT
    } else if x < w - rb && x > rb && y > h - rb {
        RESIZE_BOTTOM
    } else if x < rb && y > h - rb {
        RESIZE_BOTTOMLEFT
    } else if x < rb && y < h - rb && y > rb {
        RESIZE_LEFT
    } else {
        NORMAL
    }
}

#[cfg(test)]
mod grid {
    use super::classify;

    /// Prints `x y code` over the same grid as the C harness; the two outputs are diffed.
    #[test]
    fn print_grid() {
        for &(w, h) in &[(400, 300), (50, 40), (3, 3), (1, 1), (1000, 20)] {
            for &(t, c, b) in &[(0, 0, 0), (30, 90, 6), (30, 0, 6), (1000, 5, 5), (10, 10, 1), (30, 90, 40), (-5, -5, -5), (0, 0, 100)] {
                for y in -3..h + 4 {
                    for x in -3..w + 4 {
                        println!("{w} {h} {t} {c} {b} {x} {y} {}", classify(x, y, w, h, t, c, b));
                    }
                }
            }
        }
    }
}
