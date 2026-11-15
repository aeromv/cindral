"""Generates Cindral's original title-screen art: a six-face panorama and a logo.

The panorama is computed per pixel from the direction it shows, so the six
cube faces meet without seams whatever the face orientation: the colour only
depends on the elevation, and the stars are points. Usage:
    python -I gen_art.py OUT_DIR
"""
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

SIZE = 1024
out = Path(sys.argv[1])
(out / "panorama").mkdir(parents=True, exist_ok=True)


def lerp(a, b, t):
    return a + (b - a) * t[..., None]


def smooth(t):
    t = np.clip(t, 0.0, 1.0)
    return t * t * (3 - 2 * t)


# Colours (linear-ish sRGB values 0..1).
TOP = np.array([0.035, 0.045, 0.11])
MID = np.array([0.20, 0.12, 0.30])
HORIZON = np.array([0.98, 0.45, 0.20])
GROUND_NEAR = np.array([0.16, 0.07, 0.06])
GROUND_FAR = np.array([0.025, 0.022, 0.035])


def face_directions(face):
    """Unit view directions of one face; faces 4 and 5 look up and down."""
    lin = (np.arange(SIZE) + 0.5) / SIZE * 2 - 1
    u, v = np.meshgrid(lin, lin)
    one = np.ones_like(u)
    if face < 4:
        # A side face: x across, y up (v grows downwards in the image).
        d = np.stack([u, -v, one], axis=-1)
    elif face == 4:
        d = np.stack([u, one, v], axis=-1)
    else:
        d = np.stack([u, -one, v], axis=-1)
    return d / np.linalg.norm(d, axis=-1, keepdims=True)


rng = np.random.default_rng(1844)
for face in range(6):
    d = face_directions(face)
    e = d[..., 1]  # elevation, -1 down .. 1 up
    sky = np.where(
        (e < 0.28)[..., None],
        lerp(HORIZON, MID, smooth(e / 0.28)),
        lerp(MID, TOP, smooth((e - 0.28) / 0.6)),
    )
    ground = lerp(GROUND_NEAR, GROUND_FAR, smooth(-e / 0.35))
    colour = np.where((e >= 0)[..., None], sky, ground)
    # A warm glow along the horizon, like embers under the sky.
    glow = np.exp(-((e / 0.045) ** 2))
    colour = colour + glow[..., None] * np.array([0.35, 0.12, 0.03])
    # Stars: sparse points, brighter and denser higher up.
    stars = rng.random((SIZE, SIZE))
    density = np.clip((e - 0.12) / 0.6, 0, 1) * 0.0016
    lit = stars < density
    twinkle = rng.random((SIZE, SIZE)) * 0.55 + 0.45
    star_layer = np.zeros((SIZE, SIZE))
    star_layer[lit] = twinkle[lit]
    star_img = Image.fromarray((star_layer * 255).astype(np.uint8)).filter(
        ImageFilter.GaussianBlur(0.7)
    )
    star_soft = np.asarray(star_img, dtype=np.float64) / 255 * 1.6
    colour = colour + star_soft[..., None] * np.array([1.0, 0.95, 0.88])
    # A little noise so the gradient does not band in 8 bits.
    colour = colour + (rng.random((SIZE, SIZE, 1)) - 0.5) / 255
    rgb = (np.clip(colour, 0, 1) * 255 + 0.5).astype(np.uint8)
    Image.fromarray(rgb, "RGB").save(out / "panorama" / f"panorama_{face}.png", optimize=True)

# The logo: an ember (a rounded flame cut from a ring), white on transparent,
# drawn four times larger and reduced for smooth edges.
S = 4
W, H = 359, 354
big = Image.new("L", (W * S, H * S), 0)
draw = ImageDraw.Draw(big)
cx, cy = W * S / 2, H * S / 2 + 6 * S
r_out, r_in = 150 * S, 112 * S
draw.ellipse([cx - r_out, cy - r_out, cx + r_out, cy + r_out], fill=255)
draw.ellipse([cx - r_in, cy - r_in, cx + r_in, cy + r_in], fill=0)
# Open the ring on the right: a "C".
draw.polygon(
    [(cx, cy), (cx + 2 * r_out, cy - 0.95 * r_out), (cx + 2 * r_out, cy + 0.95 * r_out)],
    fill=0,
)
# The flame in the middle: two arcs meeting in a point at the top.
flame = [
    (cx, cy - 92 * S),
    (cx + 38 * S, cy - 18 * S),
    (cx + 46 * S, cy + 22 * S),
    (cx + 30 * S, cy + 56 * S),
    (cx, cy + 70 * S),
    (cx - 30 * S, cy + 56 * S),
    (cx - 46 * S, cy + 22 * S),
    (cx - 38 * S, cy - 18 * S),
]
draw.polygon(flame, fill=255)
# The inner tongue of the flame, cut out.
inner = [
    (cx + 4 * S, cy - 18 * S),
    (cx + 20 * S, cy + 22 * S),
    (cx + 10 * S, cy + 46 * S),
    (cx - 8 * S, cy + 48 * S),
    (cx - 18 * S, cy + 28 * S),
]
draw.polygon(inner, fill=0)
alpha = big.resize((W, H), Image.LANCZOS).filter(ImageFilter.GaussianBlur(0.3))
logo = Image.merge("RGBA", (Image.new("L", (W, H), 255),) * 3 + (alpha,))
logo.save(out / "logo.png", optimize=True)
print("ok", out)
