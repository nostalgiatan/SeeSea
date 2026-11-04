#!/usr/bin/env python3
"""Redraw icons/* from the geometry of icons/mark.svg (an eye whose iris holds a wave). Needs Pillow only."""
import math, pathlib
from PIL import Image, ImageDraw

OUT = pathlib.Path(__file__).resolve().parent.parent / "icons"
PAPER, INK, BRASS = (232, 229, 225, 255), (17, 17, 17, 255), (155, 114, 71, 255)

def quad(p0, p1, p2, n=48):
    return [((1-t)**2*p0[0]+2*(1-t)*t*p1[0]+t*t*p2[0], (1-t)**2*p0[1]+2*(1-t)*t*p1[1]+t*t*p2[1]) for t in (i/n for i in range(n+1))]

def draw(size):
    k = 8; S = size*k; u = S/100
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0)); d = ImageDraw.Draw(img)
    d.rounded_rectangle((0, 0, S-1, S-1), radius=S*0.22, fill=PAPER)
    P = lambda pts: [(x*u, y*u) for x, y in pts]
    eye = quad((10, 50), (50, 14), (90, 50)) + quad((90, 50), (50, 86), (10, 50))
    d.line(P(eye + eye[:2]), fill=INK, width=int(6*u), joint="curve")
    d.ellipse(((50-15)*u, (50-15)*u, (50+15)*u, (50+15)*u), fill=BRASS)
    wave, x = [], 37.0
    for i in range(4):
        c = (x+3.25, 50-6 if i % 2 == 0 else 50+6)
        wave += quad((x, 50), c, (x+6.5, 50), 16); x += 6.5
    d.line(P(wave), fill=PAPER, width=int(3.5*u), joint="curve")
    for ex, ey in (wave[0], wave[-1]):
        r = 1.75*u; d.ellipse((ex*u-r, ey*u-r, ex*u+r, ey*u+r), fill=PAPER)
    for x0 in range(10, 90, 7):
        d.line(P([(x0, 74), (x0+2, 74)]), fill=INK, width=max(1, int(2*u)))
    return img.resize((size, size), Image.LANCZOS)

big = draw(1024)
big.resize((512, 512), Image.LANCZOS).save(OUT/"icon.png")
draw(128).save(OUT/"128x128.png"); draw(32).save(OUT/"32x32.png")
draw(256).save(OUT/"128x128@2x.png")
big.save(OUT/"icon.ico", sizes=[(16,16),(32,32),(48,48),(64,64),(128,128),(256,256)])
big.save(OUT/"icon.icns")
print("ok")
