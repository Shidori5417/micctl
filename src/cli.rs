use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use colored::*;

use crate::dsp;
use crate::state::{self, State, StateBackup};

pub fn parse_num_arg(curr: f64, args: &[String], min_v: f64, max_v: f64) -> f64 {
    let mut tokens: Vec<String> = Vec::new();
    for a in args {
        let clean = a.to_lowercase()
            .replace("set", "")
            .replace("db", "")
            .replace('%', "")
            .trim()
            .to_string();
        if !clean.is_empty() {
            tokens.push(clean);
        }
    }

    if tokens.is_empty() {
        return curr;
    }

    let raw = &tokens[tokens.len() - 1];
    let val = if raw.starts_with('+') {
        if let Ok(num) = raw[1..].parse::<f64>() {
            curr + num
        } else {
            curr
        }
    } else if raw.starts_with('-') && args[args.len() - 1].starts_with('-') && !args[args.len() - 1].to_lowercase().contains("set") {
        if let Ok(num) = raw[1..].parse::<f64>() {
            curr - num
        } else {
            curr
        }
    } else if let Ok(num) = raw.parse::<f64>() {
        num
    } else {
        curr
    };

    let clamped = val.clamp(min_v, max_v);
    (clamped * 10.0).round() / 10.0
}

pub fn print_status(st: &State) {
    let is_running = dsp::is_daemon_running();
    let pid = dsp::get_daemon_pid();
    let default_src = dsp::get_default_source();
    let is_default = default_src.contains("micctl_source");
    let rnnoise_installed = dsp::find_rnnoise_plugin().is_some();
    let muted = dsp::is_muted();
    let autostart = dsp::is_systemd_enabled();
    let is_playing = dsp::is_audio_playing();
    let fx = st.fx_mode.as_str();

    println!("\n{}", "🎙️  micctl — Mikrofon DSP & Soundboard Durumu".bold().cyan());
    println!("{}", "═".repeat(65).dimmed());

    let motor_str = if is_running {
        format!("{} (PID: {})", "● Aktif".green().bold(), pid.unwrap_or(0))
    } else {
        "○ Durduruldu".red().bold().to_string()
    };
    println!("  {:<24} : {}", "Filtre Motoru".dimmed(), motor_str);

    let mute_str = if muted {
        "● SUSTURULDU (MUTED)".red().bold().to_string()
    } else {
        "○ Açık (Unmuted)".green().to_string()
    };
    println!("  {:<24} : {}", "Mikrofon Durumu".dimmed(), mute_str);

    let def_str = if is_default {
        "✓ micctl (Sanal Mikrofon - Sistem Geneli)".green().to_string()
    } else {
        format!("⚠ {} ('micctl default' ile seç)", default_src).yellow().to_string()
    };
    println!("  {:<24} : {}", "Varsayılan Mikrofon".dimmed(), def_str);

    let fx_str = match fx {
        "ultra_nuke" => "💥💥 ULTRA NUKE / KIYAMET (+50dB Bas)".red().bold().blink().to_string(),
        "nuke" => "💥 NUKE / KULAK PATLATAN (AKTİF)".red().bold().to_string(),
        "robot" => "🤖 Metalik Robot (Cybernetic Dalek)".cyan().bold().to_string(),
        "rabbit" => "🐰 Tavşan / Helyum (İnce Ses)".yellow().bold().to_string(),
        "demon" => "👹 Canavar / Demon (Karanlık Kalın Ses)".red().bold().to_string(),
        "walkie_talkie" => "📻 Telsiz / Megafon (Askeri / Polis)".yellow().bold().to_string(),
        "telephone" => "☎️  Eski Telefon (Sabit Hat Bandı)".yellow().to_string(),
        _ => "Normal (Stüdyo EQ)".normal().to_string(),
    };
    println!("  {:<24} : {}", "Özel Efekt Modu".dimmed(), fx_str);

    let sb_str = if is_playing {
        "🔊 Çalıyor... ('micctl stop-play' ile durdur)".green().bold().to_string()
    } else {
        "Hazır ('micctl play <dosya>' veya 'micctl sfx <isim>')".dimmed().to_string()
    };
    println!("  {:<24} : {}", "Soundboard / Ses".dimmed(), sb_str);

    println!("{}", "─".repeat(65).dimmed());

    // Audio parameters
    let bass_str = if st.bass > 0.0 {
        format!("{:+5.1} dB", st.bass).green().bold().to_string()
    } else if st.bass < 0.0 {
        format!("{:+5.1} dB", st.bass).yellow().to_string()
    } else {
        format!("{:+5.1} dB", st.bass).normal().to_string()
    };
    println!("  {:<24} : {}", "Bass (-20..+30dB)".dimmed(), bass_str);

    let mid_str = if st.mid > 0.0 {
        format!("{:+5.1} dB", st.mid).green().bold().to_string()
    } else if st.mid < 0.0 {
        format!("{:+5.1} dB", st.mid).yellow().to_string()
    } else {
        format!("{:+5.1} dB", st.mid).normal().to_string()
    };
    println!("  {:<24} : {}", "Mid (-18..+20dB)".dimmed(), mid_str);

    let treble_str = if st.treble > 0.0 {
        format!("{:+5.1} dB", st.treble).green().bold().to_string()
    } else if st.treble < 0.0 {
        format!("{:+5.1} dB", st.treble).yellow().to_string()
    } else {
        format!("{:+5.1} dB", st.treble).normal().to_string()
    };
    println!("  {:<24} : {}", "Tiz (-18..+20dB)".dimmed(), treble_str);

    let lc_str = if st.lowcut {
        "Açık (80 Hz Low-cut)".green().to_string()
    } else {
        "Kapalı".red().to_string()
    };
    println!("  {:<24} : {}", "Low-cut Filtre".dimmed(), lc_str);

    let noise_str = if !rnnoise_installed {
        "Kurulu Değil (pacman -S noise-suppression-for-voice)".dimmed().to_string()
    } else if st.noise_suppression {
        "Açık (RNNoise AI)".green().bold().to_string()
    } else {
        "Kapalı".red().to_string()
    };
    println!("  {:<24} : {}", "Gürültü Engelleme".dimmed(), noise_str);

    println!("  {:<24} : {} %", "Çıkış Kazancı".dimmed(), st.gain);

    let mon_str = if dsp::is_monitoring() {
        "● Dinleniyor ('micctl listen off' ile kapat)".green().bold().to_string()
    } else {
        "Kapalı ('micctl listen on' ile kulaklıktan duy)".dimmed().to_string()
    };
    println!("  {:<24} : {}", "Canlı Monitör".dimmed(), mon_str);

    let auto_str = if autostart {
        "Açık (systemd user)".green().to_string()
    } else {
        "Kapalı ('micctl autostart on' ile aç)".dimmed().to_string()
    };
    println!("  {:<24} : {}", "Açılışta Başlatma".dimmed(), auto_str);
    println!("  {:<24} : {}", "Aktif Profil".dimmed(), st.preset.cyan().bold());
    println!("{}\n", "═".repeat(65).dimmed());
}

pub fn handle_nuke_toggle(st: &mut State, ultra: bool) {
    let target_mode = if ultra { "ultra_nuke" } else { "nuke" };
    let curr_fx = st.fx_mode.clone();

    if curr_fx != target_mode {
        st.backup_state = Some(Box::new(StateBackup {
            bass: st.bass,
            mid: st.mid,
            treble: st.treble,
            gain: st.gain,
            lowcut: st.lowcut,
            preset: st.preset.clone(),
            fx_mode: "none".to_string(),
        }));
        st.fx_mode = target_mode.to_string();
        st.preset = if ultra { "ultra-nuke".into() } else { "nuke".into() };
        st.gain = if ultra { 200 } else { 150 };
        let _ = state::save_state(st);

        if !dsp::is_daemon_running() {
            dsp::start_daemon(st);
        } else {
            dsp::restart_daemon(st);
        }
        dsp::apply_gain(st.gain);
        dsp::play_sfx("nuke");

        let title = if ultra { "💥💥 ULTRA NUKE AKTİF!" } else { "💥 NUKE AKTİF!" };
        dsp::send_notification(title, "Kulak Patlatan Modu Devrede!", "critical");
        println!("{}", title.red().bold());
        println!("{}", "Normale dönmek için: micctl nuke (veya micctl ultra)".dimmed());
    } else {
        if let Some(b) = st.backup_state.take() {
            st.fx_mode = "none".to_string();
            st.bass = b.bass;
            st.mid = b.mid;
            st.treble = b.treble;
            st.gain = b.gain;
            st.lowcut = b.lowcut;
            st.preset = b.preset;
        } else {
            st.fx_mode = "none".to_string();
            st.preset = "default".to_string();
            st.gain = 100;
        }
        let _ = state::save_state(st);

        if dsp::is_daemon_running() {
            dsp::restart_daemon(st);
        }
        dsp::apply_gain(st.gain);
        dsp::send_notification("🎙️ micctl", "Kulak Patlatan modu kapatıldı (Normal ses)", "normal");
        println!("{}", "✓ Kulak Patlatan modu kapatıldı, normal stüdyo tonuna dönüldü.".green());
    }
}

pub fn handle_reset() {
    let st = state::reset_state();
    dsp::set_mute(Some(false));
    dsp::stop_audio_playback();
    if dsp::is_daemon_running() {
        dsp::restart_daemon(&st);
    }
    dsp::apply_gain(100);
    dsp::play_sfx("beep");
    dsp::send_notification("🎙️ micctl", "Tüm ayarlar varsayılana sıfırlandı.", "normal");
    println!("{}", "✓ Tüm mikrofon ayarları, filtreler ve ses seviyesi varsayılana sıfırlandı.".green());
    println!("{}", "Varsayılan: Bass +3dB, Mid 0dB, Treble +1.5dB, Gain 100%, Low-cut Açık.".dimmed());
}

pub fn handle_play(args: &[String]) {
    if args.is_empty() {
        println!("{}", "Kullanım: micctl play <ses_dosyası.mp3/wav/ogg> [ses_yüzdesi]".yellow());
        println!("Örnek: micctl play ~/Müzik/meme.mp3 100");
        return;
    }

    let target_file = &args[0];
    let mut vol = 100u32;
    if args.len() >= 2 {
        if let Ok(v) = args[1].parse::<u32>() {
            vol = v.clamp(10, 150);
        }
    }

    let p = PathBuf::from(shellexpand_path(target_file));
    if !p.exists() {
        println!("{} {}", "✗ Ses dosyası bulunamadı:".red(), target_file);
        return;
    }

    let st = state::load_state();
    if !dsp::is_daemon_running() {
        dsp::start_daemon(&st);
    }

    let file_name = p.file_name().and_then(|f| f.to_str()).unwrap_or(target_file);
    println!("{} {} (%{} ses)", "🔊 Mikrofona çalınıyor:".cyan().bold(), file_name, vol);
    println!("{}", "Durdurmak için: micctl stop-play".dimmed());
    dsp::play_audio_to_mic(&p, vol);
}

fn shellexpand_path(raw: &str) -> PathBuf {
    if raw.starts_with("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(&raw[2..]);
        }
    }
    PathBuf::from(raw)
}

pub fn output_waybar_json(st: &State) {
    let muted = dsp::is_muted();
    let is_running = dsp::is_daemon_running();
    let is_playing = dsp::is_audio_playing();
    let fx = st.fx_mode.as_str();

    let (text, tooltip, class_name) = if !is_running {
        ("🎙️ OFF".to_string(), "micctl: Servis kapalı".to_string(), "stopped")
    } else if muted {
        ("🎙️ MUTED".to_string(), "micctl: Mikrofon susturuldu".to_string(), "muted")
    } else if is_playing {
        ("🔊 PLAY".to_string(), "micctl: Mikrofona ses çalınıyor".to_string(), "playing")
    } else if fx == "ultra_nuke" {
        ("💥 ULTRA".to_string(), "micctl: Ultra Nuke Kıyamet Modu!".to_string(), "nuke")
    } else if fx == "nuke" {
        ("💥 NUKE".to_string(), "micctl: Kulak Patlatan Modu Aktif!".to_string(), "nuke")
    } else if fx == "robot" {
        ("🤖 ROBOT".to_string(), "micctl: Robot Modu".to_string(), "fx")
    } else if fx == "rabbit" {
        ("🐰 TAVŞAN".to_string(), "micctl: Tavşan / Helyum Modu".to_string(), "fx")
    } else if fx == "demon" {
        ("👹 DEMON".to_string(), "micctl: Canavar / Demon Modu".to_string(), "fx")
    } else if fx == "walkie_talkie" {
        ("📻 Telsiz".to_string(), "micctl: Walkie-Talkie FX".to_string(), "fx")
    } else {
        let bass = st.bass;
        let p = &st.preset;
        let t = format!("🎙️ {:+.1}dB", bass);
        let tip = format!("micctl Aktif\nProfil: {}\nBass: {:+.1}dB\nGain: {}%", p, bass, st.gain);
        (t, tip, "active")
    };

    let obj = serde_json::json!({
        "text": text,
        "tooltip": tooltip,
        "class": class_name
    });
    println!("{}", serde_json::to_string(&obj).unwrap_or_default());
}

pub fn handle_cli(args: Vec<String>) {
    if args.is_empty() {
        crate::tui::run_tui();
        return;
    }

    let cmd = args[0].to_lowercase();
    let mut st = state::load_state();

    match cmd.as_str() {
        "status" | "s" | "info" => {
            print_status(&st);
        }
        "tui" | "ui" => {
            crate::tui::run_tui();
        }
        "play" | "çal" | "soundboard" => {
            handle_play(&args[1..]);
        }
        "stop-play" | "stop" | "dur" | "durdur" => {
            dsp::stop_audio_playback();
            println!("{}", "✓ Çalan ses durduruldu.".green());
        }
        "sfx" | "efekt" => {
            if args.len() < 2 {
                println!("Kullanılabilir SFX: roger (telsiz bip), nuke (siren/patlama), airhorn (korna), beep (bip)");
                return;
            }
            let sfx_name = args[1].to_lowercase();
            dsp::play_sfx(&sfx_name);
            println!("{} '{}' efekti mikrofona ve kulaklığa çalındı.", "✓".green(), sfx_name);
        }
        "ptt" => {
            if args.len() >= 2 && args[1] == "start" {
                dsp::set_mute(Some(false));
            } else {
                dsp::set_mute(Some(true));
            }
        }
        "hold-nuke" => {
            if args.len() >= 2 && args[1] == "start" {
                if st.fx_mode != "nuke" {
                    handle_nuke_toggle(&mut st, false);
                }
            } else if st.fx_mode == "nuke" {
                handle_nuke_toggle(&mut st, false);
            }
        }
        "reset" | "sifirla" | "sıfırla" => {
            handle_reset();
        }
        "test" | "hear" | "kaydet-dinle" | "ses-testi" => {
            let mut sec = 3u32;
            if args.len() >= 2 {
                if let Ok(s) = args[1].parse::<u32>() {
                    sec = s.clamp(1, 10);
                }
            }
            if !dsp::is_daemon_running() {
                dsp::start_daemon(&st);
            }

            println!("{} {} saniyelik ses testi başlıyor... Mikrofonuna bir şeyler söyle!", "🎙️".cyan().bold(), sec);
            let test_wav = "/tmp/micctl_test_sample.wav";

            let rec_proc = Command::new("pw-record")
                .args(["--target", "micctl_source", test_wav])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();

            print!("  Kaydediliyor: ");
            for _ in 0..(sec * 10) {
                thread::sleep(Duration::from_millis(100));
                print!("#");
                let _ = std::io::Write::flush(&mut std::io::stdout());
            }
            println!(" [Tamam]");

            if let Ok(mut p) = rec_proc {
                let _ = p.kill();
                let _ = p.wait();
            }

            println!("{}", "🔊 Kayıt tamamlandı! Kulaklığından oynatılıyor...".green().bold());
            let _ = Command::new("pw-play")
                .arg(test_wav)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            println!("{}", "✓ Test tamamlandı.".green());
        }
        "autostart" | "baslangic" | "açılış" => {
            if args.len() < 2 || matches!(args[1].as_str(), "status" | "durum") {
                let en = dsp::is_systemd_enabled();
                let st_text = if en { "Açık".green() } else { "Kapalı".red() };
                println!("Bilgisayar açılışında otomatik başlama: {}", st_text);
                return;
            }
            let enable = matches!(args[1].to_lowercase().as_str(), "on" | "1" | "true" | "enable" | "ac" | "aç");
            dsp::set_systemd_autostart(enable);
            if enable {
                println!("{}", "✓ Otomatik başlama AÇILDI (systemd service).".green());
            } else {
                println!("{}", "✓ Otomatik başlama KAPATILDI.".green());
            }
        }
        "waybar" | "polybar" => {
            output_waybar_json(&st);
        }
        "ultra" | "ultra-nuke" | "kiyamet" | "kıyamet" => {
            handle_nuke_toggle(&mut st, true);
        }
        "nuke" | "earrape" | "ear-rape" | "kulak-patlatan" => {
            handle_nuke_toggle(&mut st, false);
        }
        "mute" | "sustur" => {
            let new_mute = if args.len() < 2 {
                dsp::set_mute(None)
            } else {
                let arg = args[1].to_lowercase();
                if arg == "toggle" || arg == "degis" {
                    dsp::set_mute(None)
                } else {
                    let target = matches!(arg.as_str(), "on" | "1" | "true" | "ac" | "aç" | "yes" | "sustur");
                    dsp::set_mute(Some(target))
                }
            };
            if new_mute {
                println!("{}", "● Mikrofon SUSTURULDU (MUTED)".red().bold());
            } else {
                println!("{}", "○ Mikrofon AÇILDI (UNMUTED)".green().bold());
            }
        }
        "unmute" | "ac" => {
            dsp::set_mute(Some(false));
            println!("{}", "○ Mikrofon AÇILDI (UNMUTED)".green().bold());
        }
        "fx" | "effect" => {
            if args.len() < 2 {
                println!("Aktif FX: {}", st.fx_mode.cyan().bold());
                println!("Kullanılabilir FX modları: none, nuke, ultra_nuke, robot, rabbit, demon, walkie_talkie, telephone");
                return;
            }
            let target_fx = args[1].to_lowercase().replace('-', "_");
            match target_fx.as_str() {
                "none" | "normal" | "off" | "standart" => {
                    st.fx_mode = "none".to_string();
                }
                "nuke" | "earrape" => {
                    handle_nuke_toggle(&mut st, false);
                    return;
                }
                "ultra" | "ultra_nuke" | "kiyamet" => {
                    handle_nuke_toggle(&mut st, true);
                    return;
                }
                "robot" | "dalek" => {
                    st.fx_mode = "robot".to_string();
                    dsp::play_sfx("beep");
                }
                "rabbit" | "tavsan" | "tavşan" | "chipmunk" | "sincap" | "helium" | "helyum" => {
                    st.fx_mode = "rabbit".to_string();
                }
                "demon" | "canavar" | "seytan" | "şeytan" => {
                    st.fx_mode = "demon".to_string();
                }
                "walkie" | "walkie_talkie" | "telsiz" | "radio" => {
                    st.fx_mode = "walkie_talkie".to_string();
                    dsp::play_sfx("roger");
                }
                "phone" | "telephone" | "telefon" => {
                    st.fx_mode = "telephone".to_string();
                }
                _ => {
                    println!("{} {}", "Bilinmeyen FX modu:".red(), target_fx);
                    return;
                }
            }
            let _ = state::save_state(&st);
            if dsp::is_daemon_running() {
                dsp::restart_daemon(&st);
            }
            println!("{} {}", "✓ FX modu değiştirildi:".green(), st.fx_mode.bold());
        }
        "start" => {
            if dsp::is_daemon_running() {
                println!("{}", "Filtre servisi zaten çalışıyor.".yellow());
            } else {
                println!("{}", "Filtre servisi başlatılıyor...".cyan());
                if dsp::start_daemon(&st) {
                    println!("{}", "✓ Servis başarıyla başlatıldı ve 'micctl_source' oluşturuldu.".green());
                } else {
                    println!("{}", "✗ Servis başlatılamadı. PipeWire durumunu kontrol edin.".red());
                }
            }
        }
        "stop-daemon" => {
            dsp::stop_daemon();
            println!("{}", "✓ Filtre servisi durduruldu.".green());
        }
        "restart" => {
            println!("{}", "Filtre servisi yeniden başlatılıyor...".cyan());
            if dsp::restart_daemon(&st) {
                println!("{}", "✓ Servis yeniden başlatıldı.".green());
            } else {
                println!("{}", "✗ Başlatma hatası.".red());
            }
        }
        "default" | "set-default" => {
            if !dsp::is_daemon_running() {
                dsp::start_daemon(&st);
            }
            if dsp::set_as_default_mic() {
                println!("{}", "✓ 'micctl_source' varsayılan sistem mikrofonu yapıldı.".green());
            } else {
                println!("{}", "✗ Varsayılan mikrofon ayarlanamadı.".red());
            }
        }
        "bass" | "b" => {
            if args.len() < 2 {
                println!("Mevcut bass: {:+.1} dB", st.bass);
                println!("Kullanım: micctl bass +5 | micctl bass -3 | micctl bass set 20 (-20..+30 dB)");
                return;
            }
            let val = parse_num_arg(st.bass, &args[1..], -20.0, 30.0);
            st.bass = val;
            st.preset = "özel".to_string();
            st.fx_mode = "none".to_string();
            let _ = state::save_state(&st);
            if dsp::is_daemon_running() {
                dsp::update_live_eq(st.bass, st.mid, st.treble);
            }
            println!("{} {:+.1} dB", "✓ Bass ayarlandı:".green(), val);
        }
        "mid" | "m" => {
            if args.len() < 2 {
                println!("Mevcut mid: {:+.1} dB", st.mid);
                return;
            }
            let val = parse_num_arg(st.mid, &args[1..], -18.0, 20.0);
            st.mid = val;
            st.preset = "özel".to_string();
            st.fx_mode = "none".to_string();
            let _ = state::save_state(&st);
            if dsp::is_daemon_running() {
                dsp::update_live_eq(st.bass, st.mid, st.treble);
            }
            println!("{} {:+.1} dB", "✓ Mid ayarlandı:".green(), val);
        }
        "treble" | "t" => {
            if args.len() < 2 {
                println!("Mevcut tiz: {:+.1} dB", st.treble);
                return;
            }
            let val = parse_num_arg(st.treble, &args[1..], -18.0, 20.0);
            st.treble = val;
            st.preset = "özel".to_string();
            st.fx_mode = "none".to_string();
            let _ = state::save_state(&st);
            if dsp::is_daemon_running() {
                dsp::update_live_eq(st.bass, st.mid, st.treble);
            }
            println!("{} {:+.1} dB", "✓ Tiz ayarlandı:".green(), val);
        }
        "gain" | "vol" | "volume" => {
            if args.len() < 2 {
                println!("Mevcut kazanç: {}%", st.gain);
                return;
            }
            let val = parse_num_arg(st.gain as f64, &args[1..], 0.0, 200.0) as u32;
            st.gain = val;
            let _ = state::save_state(&st);
            dsp::apply_gain(val);
            println!("{} {}%", "✓ Kazanç ayarlandı:".green(), val);
        }
        "lowcut" | "hp" => {
            let target = if args.len() < 2 {
                !st.lowcut
            } else {
                matches!(args[1].to_lowercase().as_str(), "on" | "1" | "true" | "enable" | "ac" | "aç")
            };
            st.lowcut = target;
            let _ = state::save_state(&st);
            if dsp::is_daemon_running() {
                dsp::restart_daemon(&st);
            }
            let status = if target { "Açık".green() } else { "Kapalı".red() };
            println!("{} {}", "✓ Low-cut filtresi (80 Hz):".green(), status);
        }
        "noise" | "rnnoise" => {
            if dsp::find_rnnoise_plugin().is_none() {
                println!("{}", "RNNoise LADSPA eklentisi sistemde bulunamadı.".yellow());
                println!("Yüklemek için: {}", "sudo pacman -S noise-suppression-for-voice".cyan().bold());
                return;
            }
            let target = if args.len() < 2 {
                !st.noise_suppression
            } else {
                matches!(args[1].to_lowercase().as_str(), "on" | "1" | "true" | "enable" | "ac" | "aç")
            };
            st.noise_suppression = target;
            let _ = state::save_state(&st);
            if dsp::is_daemon_running() {
                dsp::restart_daemon(&st);
            }
            let status = if target { "Açık (RNNoise AI)".green() } else { "Kapalı".red() };
            println!("{} {}", "✓ Gürültü engelleme:".green(), status);
        }
        "listen" | "monitor" => {
            let target = if args.len() < 2 {
                !dsp::is_monitoring()
            } else {
                matches!(args[1].to_lowercase().as_str(), "on" | "1" | "true" | "enable" | "ac" | "aç")
            };
            if target {
                if !dsp::is_daemon_running() {
                    dsp::start_daemon(&st);
                }
                dsp::start_monitoring();
                println!("{}", "✓ Kendi sesini dinleme açıldı (Kulaklıktan duyuluyor).".green());
                println!("{}", "Kapatmak için: micctl listen off".dimmed());
            } else {
                dsp::stop_monitoring();
                println!("{}", "✓ Dinleme kapatıldı.".green());
            }
        }
        "preset" | "p" => {
            if args.len() < 2 || matches!(args[1].as_str(), "list" | "ls") {
                let presets = state::list_presets();
                println!("{}", "Kullanılabilir Profiller:".cyan().bold());
                for p in presets {
                    let active = if p == st.preset { " (Aktif)".green().to_string() } else { "".to_string() };
                    println!("  • {}{}", p, active);
                }
                return;
            }
            let action = &args[1];
            if action == "load" && args.len() >= 3 {
                let name = &args[2];
                match state::load_preset(name) {
                    Ok(p_data) => {
                        st.bass = p_data.bass;
                        st.mid = p_data.mid;
                        st.treble = p_data.treble;
                        st.lowcut = p_data.lowcut;
                        st.noise_suppression = p_data.noise_suppression;
                        st.gain = p_data.gain;
                        st.fx_mode = p_data.fx_mode;
                        st.preset = name.to_string();
                        let _ = state::save_state(&st);
                        if dsp::is_daemon_running() {
                            dsp::restart_daemon(&st);
                        }
                        dsp::send_notification("🎙️ micctl", &format!("'{}' profili yüklendi", name), "normal");
                        println!("{} '{}' profili yüklendi ve uygulandı.", "✓".green(), name);
                    }
                    Err(e) => {
                        println!("{} {}", "✗".red(), e);
                    }
                }
                return;
            }
            if action == "save" && args.len() >= 3 {
                let name = &args[2];
                let _ = state::save_preset(name, &st);
                st.preset = name.to_string();
                let _ = state::save_state(&st);
                println!("{} Mevcut ayarlar '{}' profili olarak kaydedildi.", "✓".green(), name);
                return;
            }
        }
        _ => {
            println!("{} {}", "Bilinmeyen komut:".red(), cmd);
            println!("Komutlar: status, ultra, nuke, fx, play, stop-play, sfx, reset, test, bass, mid, treble, gain, mute, listen, preset, waybar, tui");
        }
    }
}
