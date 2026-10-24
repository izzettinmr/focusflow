#!/usr/bin/env python3
"""Original FocusFlow chimes, synthesized from sine partials; no third-party samples."""
from pathlib import Path
import array, math, sys, wave
RATE = 44100
ROOT = Path(__file__).resolve().parent.parent / 'assets/sounds'
def render(name, notes, length):
    samples = []
    for frame in range(round(length * RATE)):
        time = frame / RATE
        value = 0.0
        for start, frequency, gain in notes:
            t = time - start
            if not 0 <= t < 1.4:
                continue
            attack = min(1.0, t / .006)
            tail = min(1.0, (length - time) / .06)
            value += gain * attack * tail * (
                math.sin(2 * math.pi * frequency * t) * math.exp(-4.5*t)
                + .25 * math.sin(2 * math.pi * frequency * 2.006 * t) * math.exp(-7*t)
                + .08 * math.sin(2 * math.pi * frequency * 3.01 * t) * math.exp(-10*t))
        samples.append(value)
    peak = max(abs(s) for s in samples)
    pcm = array.array('h', (round(s / peak * 0.82 * 32767) for s in samples))
    if sys.byteorder != 'little': pcm.byteswap()
    with wave.open(str(ROOT/name), 'wb') as out:
        out.setparams((1, 2, RATE, len(pcm), 'NONE', 'not compressed'))
        out.writeframes(pcm.tobytes())
ROOT.mkdir(parents=True, exist_ok=True)
render('start.wav', [(0, 783.991, .65), (.18, 1046.502, .65)], .85)
render('complete.wav', [(t, hz, .65) for t,hz in [(0,1046.502),(.65,1318.51),(1.3,1046.502)]], 2.7)
