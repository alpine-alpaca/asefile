use image::Rgba;
// based on https://github.com/aseprite/aseprite/blob/master/src/doc/blend_funcs.cpp

pub type Color8 = Rgba<u8>;

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
// fn mul_un8()

/*

67:#define MUL_UN8(a, b, t)                                             \
68-    ((t) = (a) * (uint16_t)(b) + ONE_HALF, ((((t) >> G_SHIFT ) + (t) ) >> G_SHIFT ))

*/
