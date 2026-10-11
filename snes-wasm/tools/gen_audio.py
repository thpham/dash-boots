#!/usr/bin/env python3
"""Generate original 16-bit style music + SFX as WAV files (stdlib only, deterministic).

Outputs:
  public/audio/music/level1.wav  seamless loop (16 bars, 150 BPM, D major)
  public/audio/sfx/*.wav         jump, dash, coin, stomp, hurt, death, powerup, checkpoint, clear, bump, pause

Usage: python3 tools/gen_audio.py
"""

import math
import os
import random
import struct
import wave

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SR = 22050

NOTE_INDEX = {'C': 0, 'C#': 1, 'D': 2, 'D#': 3, 'E': 4, 'F': 5, 'F#': 6, 'G': 7, 'G#': 8, 'A': 9, 'A#': 10, 'B': 11}


def note_freq(name):
    """'F#5' -> Hz (A4 = 440)."""
    pitch, octave = name[:-1], int(name[-1])
    midi = 12 * (octave + 1) + NOTE_INDEX[pitch]
    return 440.0 * 2 ** ((midi - 69) / 12)


# --------------------------------------------------------------------------
# Oscillators
# --------------------------------------------------------------------------

def pulse(phase, duty):
    return 1.0 if (phase % 1.0) < duty else -1.0


def triangle(phase):
    p = phase % 1.0
    return 4.0 * p - 1.0 if p < 0.5 else 3.0 - 4.0 * p


class Noise:
    def __init__(self, seed=1):
        self.rnd = random.Random(seed)
        self.value = 0.0
        self.counter = 0.0

    def sample(self, rate_hz):
        """Sample-and-hold noise; lower rate = darker noise."""
        self.counter += rate_hz / SR
        if self.counter >= 1.0:
            self.counter %= 1.0
            self.value = self.rnd.uniform(-1.0, 1.0)
        return self.value


def env_adsr(t, dur, a=0.005, d=0.08, s=0.6, r=0.04):
    if t < a:
        return t / a
    if t < a + d:
        return 1.0 - (1.0 - s) * (t - a) / d
    if t < dur:
        return s
    if t < dur + r:
        return s * (1.0 - (t - dur) / r)
    return 0.0


# --------------------------------------------------------------------------
# Music
# --------------------------------------------------------------------------

BPM = 150
STEP = 60.0 / BPM / 4  # one 16th note in seconds

LEAD = [
    # A section
    "F#5-3 A5-1 r-2 A5-2 B5-2 A5-2 F#5-2 D5-2",
    "E5-3 C#5-1 r-2 E5-2 A5-4 G5-2 F#5-2",
    "F#5-3 D5-1 r-2 F#5-2 B5-4 A5-2 F#5-2",
    "G5-4 F#5-2 E5-2 D5-4 B4-4",
    "F#5-3 A5-1 r-2 A5-2 D6-4 C#6-2 B5-2",
    "C#6-3 A5-1 r-2 E5-2 A5-4 B5-2 C#6-2",
    "D6-4 B5-2 G5-2 E6-4 D6-2 B5-2",
    "C#6-6 B5-2 A5-4 r-4",
    # B section
    "B4-2 D5-2 G5-2 D5-2 B5-4 A5-2 G5-2",
    "C#5-2 E5-2 A5-2 E5-2 C#6-4 B5-2 A5-2",
    "A5-3 F#5-1 r-2 C#5-2 F#5-4 E5-2 F#5-2",
    "D6-4 C#6-2 B5-2 F#5-6 r-2",
    "G5-2 A5-2 B5-2 D6-2 r-2 B5-2 G5-4",
    "A5-2 B5-2 C#6-2 E6-2 r-2 C#6-2 A5-4",
    "D6-3 A5-1 F#5-2 A5-2 D6-2 F#6-2 E6-2 D6-2",
    "E6-4 C#6-2 A5-2 E5-4 A4-2 C#5-2",
]

# Chord per bar: (root for bass, chord tones for the arpeggio).
CHORDS = {
    'D': ('D', ['D4', 'F#4', 'A4', 'D5']),
    'A': ('A', ['A3', 'C#4', 'E4', 'A4']),
    'Bm': ('B', ['B3', 'D4', 'F#4', 'B4']),
    'G': ('G', ['G3', 'B3', 'D4', 'G4']),
    'F#m': ('F#', ['F#3', 'A3', 'C#4', 'F#4']),
}
PROGRESSION = ['D', 'A', 'Bm', 'G', 'D', 'A', 'G', 'A',
               'G', 'A', 'F#m', 'Bm', 'G', 'A', 'D', 'A']

FIFTH = {'D': 'A', 'A': 'E', 'B': 'F#', 'G': 'D', 'F#': 'C#'}


def parse_line(line):
    out = []
    for tok in line.split():
        name, dur = tok.rsplit('-', 1)
        out.append((None if name == 'r' else name, int(dur)))
    assert sum(d for _, d in out) == 16, f'bar does not sum to 16: {line}'
    return out


def render_voice(buf, start, dur, freq, wave_fn, vol, env, vibrato=0.0):
    n = int(dur * SR)
    tail = int(0.06 * SR)
    phase = 0.0
    for i in range(n + tail):
        idx = start + i
        if idx >= len(buf):
            break
        t = i / SR
        f = freq
        if vibrato and t > 0.12:
            f *= 1.0 + vibrato * math.sin(2 * math.pi * 5.5 * t)
        phase += f / SR
        buf[idx] += wave_fn(phase) * vol * env(t, dur)


def render_song(loops):
    bar_len = 16 * STEP
    total = int(round(len(PROGRESSION) * bar_len * loops * SR))
    lead = [0.0] * total
    rest = [0.0] * total
    noise = Noise(42)

    for loop in range(loops):
        for bar, chord_name in enumerate(PROGRESSION):
            bar_start = (loop * len(PROGRESSION) + bar) * bar_len
            # Lead (25% pulse with vibrato).
            t = bar_start
            for (note, d) in parse_line(LEAD[bar]):
                if note:
                    render_voice(lead, int(t * SR), d * STEP * 0.92, note_freq(note),
                                 lambda p: pulse(p, 0.25), 0.22,
                                 lambda tt, dd: env_adsr(tt, dd, 0.004, 0.10, 0.55, 0.05), vibrato=0.006)
                t += d * STEP
            # Arpeggio (12.5% pulse, quiet), 16ths cycling chord tones.
            root, tones = CHORDS[chord_name]
            for s in range(16):
                f = note_freq(tones[s % 4]) * 2
                render_voice(rest, int((bar_start + s * STEP) * SR), STEP * 0.55, f,
                             lambda p: pulse(p, 0.125), 0.055,
                             lambda tt, dd: env_adsr(tt, dd, 0.002, 0.03, 0.4, 0.02))
            # Bass (triangle), driving 8ths: root low/high + fifth.
            pattern = [root + '2', root + '3', root + '2', root + '3',
                       FIFTH[root] + '2', root + '3', root + '2', FIFTH[root] + '2']
            for s, n in enumerate(pattern):
                render_voice(rest, int((bar_start + s * 2 * STEP) * SR), 2 * STEP * 0.8, note_freq(n),
                             triangle, 0.30, lambda tt, dd: env_adsr(tt, dd, 0.003, 0.05, 0.8, 0.02))
            # Drums.
            fill = bar % 8 == 7
            for s in range(16):
                at = int((bar_start + s * STEP) * SR)
                if s in (0, 8) or (s == 10 and bar % 2 == 1):
                    drum_kick(rest, at)
                if s in (4, 12) or (fill and s >= 12):
                    drum_snare(rest, at, noise, 0.20 if not (fill and s > 12) else 0.15)
                if s % 2 == 0:
                    drum_hat(rest, at, noise, 0.05 if s % 4 else 0.07)
    return lead, rest


def drum_kick(buf, at):
    n = int(0.12 * SR)
    phase = 0.0
    for i in range(n):
        if at + i >= len(buf):
            break
        t = i / SR
        f = 140 * math.exp(-t * 28) + 45
        phase += f / SR
        buf[at + i] += triangle(phase) * 0.45 * math.exp(-t * 22)


def drum_snare(buf, at, noise, vol):
    n = int(0.16 * SR)
    for i in range(n):
        if at + i >= len(buf):
            break
        t = i / SR
        buf[at + i] += noise.sample(9000) * vol * math.exp(-t * 26)


def drum_hat(buf, at, noise, vol):
    n = int(0.03 * SR)
    for i in range(n):
        if at + i >= len(buf):
            break
        t = i / SR
        buf[at + i] += noise.sample(20000) * vol * math.exp(-t * 120)


def apply_echo(buf, delay_s, feedback, mix):
    d = int(delay_s * SR)
    out = list(buf)
    for i in range(d, len(out)):
        out[i] += out[i - d] * feedback
    return [buf[i] * (1 - mix) + out[i] * mix for i in range(len(buf))]


def build_music():
    # Render two loops and keep the second one so echo tails wrap seamlessly.
    lead, rest = render_song(loops=2)
    lead = apply_echo(lead, STEP * 3, 0.35, 0.5)
    mix = [lead[i] + rest[i] for i in range(len(lead))]
    half = len(mix) // 2
    return soft_clip(mix[half:], 0.9)


# --------------------------------------------------------------------------
# SFX
# --------------------------------------------------------------------------

def sweep(dur, f0, f1, wave_fn, vol, curve='exp', decay=6.0):
    n = int(dur * SR)
    out = []
    phase = 0.0
    for i in range(n):
        t = i / n
        f = f0 * (f1 / f0) ** t if curve == 'exp' else f0 + (f1 - f0) * t
        phase += f / SR
        out.append(wave_fn(phase) * vol * math.exp(-t * decay / 3) * min(1.0, (n - i) / (0.01 * SR)))
    return out


def notes(seq, wave_fn, vol, gap=0.0):
    out = []
    for (name, dur) in seq:
        n = int(dur * SR)
        f = note_freq(name)
        phase = 0.0
        for i in range(n):
            t = i / SR
            phase += f / SR
            out.append(wave_fn(phase) * vol * math.exp(-t * 4) * min(1.0, (n - i) / (0.008 * SR)))
        out.extend([0.0] * int(gap * SR))
    return out


def mix_into(base, other, offset=0):
    need = offset + len(other)
    if len(base) < need:
        base.extend([0.0] * (need - len(base)))
    for i, v in enumerate(other):
        base[offset + i] += v
    return base


def sfx_jump():
    return sweep(0.16, 260, 640, lambda p: pulse(p, 0.25), 0.5)


def sfx_dash():
    nz = Noise(7)
    n = int(0.28 * SR)
    out = []
    lp = 0.0
    for i in range(n):
        t = i / n
        lp += (nz.sample(16000) - lp) * (0.6 - 0.5 * t)
        out.append(lp * 0.7 * (1 - t) ** 1.5)
    return mix_into(out, sweep(0.2, 900, 180, lambda p: pulse(p, 0.5), 0.25))


def sfx_coin():
    return notes([('B5', 0.07), ('E6', 0.38)], lambda p: pulse(p, 0.5), 0.35)


def sfx_stomp():
    out = sweep(0.14, 520, 110, lambda p: pulse(p, 0.5), 0.45, decay=8)
    nz = Noise(3)
    thump = [nz.sample(4000) * 0.4 * math.exp(-i / SR * 40) for i in range(int(0.08 * SR))]
    return mix_into(out, thump)


def sfx_hurt():
    n = int(0.4 * SR)
    out = []
    phase = 0.0
    for i in range(n):
        t = i / SR
        f = 520 * (1 - 0.5 * t / 0.4) * (1 + 0.08 * math.sin(2 * math.pi * 28 * t))
        phase += f / SR
        out.append(pulse(phase, 0.5) * 0.4 * (1 - t / 0.4))
    return out


def sfx_death():
    return notes([('B4', 0.11), ('F5', 0.11), ('F5', 0.11), ('F5', 0.15), ('E5', 0.15),
                  ('D5', 0.15), ('C5', 0.5)], lambda p: pulse(p, 0.5), 0.35)


def sfx_powerup():
    seq = ['C5', 'E5', 'G5', 'C6', 'D5', 'F#5', 'A5', 'D6', 'E5', 'G#5', 'B5', 'E6']
    return notes([(n, 0.045) for n in seq] + [('E6', 0.25)], lambda p: pulse(p, 0.25), 0.35)


def sfx_checkpoint():
    a = notes([('E6', 0.1), ('B6', 0.45)], triangle, 0.5)
    b = notes([('E5', 0.1), ('B5', 0.45)], lambda p: pulse(p, 0.125), 0.18)
    return mix_into(a, b)


def sfx_clear():
    lead = notes([('A5', 0.13), ('D6', 0.13), ('F#6', 0.13), ('A6', 0.4), ('F#6', 0.13),
                  ('A6', 0.9)], lambda p: pulse(p, 0.25), 0.3)
    harm = notes([('F#5', 0.13), ('A5', 0.13), ('D6', 0.13), ('F#6', 0.4), ('D6', 0.13),
                  ('F#6', 0.9)], lambda p: pulse(p, 0.5), 0.13)
    bass = notes([('D3', 0.39), ('A2', 0.4), ('D3', 0.13), ('D3', 0.9)], triangle, 0.45)
    out = mix_into(lead, harm)
    return mix_into(out, bass)


def sfx_bump():
    return sweep(0.07, 180, 90, lambda p: pulse(p, 0.5), 0.45, decay=4)


def sfx_pause():
    return notes([('E6', 0.06), ('B5', 0.06), ('E6', 0.12)], lambda p: pulse(p, 0.5), 0.3)


def sfx_rift():
    """Crystalline digital glitch: fast high arpeggio, rising sweep, noise burst."""
    rnd = random.Random(5)
    seq = [rnd.choice(['E6', 'B6', 'F#6', 'C#7', 'G#6', 'D#7']) for _ in range(10)]
    out = notes([(n, 0.03) for n in seq], lambda p: pulse(p, 0.125), 0.28)
    out = mix_into(out, sweep(0.5, 400, 2400, triangle, 0.3, decay=3))
    nz = Noise(11)
    burst = [nz.sample(18000) * 0.25 * math.exp(-i / SR * 18) for i in range(int(0.25 * SR))]
    return mix_into(out, burst)


def sfx_return():
    """Warm, slightly detuned D major chord with a slow tape wow."""
    n = int(1.6 * SR)
    freqs = [note_freq(x) for x in ['D4', 'F#4', 'A4', 'D5']]
    phases = [0.0] * (len(freqs) * 2)
    out = []
    for i in range(n):
        t = i / SR
        wow = 1.0 + 0.004 * math.sin(2 * math.pi * 0.8 * t)
        env = min(1.0, t / 0.08) * math.exp(-t * 1.6)
        s = 0.0
        for k, f in enumerate(freqs):
            for j, det in enumerate((0.997, 1.003)):
                idx = k * 2 + j
                phases[idx] += f * det * wow / SR
                s += triangle(phases[idx])
        out.append(s / len(phases) * 0.8 * env)
    return out


SFX = {
    'jump': sfx_jump, 'dash': sfx_dash, 'coin': sfx_coin, 'stomp': sfx_stomp,
    'hurt': sfx_hurt, 'death': sfx_death, 'powerup': sfx_powerup,
    'checkpoint': sfx_checkpoint, 'clear': sfx_clear, 'bump': sfx_bump, 'pause': sfx_pause,
    'rift': sfx_rift, 'return': sfx_return,
}


# --------------------------------------------------------------------------
# Output
# --------------------------------------------------------------------------

def soft_clip(samples, drive):
    return [math.tanh(s * drive * 1.4) / math.tanh(1.4) for s in samples]


def write_wav(path, samples):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    peak = max(1e-9, max(abs(s) for s in samples))
    gain = min(1.0, 0.95 / peak)
    data = b''.join(struct.pack('<h', int(max(-1.0, min(1.0, s * gain)) * 32767)) for s in samples)
    with wave.open(path, 'wb') as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes(data)
    print(f'wrote {path} ({len(samples) / SR:.2f}s)')


def main():
    for name, fn in SFX.items():
        write_wav(os.path.join(ROOT, 'public', 'audio', 'sfx', f'{name}.wav'), fn())
    write_wav(os.path.join(ROOT, 'public', 'audio', 'music', 'level1.wav'), build_music())


if __name__ == '__main__':
    main()
