"""
micctl - State and Preset Manager
"""
import json
import os
from pathlib import Path

CONFIG_DIR = Path.home() / ".config" / "micctl"
CONFIG_FILE = CONFIG_DIR / "config.json"
PRESETS_DIR = CONFIG_DIR / "presets"

DEFAULT_STATE = {
    "enabled": True,
    "bass": 3.0,          # dB (-20.0 to +30.0)
    "bass_freq": 120.0,   # Hz
    "mid": 0.0,           # dB (-18.0 to +20.0)
    "mid_freq": 1800.0,   # Hz
    "treble": 1.5,        # dB (-18.0 to +20.0)
    "treble_freq": 6000.0,# Hz
    "lowcut": True,       # 80Hz rumble cut
    "noise_suppression": False, # RNNoise
    "gain": 100,          # Output volume % (0 to 200)
    "listen": False,      # Loopback monitoring
    "preset": "default",
    "fx_mode": "none",    # "none", "nuke", "ultra_nuke", "robot", "rabbit", "demon", "walkie_talkie", "telephone"
    "backup_state": None, # Saved state when entering temporary FX modes
    "notify": True
}

BUILTIN_PRESETS = {
    "podcast": {
        "bass": 5.0,
        "mid": 1.5,
        "treble": 2.5,
        "lowcut": True,
        "noise_suppression": False,
        "gain": 100,
        "fx_mode": "none"
    },
    "gaming": {
        "bass": 2.0,
        "mid": 2.5,
        "treble": 3.0,
        "lowcut": True,
        "noise_suppression": True,
        "gain": 100,
        "fx_mode": "none"
    },
    "natural": {
        "bass": 0.0,
        "mid": 0.0,
        "treble": 0.0,
        "lowcut": True,
        "noise_suppression": False,
        "gain": 100,
        "fx_mode": "none"
    },
    "deep-bass": {
        "bass": 12.0,
        "mid": -1.0,
        "treble": 1.0,
        "lowcut": True,
        "noise_suppression": False,
        "gain": 105,
        "fx_mode": "none"
    },
    "extreme-bass": {
        "bass": 24.0,
        "mid": -2.0,
        "treble": 0.0,
        "lowcut": False,
        "noise_suppression": False,
        "gain": 120,
        "fx_mode": "none"
    },
    "nuke": {
        "bass": 18.0,
        "mid": 10.0,
        "treble": 6.0,
        "lowcut": False,
        "noise_suppression": False,
        "gain": 150,
        "fx_mode": "nuke"
    },
    "ultra-nuke": {
        "bass": 30.0,
        "mid": 15.0,
        "treble": 10.0,
        "lowcut": False,
        "noise_suppression": False,
        "gain": 200,
        "fx_mode": "ultra_nuke"
    },
    "robot": {
        "bass": 0.0,
        "mid": 12.0,
        "treble": -4.0,
        "lowcut": True,
        "noise_suppression": False,
        "gain": 115,
        "fx_mode": "robot"
    },
    "rabbit": {
        "bass": -18.0,
        "mid": 8.0,
        "treble": 16.0,
        "lowcut": True,
        "noise_suppression": False,
        "gain": 130,
        "fx_mode": "rabbit"
    },
    "demon": {
        "bass": 22.0,
        "mid": -6.0,
        "treble": -14.0,
        "lowcut": False,
        "noise_suppression": False,
        "gain": 125,
        "fx_mode": "demon"
    },
    "walkie-talkie": {
        "bass": -12.0,
        "mid": 9.0,
        "treble": -8.0,
        "lowcut": True,
        "noise_suppression": False,
        "gain": 115,
        "fx_mode": "walkie_talkie"
    },
    "asmr": {
        "bass": 4.0,
        "mid": 1.0,
        "treble": 8.5,
        "lowcut": True,
        "noise_suppression": False,
        "gain": 135,
        "fx_mode": "none"
    }
}

def ensure_dirs():
    CONFIG_DIR.mkdir(parents=True, exist_ok=True)
    PRESETS_DIR.mkdir(parents=True, exist_ok=True)
    for name, data in BUILTIN_PRESETS.items():
        preset_file = PRESETS_DIR / f"{name}.json"
        with open(preset_file, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=2)

def load_state() -> dict:
    ensure_dirs()
    if not CONFIG_FILE.exists():
        save_state(DEFAULT_STATE)
        return dict(DEFAULT_STATE)
    try:
        with open(CONFIG_FILE, "r", encoding="utf-8") as f:
            data = json.load(f)
        state = dict(DEFAULT_STATE)
        state.update(data)
        return state
    except Exception:
        return dict(DEFAULT_STATE)

def save_state(state: dict):
    ensure_dirs()
    with open(CONFIG_FILE, "w", encoding="utf-8") as f:
        json.dump(state, f, indent=2)

def reset_state() -> dict:
    ensure_dirs()
    state = dict(DEFAULT_STATE)
    save_state(state)
    return state

def list_presets():
    ensure_dirs()
    return sorted([p.stem for p in PRESETS_DIR.glob("*.json")])

def load_preset(name: str) -> dict:
    ensure_dirs()
    preset_file = PRESETS_DIR / f"{name}.json"
    if not preset_file.exists():
        raise FileNotFoundError(f"Ön ayar bulunamadı: {name}")
    with open(preset_file, "r", encoding="utf-8") as f:
        return json.load(f)

def save_preset(name: str, state: dict):
    ensure_dirs()
    preset_file = PRESETS_DIR / f"{name}.json"
    preset_data = {
        k: state[k] for k in ["bass", "mid", "treble", "lowcut", "noise_suppression", "gain", "fx_mode"]
        if k in state
    }
    with open(preset_file, "w", encoding="utf-8") as f:
        json.dump(preset_data, f, indent=2)
