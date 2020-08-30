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


int main(int argc, char* argv[]) {
    color_t back = rgba(0, 205, 249, 255);
    color_t pixel = rgba(237, 118, 20, 255);

    color_t result = rgba_blender_merge(back, pixel, 128);
    printf("%d %d %d %d", rgba_getr(result), rgba_getg(result),
        rgba_getb(result), rgba_geta(result));
}