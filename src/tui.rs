use std::io::stdout;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

use crate::dsp;
use crate::state;

pub fn make_slider(val: f64, min_v: f64, max_v: f64, width: usize) -> String {
    let ratio = ((val - min_v) / (max_v - min_v)).clamp(0.0, 1.0);
    let pos = (ratio * (width.saturating_sub(1) as f64)).round() as usize;
    let mut bar = vec!['─'; width];

    let zero_ratio = ((0.0 - min_v) / (max_v - min_v)).clamp(0.0, 1.0);
    let zero_pos = (zero_ratio * (width.saturating_sub(1) as f64)).round() as usize;
    if zero_pos < width {
        bar[zero_pos] = '│';
    }

    if pos < width {
        bar[pos] = '█';
    }

    format!("[{}]", bar.into_iter().collect::<String>())
}

pub fn run_tui() {
    let _ = enable_raw_mode();
    let mut out = stdout();
    let _ = execute!(out, EnterAlternateScreen);
    let backend = CrosstermBackend::new(out);
    let mut terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(_) => {
            let _ = disable_raw_mode();
            return;
        }
    };

    let mut st = state::load_state();
    let fx_list = [
        "none",
        "nuke",
        "ultra_nuke",
        "robot",
        "rabbit",
        "demon",
        "walkie_talkie",
        "telephone",
    ];

    let mut presets = state::list_presets();
    if presets.is_empty() {
        presets.push("default".to_string());
    }

    let menu_items = [
        "Özel FX Modu",
        "Soundboard / SFX",
        "Bass (-20..+30dB)",
        "Mid (-18..+20dB)",
        "Tiz (-18..+20dB)",
        "Çıkış Kazancı (Gain)",
        "Low-cut (80 Hz)",
        "Gürültü Engelleme (RNNoise)",
        "Canlı Dinleme (Monitör)",
        "Mikrofon Mute",
        "Hazır Profil (Preset)",
        "Filtre Motoru",
    ];

    let mut selected: usize = 0;
    let mut status_msg = String::new();
    let mut status_msg_time = Instant::now();
    let mut last_save_time = Instant::now();
    let mut pending_save = false;

    let test_recording = Arc::new(AtomicBool::new(false));

    loop {
        let is_running = dsp::is_daemon_running();
        let is_muted = dsp::is_muted();
        let is_playing = dsp::is_audio_playing();
        let is_mon = dsp::is_monitoring();
        let rnnoise_installed = dsp::find_rnnoise_plugin().is_some();
        let default_src = dsp::get_default_source();
        let is_default = default_src.contains("micctl_source");
        let fx = st.fx_mode.clone();

        let _ = terminal.draw(|f| {
            let size = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Header title & status
                    Constraint::Min(14),   // Main menu
                    Constraint::Length(3), // Footer hotkey help
                ])
                .split(size);

            // 1. Header
            let mut header_spans = vec![
                Span::styled("🎙️ micctl  ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ];

            if is_running {
                header_spans.push(Span::styled("● AKTİF ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)));
            } else {
                header_spans.push(Span::styled("○ DURDURULDU ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)));
            }

            if is_muted {
                header_spans.push(Span::styled("[ ● SUSTURULDU ] ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)));
            } else {
                header_spans.push(Span::styled("[ ○ Açık ] ", Style::default().fg(Color::Green)));
            }

            if is_default {
                header_spans.push(Span::styled("[ ✓ Varsayılan ] ", Style::default().fg(Color::Blue)));
            }

            match fx.as_str() {
                "ultra_nuke" => {
                    header_spans.push(Span::styled(
                        "💥💥 ULTRA NUKE (KIYAMET) 💥💥 ",
                        Style::default().fg(Color::White).bg(Color::Red).add_modifier(Modifier::BOLD),
                    ));
                }
                "nuke" => {
                    header_spans.push(Span::styled(
                        "💥 NUKE AKTİF 💥 ",
                        Style::default().fg(Color::White).bg(Color::Red).add_modifier(Modifier::BOLD),
                    ));
                }
                "robot" => {
                    header_spans.push(Span::styled("🤖 ROBOT SESİ ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
                }
                "rabbit" => {
                    header_spans.push(Span::styled("🐰 TAVŞAN / HELYUM ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
                }
                "demon" => {
                    header_spans.push(Span::styled("👹 CANAVAR / DEMON ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)));
                }
                "walkie_talkie" => {
                    header_spans.push(Span::styled("📻 TELSİZ / MEGAFON ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
                }
                "telephone" => {
                    header_spans.push(Span::styled("☎️ ESKİ TELEFON ", Style::default().fg(Color::Yellow)));
                }
                _ => {}
            }

            if is_playing {
                header_spans.push(Span::styled("🔊 Ses Çalınıyor... ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)));
            } else if is_mon {
                header_spans.push(Span::styled("● Dinleme Açık ", Style::default().fg(Color::Yellow)));
            }

            if test_recording.load(Ordering::Relaxed) {
                header_spans.push(Span::styled("● TEST KAYDEDİLİYOR... ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)));
            }

            let header = Paragraph::new(Line::from(header_spans))
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
            f.render_widget(header, chunks[0]);

            // 2. Menu Content
            let mut menu_lines: Vec<Line> = Vec::new();
            for (idx, &item_name) in menu_items.iter().enumerate() {
                let is_sel = idx == selected;
                let prefix = if is_sel { " ❯ " } else { "   " };
                let base_style = if is_sel {
                    Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                let line = match idx {
                    0 => {
                        // FX Mode
                        let fx_label = match fx.as_str() {
                            "none" => "Normal (Stüdyo EQ)",
                            "nuke" => "💥 Nuke (Kulak Patlatan)",
                            "ultra_nuke" => "💥💥 Ultra Nuke (Kıyamet)",
                            "robot" => "🤖 Robot (Metalik Dalek)",
                            "rabbit" => "🐰 Tavşan / Helyum",
                            "demon" => "👹 Canavar / Demon",
                            "walkie_talkie" => "📻 Telsiz / Megafon",
                            "telephone" => "☎️ Eski Telefon",
                            other => other,
                        };
                        format!("{:<28} ◀  {:<26} ▶", format!("{}{}", prefix, item_name), fx_label)
                    }
                    1 => {
                        // Soundboard
                        let sb_lbl = if is_playing {
                            "[ 🔊 ÇALINIYOR... (S: Durdur) ]"
                        } else {
                            "[ 1: Roger  2: Nuke  3: Airhorn  4: Bip ]"
                        };
                        format!("{:<28} {}", format!("{}{}", prefix, item_name), sb_lbl)
                    }
                    2 => {
                        // Bass
                        let slider = make_slider(st.bass, -20.0, 30.0, 22);
                        format!("{:<28} {}  {:>+5.1} dB", format!("{}{}", prefix, item_name), slider, st.bass)
                    }
                    3 => {
                        // Mid
                        let slider = make_slider(st.mid, -18.0, 20.0, 22);
                        format!("{:<28} {}  {:>+5.1} dB", format!("{}{}", prefix, item_name), slider, st.mid)
                    }
                    4 => {
                        // Treble
                        let slider = make_slider(st.treble, -18.0, 20.0, 22);
                        format!("{:<28} {}  {:>+5.1} dB", format!("{}{}", prefix, item_name), slider, st.treble)
                    }
                    5 => {
                        // Gain
                        let slider = make_slider(st.gain as f64, 0.0, 200.0, 22);
                        format!("{:<28} {}  {:>5} %", format!("{}{}", prefix, item_name), slider, st.gain)
                    }
                    6 => {
                        // Lowcut
                        let val_str = if st.lowcut { "[ Açık (80 Hz Low-cut) ]" } else { "[ Kapalı ]" };
                        format!("{:<28} ◀  {:<26} ▶", format!("{}{}", prefix, item_name), val_str)
                    }
                    7 => {
                        // Noise suppression
                        let val_str = if !rnnoise_installed {
                            "[ Kurulu Değil (pacman -S noise-suppression-for-voice) ]"
                        } else if st.noise_suppression {
                            "[ Açık (RNNoise AI) ]"
                        } else {
                            "[ Kapalı ]"
                        };
                        format!("{:<28} ◀  {:<26} ▶", format!("{}{}", prefix, item_name), val_str)
                    }
                    8 => {
                        // Listen
                        let val_str = if is_mon { "[ Açık (Kulaklıktan Dinleniyor) ]" } else { "[ Kapalı ]" };
                        format!("{:<28} ◀  {:<26} ▶", format!("{}{}", prefix, item_name), val_str)
                    }
                    9 => {
                        // Mute
                        let val_str = if is_muted { "[ ● SUSTURULDU ]" } else { "[ ○ Açık (Unmuted) ]" };
                        format!("{:<28} ◀  {:<26} ▶", format!("{}{}", prefix, item_name), val_str)
                    }
                    10 => {
                        // Preset
                        format!("{:<28} ◀  {:<26} ▶", format!("{}{}", prefix, item_name), st.preset)
                    }
                    11 => {
                        // Daemon
                        let val_str = if is_running { "[ Aktif - Yeniden Başlatmak için Enter ]" } else { "[ Durduruldu - Başlatmak için Enter ]" };
                        format!("{:<28} {}", format!("{}{}", prefix, item_name), val_str)
                    }
                    _ => String::new(),
                };

                menu_lines.push(Line::from(vec![Span::styled(line, base_style)]));
            }

            let title_text = if !status_msg.is_empty() && status_msg_time.elapsed() < Duration::from_millis(2500) {
                format!(" micctl Kontrol Paneli  —  {} ", status_msg)
            } else {
                " micctl Kontrol Paneli (Sıfır Gecikmeli Canlı DSP) ".to_string()
            };

            let menu_block = Block::default()
                .title(Span::styled(title_text, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray));

            let menu_paragraph = Paragraph::new(menu_lines).block(menu_block);
            f.render_widget(menu_paragraph, chunks[1]);

            // 3. Footer Hotkey Help
            let help_spans = vec![
                Span::styled("[↑/↓]:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw("Gezin "),
                Span::styled("[←/→]:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw("Değiştir "),
                Span::styled("[1-4]:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw("SFX "),
                Span::styled("[S]:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw("Durdur "),
                Span::styled("[N]:", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::raw("Nuke "),
                Span::styled("[U]:", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::raw("Ultra "),
                Span::styled("[X]:", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                Span::raw("Mute "),
                Span::styled("[T]:", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw("Test "),
                Span::styled("[R]:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw("Reset "),
                Span::styled("[D]:", Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
                Span::raw("Varsayılan "),
                Span::styled("[Q]:", Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
                Span::raw("Çıkış"),
            ];

            let footer = Paragraph::new(Line::from(help_spans))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
            f.render_widget(footer, chunks[2]);
        });

        // Debounced save
        if pending_save && last_save_time.elapsed() > Duration::from_millis(300) {
            let _ = state::save_state(&st);
            pending_save = false;
        }

        // Event polling
        if event::poll(Duration::from_millis(40)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                            if pending_save {
                                let _ = state::save_state(&st);
                            }
                            break;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if selected > 0 {
                                selected -= 1;
                            } else {
                                selected = menu_items.len() - 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if selected + 1 < menu_items.len() {
                                selected += 1;
                            } else {
                                selected = 0;
                            }
                        }
                        KeyCode::Left | KeyCode::Char('h') => {
                            match selected {
                                0 => {
                                    // FX mode prev
                                    let curr_idx = fx_list.iter().position(|&x| x == st.fx_mode).unwrap_or(0);
                                    let next_idx = if curr_idx > 0 { curr_idx - 1 } else { fx_list.len() - 1 };
                                    st.fx_mode = fx_list[next_idx].to_string();
                                    pending_save = true;
                                    last_save_time = Instant::now();
                                    if dsp::is_daemon_running() {
                                        dsp::restart_daemon(&st);
                                    }
                                    status_msg = format!("FX: {}", st.fx_mode);
                                    status_msg_time = Instant::now();
                                }
                                2 => {
                                    // Bass -
                                    st.bass = (st.bass - 0.5).clamp(-20.0, 30.0);
                                    st.bass = (st.bass * 10.0).round() / 10.0;
                                    st.preset = "özel".into();
                                    st.fx_mode = "none".into();
                                    pending_save = true;
                                    last_save_time = Instant::now();
                                    if dsp::is_daemon_running() {
                                        dsp::update_live_eq(st.bass, st.mid, st.treble);
                                    }
                                }
                                3 => {
                                    // Mid -
                                    st.mid = (st.mid - 0.5).clamp(-18.0, 20.0);
                                    st.mid = (st.mid * 10.0).round() / 10.0;
                                    st.preset = "özel".into();
                                    st.fx_mode = "none".into();
                                    pending_save = true;
                                    last_save_time = Instant::now();
                                    if dsp::is_daemon_running() {
                                        dsp::update_live_eq(st.bass, st.mid, st.treble);
                                    }
                                }
                                4 => {
                                    // Treble -
                                    st.treble = (st.treble - 0.5).clamp(-18.0, 20.0);
                                    st.treble = (st.treble * 10.0).round() / 10.0;
                                    st.preset = "özel".into();
                                    st.fx_mode = "none".into();
                                    pending_save = true;
                                    last_save_time = Instant::now();
                                    if dsp::is_daemon_running() {
                                        dsp::update_live_eq(st.bass, st.mid, st.treble);
                                    }
                                }
                                5 => {
                                    // Gain -
                                    st.gain = st.gain.saturating_sub(5).clamp(0, 200);
                                    pending_save = true;
                                    last_save_time = Instant::now();
                                    dsp::apply_gain(st.gain);
                                }
                                6 => {
                                    // Lowcut toggle
                                    st.lowcut = !st.lowcut;
                                    pending_save = true;
                                    last_save_time = Instant::now();
                                    if dsp::is_daemon_running() {
                                        dsp::restart_daemon(&st);
                                    }
                                }
                                7 => {
                                    // Noise suppression toggle
                                    if rnnoise_installed {
                                        st.noise_suppression = !st.noise_suppression;
                                        pending_save = true;
                                        last_save_time = Instant::now();
                                        if dsp::is_daemon_running() {
                                            dsp::restart_daemon(&st);
                                        }
                                    }
                                }
                                8 => {
                                    // Listen toggle
                                    if is_mon {
                                        dsp::stop_monitoring();
                                    } else {
                                        if !dsp::is_daemon_running() {
                                            dsp::start_daemon(&st);
                                        }
                                        dsp::start_monitoring();
                                    }
                                }
                                9 => {
                                    // Mute toggle
                                    dsp::set_mute(None);
                                }
                                10 => {
                                    // Preset prev
                                    let curr_idx = presets.iter().position(|p| p == &st.preset).unwrap_or(0);
                                    let next_idx = if curr_idx > 0 { curr_idx - 1 } else { presets.len() - 1 };
                                    let next_name = &presets[next_idx];
                                    if let Ok(p_data) = state::load_preset(next_name) {
                                        st.bass = p_data.bass;
                                        st.mid = p_data.mid;
                                        st.treble = p_data.treble;
                                        st.lowcut = p_data.lowcut;
                                        st.noise_suppression = p_data.noise_suppression;
                                        st.gain = p_data.gain;
                                        st.fx_mode = p_data.fx_mode;
                                        st.preset = next_name.clone();
                                        pending_save = true;
                                        last_save_time = Instant::now();
                                        if dsp::is_daemon_running() {
                                            dsp::restart_daemon(&st);
                                        }
                                        status_msg = format!("Profil yüklendi: {}", next_name);
                                        status_msg_time = Instant::now();
                                    }
                                }
                                _ => {}
                            }
                        }
                        KeyCode::Right => {
                            match selected {
                                0 => {
                                    // FX mode next
                                    let curr_idx = fx_list.iter().position(|&x| x == st.fx_mode).unwrap_or(0);
                                    let next_idx = (curr_idx + 1) % fx_list.len();
                                    st.fx_mode = fx_list[next_idx].to_string();
                                    pending_save = true;
                                    last_save_time = Instant::now();
                                    if dsp::is_daemon_running() {
                                        dsp::restart_daemon(&st);
                                    }
                                    status_msg = format!("FX: {}", st.fx_mode);
                                    status_msg_time = Instant::now();
                                }
                                2 => {
                                    // Bass +
                                    st.bass = (st.bass + 0.5).clamp(-20.0, 30.0);
                                    st.bass = (st.bass * 10.0).round() / 10.0;
                                    st.preset = "özel".into();
                                    st.fx_mode = "none".into();
                                    pending_save = true;
                                    last_save_time = Instant::now();
                                    if dsp::is_daemon_running() {
                                        dsp::update_live_eq(st.bass, st.mid, st.treble);
                                    }
                                }
                                3 => {
                                    // Mid +
                                    st.mid = (st.mid + 0.5).clamp(-18.0, 20.0);
                                    st.mid = (st.mid * 10.0).round() / 10.0;
                                    st.preset = "özel".into();
                                    st.fx_mode = "none".into();
                                    pending_save = true;
                                    last_save_time = Instant::now();
                                    if dsp::is_daemon_running() {
                                        dsp::update_live_eq(st.bass, st.mid, st.treble);
                                    }
                                }
                                4 => {
                                    // Treble +
                                    st.treble = (st.treble + 0.5).clamp(-18.0, 20.0);
                                    st.treble = (st.treble * 10.0).round() / 10.0;
                                    st.preset = "özel".into();
                                    st.fx_mode = "none".into();
                                    pending_save = true;
                                    last_save_time = Instant::now();
                                    if dsp::is_daemon_running() {
                                        dsp::update_live_eq(st.bass, st.mid, st.treble);
                                    }
                                }
                                5 => {
                                    // Gain +
                                    st.gain = (st.gain + 5).clamp(0, 200);
                                    pending_save = true;
                                    last_save_time = Instant::now();
                                    dsp::apply_gain(st.gain);
                                }
                                6 => {
                                    // Lowcut toggle
                                    st.lowcut = !st.lowcut;
                                    pending_save = true;
                                    last_save_time = Instant::now();
                                    if dsp::is_daemon_running() {
                                        dsp::restart_daemon(&st);
                                    }
                                }
                                7 => {
                                    // Noise suppression toggle
                                    if rnnoise_installed {
                                        st.noise_suppression = !st.noise_suppression;
                                        pending_save = true;
                                        last_save_time = Instant::now();
                                        if dsp::is_daemon_running() {
                                            dsp::restart_daemon(&st);
                                        }
                                    }
                                }
                                8 => {
                                    // Listen toggle
                                    if is_mon {
                                        dsp::stop_monitoring();
                                    } else {
                                        if !dsp::is_daemon_running() {
                                            dsp::start_daemon(&st);
                                        }
                                        dsp::start_monitoring();
                                    }
                                }
                                9 => {
                                    // Mute toggle
                                    dsp::set_mute(None);
                                }
                                10 => {
                                    // Preset next
                                    let curr_idx = presets.iter().position(|p| p == &st.preset).unwrap_or(0);
                                    let next_idx = (curr_idx + 1) % presets.len();
                                    let next_name = &presets[next_idx];
                                    if let Ok(p_data) = state::load_preset(next_name) {
                                        st.bass = p_data.bass;
                                        st.mid = p_data.mid;
                                        st.treble = p_data.treble;
                                        st.lowcut = p_data.lowcut;
                                        st.noise_suppression = p_data.noise_suppression;
                                        st.gain = p_data.gain;
                                        st.fx_mode = p_data.fx_mode;
                                        st.preset = next_name.clone();
                                        pending_save = true;
                                        last_save_time = Instant::now();
                                        if dsp::is_daemon_running() {
                                            dsp::restart_daemon(&st);
                                        }
                                        status_msg = format!("Profil yüklendi: {}", next_name);
                                        status_msg_time = Instant::now();
                                    }
                                }
                                _ => {}
                            }
                        }
                        KeyCode::Enter => {
                            match selected {
                                6 => {
                                    st.lowcut = !st.lowcut;
                                    pending_save = true;
                                    last_save_time = Instant::now();
                                    if dsp::is_daemon_running() {
                                        dsp::restart_daemon(&st);
                                    }
                                }
                                7 => {
                                    if rnnoise_installed {
                                        st.noise_suppression = !st.noise_suppression;
                                        pending_save = true;
                                        last_save_time = Instant::now();
                                        if dsp::is_daemon_running() {
                                            dsp::restart_daemon(&st);
                                        }
                                    }
                                }
                                8 => {
                                    if is_mon {
                                        dsp::stop_monitoring();
                                    } else {
                                        if !dsp::is_daemon_running() {
                                            dsp::start_daemon(&st);
                                        }
                                        dsp::start_monitoring();
                                    }
                                }
                                9 => {
                                    dsp::set_mute(None);
                                }
                                11 => {
                                    if is_running {
                                        dsp::restart_daemon(&st);
                                        status_msg = "Motor yeniden başlatıldı".into();
                                    } else {
                                        dsp::start_daemon(&st);
                                        status_msg = "Motor başlatıldı".into();
                                    }
                                    status_msg_time = Instant::now();
                                }
                                _ => {}
                            }
                        }
                        KeyCode::Char('1') => {
                            dsp::play_sfx("roger");
                            status_msg = "📯 Roger SFX Çalındı".into();
                            status_msg_time = Instant::now();
                        }
                        KeyCode::Char('2') => {
                            dsp::play_sfx("nuke");
                            status_msg = "💥 Nuke Alarm SFX Çalındı".into();
                            status_msg_time = Instant::now();
                        }
                        KeyCode::Char('3') => {
                            dsp::play_sfx("airhorn");
                            status_msg = "📯 Airhorn SFX Çalındı".into();
                            status_msg_time = Instant::now();
                        }
                        KeyCode::Char('4') => {
                            dsp::play_sfx("beep");
                            status_msg = "🔔 Bip SFX Çalındı".into();
                            status_msg_time = Instant::now();
                        }
                        KeyCode::Char('s') | KeyCode::Char('S') => {
                            dsp::stop_audio_playback();
                            status_msg = "⏹️ Çalan ses durduruldu".into();
                            status_msg_time = Instant::now();
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') => {
                            crate::cli::handle_nuke_toggle(&mut st, false);
                            status_msg = if st.fx_mode == "nuke" { "💥 Nuke Modu Açıldı!".into() } else { "Normal moda dönüldü".into() };
                            status_msg_time = Instant::now();
                        }
                        KeyCode::Char('u') | KeyCode::Char('U') => {
                            crate::cli::handle_nuke_toggle(&mut st, true);
                            status_msg = if st.fx_mode == "ultra_nuke" { "💥💥 Ultra Nuke Açıldı!".into() } else { "Normal moda dönüldü".into() };
                            status_msg_time = Instant::now();
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            st = state::reset_state();
                            dsp::set_mute(Some(false));
                            dsp::stop_audio_playback();
                            if dsp::is_daemon_running() {
                                dsp::restart_daemon(&st);
                            }
                            dsp::apply_gain(100);
                            status_msg = "🔄 Fabrika ayarlarına sıfırlandı".into();
                            status_msg_time = Instant::now();
                        }
                        KeyCode::Char('x') | KeyCode::Char('X') => {
                            let muted = dsp::set_mute(None);
                            status_msg = if muted { "● Mikrofon Susturuldu".into() } else { "○ Mikrofon Açıldı".into() };
                            status_msg_time = Instant::now();
                        }
                        KeyCode::Char('l') | KeyCode::Char('L') => {
                            if is_mon {
                                dsp::stop_monitoring();
                                status_msg = "Dinleme kapatıldı".into();
                            } else {
                                if !dsp::is_daemon_running() {
                                    dsp::start_daemon(&st);
                                }
                                dsp::start_monitoring();
                                status_msg = "Dinleme açıldı (Kulaklık)".into();
                            }
                            status_msg_time = Instant::now();
                        }
                        KeyCode::Char('d') | KeyCode::Char('D') => {
                            if !dsp::is_daemon_running() {
                                dsp::start_daemon(&st);
                            }
                            if dsp::set_as_default_mic() {
                                status_msg = "✓ Varsayılan mikrofon yapıldı".into();
                            } else {
                                status_msg = "✗ Varsayılan mikrofon ayarlanamadı".into();
                            }
                            status_msg_time = Instant::now();
                        }
                        KeyCode::Char('t') | KeyCode::Char('T') => {
                            if !test_recording.load(Ordering::Relaxed) {
                                test_recording.store(true, Ordering::Relaxed);
                                let flag = Arc::clone(&test_recording);
                                thread::spawn(move || {
                                    let test_wav = "/tmp/micctl_test_sample.wav";
                                    let rec = Command::new("pw-record")
                                        .args(["--target", "micctl_source", test_wav])
                                        .stdout(Stdio::null())
                                        .stderr(Stdio::null())
                                        .spawn();
                                    thread::sleep(Duration::from_secs(2));
                                    if let Ok(mut p) = rec {
                                        let _ = p.kill();
                                        let _ = p.wait();
                                    }
                                    flag.store(false, Ordering::Relaxed);
                                    let _ = Command::new("pw-play")
                                        .arg(test_wav)
                                        .stdout(Stdio::null())
                                        .stderr(Stdio::null())
                                        .status();
                                });
                                status_msg = "🎙️ 2 sn ses kaydı alınıyor...".into();
                                status_msg_time = Instant::now();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
}
