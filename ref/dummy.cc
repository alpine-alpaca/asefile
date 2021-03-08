#include "stdint.h"
#include <stdio.h>

typedef uint32_t color_t;

#define ONE_HALF 0x80

#define A_SHIFT 8 * 3
#define R_SHIFT 8 * 2
#define G_SHIFT 8
#define A_MASK 0xff000000
#define R_MASK 0xff0000
#define G_MASK 0xff00

const uint32_t rgba_r_shift = 0;
const uint32_t rgba_g_shift = 8;
const uint32_t rgba_b_shift = 16;
const uint32_t rgba_a_shift = 24;

const uint32_t rgba_r_mask = 0x000000ff;
const uint32_t rgba_g_mask = 0x0000ff00;
const uint32_t rgba_b_mask = 0x00ff0000;
const uint32_t rgba_rgb_mask = 0x00ffffff;
const uint32_t rgba_a_mask = 0xff000000;

 inline uint8_t rgba_getr(uint32_t c) {
    return (c >> rgba_r_shift) & 0xff;
  }

  inline uint8_t rgba_getg(uint32_t c) {
    return (c >> rgba_g_shift) & 0xff;
  }

  inline uint8_t rgba_getb(uint32_t c) {
    return (c >> rgba_b_shift) & 0xff;
  }

  inline uint8_t rgba_geta(uint32_t c) {
    return (c >> rgba_a_shift) & 0xff;
  }

 inline uint32_t rgba(uint8_t r, uint8_t g, uint8_t b, uint8_t a) {
    return ((r << rgba_r_shift) |
            (g << rgba_g_shift) |
            (b << rgba_b_shift) |
            (a << rgba_a_shift));
  }


#define MUL_UN8(a, b, t)                                             \
    ((t) = (a) * (uint16_t)(b) + ONE_HALF, ((((t) >> G_SHIFT ) + (t) ) >> G_SHIFT ))

// #define MUL_UN8(a, b, t)                               \
//   ((t) = (a) * (b) + 0x80, ((((t) >> 8) + (t)) >> 8))

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

color_t rgba_blender_normal(color_t backdrop, color_t src, int opacity)
{
  int t;

  if (!(backdrop & rgba_a_mask)) {
    int a = rgba_geta(src);
    a = MUL_UN8(a, opacity, t);
    a <<= rgba_a_shift;
    return (src & rgba_rgb_mask) | a;
  }
  else if (!(src & rgba_a_mask)) {
    return backdrop;
  }

  const int Br = rgba_getr(backdrop);
  const int Bg = rgba_getg(backdrop);
  const int Bb = rgba_getb(backdrop);
  const int Ba = rgba_geta(backdrop);

  const int Sr = rgba_getr(src);
  const int Sg = rgba_getg(src);
  const int Sb = rgba_getb(src);
  int Sa = rgba_geta(src);
  Sa = MUL_UN8(Sa, opacity, t);

  // Ra = Sa + Ba*(1-Sa)
  //    = Sa + Ba - Ba*Sa
  const int Ra = Sa + Ba - MUL_UN8(Ba, Sa, t);

  // Ra = Sa + Ba*(1-Sa)
  // Ba = (Ra-Sa) / (1-Sa)
  // Rc = (Sc*Sa + Bc*Ba*(1-Sa)) / Ra                Replacing Ba with (Ra-Sa) / (1-Sa)...
  //    = (Sc*Sa + Bc*(Ra-Sa)/(1-Sa)*(1-Sa)) / Ra
  //    = (Sc*Sa + Bc*(Ra-Sa)) / Ra
  //    = Sc*Sa/Ra + Bc*Ra/Ra - Bc*Sa/Ra
  //    = Sc*Sa/Ra + Bc - Bc*Sa/Ra
  //    = Bc + (Sc-Bc)*Sa/Ra
  const int Rr = Br + (Sr-Br) * Sa / Ra;
  const int Rg = Bg + (Sg-Bg) * Sa / Ra;
  const int Rb = Bb + (Sb-Bb) * Sa / Ra;

  return rgba(Rr, Rg, Rb, Ra);
}

#define blend_multiply(b, s, t)   (MUL_UN8((b), (s), (t)))

color_t rgba_blender_multiply(color_t backdrop, color_t src, int opacity)
{
  int t;
  int r = blend_multiply(rgba_getr(backdrop), rgba_getr(src), t);
  int g = blend_multiply(rgba_getg(backdrop), rgba_getg(src), t);
  int b = blend_multiply(rgba_getb(backdrop), rgba_getb(src), t);
  src = rgba(r, g, b, 0) | (src & rgba_a_mask);
  return rgba_blender_normal(backdrop, src, opacity);
}

// New Blender Method macros
#define RGBA_BLENDER_N(name)                                                    \
color_t rgba_blender_##name##_n(color_t backdrop, color_t src, int opacity) {   \
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
}

RGBA_BLENDER_N(multiply)






void test_merge() {
    color_t back = rgba(0, 205, 249, 255);
    color_t pixel = rgba(237, 118, 20, 255);

    color_t result = rgba_blender_merge(back, pixel, 128);
    printf("%d %d %d %d", rgba_getr(result), rgba_getg(result),
        rgba_getb(result), rgba_geta(result));
}

void test_multiply() {
    color_t back = rgba(245, 65, 48, 10);
    color_t pixel = rgba(42, 41, 227, 209);

    color_t result = rgba_blender_multiply_n(back, pixel, 255);
    printf("%d %d %d %d", rgba_getr(result), rgba_getg(result),
        rgba_getb(result), rgba_geta(result));
}

// -----------------------------------------------------------------------------


static double lum(double r, double g, double b)
{
  return 0.3*r + 0.59*g + 0.11*b;
}

static double maxd(double a, double b) {
    if (a > b) {
        return a;
    } else {
        return b;
    }
}

static double mind(double a, double b) {
    if (a < b) {
        return a;
    } else {
        return b;
    }
}


static double sat(double r, double g, double b)
{
  return maxd(r, maxd(g, b)) - mind(r, mind(g, b));
}

static void clip_color(double& r, double& g, double& b)
{
  double l = lum(r, g, b);
  double n = mind(r, mind(g, b));
  double x = maxd(r, maxd(g, b));

  if (n < 0) {
    r = l + (((r - l) * l) / (l - n));
    g = l + (((g - l) * l) / (l - n));
    b = l + (((b - l) * l) / (l - n));
  }

  if (x > 1) {
    r = l + (((r - l) * (1 - l)) / (x - l));
    g = l + (((g - l) * (1 - l)) / (x - l));
    b = l + (((b - l) * (1 - l)) / (x - l));
  }
}

static void set_lum(double& r, double& g, double& b, double l)
{
  double d = l - lum(r, g, b);
  r += d;
  g += d;
  b += d;
  clip_color(r, g, b);
}

// TODO replace this with a better impl (and test this, not sure if it's correct)
static void set_sat(double& r, double& g, double& b, double s)
{
#undef MIN
#undef MAX
#undef MID
#define MIN(x,y)     (((x) < (y)) ? (x) : (y))
#define MAX(x,y)     (((x) > (y)) ? (x) : (y))
#define MID(x,y,z)   ((x) > (y) ? ((y) > (z) ? (y) : ((x) > (z) ?    \
                       (z) : (x))) : ((y) > (z) ? ((z) > (x) ? (z) : \
                       (x)): (y)))

  double& min = MIN(r, MIN(g, b));
  double& mid = MID(r, g, b);
  double& max = MAX(r, MAX(g, b));

  if (max > min) {
    mid = ((mid - min)*s) / (max - min);
    max = s;
  }
  else
    mid = max = 0;

  min = 0;
}

// TODO replace this with a better impl (and test this, not sure if it's correct)
static void set_sat2(double* r, double* g, double* b, double s)
{
  double *tmp;

#define SWAP(x,y)  if (!((*x) < (*y))) { tmp = (x); (x) = (y); (y) = tmp; }
  double *min = r;
  double *mid = g;
  double *max = b;
  SWAP(min, mid);
  SWAP(min, max);
  SWAP(mid, max);
  printf("min:%f mid:%f max:%f", *min, *mid, *max);

  if (*max > *min) {
    *mid = ((*mid - *min) * s) / (*max - *min);
    *max = s;
  }
  else
    *mid = *max = 0;

  *min = 0;

#undef SWAP
}

// -------

color_t rgba_blender_hsl_saturation(color_t backdrop, color_t src, int opacity)
{
  double r = rgba_getr(src)/255.0;
  double g = rgba_getg(src)/255.0;
  double b = rgba_getb(src)/255.0;
  printf("src: %f %f %f\n", r, g, b);
  double s = sat(r, g, b);
  printf("sat: %f\n", s);

  r = rgba_getr(backdrop)/255.0;
  g = rgba_getg(backdrop)/255.0;
  b = rgba_getb(backdrop)/255.0;
  printf("back: %f %f %f\n", r, g, b);
  double l = lum(r, g, b);
  printf("lum: %f\n", l);

  set_sat2(&r, &g, &b, s);
  printf("applied sat: %f %f %f\n", r, g, b);
  set_lum(r, g, b, l);
  printf("applied lum: %f %f %f\n", r, g, b);

  src = rgba(int(255.0*r), int(255.0*g), int(255.0*b), 0) | (src & rgba_a_mask);
  printf("final src: %d %d %d %d\n", rgba_getr(src), rgba_getg(src),
        rgba_getb(src), rgba_geta(src));
  return rgba_blender_normal(backdrop, src, opacity);
}

RGBA_BLENDER_N(hsl_saturation);

void test_hsl_saturation() {
    // let back = Rgba([81, 81, 163, 129]);
    // let src = Rgba([50, 104, 58, 189]);
    color_t back = rgba(81, 81, 163, 129);
    color_t pixel = rgba(50, 104, 58, 189);

    color_t result = rgba_blender_hsl_saturation(back, pixel, 255);
    printf("%d %d %d %d\n", rgba_getr(result), rgba_getg(result),
        rgba_getb(result), rgba_geta(result));
    result = rgba_blender_hsl_saturation_n(back, pixel, 255);
    printf("%d %d %d %d\n", rgba_getr(result), rgba_getg(result),
        rgba_getb(result), rgba_geta(result));
}


int main(int argc, char* argv[]) {
    test_hsl_saturation();
}
