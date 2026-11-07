"""
micctl - Command Line Interface (High-Performance & Low-Latency)
"""
import sys
import json
import time
import subprocess
from pathlib import Path
from rich.console import Console
from rich.table import Table
from rich.progress import Progress, SpinnerColumn, TextColumn, BarColumn

import dsp
import state as state_module

console = Console()

def parse_num_arg(curr: float, raw_args: list[str], min_v: float, max_v: float) -> float:
    tokens = [a.lower().replace("set", "").replace("db", "").replace("%", "").strip() for a in raw_args]
    tokens = [t for t in tokens if t]
    if not tokens:
        return curr
    raw = tokens[-1]
    if raw.startswith("+") or (raw.startswith("-") and not (len(raw) > 1 and raw[1:].replace(".", "", 1).isdigit() and not raw_args[-1].startswith("-"))):
        val = curr + float(raw)
    else:
        val = float(raw)
    return max(min_v, min(max_v, round(val, 1)))

def get_current_default_source() -> str:
    try:
        res = subprocess.run(["pactl", "get-default-source"], capture_output=True, text=True)
        return res.stdout.strip()
    except Exception:
        return "Bilinmiyor"

def print_status(st: dict):
    is_running = dsp.is_daemon_running()
    pid = dsp.get_daemon_pid()
    default_src = get_current_default_source()
    is_default = "micctl_source" in default_src
    rnnoise_installed = dsp.find_rnnoise_plugin() is not None
    muted = dsp.is_muted()
    autostart = dsp.is_systemd_enabled()
    is_playing = dsp.is_audio_playing()

    fx = st.get("fx_mode", "none")

    table = Table(title="[bold cyan]🎙️ micctl - Mikrofon DSP & Soundboard Durumu[/bold cyan]", show_header=True, header_style="bold magenta")
    table.add_column("Parametre", style="dim", width=22)
    table.add_column("Değer", style="bold")
    table.add_column("Açıklama")

    status_str = f"[green]● Aktif[/green] (PID: {pid})" if is_running else "[red]○ Durduruldu[/red]"
    table.add_row("Filtre Motoru", status_str, "PipeWire filter-chain arka plan servisi")

    mute_str = "[red]● SUSTURULDU (MUTED)[/red]" if muted else "[green]○ Açık (Unmuted)[/green]"
    table.add_row("Mikrofon Durumu", mute_str, "'micctl mute toggle' ile sustur")

    def_mic_str = "[green]✓ micctl (Sanal Mikrofon)[/green]" if is_default else f"[yellow]⚠ {default_src}[/yellow]"
    table.add_row("Varsayılan Mikrofon", def_mic_str, "'micctl default' ile varsayılan yap" if not is_default else "Sistem geneli aktif")

    # Special FX Mode
    if fx == "ultra_nuke":
        fx_str = "[bold red blink]💥💥 ULTRA NUKE / KIYAMET (+50dB Bas)[/bold red blink]"
        table.add_row("Özel Efekt Modu", fx_str, "'micctl ultra' ile kapat")
    elif fx == "nuke":
        fx_str = "[bold red blink]💥 NUKE / KULAK PATLATAN (AKTİF)[/bold red blink]"
        table.add_row("Özel Efekt Modu", fx_str, "'micctl nuke' ile kapat")
    elif fx == "robot":
        fx_str = "[bold cyan]🤖 Metalik Robot[/bold cyan]"
        table.add_row("Özel Efekt Modu", fx_str, "Cybernetic Dalek / Robot filtresi")
    elif fx == "rabbit":
        fx_str = "[bold yellow]🐰 Tavşan / Helyum[/bold yellow]"
        table.add_row("Özel Efekt Modu", fx_str, "İnce ses, çizgi film tavşanı efekti")
    elif fx == "demon":
        fx_str = "[bold red]👹 Canavar / Demon[/bold red]"
        table.add_row("Özel Efekt Modu", fx_str, "Karanlık, derin canavar hırıltısı")
    elif fx == "walkie_talkie":
        fx_str = "[bold yellow]📻 Telsiz / Megafon[/bold yellow]"
        table.add_row("Özel Efekt Modu", fx_str, "Polis / Askeri telsiz efekti")
    elif fx == "telephone":
        fx_str = "[bold yellow]☎️ Eski Telefon[/bold yellow]"
        table.add_row("Özel Efekt Modu", fx_str, "Sabit hat telefon filtre bandı")
    else:
        table.add_row("Özel Efekt Modu", "[dim]Normal (Stüdyo EQ)[/dim]", "'micctl fx <mod>'")

    # Soundboard playback status
    sb_str = "[green]🔊 Çalıyor... ('micctl stop')[/green]" if is_playing else "[dim]Hazır ('micctl play <dosya>')[/dim]"
    table.add_row("Soundboard / Ses", sb_str, "Mikrofona ses dosyası çalma")

    # Audio params
    bass_val = st.get("bass", 0.0)
    bass_color = "green" if bass_val > 0 else ("yellow" if bass_val < 0 else "white")
    table.add_row("Bass (-20..+30dB)", f"[{bass_color}]{bass_val:+.1f} dB[/{bass_color}]", "Düşük frekans dolgunluğu")

    mid_val = st.get("mid", 0.0)
    mid_color = "green" if mid_val > 0 else ("yellow" if mid_val < 0 else "white")
    table.add_row("Mid (-18..+20dB)", f"[{mid_color}]{mid_val:+.1f} dB[/{mid_color}]", "Ses netliği ve konuşma açıklığı")

    treble_val = st.get("treble", 0.0)
    treble_color = "green" if treble_val > 0 else ("yellow" if treble_val < 0 else "white")
    table.add_row("Tiz (-18..+20dB)", f"[{treble_color}]{treble_val:+.1f} dB[/{treble_color}]", "Hava hissi ve parlaklık")

    lowcut_str = "[green]Açık (80 Hz)[/green]" if st.get("lowcut", True) else "[red]Kapalı[/red]"
    table.add_row("Low-cut Filtre", lowcut_str, "Masa titreşimi & derin uğultu kesici")

    noise_active = st.get("noise_suppression", False)
    if not rnnoise_installed:
        noise_str = "[dim]Kurulu Değil (pacman -S noise-suppression-for-voice)[/dim]"
    else:
        noise_str = "[green]Açık (RNNoise AI)[/green]" if noise_active else "[red]Kapalı[/red]"
    table.add_row("Gürültü Engelleme", noise_str, "Yapay zeka gürültü filtresi")

    gain_val = st.get("gain", 100)
    table.add_row("Çıkış Kazancı", f"{gain_val}%", "Sanal mikrofon ses seviyesi")

    mon_str = "[green]● Dinleniyor[/green]" if dsp.is_monitoring() else "[dim]Kapalı[/dim]"
    table.add_row("Canlı Monitör", mon_str, "Kendi sesini kulaklıktan duyma ('micctl listen on')")

    autostart_str = "[green]Açık (systemd)[/green]" if autostart else "[dim]Kapalı ('micctl autostart on')[/dim]"
    table.add_row("Açılışta Başlatma", autostart_str, "Bilgisayar açılışında otomatik çalışma")

    table.add_row("Aktif Profil", f"[cyan]{st.get('preset', 'default')}[/cyan]", "Hazır EQ şablonu")

    console.print(table)

def handle_nuke_toggle(st: dict, ultra: bool = False):
    target_mode = "ultra_nuke" if ultra else "nuke"
    curr_fx = st.get("fx_mode", "none")
    if curr_fx != target_mode:
        st["backup_state"] = {
            "bass": st.get("bass", 3.0),
            "mid": st.get("mid", 0.0),
            "treble": st.get("treble", 1.5),
            "gain": st.get("gain", 100),
            "lowcut": st.get("lowcut", True),
            "preset": st.get("preset", "default"),
            "fx_mode": "none"
        }
        st["fx_mode"] = target_mode
        st["preset"] = "ultra-nuke" if ultra else "nuke"
        st["gain"] = 200 if ultra else 150
        state_module.save_state(st)
        if not dsp.is_daemon_running():
            dsp.start_daemon(st)
        else:
            dsp.restart_daemon(st)
        dsp.apply_gain(st["gain"])
        dsp.play_sfx("nuke")
        title = "💥💥 ULTRA NUKE AKTİF!" if ultra else "💥 NUKE AKTİF!"
        dsp.send_notification(title, "Kulak Patlatan Modu Devrede!", "critical")
        console.print(f"[bold red blink]{title} 💥[/bold red blink]")
        console.print("[dim]Normale dönmek için: micctl nuke (veya micctl ultra)[/dim]")
    else:
        b = st.get("backup_state") or {}
        st["fx_mode"] = "none"
        st["bass"] = b.get("bass", 3.0)
        st["mid"] = b.get("mid", 0.0)
        st["treble"] = b.get("treble", 1.5)
        st["gain"] = b.get("gain", 100)
        st["lowcut"] = b.get("lowcut", True)
        st["preset"] = b.get("preset", "default")
        st["backup_state"] = None
        state_module.save_state(st)
        if dsp.is_daemon_running():
            dsp.restart_daemon(st)
        dsp.apply_gain(st["gain"])
        dsp.send_notification("🎙️ micctl", "Kulak Patlatan modu kapatıldı (Normal ses)")
        console.print("[green]✓ Kulak Patlatan modu kapatıldı, normal stüdyo tonuna dönüldü.[/green]")

def handle_reset():
    st = state_module.reset_state()
    dsp.set_mute(False)
    dsp.stop_audio_playback()
    if dsp.is_daemon_running():
        dsp.restart_daemon(st)
    dsp.apply_gain(100)
    dsp.play_sfx("beep")
    dsp.send_notification("🎙️ micctl", "Tüm ayarlar varsayılana sıfırlandı.")
    console.print("[green]✓ Tüm mikrofon ayarları, filtreler ve ses seviyesi varsayılana (Fabrika ayarlarına) sıfırlandı.[/green]")
    console.print("[dim]Varsayılan: Bass +3dB, Mid 0dB, Treble +1.5dB, Gain 100%, Low-cut Açık.[/dim]")

def handle_play(args: list[str]):
    if not args:
        console.print("[yellow]Kullanım:[/yellow] micctl play <ses_dosyası.mp3/wav/ogg> [ses_yüzdesi]")
        console.print("Örnek: micctl play ~/Müzik/meme.mp3 100")
        return

    target_file = args[0]
    vol = 100
    if len(args) >= 2 and args[1].isdigit():
        vol = max(10, min(150, int(args[1])))

    p = Path(target_file).expanduser().resolve()
    if not p.exists():
        console.print(f"[red]✗ Ses dosyası bulunamadı:[/red] {target_file}")
        return

    st = state_module.load_state()
    if not dsp.is_daemon_running():
        dsp.start_daemon(st)

    console.print(f"[bold cyan]🔊 Mikrofona çalınıyor:[/bold cyan] {p.name} (%{vol} ses)")
    console.print("[dim]Durdurmak için: micctl stop[/dim]")
    dsp.play_audio_to_mic(str(p), volume=vol)

def output_waybar_json(st: dict):
    muted = dsp.is_muted()
    is_running = dsp.is_daemon_running()
    is_playing = dsp.is_audio_playing()
    fx = st.get("fx_mode", "none")

    if not is_running:
        data = {"text": "🎙️ OFF", "tooltip": "micctl: Servis kapalı", "class": "stopped"}
    elif muted:
        data = {"text": "🎙️ MUTED", "tooltip": "micctl: Mikrofon susturuldu", "class": "muted"}
    elif is_playing:
        data = {"text": "🔊 PLAY", "tooltip": "micctl: Mikrofona ses çalınıyor", "class": "playing"}
    elif fx == "ultra_nuke":
        data = {"text": "💥 ULTRA", "tooltip": "micctl: Ultra Nuke Kıyamet Modu!", "class": "nuke"}
    elif fx == "nuke":
        data = {"text": "💥 NUKE", "tooltip": "micctl: Kulak Patlatan Modu Aktif!", "class": "nuke"}
    elif fx == "robot":
        data = {"text": "🤖 ROBOT", "tooltip": "micctl: Robot Modu", "class": "fx"}
    elif fx == "rabbit":
        data = {"text": "🐰 TAVŞAN", "tooltip": "micctl: Tavşan / Helyum Modu", "class": "fx"}
    elif fx == "demon":
        data = {"text": "👹 DEMON", "tooltip": "micctl: Canavar / Demon Modu", "class": "fx"}
    elif fx == "walkie_talkie":
        data = {"text": "📻 Telsiz", "tooltip": "micctl: Walkie-Talkie FX", "class": "fx"}
    else:
        bass = st.get("bass", 0.0)
        p = st.get("preset", "default")
        data = {
            "text": f"🎙️ {bass:+.1f}dB",
            "tooltip": f"micctl Aktif\nProfil: {p}\nBass: {bass:+.1f}dB\nGain: {st.get('gain', 100)}%",
            "class": "active"
        }
    print(json.dumps(data))

def handle_cli(args: list[str]):
    if not args:
        from tui import run_tui
        run_tui()
        return

    cmd = args[0].lower()
    st = state_module.load_state()

    if cmd in ["status", "s", "info"]:
        print_status(st)
        return

    if cmd in ["play", "çal", "soundboard"]:
        handle_play(args[1:])
        return

    if cmd in ["stop-play", "dur", "durdur"]:
        dsp.stop_audio_playback()
        console.print("[green]✓ Çalan ses durduruldu.[/green]")
        return

    if cmd in ["sfx", "efekt"]:
        if len(args) < 2:
            console.print("Kullanılabilir SFX: roger (telsiz bip), nuke (siren/patlama), airhorn (korna), beep (bip)")
            return
        sfx_name = args[1].lower()
        dsp.play_sfx(sfx_name)
        console.print(f"[green]✓ '{sfx_name}' efekti mikrofona ve kulaklığa çalındı.[/green]")
        return

    if cmd == "ptt":
        if len(args) >= 2 and args[1] == "start":
            dsp.set_mute(False)
        else:
            dsp.set_mute(True)
        return

    if cmd == "hold-nuke":
        if len(args) >= 2 and args[1] == "start":
            if st.get("fx_mode") != "nuke":
                handle_nuke_toggle(st)
        else:
            if st.get("fx_mode") == "nuke":
                handle_nuke_toggle(st)
        return

    if cmd in ["reset", "default-settings", "restore-defaults", "sifirla", "sıfırla"]:
        handle_reset()
        return

    if cmd in ["test", "hear", "kaydet-dinle", "ses-testi"]:
        sec = 3
        if len(args) >= 2 and args[1].isdigit():
            sec = max(1, min(10, int(args[1])))
        if not dsp.is_daemon_running():
            dsp.start_daemon(st)

        console.print(f"[bold cyan]🎙️ {sec} saniyelik ses testi başlıyor... Mikrofonuna bir şeyler söyle![/bold cyan]")
        with Progress(
            SpinnerColumn(),
            TextColumn("[progress.description]{task.description}"),
            BarColumn(),
            console=console
        ) as progress:
            rec_task = progress.add_task("[red]● Kaydediliyor...", total=sec)
            rec_proc = subprocess.Popen(
                ["pw-record", "--target", "micctl_source", "/tmp/micctl_test_sample.wav"],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL
            )
            for _ in range(sec * 10):
                time.sleep(0.1)
                progress.advance(rec_task, 0.1)
            rec_proc.terminate()
            rec_proc.wait()

        console.print("[bold green]🔊 Kayıt tamamlandı! Şimdi kulaklığından oynatılıyor...[/bold green]")
        subprocess.run(["pw-play", "/tmp/micctl_test_sample.wav"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        console.print("[green]✓ Test tamamlandı.[/green]")
        return

    if cmd in ["autostart", "baslangic", "açılış"]:
        if len(args) < 2 or args[1] in ["status", "durum"]:
            en = dsp.is_systemd_enabled()
            st_text = "[green]Açık[/green]" if en else "[red]Kapalı[/red]"
            console.print(f"Bilgisayar açılışında otomatik başlama: {st_text}")
            return
        enable = args[1].lower() in ["on", "1", "true", "enable", "ac", "aç"]
        dsp.set_systemd_autostart(enable)
        msg = "[green]✓ Otomatik başlama AÇILDI.[/green]" if enable else "[green]✓ Otomatik başlama KAPATILDI.[/green]"
        console.print(msg)
        return

    if cmd in ["waybar", "polybar"]:
        output_waybar_json(st)
        return

    if cmd in ["ultra", "ultra-nuke", "kiyamet", "kıyamet"]:
        handle_nuke_toggle(st, ultra=True)
        return

    if cmd in ["nuke", "earrape", "ear-rape", "kulak-patlatan"]:
        handle_nuke_toggle(st, ultra=False)
        return

    if cmd in ["mute", "sustur"]:
        if len(args) < 2:
            new_mute = dsp.set_mute(None)
        else:
            arg = args[1].lower()
            if arg in ["toggle", "degis"]:
                new_mute = dsp.set_mute(None)
            else:
                target = arg in ["on", "1", "true", "ac", "aç", "yes", "sustur"]
                new_mute = dsp.set_mute(target)
        if new_mute:
            console.print("[red]● Mikrofon SUSTURULDU (MUTED)[/red]")
        else:
            console.print("[green]○ Mikrofon AÇILDI (UNMUTED)[/green]")
        return

    if cmd in ["unmute", "ac"]:
        dsp.set_mute(False)
        console.print("[green]○ Mikrofon AÇILDI (UNMUTED)[/green]")
        return

    if cmd in ["fx", "effect"]:
        if len(args) < 2:
            console.print(f"Aktif FX: [bold cyan]{st.get('fx_mode', 'none')}[/bold cyan]")
            console.print("Kullanılabilir FX modları: none, nuke, ultra_nuke, robot, rabbit, demon, walkie_talkie, telephone")
            return
        target_fx = args[1].lower().replace("-", "_")
        if target_fx in ["none", "normal", "off", "standart"]:
            st["fx_mode"] = "none"
        elif target_fx in ["nuke", "earrape"]:
            handle_nuke_toggle(st, ultra=False)
            return
        elif target_fx in ["ultra", "ultra_nuke", "kiyamet"]:
            handle_nuke_toggle(st, ultra=True)
            return
        elif target_fx in ["robot", "dalek"]:
            st["fx_mode"] = "robot"
            dsp.play_sfx("beep")
        elif target_fx in ["rabbit", "tavsan", "tavşan", "chipmunk", "sincap", "helium", "helyum"]:
            st["fx_mode"] = "rabbit"
        elif target_fx in ["demon", "canavar", "seytan", "şeytan"]:
            st["fx_mode"] = "demon"
        elif target_fx in ["walkie", "walkie_talkie", "telsiz", "radio"]:
            st["fx_mode"] = "walkie_talkie"
            dsp.play_sfx("roger")
        elif target_fx in ["phone", "telephone", "telefon"]:
            st["fx_mode"] = "telephone"
        else:
            console.print(f"[red]Bilinmeyen FX modu:[/red] {target_fx}")
            return
        state_module.save_state(st)
        if dsp.is_daemon_running():
            dsp.restart_daemon(st)
        console.print(f"[green]✓ FX modu değiştirildi:[/green] [bold]{st['fx_mode']}[/bold]")
        return

    if cmd == "start":
        if dsp.is_daemon_running():
            console.print("[yellow]Filtre servisi zaten çalışıyor.[/yellow]")
        else:
            console.print("[cyan]Filtre servisi başlatılıyor...[/cyan]")
            if dsp.start_daemon(st):
                console.print("[green]✓ Servis başarıyla başlatıldı ve 'micctl_source' oluşturuldu.[/green]")
            else:
                console.print("[red]✗ Servis başlatılamadı. PipeWire durumunu kontrol edin.[/red]")
        return

    if cmd == "stop":
        dsp.stop_daemon()
        console.print("[green]✓ Filtre servisi durduruldu.[/green]")
        return

    if cmd == "restart":
        console.print("[cyan]Filtre servisi yeniden başlatılıyor...[/cyan]")
        if dsp.restart_daemon(st):
            console.print("[green]✓ Servis yeniden başlatıldı.[/green]")
        else:
            console.print("[red]✗ Başlatma hatası.[/red]")
        return

    if cmd in ["default", "set-default"]:
        if not dsp.is_daemon_running():
            dsp.start_daemon(st)
        if dsp.set_as_default_mic():
            console.print("[green]✓ 'micctl_source' varsayılan sistem mikrofonu yapıldı.[/green]")
        else:
            console.print("[red]✗ Varsayılan mikrofon ayarlanamadı.[/red]")
        return

    # Ultra-fast real-time EQ updates (< 5ms)
    if cmd in ["bass", "b"]:
        if len(args) < 2:
            console.print(f"Mevcut bass: [bold]{st.get('bass', 0.0):+.1f} dB[/bold]")
            console.print("Kullanım: micctl bass +5 | micctl bass -3 | micctl bass set 20 (-20..+30 dB)")
            return
        val = parse_num_arg(st.get("bass", 0.0), args[1:], -20.0, 30.0)
        st["bass"] = val
        st["preset"] = "özel"
        st["fx_mode"] = "none"
        state_module.save_state(st)
        if dsp.is_daemon_running():
            dsp.update_live_eq(st["bass"], st.get("mid", 0.0), st.get("treble", 0.0))
        console.print(f"[green]✓ Bass ayarlandı:[/green] [bold]{val:+.1f} dB[/bold]")
        return

    if cmd in ["mid", "m"]:
        if len(args) < 2:
            console.print(f"Mevcut mid: [bold]{st.get('mid', 0.0):+.1f} dB[/bold]")
            return
        val = parse_num_arg(st.get("mid", 0.0), args[1:], -18.0, 20.0)
        st["mid"] = val
        st["preset"] = "özel"
        st["fx_mode"] = "none"
        state_module.save_state(st)
        if dsp.is_daemon_running():
            dsp.update_live_eq(st.get("bass", 0.0), st["mid"], st.get("treble", 0.0))
        console.print(f"[green]✓ Mid ayarlandı:[/green] [bold]{val:+.1f} dB[/bold]")
        return

    if cmd in ["treble", "t"]:
        if len(args) < 2:
            console.print(f"Mevcut tiz: [bold]{st.get('treble', 0.0):+.1f} dB[/bold]")
            return
        val = parse_num_arg(st.get("treble", 0.0), args[1:], -18.0, 20.0)
        st["treble"] = val
        st["preset"] = "özel"
        st["fx_mode"] = "none"
        state_module.save_state(st)
        if dsp.is_daemon_running():
            dsp.update_live_eq(st.get("bass", 0.0), st.get("mid", 0.0), st["treble"])
        console.print(f"[green]✓ Tiz ayarlandı:[/green] [bold]{val:+.1f} dB[/bold]")
        return

    if cmd in ["gain", "vol", "volume"]:
        if len(args) < 2:
            console.print(f"Mevcut kazanç: [bold]{st.get('gain', 100)}%[/bold]")
            return
        val = int(parse_num_arg(float(st.get("gain", 100)), args[1:], 0.0, 200.0))
        st["gain"] = val
        state_module.save_state(st)
        dsp.apply_gain(val)
        console.print(f"[green]✓ Kazanç ayarlandı:[/green] [bold]{val}%[/bold]")
        return

    if cmd in ["lowcut", "hp"]:
        if len(args) < 2:
            target = not st.get("lowcut", True)
        else:
            target = args[1].lower() in ["on", "1", "true", "enable", "ac", "aç"]
        st["lowcut"] = target
        state_module.save_state(st)
        if dsp.is_daemon_running():
            dsp.restart_daemon(st)
        status = "[green]Açık[/green]" if target else "[red]Kapalı[/red]"
        console.print(f"[green]✓ Low-cut filtresi (80 Hz):[/green] {status}")
        return

    if cmd in ["noise", "rnnoise"]:
        if not dsp.find_rnnoise_plugin():
            console.print("[yellow]RNNoise LADSPA eklentisi sistemde bulunamadı.[/yellow]")
            console.print("Yüklemek için: [bold cyan]sudo pacman -S noise-suppression-for-voice[/bold cyan]")
            return
        if len(args) < 2:
            target = not st.get("noise_suppression", False)
        else:
            target = args[1].lower() in ["on", "1", "true", "enable", "ac", "aç"]
        st["noise_suppression"] = target
        state_module.save_state(st)
        if dsp.is_daemon_running():
            dsp.restart_daemon(st)
        status = "[green]Açık (RNNoise AI)[/green]" if target else "[red]Kapalı[/red]"
        console.print(f"[green]✓ Gürültü engelleme:[/green] {status}")
        return

    if cmd in ["listen", "monitor"]:
        if len(args) < 2:
            target = not dsp.is_monitoring()
        else:
            target = args[1].lower() in ["on", "1", "true", "enable", "ac", "aç"]
        if target:
            if not dsp.is_daemon_running():
                dsp.start_daemon(st)
            dsp.start_monitoring()
            console.print("[green]✓ Kendi sesini dinleme açıldı (Kulaklıktan duyuluyor).[/green]")
            console.print("[dim]Kapatmak için: micctl listen off[/dim]")
        else:
            dsp.stop_monitoring()
            console.print("[green]✓ Dinleme kapatıldı.[/green]")
        return

    if cmd in ["preset", "p"]:
        if len(args) < 2 or args[1] in ["list", "ls"]:
            presets = state_module.list_presets()
            console.print("[bold cyan]Kullanılabilir Profiller:[/bold cyan]")
            for p in presets:
                active = " [green](Aktif)[/green]" if p == st.get("preset") else ""
                console.print(f"  • {p}{active}")
            return
        action = args[1]
        if action == "load" and len(args) >= 3:
            name = args[2]
            try:
                p_data = state_module.load_preset(name)
                st.update(p_data)
                st["preset"] = name
                state_module.save_state(st)
                if dsp.is_daemon_running():
                    dsp.restart_daemon(st)
                dsp.send_notification("🎙️ micctl", f"'{name}' profili yüklendi")
                console.print(f"[green]✓ '{name}' profili yüklendi ve uygulandı.[/green]")
            except FileNotFoundError:
                console.print(f"[red]✗ Ön ayar bulunamadı: {name}[/red]")
            return
        if action == "save" and len(args) >= 3:
            name = args[2]
            state_module.save_preset(name, st)
            st["preset"] = name
            state_module.save_state(st)
            console.print(f"[green]✓ Mevcut ayarlar '{name}' profili olarak kaydedildi.[/green]")
            return

    if cmd in ["tui", "ui"]:
        from tui import run_tui
        run_tui()
        return

    console.print(f"[red]Bilinmeyen komut:[/red] {cmd}")
    console.print("Komutlar: status, ultra, nuke, fx, play, stop, sfx, reset, test, bass, mid, treble, gain, mute, listen, preset, waybar, tui")
