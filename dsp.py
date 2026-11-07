"""
micctl - PipeWire Filter-Chain DSP & Soundboard Engine
"""
import os
import json
import signal
import subprocess
import time
from pathlib import Path
import sfx

CONFIG_DIR = Path.home() / ".config" / "micctl"
FILTER_CONF = CONFIG_DIR / "filter-chain.conf"
PID_FILE = CONFIG_DIR / "daemon.pid"
MONITOR_PID_FILE = CONFIG_DIR / "monitor.pid"
PLAYBACK_PID_FILE = CONFIG_DIR / "playback.pid"

_CACHED_NODE_ID = None
_PLAYBACK_PROC = None

RNNOISE_CANDIDATE_PATHS = [
    "/usr/lib/ladspa/librnnoise_ladspa.so",
    "/usr/lib64/ladspa/librnnoise_ladspa.so",
    "/usr/local/lib/ladspa/librnnoise_ladspa.so",
]

def find_rnnoise_plugin():
    for p in RNNOISE_CANDIDATE_PATHS:
        if os.path.exists(p):
            return p
    return None

def send_notification(title: str, message: str, urgency: str = "normal"):
    try:
        subprocess.run(
            [
                "notify-send",
                "-a", "micctl",
                "-h", "string:x-canonical-private-synchronous:micctl",
                "-u", urgency,
                title,
                message
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL
        )
    except Exception:
        pass

def get_capture_node_id() -> int | None:
    """Finds the PipeWire object ID for micctl_capture."""
    global _CACHED_NODE_ID
    if _CACHED_NODE_ID is not None:
        return _CACHED_NODE_ID
    try:
        out = subprocess.check_output(["pw-dump"], text=True)
        dump = json.loads(out)
        for obj in dump:
            if obj.get("info", {}).get("props", {}).get("node.name") == "micctl_capture":
                _CACHED_NODE_ID = obj["id"]
                return _CACHED_NODE_ID
    except Exception:
        pass
    return None

def update_live_eq(bass: float, mid: float, treble: float) -> bool:
    """Ultra-fast (0ms latency) in-memory PipeWire kernel param update."""
    node_id = get_capture_node_id()
    if not node_id:
        return False
    cmd = [
        "pw-cli", "s", str(node_id), "Props",
        f'{{ "params": [ "eq_bass:Gain", {bass:.1f}, "eq_mid:Gain", {mid:.1f}, "eq_treble:Gain", {treble:.1f} ] }}'
    ]
    res = subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return res.returncode == 0

def generate_filter_conf(state: dict) -> str:
    """Generates PipeWire SPA-JSON configuration for filter-chain."""
    nodes = []
    fx = state.get("fx_mode", "none")

    if fx == "ultra_nuke":
        # Ultra Nuke / Kıyamet Modu: Multi-stage sub bass saturation (+50 dB total boost)
        nodes = [
            {
                "type": "builtin",
                "name": "u_sub60",
                "label": "bq_lowshelf",
                "control": '{ "Freq" = 60.0 "Q" = 2.5 "Gain" = 18.0 }'
            },
            {
                "type": "builtin",
                "name": "u_bass100",
                "label": "bq_lowshelf",
                "control": '{ "Freq" = 100.0 "Q" = 2.0 "Gain" = 18.0 }'
            },
            {
                "type": "builtin",
                "name": "u_bass160",
                "label": "bq_lowshelf",
                "control": '{ "Freq" = 160.0 "Q" = 1.5 "Gain" = 15.0 }'
            },
            {
                "type": "builtin",
                "name": "u_mid_dist",
                "label": "bq_peaking",
                "control": '{ "Freq" = 1400.0 "Q" = 2.5 "Gain" = 15.0 }'
            },
            {
                "type": "builtin",
                "name": "u_high_air",
                "label": "bq_highshelf",
                "control": '{ "Freq" = 5000.0 "Q" = 1.0 "Gain" = 10.0 }'
            }
        ]
    elif fx == "nuke":
        # Standard Nuke Mode
        nodes = [
            {
                "type": "builtin",
                "name": "nuke_sub",
                "label": "bq_lowshelf",
                "control": '{ "Freq" = 80.0 "Q" = 2.0 "Gain" = 15.0 }'
            },
            {
                "type": "builtin",
                "name": "nuke_bass",
                "label": "bq_lowshelf",
                "control": '{ "Freq" = 150.0 "Q" = 1.5 "Gain" = 15.0 }'
            },
            {
                "type": "builtin",
                "name": "nuke_mid",
                "label": "bq_peaking",
                "control": '{ "Freq" = 1200.0 "Q" = 2.0 "Gain" = 12.0 }'
            },
            {
                "type": "builtin",
                "name": "nuke_high",
                "label": "bq_highshelf",
                "control": '{ "Freq" = 4500.0 "Q" = 1.0 "Gain" = 8.0 }'
            }
        ]
    elif fx == "robot":
        # Metallic Robot / Cybernetic Dalek Mode (Narrow comb resonance)
        nodes = [
            {
                "type": "builtin",
                "name": "rob_hp",
                "label": "bq_highpass",
                "control": '{ "Freq" = 180.0 "Q" = 1.0 }'
            },
            {
                "type": "builtin",
                "name": "rob_res1",
                "label": "bq_peaking",
                "control": '{ "Freq" = 450.0 "Q" = 7.0 "Gain" = 14.0 }'
            },
            {
                "type": "builtin",
                "name": "rob_res2",
                "label": "bq_peaking",
                "control": '{ "Freq" = 950.0 "Q" = 8.0 "Gain" = 16.0 }'
            },
            {
                "type": "builtin",
                "name": "rob_res3",
                "label": "bq_peaking",
                "control": '{ "Freq" = 1900.0 "Q" = 7.0 "Gain" = 14.0 }'
            },
            {
                "type": "builtin",
                "name": "rob_lp",
                "label": "bq_lowpass",
                "control": '{ "Freq" = 3600.0 "Q" = 1.0 }'
            }
        ]
    elif fx == "rabbit":
        # Rabbit / Chipmunk / Helium Mode (High-pitch formant resonance)
        nodes = [
            {
                "type": "builtin",
                "name": "rab_hp",
                "label": "bq_highpass",
                "control": '{ "Freq" = 550.0 "Q" = 1.5 }'
            },
            {
                "type": "builtin",
                "name": "rab_cut_bass",
                "label": "bq_lowshelf",
                "control": '{ "Freq" = 400.0 "Q" = 0.707 "Gain" = -18.0 }'
            },
            {
                "type": "builtin",
                "name": "rab_nasal",
                "label": "bq_peaking",
                "control": '{ "Freq" = 2600.0 "Q" = 3.0 "Gain" = 14.0 }'
            },
            {
                "type": "builtin",
                "name": "rab_squeak",
                "label": "bq_highshelf",
                "control": '{ "Freq" = 4800.0 "Q" = 1.5 "Gain" = 16.0 }'
            }
        ]
    elif fx == "demon":
        # Demon / Monster Deep Growl Mode (Dark sub-bass & muffled high end)
        nodes = [
            {
                "type": "builtin",
                "name": "dem_sub",
                "label": "bq_lowshelf",
                "control": '{ "Freq" = 75.0 "Q" = 2.0 "Gain" = 18.0 }'
            },
            {
                "type": "builtin",
                "name": "dem_growl",
                "label": "bq_peaking",
                "control": '{ "Freq" = 190.0 "Q" = 2.5 "Gain" = 14.0 }'
            },
            {
                "type": "builtin",
                "name": "dem_lp",
                "label": "bq_lowpass",
                "control": '{ "Freq" = 1100.0 "Q" = 1.2 }'
            },
            {
                "type": "builtin",
                "name": "dem_cut_high",
                "label": "bq_highshelf",
                "control": '{ "Freq" = 1800.0 "Q" = 0.707 "Gain" = -16.0 }'
            }
        ]
    elif fx == "walkie_talkie":
        nodes = [
            {
                "type": "builtin",
                "name": "hp_radio",
                "label": "bq_highpass",
                "control": '{ "Freq" = 500.0 "Q" = 1.0 }'
            },
            {
                "type": "builtin",
                "name": "lp_radio",
                "label": "bq_lowpass",
                "control": '{ "Freq" = 3200.0 "Q" = 1.0 }'
            },
            {
                "type": "builtin",
                "name": "peak_radio1",
                "label": "bq_peaking",
                "control": '{ "Freq" = 1800.0 "Q" = 3.0 "Gain" = 10.0 }'
            },
            {
                "type": "builtin",
                "name": "peak_radio2",
                "label": "bq_peaking",
                "control": '{ "Freq" = 2600.0 "Q" = 2.5 "Gain" = 6.0 }'
            }
        ]
    elif fx == "telephone":
        nodes = [
            {
                "type": "builtin",
                "name": "hp_phone",
                "label": "bq_highpass",
                "control": '{ "Freq" = 300.0 "Q" = 1.0 }'
            },
            {
                "type": "builtin",
                "name": "lp_phone",
                "label": "bq_lowpass",
                "control": '{ "Freq" = 3400.0 "Q" = 1.0 }'
            },
            {
                "type": "builtin",
                "name": "peak_phone",
                "label": "bq_peaking",
                "control": '{ "Freq" = 1200.0 "Q" = 1.5 "Gain" = 6.0 }'
            }
        ]
    else:
        # Standard Studio EQ Chain (with dynamically adjustable control ports)
        if state.get("lowcut", True):
            nodes.append({
                "type": "builtin",
                "name": "eq_hp",
                "label": "bq_highpass",
                "control": '{ "Freq" = 80.0 "Q" = 0.707 }'
            })

        bass_val = float(state.get("bass", 3.0))
        bass_freq = float(state.get("bass_freq", 120.0))
        nodes.append({
            "type": "builtin",
            "name": "eq_bass",
            "label": "bq_lowshelf",
            "control": f'{{ "Freq" = {bass_freq:.1f} "Q" = 0.707 "Gain" = {bass_val:.1f} }}'
        })

        mid_val = float(state.get("mid", 0.0))
        mid_freq = float(state.get("mid_freq", 1800.0))
        nodes.append({
            "type": "builtin",
            "name": "eq_mid",
            "label": "bq_peaking",
            "control": f'{{ "Freq" = {mid_freq:.1f} "Q" = 1.0 "Gain" = {mid_val:.1f} }}'
        })

        treble_val = float(state.get("treble", 1.5))
        treble_freq = float(state.get("treble_freq", 6000.0))
        nodes.append({
            "type": "builtin",
            "name": "eq_treble",
            "label": "bq_highshelf",
            "control": f'{{ "Freq" = {treble_freq:.1f} "Q" = 0.707 "Gain" = {treble_val:.1f} }}'
        })

        rnnoise_plugin = find_rnnoise_plugin()
        if state.get("noise_suppression", False) and rnnoise_plugin:
            nodes.insert(1, {
                "type": "ladspa",
                "name": "rnnoise",
                "plugin": "librnnoise_ladspa",
                "label": "noise_suppressor_stereo",
                "control": '{ "VAD Threshold (%)" = 50.0 }'
            })

    links = []
    for i in range(len(nodes) - 1):
        links.append(f'{{ output = "{nodes[i]["name"]}:Out" input = "{nodes[i+1]["name"]}:In" }}')

    nodes_str = "\n".join([
        f"""                    {{
                        type    = {n['type']}
                        name    = {n['name']}
                        {"plugin  = " + chr(34) + n['plugin'] + chr(34) if 'plugin' in n else ""}
                        label   = {n['label']}
                        control = {n['control']}
                    }}""" for n in nodes
    ])

    links_str = "\n".join([f"                    {link}" for link in links])

    conf_template = f"""# micctl auto-generated configuration
context.properties = {{
    log.level = 0
}}

context.spa-libs = {{
    audio.convert.* = audioconvert/libspa-audioconvert
    support.*       = support/libspa-support
}}

context.modules = [
    {{ name = libpipewire-module-protocol-native }}
    {{ name = libpipewire-module-client-node }}
    {{ name = libpipewire-module-adapter }}
    {{ name = libpipewire-module-filter-chain
        args = {{
            node.description = "Enhanced Microphone (micctl)"
            media.name       = "micctl Virtual Source"
            filter.graph = {{
                nodes = [
{nodes_str}
                ]
                links = [
{links_str}
                ]
            }}
            audio.channels = 2
            audio.position = [ FL FR ]
            capture.props = {{
                node.name    = "micctl_capture"
                node.passive = true
            }}
            playback.props = {{
                node.name        = "micctl_source"
                node.description = "Enhanced Microphone (micctl)"
                media.class      = "Audio/Source"
            }}
        }}
    }}
]
"""
    return conf_template

def is_daemon_running() -> bool:
    if not PID_FILE.exists():
        return False
    try:
        pid = int(PID_FILE.read_text().strip())
        os.kill(pid, 0)
        return True
    except (ValueError, OSError):
        if PID_FILE.exists():
            PID_FILE.unlink(missing_ok=True)
        return False

def get_daemon_pid() -> int | None:
    if is_daemon_running():
        try:
            return int(PID_FILE.read_text().strip())
        except ValueError:
            return None
    return None

def write_config(state: dict):
    CONFIG_DIR.mkdir(parents=True, exist_ok=True)
    content = generate_filter_conf(state)
    FILTER_CONF.write_text(content, encoding="utf-8")

def apply_gain(gain_pct: int):
    gain_pct = max(0, min(200, int(gain_pct)))
    subprocess.run(
        ["pactl", "set-source-volume", "micctl_source", f"{gain_pct}%"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL
    )

def is_muted() -> bool:
    res = subprocess.run(["pactl", "get-source-mute", "micctl_source"], capture_output=True, text=True)
    return "yes" in res.stdout.lower()

def set_mute(val: bool | None = None) -> bool:
    if val is None:
        arg = "toggle"
    else:
        arg = "1" if val else "0"
    subprocess.run(["pactl", "set-source-mute", "micctl_source", arg], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    muted = is_muted()
    status_str = "Susturuldu (Muted)" if muted else "Açıldı (Unmuted)"
    send_notification("🎙️ Mikrofon", status_str)
    return muted

def ensure_soundboard():
    sinks = subprocess.run(["pactl", "list", "sinks", "short"], capture_output=True, text=True).stdout
    if "micctl_soundboard" not in sinks:
        subprocess.run([
            "pactl", "load-module", "module-null-sink",
            "sink_name=micctl_soundboard",
            "sink_properties=device.description=micctl_Soundboard"
        ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        time.sleep(0.1)

    subprocess.run(["pw-link", "micctl_soundboard:monitor_FL", "micctl_capture:input_FL"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    subprocess.run(["pw-link", "micctl_soundboard:monitor_FR", "micctl_capture:input_FR"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    def_sink = subprocess.run(["pactl", "get-default-sink"], capture_output=True, text=True).stdout.strip()
    if def_sink and def_sink != "micctl_soundboard":
        subprocess.run(["pw-link", "micctl_soundboard:monitor_FL", f"{def_sink}:playback_FL"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        subprocess.run(["pw-link", "micctl_soundboard:monitor_FR", f"{def_sink}:playback_FR"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

def play_audio_to_mic(file_path: str, volume: int = 100) -> bool:
    global _PLAYBACK_PROC
    p = Path(file_path).expanduser().resolve()
    if not p.exists():
        return False

    ensure_soundboard()
    stop_audio_playback()

    cmd = [
        "mpv",
        "--no-config",
        "--keep-open=no",
        "--no-video",
        "--audio-device=pulse/micctl_soundboard",
        f"--volume={volume}",
        str(p)
    ]
    _PLAYBACK_PROC = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    PLAYBACK_PID_FILE.write_text(str(_PLAYBACK_PROC.pid))
    return True

def stop_audio_playback() -> bool:
    global _PLAYBACK_PROC
    if _PLAYBACK_PROC is not None:
        try:
            _PLAYBACK_PROC.terminate()
            _PLAYBACK_PROC.wait(timeout=0.2)
        except Exception:
            pass
        _PLAYBACK_PROC = None

    if PLAYBACK_PID_FILE.exists():
        try:
            pid = int(PLAYBACK_PID_FILE.read_text().strip())
            os.kill(pid, signal.SIGTERM)
        except (ValueError, OSError):
            pass
        PLAYBACK_PID_FILE.unlink(missing_ok=True)
    subprocess.run(["pkill", "-f", "mpv.*micctl_soundboard"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return True

def is_audio_playing() -> bool:
    global _PLAYBACK_PROC
    if _PLAYBACK_PROC is not None:
        if _PLAYBACK_PROC.poll() is not None:
            _PLAYBACK_PROC = None
            PLAYBACK_PID_FILE.unlink(missing_ok=True)
            return False
        return True

    if not PLAYBACK_PID_FILE.exists():
        return False
    try:
        pid = int(PLAYBACK_PID_FILE.read_text().strip())
        stat_path = Path(f"/proc/{pid}/stat")
        if not stat_path.exists():
            PLAYBACK_PID_FILE.unlink(missing_ok=True)
            return False
        state = stat_path.read_text().split()[2]
        if state == 'Z':
            PLAYBACK_PID_FILE.unlink(missing_ok=True)
            return False
        return True
    except Exception:
        PLAYBACK_PID_FILE.unlink(missing_ok=True)
        return False

def play_sfx(name: str):
    sfx.ensure_sounds()
    sound_files = {
        "roger": sfx.SOUNDS_DIR / "roger_beep.wav",
        "nuke": sfx.SOUNDS_DIR / "nuke_alarm.wav",
        "airhorn": sfx.SOUNDS_DIR / "airhorn.wav",
        "beep": sfx.SOUNDS_DIR / "beep.wav"
    }
    target = sound_files.get(name.lower())
    if target and target.exists():
        play_audio_to_mic(str(target), volume=100)

def start_daemon(state: dict) -> bool:
    global _CACHED_NODE_ID
    _CACHED_NODE_ID = None
    if is_daemon_running():
        ensure_soundboard()
        return True

    write_config(state)
    proc = subprocess.Popen(
        ["pipewire", "-c", str(FILTER_CONF)],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True
    )
    PID_FILE.write_text(str(proc.pid))

    for _ in range(10):
        time.sleep(0.1)
        check = subprocess.run(["pactl", "list", "sources", "short"], capture_output=True, text=True)
        if "micctl_source" in check.stdout:
            apply_gain(state.get("gain", 100))
            ensure_soundboard()
            return True
    return False

def stop_daemon() -> bool:
    global _CACHED_NODE_ID
    _CACHED_NODE_ID = None
    pid = get_daemon_pid()
    if pid:
        try:
            os.kill(pid, signal.SIGTERM)
            time.sleep(0.1)
        except OSError:
            pass
        PID_FILE.unlink(missing_ok=True)
    stop_monitoring()
    stop_audio_playback()
    return True

def restart_daemon(state: dict) -> bool:
    stop_daemon()
    return start_daemon(state)

def is_monitoring() -> bool:
    if not MONITOR_PID_FILE.exists():
        return False
    try:
        pid = int(MONITOR_PID_FILE.read_text().strip())
        os.kill(pid, 0)
        return True
    except (ValueError, OSError):
        MONITOR_PID_FILE.unlink(missing_ok=True)
        return False

def start_monitoring() -> bool:
    if is_monitoring():
        return True
    proc = subprocess.Popen(
        ["pw-loopback", "-C", "micctl_source"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True
    )
    MONITOR_PID_FILE.write_text(str(proc.pid))
    return True

def stop_monitoring() -> bool:
    if not MONITOR_PID_FILE.exists():
        return True
    try:
        pid = int(MONITOR_PID_FILE.read_text().strip())
        os.kill(pid, signal.SIGTERM)
    except OSError:
        pass
    MONITOR_PID_FILE.unlink(missing_ok=True)
    return True

def set_as_default_mic() -> bool:
    res = subprocess.run(
        ["pactl", "set-default-source", "micctl_source"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL
    )
    return res.returncode == 0

def is_systemd_enabled() -> bool:
    res = subprocess.run(["systemctl", "--user", "is-enabled", "micctl"], capture_output=True, text=True)
    return "enabled" in res.stdout

def set_systemd_autostart(enable: bool) -> bool:
    cmd = "enable" if enable else "disable"
    res = subprocess.run(["systemctl", "--user", cmd, "micctl"], capture_output=True, text=True)
    return res.returncode == 0
