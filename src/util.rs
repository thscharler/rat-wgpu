use std::cmp::{max, min};

#[inline]
pub(crate) fn clip_uv(a: (i32, i32, u32, u32), clip: (i32, i32, u32, u32)) -> Option<[f32; 4]> {
    let Some(c) = intersect(a, clip) else {
        return None;
    };

    let a0 = (a.0, a.1);

    let c0 = (c.0, c.1);
    let c1 = (c.0 + c.2 as i32, c.1 + c.3 as i32);

    let a_width = a.2 as f32;
    let a_height = a.3 as f32;

    Some([
        (c0.0 - a0.0) as f32 / a_width,
        (c0.1 - a0.1) as f32 / a_height,
        (c1.0 - a0.0) as f32 / a_width,
        (c1.1 - a0.1) as f32 / a_height,
    ])
}

#[inline]
pub(crate) fn intersect(
    a: (i32, i32, u32, u32),
    b: (i32, i32, u32, u32),
) -> Option<(i32, i32, u32, u32)> {
    let a0 = (a.0, a.1);
    let a1 = (a.0 + a.2 as i32, a.1 + a.3 as i32);

    let b0 = (b.0, b.1);
    let b1 = (b.0 + b.2 as i32, b.1 + b.3 as i32);

    let c0 = (max(a0.0, b0.0), max(a0.1, b0.1));
    let c1 = (min(a1.0, b1.0), min(a1.1, b1.1));

    if c0.0 > c1.0 || c0.1 > c1.1 {
        None
    } else {
        let w = c1.0 - c0.0;
        let h = c1.1 - c0.1;
        debug_assert!(w >= 0);
        debug_assert!(h >= 0);
        Some((c0.0, c0.1, w as u32, h as u32))
    }
}
