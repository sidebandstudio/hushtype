#!/usr/bin/env python3
"""Synthesize hushtype's recording start/stop feedback sounds.

The sounds are generated from scratch here so their provenance is recorded:
run this script to reproduce src-tauri/resources/{marimba,pop}_{start,stop}.wav.
Standard library only.
"""

import math
import struct
import wave
from pathlib import Path

RATE = 44100
PEAK = 0.35  # about -9 dBFS, quiet enough to sit under speech
OUT = Path(__file__).resolve().parent.parent / "src-tauri" / "resources"


def mallet(freq, duration):
    """A struck wooden bar: a fundamental plus the bar's inharmonic partials,
    each decaying faster than the one below it."""
    partials = [(1.0, 1.0, 0.16), (3.93, 0.28, 0.05), (9.2, 0.07, 0.018)]
    samples = []
    for i in range(int(duration * RATE)):
        t = i / RATE
        attack = min(1.0, t / 0.003)
        value = sum(
            amp * math.exp(-t / tau) * math.sin(2 * math.pi * freq * ratio * t)
            for ratio, amp, tau in partials
        )
        samples.append(attack * value)
    return samples


def pop(start_freq, end_freq, duration):
    """A short pitched pop: a sine whose pitch glides between two notes while
    it decays."""
    samples = []
    phase = 0.0
    for i in range(int(duration * RATE)):
        t = i / RATE
        glide = min(1.0, t / 0.045)
        freq = start_freq * (end_freq / start_freq) ** glide
        phase += 2 * math.pi * freq / RATE
        attack = min(1.0, t / 0.002)
        samples.append(attack * math.exp(-t / 0.035) * math.sin(phase))
    return samples


def mix(length, *parts):
    """Sum (offset_seconds, samples) parts into one buffer of `length` seconds."""
    out = [0.0] * int(length * RATE)
    for offset, samples in parts:
        start = int(offset * RATE)
        for i, value in enumerate(samples):
            if start + i < len(out):
                out[start + i] += value
    return out


def write(name, samples):
    peak = max(abs(s) for s in samples)
    fade = int(0.01 * RATE)
    frames = bytearray()
    for i, value in enumerate(samples):
        gain = min(1.0, (len(samples) - i) / fade)
        frames += struct.pack("<h", round(value / peak * PEAK * gain * 32767))
    with wave.open(str(OUT / name), "wb") as f:
        f.setnchannels(1)
        f.setsampwidth(2)
        f.setframerate(RATE)
        f.writeframes(bytes(frames))


G5, C6 = 783.99, 1046.50
write("marimba_start.wav", mix(0.6, (0.0, mallet(G5, 0.6)), (0.09, mallet(C6, 0.5))))
write("marimba_stop.wav", mix(0.7, (0.0, mallet(C6, 0.6)), (0.11, mallet(G5, 0.6))))
write("pop_start.wav", mix(0.3, (0.0, pop(520, 980, 0.3))))
write("pop_stop.wav", mix(0.3, (0.0, pop(900, 470, 0.3))))
