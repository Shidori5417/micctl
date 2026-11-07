"""
micctl - Procedural Sound Effects Generator & Player
"""
import math
import struct
import wave
import subprocess
from pathlib import Path

SOUNDS_DIR = Path.home() / ".config" / "micctl" / "sounds"

def ensure_sounds():
    SOUNDS_DIR.mkdir(parents=True, exist_ok=True)
    roger = SOUNDS_DIR / "roger_beep.wav"
    nuke = SOUNDS_DIR / "nuke_alarm.wav"
    airhorn = SOUNDS_DIR / "airhorn.wav"
    beep = SOUNDS_DIR / "beep.wav"

    if not roger.exists():
        _generate_roger_beep(roger)
    if not nuke.exists():
        _generate_nuke_alarm(nuke)
    if not airhorn.exists():
        _generate_airhorn(airhorn)
    if not beep.exists():
        _generate_beep(beep)

def _write_wav(path: Path, samples: list[float], sample_rate: int = 48000):
    with wave.open(str(path), "w") as f:
        f.setnchannels(2)
        f.setsampwidth(2)
        f.setframerate(sample_rate)
        raw = bytearray()
        for s in samples:
            val = max(-32767, min(32767, int(s * 32767)))
            raw.extend(struct.pack("<hh", val, val))
        f.writeframesraw(raw)

def _generate_roger_beep(path: Path):
    sr = 48000
    samples = []
    # 1. Beep 1 (1200 Hz for 70ms)
    for i in range(int(sr * 0.07)):
        samples.append(0.4 * math.sin(2 * math.pi * 1200 * i / sr))
    # 2. Beep 2 (1800 Hz for 80ms)
    for i in range(int(sr * 0.08)):
        samples.append(0.4 * math.sin(2 * math.pi * 1800 * i / sr))
    # 3. Radio burst squelch noise (60ms)
    import random
    for i in range(int(sr * 0.06)):
        env = 1.0 - (i / (sr * 0.06))
        samples.append(0.35 * (random.random() * 2 - 1) * env)
    _write_wav(path, samples, sr)

def _generate_nuke_alarm(path: Path):
    sr = 48000
    samples = []
    import random
    duration = 1.4
    total = int(sr * duration)
    for i in range(total):
        t = i / sr
        # Rising siren: 400Hz to 1100Hz and back
        freq = 400 + 700 * (0.5 - 0.5 * math.cos(2 * math.pi * 2.0 * t))
        siren = 0.5 * math.sin(2 * math.pi * freq * t)
        # Deep sub bass rumble (60 Hz)
        sub = 0.4 * math.sin(2 * math.pi * 60 * t)
        # Harsh harmonic distortion
        sample = siren + sub
        if sample > 0.6:
            sample = 0.6
        elif sample < -0.6:
            sample = -0.6
        samples.append(sample * 1.4)
    _write_wav(path, samples, sr)

def _generate_airhorn(path: Path):
    sr = 48000
    samples = []
    duration = 0.65
    total = int(sr * duration)
    # Tri-tone chord: 466.16 Hz (Bb4), 587.33 Hz (D5), 698.46 Hz (F5)
    f1, f2, f3 = 466.16, 587.33, 698.46
    for i in range(total):
        t = i / sr
        # Envelope: punch attack, steady, quick decay
        env = 1.0 if t < 0.5 else max(0.0, 1.0 - (t - 0.5) / 0.15)
        val = (
            0.35 * math.sin(2 * math.pi * f1 * t) +
            0.35 * math.sin(2 * math.pi * f2 * t) +
            0.30 * math.sin(2 * math.pi * f3 * t)
        )
        # Slight clipping for brassy sound
        val = max(-0.8, min(0.8, val * 1.5))
        samples.append(val * env)
    _write_wav(path, samples, sr)

def _generate_beep(path: Path):
    sr = 48000
    samples = []
    for i in range(int(sr * 0.08)):
        env = 1.0 - (i / (sr * 0.08))
        samples.append(0.3 * math.sin(2 * math.pi * 880 * i / sr) * env)
    _write_wav(path, samples, sr)
