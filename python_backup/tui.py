"""
micctl - Interactive Curses Terminal User Interface (Zero-Lag Optimized)
"""
import curses
import time
import subprocess
from pathlib import Path
import dsp
import state as state_module

def make_slider(val: float, min_v: float, max_v: float, width: int = 24) -> str:
    ratio = (val - min_v) / (max_v - min_v)
    ratio = max(0.0, min(1.0, ratio))
    pos = int(ratio * (width - 1))
    bar = ["-"] * width
    zero_ratio = (0.0 - min_v) / (max_v - min_v)
    if 0.0 <= zero_ratio <= 1.0:
        zero_pos = int(zero_ratio * (width - 1))
        bar[zero_pos] = "|"
    bar[pos] = "█"
    return "[" + "".join(bar) + "]"

def tui_main(stdscr):
    curses.curs_set(0)
    stdscr.nodelay(True)
    curses.start_color()
    curses.use_default_colors()

    # Colors
    curses.init_pair(1, curses.COLOR_CYAN, -1)     # Headers
    curses.init_pair(2, curses.COLOR_GREEN, -1)    # Success / Active
    curses.init_pair(3, curses.COLOR_YELLOW, -1)   # Warning
    curses.init_pair(4, curses.COLOR_BLACK, curses.COLOR_CYAN) # Selected row
    curses.init_pair(5, curses.COLOR_RED, -1)      # Stopped / Error / Nuke
    curses.init_pair(6, curses.COLOR_WHITE, curses.COLOR_RED) # Nuke banner

    st = state_module.load_state()
    presets = state_module.list_presets()
    fx_list = ["none", "nuke", "ultra_nuke", "robot", "rabbit", "demon", "walkie_talkie", "telephone"]
    selected = 0
    items = [
        "fx_mode",
        "soundboard",
        "bass",
        "mid",
        "treble",
        "gain",
        "lowcut",
        "noise",
        "listen",
        "mute",
        "preset",
        "daemon"
    ]

    pending_save = False
    last_change_time = 0
    pending_fx_reload = False
    status_msg = ""
    status_msg_time = 0

    while True:
        stdscr.erase()
        h, w = stdscr.getmaxyx()

        is_running = dsp.is_daemon_running()
        is_mon = dsp.is_monitoring()
        is_muted = dsp.is_muted()
        is_playing = dsp.is_audio_playing()
        rnnoise_ok = dsp.find_rnnoise_plugin() is not None
        fx = st.get("fx_mode", "none")

        # Title
        title = "🎙️  micctl — Mikrofon DSP, Bass & Soundboard (Ultra-Hızlı)"
        if w > len(title):
            stdscr.addstr(1, (w - len(title)) // 2, title, curses.color_pair(1) | curses.A_BOLD)

        # Status Line
        status_text = "Motor: AKTİF " if is_running else "Motor: DURDURULDU "
        status_color = curses.color_pair(2) if is_running else curses.color_pair(5)
        stdscr.addstr(3, 4, status_text, status_color | curses.A_BOLD)

        if is_muted:
            stdscr.addstr(3, 20, "[ ● SUSTURULDU ]", curses.color_pair(5) | curses.A_BOLD)
        else:
            stdscr.addstr(3, 20, "[ ○ Mikrofon Açık ]", curses.color_pair(2))

        if fx == "ultra_nuke":
            stdscr.addstr(3, 42, "💥💥 ULTRA NUKE (KIYAMET) 💥💥", curses.color_pair(6) | curses.A_BOLD)
        elif fx == "nuke":
            stdscr.addstr(3, 42, "💥 NUKE AKTİF 💥", curses.color_pair(6) | curses.A_BOLD)
        elif fx == "robot":
            stdscr.addstr(3, 42, "🤖 ROBOT SESİ", curses.color_pair(1) | curses.A_BOLD)
        elif fx == "rabbit":
            stdscr.addstr(3, 42, "🐰 TAVŞAN / HELYUM", curses.color_pair(3) | curses.A_BOLD)
        elif fx == "demon":
            stdscr.addstr(3, 42, "👹 CANAVAR / DEMON", curses.color_pair(5) | curses.A_BOLD)
        elif is_playing:
            stdscr.addstr(3, 42, "🔊 Ses Çalınıyor...", curses.color_pair(2))
        elif is_mon:
            stdscr.addstr(3, 42, "● Dinleme Açık", curses.color_pair(3))

        if status_msg and (time.time() - status_msg_time < 2.5):
            stdscr.addstr(3, max(4, w - len(status_msg) - 4), status_msg, curses.color_pair(2) | curses.A_BOLD)

        stdscr.addstr(4, 4, "─" * min(w - 8, 74), curses.A_DIM)

        # Draw rows
        start_y = 5
        for idx, item in enumerate(items):
            y = start_y + idx
            if y >= h - 4:
                break

            is_sel = (idx == selected)
            prefix = " ❯ " if is_sel else "   "
            row_attr = curses.color_pair(4) | curses.A_BOLD if is_sel else curses.A_NORMAL

            if item == "fx_mode":
                fx_names = {
                    "none": "Normal (EQ)",
                    "nuke": "💥 Nuke (Kulak Patlatan)",
                    "ultra_nuke": "💥💥 Ultra Nuke (Kıyamet)",
                    "robot": "🤖 Robot (Metalik)",
                    "rabbit": "🐰 Tavşan / Helyum",
                    "demon": "👹 Canavar / Demon",
                    "walkie_talkie": "📻 Telsiz / Megafon",
                    "telephone": "☎️ Eski Telefon"
                }
                curr_lbl = fx_names.get(fx, fx)
                text = f"{prefix}Özel FX Modu          ◀  {curr_lbl:<25}  ▶"
            elif item == "soundboard":
                if is_playing:
                    lbl = "[ 🔊 ÇALINIYOR... (S:Durdur) ]"
                else:
                    lbl = "[ 1:Roger  2:Nuke  3:Horn  4:Bip ]"
                text = f"{prefix}Soundboard / SFX      {lbl}"
            elif item == "bass":
                val = st.get("bass", 0.0)
                slider = make_slider(val, -20.0, 30.0, 20)
                text = f"{prefix}Bass (-20..+30dB)     {slider}  {val:+5.1f} dB"
            elif item == "mid":
                val = st.get("mid", 0.0)
                slider = make_slider(val, -18.0, 20.0, 20)
                text = f"{prefix}Mid (-18..+20dB)      {slider}  {val:+5.1f} dB"
            elif item == "treble":
                val = st.get("treble", 0.0)
                slider = make_slider(val, -18.0, 20.0, 20)
                text = f"{prefix}Tiz (-18..+20dB)      {slider}  {val:+5.1f} dB"
            elif item == "gain":
                val = st.get("gain", 100)
                slider = make_slider(val, 0, 200, 20)
                text = f"{prefix}Çıkış Kazancı (Gain)  {slider}  {val:4d} % "
            elif item == "lowcut":
                on = st.get("lowcut", True)
                state_lbl = "[ AÇIK (80 Hz) ]" if on else "[ KAPALI ]"
                text = f"{prefix}Low-cut Filtresi      {state_lbl}"
            elif item == "noise":
                on = st.get("noise_suppression", False)
                if not rnnoise_ok:
                    state_lbl = "[ KURULU DEĞİL ]"
                else:
                    state_lbl = "[ AÇIK (RNNoise AI) ]" if on else "[ KAPALI ]"
                text = f"{prefix}Gürültü Engelleme     {state_lbl}"
            elif item == "listen":
                state_lbl = "[ AÇIK (Kulaklık) ]" if is_mon else "[ KAPALI ]"
                text = f"{prefix}Canlı Monitör         {state_lbl}"
            elif item == "mute":
                state_lbl = "[ SUSTURULDU ]" if is_muted else "[ AÇIK ]"
                text = f"{prefix}Mikrofon Mute         {state_lbl}"
            elif item == "preset":
                p_name = st.get("preset", "default")
                text = f"{prefix}Profil Seçimi         ◀  {p_name:<14}  ▶"
            elif item == "daemon":
                state_lbl = "[ YENİDEN BAŞLAT ]" if is_running else "[ SERVİSİ BAŞLAT ]"
                text = f"{prefix}DSP Servisi           {state_lbl}"
            else:
                text = f"{prefix}{item}"

            stdscr.addstr(y, 4, text.ljust(min(w - 8, 72)), row_attr)

        # Footer
        footer_y = max(start_y + len(items) + 1, h - 3)
        stdscr.addstr(footer_y - 1, 4, "─" * min(w - 8, 74), curses.A_DIM)
        help_text = "[1-4] SFX [N] Nuke [U] Ultra [R] Sıfırla [T] Test [X] Mute [Q] Çıkış"
        stdscr.addstr(footer_y, 4, help_text[:w-8], curses.color_pair(3))

        stdscr.refresh()

        now = time.time()
        # Debounced disk save (doesn't block UI or audio)
        if pending_save and (now - last_change_time > 0.4):
            state_module.save_state(st)
            pending_save = False

        # FX Mode reload (only happens when switching FX modes)
        if pending_fx_reload and (now - last_change_time > 0.2):
            state_module.save_state(st)
            if is_running:
                dsp.restart_daemon(st)
            dsp.apply_gain(st.get("gain", 100))
            pending_fx_reload = False

        try:
            ch = stdscr.getch()
        except Exception:
            ch = -1

        if ch == -1:
            time.sleep(0.02)
            continue

        if ch in [ord('q'), ord('Q'), 27]:
            break

        current_item = items[selected]

        # Soundboard Hotkeys
        if ch == ord('1'):
            dsp.play_sfx("roger")
            status_msg = "📻 Roger Beep Çalındı"
            status_msg_time = now
            continue
        if ch == ord('2'):
            dsp.play_sfx("nuke")
            status_msg = "💥 Nuke Alarm Çalındı"
            status_msg_time = now
            continue
        if ch == ord('3'):
            dsp.play_sfx("airhorn")
            status_msg = "📯 Airhorn Çalındı"
            status_msg_time = now
            continue
        if ch == ord('4'):
            dsp.play_sfx("beep")
            status_msg = "🔔 Bip Çalındı"
            status_msg_time = now
            continue
        if ch in [ord('s'), ord('S')]:
            dsp.stop_audio_playback()
            status_msg = "✓ Ses Durduruldu"
            status_msg_time = now
            continue

        # Hotkeys
        if ch in [ord('r'), ord('R')]:
            st = state_module.reset_state()
            dsp.set_mute(False)
            dsp.stop_audio_playback()
            if is_running:
                dsp.restart_daemon(st)
            dsp.apply_gain(100)
            status_msg = "✓ Varsayılana Sıfırlandı!"
            status_msg_time = now
            continue

        if ch in [ord('u'), ord('U')]:
            if st.get("fx_mode") == "ultra_nuke":
                b = st.get("backup_state") or {}
                st["fx_mode"] = "none"
                st["gain"] = b.get("gain", 100)
                st["preset"] = b.get("preset", "default")
                status_msg = "Ultra Nuke Kapatıldı"
            else:
                st["backup_state"] = {
                    "bass": st.get("bass", 3.0),
                    "mid": st.get("mid", 0.0),
                    "treble": st.get("treble", 1.5),
                    "gain": st.get("gain", 100),
                    "preset": st.get("preset", "default")
                }
                st["fx_mode"] = "ultra_nuke"
                st["preset"] = "ultra-nuke"
                st["gain"] = 200
                dsp.play_sfx("nuke")
                status_msg = "💥💥 ULTRA NUKE AKTİF! 💥💥"
            status_msg_time = now
            pending_fx_reload = True
            last_change_time = now
            continue

        if ch in [ord('n'), ord('N')]:
            if st.get("fx_mode") == "nuke":
                b = st.get("backup_state") or {}
                st["fx_mode"] = "none"
                st["gain"] = b.get("gain", 100)
                st["preset"] = b.get("preset", "default")
                status_msg = "Nuke Kapatıldı"
            else:
                st["backup_state"] = {
                    "bass": st.get("bass", 3.0),
                    "mid": st.get("mid", 0.0),
                    "treble": st.get("treble", 1.5),
                    "gain": st.get("gain", 100),
                    "preset": st.get("preset", "default")
                }
                st["fx_mode"] = "nuke"
                st["preset"] = "nuke"
                st["gain"] = 150
                dsp.play_sfx("nuke")
                status_msg = "💥 NUKE AKTİF!"
            status_msg_time = now
            pending_fx_reload = True
            last_change_time = now
            continue

        if ch in [ord('x'), ord('X')]:
            dsp.set_mute(None)
            continue

        if ch in [ord('d'), ord('D')]:
            if not is_running:
                dsp.start_daemon(st)
            dsp.set_as_default_mic()
            status_msg = "Varsayılan Yapıldı"
            status_msg_time = now
            continue

        if ch in [ord('l'), ord('L')]:
            if dsp.is_monitoring():
                dsp.stop_monitoring()
            else:
                if not is_running:
                    dsp.start_daemon(st)
                dsp.start_monitoring()
            continue

        if ch in [ord('t'), ord('T')]:
            status_msg = "🎙️ Kaydediliyor (2sn)..."
            status_msg_time = now
            stdscr.addstr(3, max(4, w - len(status_msg) - 4), status_msg, curses.color_pair(5) | curses.A_BOLD)
            stdscr.refresh()
            tmp_wav = Path("/tmp/micctl_test_sample.wav")
            tmp_wav.unlink(missing_ok=True)
            p = subprocess.Popen(["pw-record", "--target", "micctl_source", str(tmp_wav)])
            time.sleep(2)
            p.terminate()
            p.wait()
            status_msg = "🔊 Kulaklığa Çalınıyor..."
            status_msg_time = time.time()
            stdscr.addstr(3, max(4, w - len(status_msg) - 4), status_msg, curses.color_pair(2) | curses.A_BOLD)
            stdscr.refresh()
            subprocess.run(["pw-play", str(tmp_wav)])
            tmp_wav.unlink(missing_ok=True)
            status_msg = "✓ Test Bitti"
            status_msg_time = time.time()
            continue

        if ch == curses.KEY_UP or ch == ord('k'):
            selected = (selected - 1) % len(items)
        elif ch == curses.KEY_DOWN or ch == ord('j'):
            selected = (selected + 1) % len(items)
        elif ch in [curses.KEY_RIGHT, curses.KEY_LEFT, ord('h'), ord('l'), ord(' '), 10, 13]:
            is_right = ch in [curses.KEY_RIGHT, ord('l')]
            is_space = ch in [ord(' '), 10, 13]
            delta = 0.5 if is_right else -0.5

            if current_item == "bass":
                st["bass"] = max(-20.0, min(30.0, round(st.get("bass", 0.0) + delta, 1)))
                st["preset"] = "özel"
                if is_running and st.get("fx_mode", "none") == "none":
                    dsp.update_live_eq(st["bass"], st["mid"], st["treble"])
                pending_save = True
                last_change_time = now
            elif current_item == "mid":
                st["mid"] = max(-18.0, min(20.0, round(st.get("mid", 0.0) + delta, 1)))
                st["preset"] = "özel"
                if is_running and st.get("fx_mode", "none") == "none":
                    dsp.update_live_eq(st["bass"], st["mid"], st["treble"])
                pending_save = True
                last_change_time = now
            elif current_item == "treble":
                st["treble"] = max(-18.0, min(20.0, round(st.get("treble", 0.0) + delta, 1)))
                st["preset"] = "özel"
                if is_running and st.get("fx_mode", "none") == "none":
                    dsp.update_live_eq(st["bass"], st["mid"], st["treble"])
                pending_save = True
                last_change_time = now
            elif current_item == "gain":
                g_delta = 5 if is_right else -5
                st["gain"] = max(0, min(200, st.get("gain", 100) + g_delta))
                dsp.apply_gain(st["gain"])
                pending_save = True
                last_change_time = now
            elif current_item == "fx_mode":
                idx_fx = fx_list.index(fx) if fx in fx_list else 0
                new_idx = (idx_fx + (1 if is_right else -1)) % len(fx_list)
                st["fx_mode"] = fx_list[new_idx]
                if st["fx_mode"] == "walkie_talkie":
                    dsp.play_sfx("roger")
                elif "nuke" in st["fx_mode"]:
                    dsp.play_sfx("nuke")
                pending_fx_reload = True
                last_change_time = now
            elif current_item == "soundboard":
                dsp.play_sfx("roger")
                status_msg = "📻 Roger Beep Çalındı"
                status_msg_time = now
            elif current_item == "lowcut" and (is_space or ch in [curses.KEY_RIGHT, curses.KEY_LEFT]):
                st["lowcut"] = not st.get("lowcut", True)
                pending_fx_reload = True
                last_change_time = now
            elif current_item == "noise" and (is_space or ch in [curses.KEY_RIGHT, curses.KEY_LEFT]):
                if rnnoise_ok:
                    st["noise_suppression"] = not st.get("noise_suppression", False)
                    pending_fx_reload = True
                    last_change_time = now
            elif current_item == "listen" and (is_space or ch in [curses.KEY_RIGHT, curses.KEY_LEFT]):
                if dsp.is_monitoring():
                    dsp.stop_monitoring()
                else:
                    if not is_running:
                        dsp.start_daemon(st)
                    dsp.start_monitoring()
            elif current_item == "mute" and (is_space or ch in [curses.KEY_RIGHT, curses.KEY_LEFT]):
                dsp.set_mute(None)
            elif current_item == "preset":
                if presets:
                    curr_p = st.get("preset", presets[0])
                    idx_p = presets.index(curr_p) if curr_p in presets else 0
                    new_idx = (idx_p + (1 if is_right else -1)) % len(presets)
                    new_p = presets[new_idx]
                    try:
                        p_data = state_module.load_preset(new_p)
                        st.update(p_data)
                        st["preset"] = new_p
                        pending_fx_reload = True
                        last_change_time = now
                    except Exception:
                        pass
            elif current_item == "daemon" and (is_space or ch in [10, 13]):
                if is_running:
                    dsp.restart_daemon(st)
                else:
                    dsp.start_daemon(st)

    state_module.save_state(st)

def run_tui():
    curses.wrapper(tui_main)
