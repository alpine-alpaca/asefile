#include "stdint.h"
#include <stdio.h>
#include <math.h>

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


// --------------------


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

static void set_sat2(double* r, double* g, double* b, double s)
{
  double *tmp;

// Use a static sorting network with three swaps.
#define SWAP(x,y)  if (!((*x) < (*y))) { tmp = (x); (x) = (y); (y) = tmp; }
  double *min = r;
  double *mid = g;
  double *max = b;
  SWAP(min, mid);
  SWAP(min, max);
  SWAP(mid, max);
  // printf("min:%f mid:%f max:%f", *min, *mid, *max);

  if (*max > *min) {
    *mid = ((*mid - *min) * s) / (*max - *min);
    *max = s;
  }
  else
    *mid = *max = 0;

  *min = 0;

#undef SWAP
}

// -----------------------------------------------------------------------------

#define STEP ((double)1.0 / 4)
#define DBG_LOG 0
#define MAX_FAILURES 5

bool test_set_sat() {
    unsigned int num_failures = 0;
    for (double s = 0.0; s <= 1.0; s += STEP) {
        for (double in_r = 0.0; in_r <= 1.0; in_r += STEP) {
            for (double in_g = 0.0; in_g <= 1.0; in_g += STEP) {
                for (double in_b = 0.0; in_b <= 1.0; in_b += STEP) {
                    double r = in_r;
                    double g = in_g;
                    double b = in_b;

                    if (DBG_LOG) {
                        printf(
                            "* col=(%.4f, %.4f, %.4f), sat=%.4f => ",
                            r, g, b, sat(r, g, b)
                        );
                    }

                    set_sat(r, g, b, s);
                    //set_sat2(&r, &g, &b, s);
                    double new_s = sat(r, g, b);

                    if (DBG_LOG) {
                        printf(
                            "set_sat(%.4f) => (%.4f, %.4f, %.4f), new_sat=%.4f\n",
                            s, r, g, b, new_s
                        );
                    }

                    if (!(r == g && g == b)) {
                        if (fabs(s - new_s) > 0.00001) {
                            printf(
                                "ERROR: set_sat(%.4f, %.4f, %.4f,  %.4f) => "
                                "(%.4f, %.4f, %.4f), sat(..) = %.4f\n",
                                in_r, in_g, in_b, s,
                                r, g, b, new_s
                            );

                            num_failures += 1;
                            if (num_failures >= MAX_FAILURES) {
                                return false;
                            }
                        }
                    }
                }
            }
        }
    }
    return (num_failures == 0);
}

int main(int argc, char* argv[]) {
    if (test_set_sat()) {
        return 0;
    } else {
        printf("There were test failures");
        return 1;
    }
}
