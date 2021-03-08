use image::Rgba;
// based on https://github.com/aseprite/aseprite/blob/master/src/doc/blend_funcs.cpp

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
pub(crate) fn multiply(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blender(backdrop, src, opacity, multiply_baseline)
}

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

fn multiply_baseline(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blend_channel(backdrop, src, opacity, blend_multiply)
}

fn blend_multiply(a: i32, b: i32) -> u8 {
    mul_un8(a, b)
}

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

fn blend_hard_light(b: i32, s: i32) -> u8 {
    if s < 128 {
        blend_multiply(b, s << 1)
    } else {
        blend_screen(b, (s << 1) - 255)
    }
}

pub(crate) fn darken(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blender(backdrop, src, opacity, darken_baseline)
}

fn darken_baseline(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blend_channel(backdrop, src, opacity, blend_darken)
}

fn blend_darken(b: i32, s: i32) -> u8 {
    b.min(s) as u8
}

pub(crate) fn lighten(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blender(backdrop, src, opacity, lighten_baseline)
}

fn lighten_baseline(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blend_channel(backdrop, src, opacity, blend_lighten)
}

fn blend_lighten(b: i32, s: i32) -> u8 {
    b.max(s) as u8
}

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

pub(crate) fn hard_light(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blender(backdrop, src, opacity, hard_light_baseline)
}

fn hard_light_baseline(backdrop: Color8, src: Color8, opacity: u8) -> Color8 {
    blend_channel(backdrop, src, opacity, blend_hard_light)
}

fn as_rgba_i32(color: Color8) -> (i32, i32, i32, i32) {
    let [r, g, b, a] = color.0;
    (r as i32, g as i32, b as i32, a as i32)
}

fn from_rgba_i32(r: i32, g: i32, b: i32, a: i32) -> Color8 {
    debug_assert!(r >= 0 && r <= 255);
    debug_assert!(g >= 0 && g <= 255);
    debug_assert!(b >= 0 && b <= 255);
    debug_assert!(a >= 0 && a <= 255);
    Rgba([r as u8, g as u8, b as u8, a as u8])
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
