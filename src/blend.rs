use std::usize;

use image::Rgba;

// Rust port of Aseprite's blend functions:
// https://github.com/aseprite/aseprite/blob/master/src/doc/blend_funcs.cpp
//
// Further references:
//  - http://www.simplefilter.de/en/basics/mixmods.html
//  - PDF Blend Modes addendum: https://www.adobe.com/content/dam/acom/en/devnet/pdf/pdf_reference_archive/blend_modes.pdf
//  - Pixman source: https://github.com/servo/pixman/blob/master/pixman/pixman-combine-float.c

pub type Color8 = Rgba<u8>;

#[allow(dead_code)]
pub(crate) fn merge(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    let [back_r, back_g, back_b, back_a] = backdrop.0;
    let [src_r, src_g, src_b, src_a] = src.0;
    let res_r;
    let res_g;
    let res_b;
    let res_a;

    if back_a == 0 {
        res_r = src_r;
        res_g = src_g;
        res_b = src_b;
    } else if src_a == 0 {
        res_r = back_r;
        res_g = back_g;
        res_b = back_b;
    } else {
        res_r = blend8(back_r, src_r, opacity);
        res_g = blend8(back_g, src_g, opacity);
        res_b = blend8(back_b, src_b, opacity);
    }
    res_a = blend8(back_a, src_a, opacity);
    if res_a == 0 {
        Rgba([0, 0, 0, 0])
    } else {
        Rgba([res_r, res_g, res_b, res_a])
    }
}

// based on: rgba_blender_normal(color_t backdrop, color_t src, int opacity)
pub(crate) fn normal(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    let (back_r, back_g, back_b, back_a) = as_rgba_i32(backdrop);
    let (src_r, src_g, src_b, src_a) = as_rgba_i32(src);

    if back_a == 0 {
        //todo!("NYI: invisible background")
        let alpha = mul_un8(src_a, opacity as i32) as i32;
        return from_rgba_i32(src_r, src_g, src_b, alpha);
    } else if src_a == 0 {
        return backdrop;
    }

    let src_a = mul_un8(src_a, opacity as i32) as i32;

    let res_a = src_a + back_a - mul_un8(back_a, src_a) as i32;

    let res_r = back_r + ((src_r - back_r) * src_a) / res_a;
    let res_g = back_g + ((src_g - back_g) * src_a) / res_a;
    let res_b = back_b + ((src_b - back_b) * src_a) / res_a;

    from_rgba_i32(res_r, res_g, res_b, res_a)
}

// --- Utilities / generic functions -------------------------------------------

/*
  if (backdrop & rgba_a_mask) {                                                 \
    color_t normal = rgba_blender_normal(backdrop, src, opacity);               \
    color_t blend = rgba_blender_##name(backdrop, src, opacity);                \
    int Ba = rgba_geta(backdrop);                                               \
    color_t normalToBlendMerge = rgba_blender_merge(normal, blend, Ba);         \
    int t;                                                                      \
    int srcTotalAlpha = MUL_UN8(rgba_geta(src), opacity, t);                    \
    int compositeAlpha = MUL_UN8(Ba, srcTotalAlpha, t);                         \
    return rgba_blender_merge(normalToBlendMerge, blend, compositeAlpha);       \
  }                                                                             \
  else                                                                          \
    return rgba_blender_normal(backdrop, src, opacity);                         \
*/
fn blender<F>(backdrop: Color8, src: Color8, opacity: u8, f: F) -> Color8
where
    F: Fn(Color8, Color8, u8) -> Color8,
{
    if backdrop[3] != 0 {
        let norm = normal(backdrop, src, opacity);
        let blend = f(backdrop, src, opacity);
        let back_alpha = backdrop[3];
        let normal_to_blend_merge = merge(norm, blend, back_alpha);
        let src_total_alpha = mul_un8(src[3] as i32, opacity as i32);
        let composite_alpha = mul_un8(back_alpha as i32, src_total_alpha as i32);
        merge(normal_to_blend_merge, blend, composite_alpha)
    //todo!()
    } else {
        normal(backdrop, src, opacity)
    }
}

/*
  int t;
  int r = blend_multiply(rgba_getr(backdrop), rgba_getr(src), t);
  int g = blend_multiply(rgba_getg(backdrop), rgba_getg(src), t);
  int b = blend_multiply(rgba_getb(backdrop), rgba_getb(src), t);
  src = rgba(r, g, b, 0) | (src & rgba_a_mask);
  return rgba_blender_normal(backdrop, src, opacity);
*/
fn blend_channel<F>(backdrop: Color8, src: Color8, opacity: u8, f: F) -> Color8
where
    F: Fn(i32, i32) -> u8,
{
    let (back_r, back_g, back_b, _) = as_rgba_i32(backdrop);
    let (src_r, src_g, src_b, _) = as_rgba_i32(src);
    let r = f(back_r, src_r);
    let g = f(back_g, src_g);
    let b = f(back_b, src_b);
    let src = Rgba([r, g, b, src[3]]);
    normal(backdrop, src, opacity)
}

// --- multiply ----------------------------------------------------------------

pub(crate) fn multiply(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blender(backdrop, src, opacity, multiply_baseline)
}

fn multiply_baseline(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blend_channel(backdrop, src, opacity, blend_multiply)
}

fn blend_multiply(a: i32, b: i32) -> u8 {
    mul_un8(a, b)
}

// --- screen ------------------------------------------------------------------

pub(crate) fn screen(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blender(backdrop, src, opacity, screen_baseline)
}

fn screen_baseline(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blend_channel(backdrop, src, opacity, blend_screen)
}

// blend_screen(b, s, t)     ((b) + (s) - MUL_UN8((b), (s), (t)))
fn blend_screen(a: i32, b: i32) -> u8 {
    (a + b - mul_un8(a, b) as i32) as u8
}

// --- overlay -----------------------------------------------------------------

pub(crate) fn overlay(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blender(backdrop, src, opacity, overlay_baseline)
}

fn overlay_baseline(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blend_channel(backdrop, src, opacity, blend_overlay)
}

// blend_overlay(b, s, t)    (blend_hard_light(s, b, t))
// blend_hard_light(b, s, t) ((s) < 128 ?                          \
//    blend_multiply((b), (s)<<1, (t)):    \
//    blend_screen((b), ((s)<<1)-255, (t)))

fn blend_overlay(b: i32, s: i32) -> u8 {
    blend_hard_light(s, b)
}

// --- darken ------------------------------------------------------------------

pub(crate) fn darken(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blender(backdrop, src, opacity, darken_baseline)
}

fn darken_baseline(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blend_channel(backdrop, src, opacity, blend_darken)
}

fn blend_darken(b: i32, s: i32) -> u8 {
    b.min(s) as u8
}

// --- lighten -----------------------------------------------------------------

pub(crate) fn lighten(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blender(backdrop, src, opacity, lighten_baseline)
}

fn lighten_baseline(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blend_channel(backdrop, src, opacity, blend_lighten)
}

fn blend_lighten(b: i32, s: i32) -> u8 {
    b.max(s) as u8
}

// --- color_dodge -------------------------------------------------------------

pub(crate) fn color_dodge(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blender(backdrop, src, opacity, color_dodge_baseline)
}

fn color_dodge_baseline(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blend_channel(backdrop, src, opacity, blend_color_dodge)
}

fn blend_color_dodge(b: i32, s: i32) -> u8 {
    if b == 0 {
        return 0;
    }
    let s = 255 - s;
    if b >= s {
        255
    } else {
        // in floating point: b / (1-s)
        div_un8(b, s)
    }
}

// --- color_burn --------------------------------------------------------------

pub(crate) fn color_burn(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blender(backdrop, src, opacity, color_burn_baseline)
}

fn color_burn_baseline(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blend_channel(backdrop, src, opacity, blend_color_burn)
}

fn blend_color_burn(b: i32, s: i32) -> u8 {
    if b == 255 {
        return 255;
    }
    let b = 255 - b;
    if b >= s {
        0
    } else {
        // in floating point: 1 - ((1-b)/s)
        255 - div_un8(b, s)
    }
}

// --- hard_light --------------------------------------------------------------

pub(crate) fn hard_light(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blender(backdrop, src, opacity, hard_light_baseline)
}

fn hard_light_baseline(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blend_channel(backdrop, src, opacity, blend_hard_light)
}

fn blend_hard_light(b: i32, s: i32) -> u8 {
    if s < 128 {
        blend_multiply(b, s << 1)
    } else {
        blend_screen(b, (s << 1) - 255)
    }
}

// --- soft_light --------------------------------------------------------------

pub(crate) fn soft_light(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blender(backdrop, src, opacity, soft_light_baseline)
}

fn soft_light_baseline(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    let (back_r, back_g, back_b, _) = as_rgba_i32(backdrop);
    let (src_r, src_g, src_b, src_a) = as_rgba_i32(src);
    let r = blend_soft_light(back_r, src_r);
    let g = blend_soft_light(back_g, src_g);
    let b = blend_soft_light(back_b, src_b);

    let src = from_rgba_i32(r, g, b, src_a);

    normal(backdrop, src, opacity)
}

fn blend_soft_light(b: i32, s: i32) -> i32 {
    // The original uses double, but since inputs & output are only 8 bits using
    // f32 should actually be enough.
    let b: f64 = b as f64 / 255.0;
    let s: f64 = s as f64 / 255.0;

    let d = if b <= 0.25 {
        ((16.0 * b - 12.0) * b + 4.0) * b
    } else {
        b.sqrt()
    };

    let r = if s <= 0.5 {
        b - (1.0 - 2.0 * s) * b * (1.0 - b)
    } else {
        b + (2.0 * s - 1.0) * (d - b)
    };

    (r * 255.0 + 0.5) as u32 as i32
}

// --- divide ------------------------------------------------------------------

pub(crate) fn divide(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blender(backdrop, src, opacity, divide_baseline)
}

fn divide_baseline(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blend_channel(backdrop, src, opacity, blend_divide)
}

fn blend_divide(b: i32, s: i32) -> u8 {
    if b == 0 {
        0
    } else if b >= s {
        255
    } else {
        div_un8(b, s)
    }
}

// --- difference ------------------------------------------------------------------

pub(crate) fn difference(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blender(backdrop, src, opacity, difference_baseline)
}

fn difference_baseline(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blend_channel(backdrop, src, opacity, blend_difference)
}

fn blend_difference(b: i32, s: i32) -> u8 {
    (b - s).abs() as u8
}

// --- exclusion ---------------------------------------------------------------

pub(crate) fn exclusion(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blender(backdrop, src, opacity, exclusion_baseline)
}

fn exclusion_baseline(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blend_channel(backdrop, src, opacity, blend_exclusion)
}

// blend_exclusion(b, s, t)  ((t) = MUL_UN8((b), (s), (t)), ((b) + (s) - 2*(t)))
fn blend_exclusion(b: i32, s: i32) -> u8 {
    let t = mul_un8(b, s) as i32;
    (b + s - 2 * t) as u8
}

// --- addition ----------------------------------------------------------------

pub(crate) fn addition(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blender(backdrop, src, opacity, addition_baseline)
}

fn addition_baseline(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    let (back_r, back_g, back_b, _) = as_rgba_i32(backdrop);
    let (src_r, src_g, src_b, src_a) = as_rgba_i32(src);
    let r = back_r + src_r;
    let g = back_g + src_g;
    let b = back_b + src_b;

    let src = from_rgba_i32(r.min(255), g.min(255), b.min(255), src_a);

    normal(backdrop, src, opacity)
}

// --- subtract ----------------------------------------------------------------

pub(crate) fn subtract(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blender(backdrop, src, opacity, subtract_baseline)
}

fn subtract_baseline(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    let (back_r, back_g, back_b, _) = as_rgba_i32(backdrop);
    let (src_r, src_g, src_b, src_a) = as_rgba_i32(src);
    let r = back_r - src_r;
    let g = back_g - src_g;
    let b = back_b - src_b;

    let src = from_rgba_i32(r.max(0), g.max(0), b.max(0), src_a);

    normal(backdrop, src, opacity)
}

// --- hsl_hue -----------------------------------------------------------------

pub(crate) fn hsl_hue(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blender(backdrop, src, opacity, hsl_hue_baseline)
}

fn hsl_hue_baseline(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    let (r, g, b) = as_rgb_f64(backdrop);
    let sat = saturation(r, g, b);
    let lum = luminosity(r, g, b);

    let (r, g, b) = as_rgb_f64(src);

    let (r, g, b) = set_saturation(r, g, b, sat);
    let (r, g, b) = set_luminocity(r, g, b, lum);

    let src = from_rgb_f64(r, g, b, src[3]);

    normal(backdrop, src, opacity)
}

// --- hsl_saturation ----------------------------------------------------------

pub(crate) fn hsl_saturation(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blender(backdrop, src, opacity, hsl_saturation_baseline)
}

fn hsl_saturation_baseline(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    //dbg!(backdrop, src);
    let (r, g, b) = as_rgb_f64(src);
    //dbg!("src", (r, g, b));
    let sat = saturation(r, g, b);
    //dbg!(sat);

    let (r, g, b) = as_rgb_f64(backdrop);
    //dbg!("back", (r, g, b));
    let lum = luminosity(r, g, b);
    //dbg!(lum);

    let (r, g, b) = set_saturation(r, g, b, sat);
    //dbg!("sat", (r, g, b));
    let (r, g, b) = set_luminocity(r, g, b, lum);

    //dbg!((r, g, b), saturation(r, g, b), luminosity(r, g, b));

    let src = from_rgb_f64(r, g, b, src[3]);
    // dbg!(src);
    normal(backdrop, src, opacity)
}

// --- hsl_color ---------------------------------------------------------------

pub(crate) fn hsl_color(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blender(backdrop, src, opacity, hsl_color_baseline)
}

fn hsl_color_baseline(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    let (r, g, b) = as_rgb_f64(backdrop);
    let lum = luminosity(r, g, b);

    let (r, g, b) = as_rgb_f64(src);

    let (r, g, b) = set_luminocity(r, g, b, lum);

    let src = from_rgb_f64(r, g, b, src[3]);
    normal(backdrop, src, opacity)
}

// --- hsl_luminosity ----------------------------------------------------------

pub(crate) fn hsl_luminosity(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blender(backdrop, src, opacity, hsl_luminosity_baseline)
}

fn hsl_luminosity_baseline(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    let (r, g, b) = as_rgb_f64(src);
    let lum = luminosity(r, g, b);

    let (r, g, b) = as_rgb_f64(backdrop);

    let (r, g, b) = set_luminocity(r, g, b, lum);

    let src = from_rgb_f64(r, g, b, src[3]);

    normal(backdrop, src, opacity)
}

// --- Hue/Saturation/Luminance Utils ------------------------------------------

// this is actually chroma, but this is how the Aseprite's blend functions
// define it, which in turn come from pixman, which in turn are the
// PDF nonseperable blend modes which are specified in the "PDF Blend Modes:
// Addendum" by Adobe.
fn saturation(r: f64, g: f64, b: f64) -> f64 {
    r.max(g.max(b)) - r.min(g.min(b))
}

fn luminosity(r: f64, g: f64, b: f64) -> f64 {
    0.3 * r + 0.59 * g + 0.11 * b
}

fn set_luminocity(r: f64, g: f64, b: f64, lum: f64) -> (f64, f64, f64) {
    let delta = lum - luminosity(r, g, b);
    clip_color(r + delta, g + delta, b + delta)
}

fn clip_color(mut r: f64, mut g: f64, mut b: f64) -> (f64, f64, f64) {
    let l = luminosity(r, g, b);
    let n = r.min(g.min(b));
    let x = r.max(g.max(b));

    if n < 0.0 {
        r = l + (((r - l) * l) / (l - n));
        g = l + (((g - l) * l) / (l - n));
        b = l + (((b - l) * l) / (l - n));
    }

    if x > 1.0 {
        r = l + (((r - l) * (1.0 - l)) / (x - l));
        g = l + (((g - l) * (1.0 - l)) / (x - l));
        b = l + (((b - l) * (1.0 - l)) / (x - l));
    }
    (r, g, b)
}

// Returns (smallest, middle, highest) where smallest is the index
// of the smallest element. I.e., 0 if it's `r`, 1 if it's `g`, etc.
//
// Implements this static sorting network. Vertical lines are swaps.
//
//  r --*--*----- min
//      |  |
//  g --*--|--*-- mid
//         |  |
//  b -----*--*-- max
//
fn static_sort3(r: f64, g: f64, b: f64) -> (usize, usize, usize) {
    let (min0, mid0, max0) = ((r, 0), (g, 1), (b, 2));
    // dbg!("--------");
    // dbg!(min0, mid0, max0);
    let (min1, mid1) = if min0.0 < mid0.0 {
        (min0, mid0)
    } else {
        (mid0, min0)
    };
    // dbg!(min1, mid1);
    let (min2, max1) = if min1.0 < max0.0 {
        (min1, max0)
    } else {
        (max0, min1)
    };
    // dbg!(min2, max1);
    let (mid2, max2) = if mid1.0 < max1.0 {
        (mid1, max1)
    } else {
        (max1, mid1)
    };
    // dbg!(mid2, max2);
    (min2.1, mid2.1, max2.1)
}

// Array based implementation as a reference for testing.
#[cfg(test)]
fn static_sort3_spec(r: f64, g: f64, b: f64) -> (usize, usize, usize) {
    let mut inp = [(r, 0), (g, 1), (b, 2)];
    inp.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let res: Vec<usize> = inp.iter().map(|p| p.1).collect();
    //dbg!(r, g, b);
    (res[0], res[1], res[2])
}

#[test]
fn test_static_sort3() {
    let (r, g, b) = (2.0, 3.0, 4.0);
    assert_eq!(static_sort3(r, g, b), static_sort3_spec(r, g, b));
    let (r, g, b) = (2.0, 4.0, 3.0);
    assert_eq!(static_sort3(r, g, b), static_sort3_spec(r, g, b));
    let (r, g, b) = (3.0, 2.0, 4.0);
    assert_eq!(static_sort3(r, g, b), static_sort3_spec(r, g, b));
    let (r, g, b) = (3.0, 4.0, 2.0);
    assert_eq!(static_sort3(r, g, b), static_sort3_spec(r, g, b));
    let (r, g, b) = (4.0, 2.0, 3.0);
    assert_eq!(static_sort3(r, g, b), static_sort3_spec(r, g, b));
    let (r, g, b) = (4.0, 3.0, 2.0);
    assert_eq!(static_sort3(r, g, b), static_sort3_spec(r, g, b));
}

// implementation used in Aseprite, even though it uses a lot of compares and
// is actually broken if r == g  and g < b.
fn static_sort3_orig(r: f64, g: f64, b: f64) -> (usize, usize, usize) {
    // min = MIN(r, MIN(g, b));
    // ((r) < (((g) < (b)) ? (g) : (b))) ? (r) : (((g) < (b)) ? (g) : (b));
    // max = MAX(r, MAX(g, b));
    // ((r) > (((g) > (b)) ? (g) : (b))) ? (r) : (((g) > (b)) ? (g) : (b))
    // mid = ((r) > (g) ?
    //          ((g) > (b) ?
    //             (g) :
    //             ((r) > (b) ?
    //                (b) :
    //                (r)
    //             )
    //          ) :
    //          ((g) > (b) ?
    //             ((b) > (r) ?
    //                (b) :
    //                (r)
    //             ) :
    //             (g)))

    let min = if r < g.min(b) {
        0 // r
    } else if g < b {
        1 // g
    } else {
        2 // b
    };
    let max = if r > g.max(b) {
        0 // r
    } else if g > b {
        1 // g
    } else {
        2 // b
    };
    let mid = if r > g {
        if g > b {
            1 // g
        } else {
            if r > b {
                2 // b
            } else {
                0 // r
            }
        }
    } else {
        if g > b {
            if b > r {
                2 // b
            } else {
                0 // r
            }
        } else {
            1 // g
        }
    };
    (min, mid, max)
}

// Ensure that we produce the same output as Aseprite, even though it's wrong.
const ASEPRITE_SATURATION_BUG_COMPATIBLE: bool = true;

fn set_saturation(r: f64, g: f64, b: f64, sat: f64) -> (f64, f64, f64) {
    let mut col = [r, g, b];

    let (min, mid, max) = if ASEPRITE_SATURATION_BUG_COMPATIBLE {
        static_sort3_orig(r, g, b)
    } else {
        static_sort3(r, g, b)
    };
    if col[max] > col[min] {
        // i.e., they're not all the same
        col[mid] = ((col[mid] - col[min]) * sat) / (col[max] - col[min]);
        col[max] = sat;
    } else {
        col[mid] = 0.0;
        col[max] = 0.0;
    }
    col[min] = 0.0;
    (col[0], col[1], col[2])
}

// This test actually fails because Aseprite's version fails this test.
#[test]
fn test_set_saturation() {
    if ASEPRITE_SATURATION_BUG_COMPATIBLE {
        // This fails for the Aseprite implementation
        return;
    }
    // Test that:
    //
    //     saturation(set_saturation(r, g, b, s) == s)
    //
    // (unless saturation(r, g, b) == 0, i.e., they're all the same color)
    let steps = [0.0_f64, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];
    for r in steps.iter().cloned() {
        for g in steps.iter().cloned() {
            for b in steps.iter().cloned() {
                for sat in steps.iter().cloned() {
                    let sat0 = saturation(r, g, b);
                    println!(
                        "* x = ({:.3}, {:.3}, {:.3}); x.sat() = {:.5}",
                        r, g, b, sat0
                    );
                    let (r1, g1, b1) = set_saturation(r, g, b, sat);
                    let sat1 = saturation(r1, g1, b1);
                    println!(
                        "  y = x.set_sat({:.5}); y = ({:.3}, {:.3}, {:.3}), y.sat() = {:.5}",
                        sat, r1, g1, b1, sat1
                    );

                    // println!("set_saturation({:.3}, {:.3}, {:.3}, {:.3}) => ({:.3}, {:.3}, {:.3}) => sat: {:.5} (input sat: {:.5})",
                    // r, g, b, sat, r1, g1, b1, sat1, sat0);

                    if !(r == g && g == b) {
                        if (sat1 - sat).abs() > 0.00001 {
                            panic!(
                                "set_saturation({:.3}, {:.3}, {:.3}, {:.3}) => ({:.3}, {:.3}, {:.3}) => sat: {:.5} (input sat: {:.5})",
                                r, g, b, sat, r1, g1, b1, sat1, sat0
                            );
                        }
                    }
                }
            }
        }
    }
}

// --- rgba utils --------------------------------------------------------------

fn as_rgba_i32(color: Color8) -> (i32, i32, i32, i32) {
    let [r, g, b, a] = color.0;
    (r as i32, g as i32, b as i32, a as i32)
}

fn as_rgb_f64(color: Color8) -> (f64, f64, f64) {
    let r = color[0] as f64 / 255.0;
    let g = color[1] as f64 / 255.0;
    let b = color[2] as f64 / 255.0;
    (r, g, b)
}

fn from_rgba_i32(r: i32, g: i32, b: i32, a: i32) -> Color8 {
    debug_assert!(r >= 0 && r <= 255);
    debug_assert!(g >= 0 && g <= 255);
    debug_assert!(b >= 0 && b <= 255);
    debug_assert!(a >= 0 && a <= 255);

    Rgba([r as u8, g as u8, b as u8, a as u8])
}

fn from_rgb_f64(r: f64, g: f64, b: f64, a: u8) -> Color8 {
    from_rgba_i32(
        (r * 255.0) as i32,
        (g * 255.0) as i32,
        (b * 255.0) as i32,
        a as i32,
    )
}

/*
color_t rgba_blender_merge(color_t backdrop, color_t src, int opacity)
{
  int Br, Bg, Bb, Ba;
  int Sr, Sg, Sb, Sa;
  int Rr, Rg, Rb, Ra;
  int t;

  Br = rgba_getr(backdrop);
  Bg = rgba_getg(backdrop);
  Bb = rgba_getb(backdrop);
  Ba = rgba_geta(backdrop);

  Sr = rgba_getr(src);
  Sg = rgba_getg(src);
  Sb = rgba_getb(src);
  Sa = rgba_geta(src);

  if (Ba == 0) {
    Rr = Sr;
    Rg = Sg;
    Rb = Sb;
  }
  else if (Sa == 0) {
    Rr = Br;
    Rg = Bg;
    Rb = Bb;
  }
  else {
    Rr = Br + MUL_UN8((Sr - Br), opacity, t);
    Rg = Bg + MUL_UN8((Sg - Bg), opacity, t);
    Rb = Bb + MUL_UN8((Sb - Bb), opacity, t);
  }
  Ra = Ba + MUL_UN8((Sa - Ba), opacity, t);
  if (Ra == 0)
    Rr = Rg = Rb = 0;

  return rgba(Rr, Rg, Rb, Ra);
}
*/

fn blend8(back: u8, src: u8, opacity: u8) -> u8 {
    let src_x = src as i32;
    let back_x = back as i32;
    let a = src_x - back_x;
    let b = opacity as i32;
    let t = a * b + 0x80;
    let r = ((t >> 8) + t) >> 8;
    (back as i32 + r) as u8
}

#[test]
fn test_blend8() {
    assert_eq!(blend8(80, 50, 0), 80);
    assert_eq!(blend8(80, 50, 128), 65);
    assert_eq!(blend8(80, 50, 255), 50);
    assert_eq!(blend8(80, 150, 128), 80 + (70 / 2));
    assert_eq!(blend8(80, 150, 51), 80 + (70 / 5));
    assert_eq!(blend8(80, 150, 36), 80 + (70 / 7));

    //assert_eq!(blend8(0, 237, 128), 0);
}

#[test]
fn test_normal() {
    let back = Rgba([0, 205, 249, 255]);
    let front = Rgba([237, 118, 20, 255]);
    let res = normal(back, front, 128);
    assert_eq!(Rgba([118, 162, 135, 255]), res);
}

fn mul_un8(a: i32, b: i32) -> u8 {
    let t = a * b + 0x80;
    let r = ((t >> 8) + t) >> 8;
    r as u8
}

// DIV_UN8(a, b)    (((uint16_t) (a) * 0xff + ((b) / 2)) / (b))
fn div_un8(a: i32, b: i32) -> u8 {
    let t = a * 0xff;
    let u = b / 2;
    let r = (t + u) / b;
    r as u8
}
// fn mul_un8()

/*

67:#define MUL_UN8(a, b, t)                                             \
68-    ((t) = (a) * (uint16_t)(b) + ONE_HALF, ((((t) >> G_SHIFT ) + (t) ) >> G_SHIFT ))

*/
