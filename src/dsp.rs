use std::fs;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::Duration;

use crate::sfx;
use crate::state::{
    get_daemon_pid_file, get_filter_conf_file, get_monitor_pid_file, get_playback_pid_file,
    get_systemd_service_file, State,
};

static CACHED_NODE_ID: AtomicU32 = AtomicU32::new(0);

const RNNOISE_CANDIDATE_PATHS: &[&str] = &[
    "/usr/lib/ladspa/librnnoise_ladspa.so",
    "/usr/lib64/ladspa/librnnoise_ladspa.so",
    "/usr/local/lib/ladspa/librnnoise_ladspa.so",
];

pub fn find_rnnoise_plugin() -> Option<&'static str> {
    for p in RNNOISE_CANDIDATE_PATHS {
        if Path::new(p).exists() {
            return Some(*p);
        }
    }
    None
}

pub fn send_notification(title: &str, message: &str, urgency: &str) {
    // Try notify-send with synchronous hint to prevent spamming notification bubbles in Hyprland
    let status = Command::new("notify-send")
        .args([
            "-a",
            "micctl",
            "-h",
            "string:x-canonical-private-synchronous:micctl",
            "-u",
            urgency,
            title,
            message,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if status.is_err() {
        let urg = match urgency {
            "critical" => notify_rust::Urgency::Critical,
            "low" => notify_rust::Urgency::Low,
            _ => notify_rust::Urgency::Normal,
        };
        let _ = notify_rust::Notification::new()
            .appname("micctl")
            .summary(title)
            .body(message)
            .urgency(urg)
            .show();
    }
}

pub fn get_capture_node_id() -> Option<u32> {
    let cached = CACHED_NODE_ID.load(Ordering::Relaxed);
    if cached > 0 {
        return Some(cached);
    }

    let output = Command::new("pw-dump").output().ok()?;
    if !output.status.success() {
        return None;
    }

    let json_val: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let arr = json_val.as_array()?;

    for obj in arr {
        if let Some(props) = obj.get("info").and_then(|i| i.get("props")) {
            if props.get("node.name").and_then(|n| n.as_str()) == Some("micctl_capture") {
                if let Some(id) = obj.get("id").and_then(|i| i.as_u64()) {
                    let id_u32 = id as u32;
                    CACHED_NODE_ID.store(id_u32, Ordering::Relaxed);
                    return Some(id_u32);
                }
            }
        }
    }

    None
}

pub fn invalidate_cached_node_id() {
    CACHED_NODE_ID.store(0, Ordering::Relaxed);
}

/// Ultra-fast (0-5ms) zero-lag in-memory PipeWire kernel param update.
pub fn update_live_eq(bass: f64, mid: f64, treble: f64) -> bool {
    let mut node_id = match get_capture_node_id() {
        Some(id) => id,
        None => return false,
    };

    let props_str = format!(
        r#"{{ "params": [ "eq_bass:Gain", {:.1}, "eq_mid:Gain", {:.1}, "eq_treble:Gain", {:.1} ] }}"#,
        bass, mid, treble
    );

    let res = Command::new("pw-cli")
        .args(["s", &node_id.to_string(), "Props", &props_str])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if let Ok(st) = res {
        if st.success() {
            return true;
        }
    }

    // If failed, cache might be invalid (e.g. node restarted)
    invalidate_cached_node_id();
    node_id = match get_capture_node_id() {
        Some(id) => id,
        None => return false,
    };

    let retry_res = Command::new("pw-cli")
        .args(["s", &node_id.to_string(), "Props", &props_str])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    retry_res.map(|s| s.success()).unwrap_or(false)
}

struct FilterNode {
    node_type: &'static str,
    name: &'static str,
    plugin: Option<&'static str>,
    label: &'static str,
    control: String,
}

pub fn generate_filter_conf(state: &State) -> String {
    let mut nodes: Vec<FilterNode> = Vec::new();
    let fx = state.fx_mode.as_str();

    match fx {
        "ultra_nuke" => {
            // Ultra Nuke / Kıyamet Modu: Multi-stage sub bass saturation (+50 dB total boost)
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "u_sub60",
                plugin: None,
                label: "bq_lowshelf",
                control: r#"{ "Freq" = 60.0 "Q" = 2.5 "Gain" = 18.0 }"#.into(),
            });
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "u_bass100",
                plugin: None,
                label: "bq_lowshelf",
                control: r#"{ "Freq" = 100.0 "Q" = 2.0 "Gain" = 18.0 }"#.into(),
            });
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "u_bass160",
                plugin: None,
                label: "bq_lowshelf",
                control: r#"{ "Freq" = 160.0 "Q" = 1.5 "Gain" = 15.0 }"#.into(),
            });
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "u_mid_dist",
                plugin: None,
                label: "bq_peaking",
                control: r#"{ "Freq" = 1400.0 "Q" = 2.5 "Gain" = 15.0 }"#.into(),
            });
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "u_high_air",
                plugin: None,
                label: "bq_highshelf",
                control: r#"{ "Freq" = 5000.0 "Q" = 1.0 "Gain" = 10.0 }"#.into(),
            });
        }
        "nuke" => {
            // Standard Nuke Mode
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "nuke_sub",
                plugin: None,
                label: "bq_lowshelf",
                control: r#"{ "Freq" = 80.0 "Q" = 2.0 "Gain" = 15.0 }"#.into(),
            });
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "nuke_bass",
                plugin: None,
                label: "bq_lowshelf",
                control: r#"{ "Freq" = 150.0 "Q" = 1.5 "Gain" = 15.0 }"#.into(),
            });
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "nuke_mid",
                plugin: None,
                label: "bq_peaking",
                control: r#"{ "Freq" = 1200.0 "Q" = 2.0 "Gain" = 12.0 }"#.into(),
            });
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "nuke_high",
                plugin: None,
                label: "bq_highshelf",
                control: r#"{ "Freq" = 4500.0 "Q" = 1.0 "Gain" = 8.0 }"#.into(),
            });
        }
        "robot" => {
            // Metallic Robot / Cybernetic Dalek Mode
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "rob_hp",
                plugin: None,
                label: "bq_highpass",
                control: r#"{ "Freq" = 180.0 "Q" = 1.0 }"#.into(),
            });
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "rob_res1",
                plugin: None,
                label: "bq_peaking",
                control: r#"{ "Freq" = 450.0 "Q" = 7.0 "Gain" = 14.0 }"#.into(),
            });
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "rob_res2",
                plugin: None,
                label: "bq_peaking",
                control: r#"{ "Freq" = 950.0 "Q" = 8.0 "Gain" = 16.0 }"#.into(),
            });
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "rob_res3",
                plugin: None,
                label: "bq_peaking",
                control: r#"{ "Freq" = 1900.0 "Q" = 7.0 "Gain" = 14.0 }"#.into(),
            });
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "rob_lp",
                plugin: None,
                label: "bq_lowpass",
                control: r#"{ "Freq" = 3600.0 "Q" = 1.0 }"#.into(),
            });
        }
        "rabbit" => {
            // Rabbit / Chipmunk / Helium Mode
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "rab_hp",
                plugin: None,
                label: "bq_highpass",
                control: r#"{ "Freq" = 550.0 "Q" = 1.5 }"#.into(),
            });
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "rab_cut_bass",
                plugin: None,
                label: "bq_lowshelf",
                control: r#"{ "Freq" = 400.0 "Q" = 0.707 "Gain" = -18.0 }"#.into(),
            });
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "rab_nasal",
                plugin: None,
                label: "bq_peaking",
                control: r#"{ "Freq" = 2600.0 "Q" = 3.0 "Gain" = 14.0 }"#.into(),
            });
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "rab_squeak",
                plugin: None,
                label: "bq_highshelf",
                control: r#"{ "Freq" = 4800.0 "Q" = 1.5 "Gain" = 16.0 }"#.into(),
            });
        }
        "demon" => {
            // Demon / Monster Deep Growl Mode
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "dem_sub",
                plugin: None,
                label: "bq_lowshelf",
                control: r#"{ "Freq" = 75.0 "Q" = 2.0 "Gain" = 18.0 }"#.into(),
            });
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "dem_growl",
                plugin: None,
                label: "bq_peaking",
                control: r#"{ "Freq" = 190.0 "Q" = 2.5 "Gain" = 14.0 }"#.into(),
            });
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "dem_lp",
                plugin: None,
                label: "bq_lowpass",
                control: r#"{ "Freq" = 1100.0 "Q" = 1.2 }"#.into(),
            });
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "dem_cut_high",
                plugin: None,
                label: "bq_highshelf",
                control: r#"{ "Freq" = 1800.0 "Q" = 0.707 "Gain" = -16.0 }"#.into(),
            });
        }
        "walkie_talkie" => {
            // Walkie-Talkie / Megaphone Mode
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "hp_radio",
                plugin: None,
                label: "bq_highpass",
                control: r#"{ "Freq" = 500.0 "Q" = 1.0 }"#.into(),
            });
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "lp_radio",
                plugin: None,
                label: "bq_lowpass",
                control: r#"{ "Freq" = 3200.0 "Q" = 1.0 }"#.into(),
            });
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "peak_radio1",
                plugin: None,
                label: "bq_peaking",
                control: r#"{ "Freq" = 1800.0 "Q" = 3.0 "Gain" = 10.0 }"#.into(),
            });
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "peak_radio2",
                plugin: None,
                label: "bq_peaking",
                control: r#"{ "Freq" = 2600.0 "Q" = 2.5 "Gain" = 6.0 }"#.into(),
            });
        }
        "telephone" => {
            // Vintage Telephone Mode
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "hp_phone",
                plugin: None,
                label: "bq_highpass",
                control: r#"{ "Freq" = 300.0 "Q" = 1.0 }"#.into(),
            });
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "lp_phone",
                plugin: None,
                label: "bq_lowpass",
                control: r#"{ "Freq" = 3400.0 "Q" = 1.0 }"#.into(),
            });
            nodes.push(FilterNode {
                node_type: "builtin",
                name: "peak_phone",
                plugin: None,
                label: "bq_peaking",
                control: r#"{ "Freq" = 1200.0 "Q" = 1.5 "Gain" = 6.0 }"#.into(),
            });
        }
        _ => {
            // Standard Studio EQ Chain (dynamically adjustable via pw-cli Props)
            if state.lowcut {
                nodes.push(FilterNode {
                    node_type: "builtin",
                    name: "eq_hp",
                    plugin: None,
                    label: "bq_highpass",
                    control: r#"{ "Freq" = 80.0 "Q" = 0.707 }"#.into(),
                });
            }

            nodes.push(FilterNode {
                node_type: "builtin",
                name: "eq_bass",
                plugin: None,
                label: "bq_lowshelf",
                control: format!(
                    r#"{{ "Freq" = {:.1} "Q" = 0.707 "Gain" = {:.1} }}"#,
                    state.bass_freq, state.bass
                ),
            });

            nodes.push(FilterNode {
                node_type: "builtin",
                name: "eq_mid",
                plugin: None,
                label: "bq_peaking",
                control: format!(
                    r#"{{ "Freq" = {:.1} "Q" = 1.0 "Gain" = {:.1} }}"#,
                    state.mid_freq, state.mid
                ),
            });

            nodes.push(FilterNode {
                node_type: "builtin",
                name: "eq_treble",
                plugin: None,
                label: "bq_highshelf",
                control: format!(
                    r#"{{ "Freq" = {:.1} "Q" = 0.707 "Gain" = {:.1} }}"#,
                    state.treble_freq, state.treble
                ),
            });

            if state.noise_suppression && find_rnnoise_plugin().is_some() {
                // Insert RNNoise after low-cut
                let insert_idx = if state.lowcut { 1 } else { 0 };
                nodes.insert(
                    insert_idx,
                    FilterNode {
                        node_type: "ladspa",
                        name: "rnnoise",
                        plugin: Some("librnnoise_ladspa"),
                        label: "noise_suppressor_stereo",
                        control: r#"{ "VAD Threshold (%)" = 50.0 }"#.into(),
                    },
                );
            }
        }
    }

    let mut links = Vec::new();
    for i in 0..nodes.len().saturating_sub(1) {
        links.push(format!(
            r#"{{ output = "{}:Out" input = "{}:In" }}"#,
            nodes[i].name, nodes[i + 1].name
        ));
    }

    let mut nodes_str = String::new();
    for n in &nodes {
        nodes_str.push_str("                    {\n");
        nodes_str.push_str(&format!("                        type    = {}\n", n.node_type));
        nodes_str.push_str(&format!("                        name    = {}\n", n.name));
        if let Some(p) = n.plugin {
            nodes_str.push_str(&format!("                        plugin  = \"{}\"\n", p));
        }
        nodes_str.push_str(&format!("                        label   = {}\n", n.label));
        nodes_str.push_str(&format!("                        control = {}\n", n.control));
        nodes_str.push_str("                    }\n");
    }

    let mut links_str = String::new();
    for link in &links {
        links_str.push_str(&format!("                    {}\n", link));
    }

    format!(
        r#"# micctl auto-generated configuration
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
{}                ]
                links = [
{}                ]
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
"#,
        nodes_str, links_str
    )
}

pub fn is_pid_alive(pid: u32) -> bool {
    let proc_path = format!("/proc/{}", pid);
    if !Path::new(&proc_path).exists() {
        return false;
    }
    // Check if not zombie
    if let Ok(stat) = fs::read_to_string(format!("{}/stat", proc_path)) {
        let parts: Vec<&str> = stat.split_whitespace().collect();
        if parts.len() > 2 && parts[2] == "Z" {
            return false;
        }
    }
    true
}

pub fn is_daemon_running() -> bool {
    let pid_file = get_daemon_pid_file();
    if !pid_file.exists() {
        return false;
    }
    if let Ok(content) = fs::read_to_string(&pid_file) {
        if let Ok(pid) = content.trim().parse::<u32>() {
            if is_pid_alive(pid) {
                return true;
            }
        }
    }
    let _ = fs::remove_file(pid_file);
    false
}

pub fn get_daemon_pid() -> Option<u32> {
    if is_daemon_running() {
        let pid_file = get_daemon_pid_file();
        if let Ok(content) = fs::read_to_string(pid_file) {
            return content.trim().parse::<u32>().ok();
        }
    }
    None
}

pub fn write_config(state: &State) -> std::io::Result<()> {
    crate::state::ensure_dirs()?;
    let content = generate_filter_conf(state);
    fs::write(get_filter_conf_file(), content)
}

pub fn apply_gain(gain_pct: u32) {
    let pct = gain_pct.min(200);
    let _ = Command::new("pactl")
        .args(["set-source-volume", "micctl_source", &format!("{}%", pct)])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

pub fn is_muted() -> bool {
    if let Ok(out) = Command::new("pactl")
        .args(["get-source-mute", "micctl_source"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&out.stdout).to_lowercase();
        return stdout.contains("yes");
    }
    false
}

pub fn set_mute(val: Option<bool>) -> bool {
    let arg = match val {
        None => "toggle",
        Some(true) => "1",
        Some(false) => "0",
    };
    let _ = Command::new("pactl")
        .args(["set-source-mute", "micctl_source", arg])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    let muted = is_muted();
    let status_str = if muted {
        "Susturuldu (Muted)"
    } else {
        "Açıldı (Unmuted)"
    };
    send_notification("🎙️ Mikrofon", status_str, "normal");
    muted
}

pub fn get_default_source() -> String {
    if let Ok(out) = Command::new("pactl")
        .arg("get-default-source")
        .output()
    {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !s.is_empty() {
            return s;
        }
    }
    "Bilinmiyor".to_string()
}

pub fn set_as_default_mic() -> bool {
    Command::new("pactl")
        .args(["set-default-source", "micctl_source"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn ensure_soundboard() {
    let sinks_out = Command::new("pactl")
        .args(["list", "sinks", "short"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    if !sinks_out.contains("micctl_soundboard") {
        let _ = Command::new("pactl")
            .args([
                "load-module",
                "module-null-sink",
                "sink_name=micctl_soundboard",
                "sink_properties=device.description=micctl_Soundboard",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        thread::sleep(Duration::from_millis(100));
    }

    // 1. Link soundboard to microphone capture node (Discord/app sees it)
    let _ = Command::new("pw-link")
        .args(["micctl_soundboard:monitor_FL", "micctl_capture:input_FL"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = Command::new("pw-link")
        .args(["micctl_soundboard:monitor_FR", "micctl_capture:input_FR"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    // 2. Link soundboard to user's headphones / default sink (User hears it)
    if let Ok(out) = Command::new("pactl").arg("get-default-sink").output() {
        let def_sink = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !def_sink.is_empty() && def_sink != "micctl_soundboard" {
            let _ = Command::new("pw-link")
                .args(["micctl_soundboard:monitor_FL", &format!("{}:playback_FL", def_sink)])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            let _ = Command::new("pw-link")
                .args(["micctl_soundboard:monitor_FR", &format!("{}:playback_FR", def_sink)])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }
}

pub fn play_audio_to_mic(file_path: &Path, volume: u32) -> bool {
    let p = match file_path.canonicalize() {
        Ok(path) => path,
        Err(_) => return false,
    };
    if !p.exists() {
        return false;
    }

    ensure_soundboard();
    stop_audio_playback();

    let mut cmd = Command::new("mpv");
    cmd.args([
        "--no-config",
        "--keep-open=no",
        "--no-video",
        "--audio-device=pulse/micctl_soundboard",
        &format!("--volume={}", volume.clamp(10, 150)),
        p.to_str().unwrap_or_default(),
    ]);
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }

    if let Ok(child_proc) = cmd.spawn() {
        let pid = child_proc.id();
        let _ = fs::write(get_playback_pid_file(), pid.to_string());
        true
    } else {
        false
    }
}

pub fn stop_audio_playback() -> bool {
    let pid_file = get_playback_pid_file();
    if pid_file.exists() {
        if let Ok(content) = fs::read_to_string(&pid_file) {
            if let Ok(pid) = content.trim().parse::<u32>() {
                unsafe {
                    libc::kill(pid as i32, libc::SIGTERM);
                }
            }
        }
        let _ = fs::remove_file(&pid_file);
    }

    let _ = Command::new("pkill")
        .args(["-f", "mpv.*micctl_soundboard"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    true
}

pub fn is_audio_playing() -> bool {
    let pid_file = get_playback_pid_file();
    if !pid_file.exists() {
        return false;
    }
    if let Ok(content) = fs::read_to_string(&pid_file) {
        if let Ok(pid) = content.trim().parse::<u32>() {
            if is_pid_alive(pid) {
                return true;
            }
        }
    }
    let _ = fs::remove_file(pid_file);
    false
}

pub fn play_sfx(name: &str) {
    let _ = sfx::ensure_sounds();
    if let Some(path) = sfx::get_sound_path(name) {
        if path.exists() {
            play_audio_to_mic(&path, 100);
        }
    }
}

pub fn start_daemon(state: &State) -> bool {
    invalidate_cached_node_id();
    if is_daemon_running() {
        ensure_soundboard();
        return true;
    }

    if write_config(state).is_err() {
        return false;
    }

    let conf_path = get_filter_conf_file();
    let mut cmd = Command::new("pipewire");
    cmd.args(["-c", conf_path.to_str().unwrap_or_default()]);
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }

    if let Ok(proc) = cmd.spawn() {
        let pid = proc.id();
        let _ = fs::write(get_daemon_pid_file(), pid.to_string());

        // Wait until micctl_source appears in pactl list sources
        for _ in 0..20 {
            thread::sleep(Duration::from_millis(100));
            if let Ok(out) = Command::new("pactl").args(["list", "sources", "short"]).output() {
                let text = String::from_utf8_lossy(&out.stdout);
                if text.contains("micctl_source") {
                    apply_gain(state.gain);
                    ensure_soundboard();
                    return true;
                }
            }
        }
    }

    false
}

pub fn stop_daemon() -> bool {
    invalidate_cached_node_id();
    if let Some(pid) = get_daemon_pid() {
        unsafe {
            libc::kill(pid as i32, libc::SIGTERM);
        }
        for _ in 0..15 {
            if !is_pid_alive(pid) {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        if is_pid_alive(pid) {
            unsafe {
                libc::kill(pid as i32, libc::SIGKILL);
            }
        }
        let _ = fs::remove_file(get_daemon_pid_file());
    }
    let _ = Command::new("pkill")
        .args(["-f", "pipewire.*filter-chain.conf"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    stop_monitoring();
    stop_audio_playback();
    true
}

pub fn restart_daemon(state: &State) -> bool {
    stop_daemon();
    thread::sleep(Duration::from_millis(100));
    start_daemon(state)
}

pub fn is_monitoring() -> bool {
    let pid_file = get_monitor_pid_file();
    if !pid_file.exists() {
        return false;
    }
    if let Ok(content) = fs::read_to_string(&pid_file) {
        if let Ok(pid) = content.trim().parse::<u32>() {
            if is_pid_alive(pid) {
                return true;
            }
        }
    }
    let _ = fs::remove_file(pid_file);
    false
}

pub fn start_monitoring() -> bool {
    if is_monitoring() {
        return true;
    }
    let mut cmd = Command::new("pw-loopback");
    cmd.args(["-C", "micctl_source"]);
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }

    if let Ok(proc) = cmd.spawn() {
        let _ = fs::write(get_monitor_pid_file(), proc.id().to_string());
        true
    } else {
        false
    }
}

pub fn stop_monitoring() -> bool {
    let pid_file = get_monitor_pid_file();
    if pid_file.exists() {
        if let Ok(content) = fs::read_to_string(&pid_file) {
            if let Ok(pid) = content.trim().parse::<u32>() {
                unsafe {
                    libc::kill(pid as i32, libc::SIGTERM);
                }
            }
        }
        let _ = fs::remove_file(&pid_file);
    }
    let _ = Command::new("pkill")
        .args(["-f", "pw-loopback.*micctl_source"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    true
}

pub fn is_systemd_enabled() -> bool {
    Command::new("systemctl")
        .args(["--user", "is-enabled", "micctl"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("enabled"))
        .unwrap_or(false)
}

pub fn ensure_systemd_service_file() -> std::io::Result<()> {
    let svc_file = get_systemd_service_file();
    if !svc_file.exists() {
        let content = r#"[Unit]
Description=micctl Microphone DSP Filter Chain Service
After=pipewire.service
PartOf=pipewire.service

[Service]
Type=simple
ExecStart=/usr/bin/pipewire -c %h/.config/micctl/filter-chain.conf
Restart=on-failure
RestartSec=1

[Install]
WantedBy=pipewire.service
"#;
        fs::write(svc_file, content)?;
        let _ = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status();
    }
    Ok(())
}

pub fn set_systemd_autostart(enable: bool) -> bool {
    let _ = ensure_systemd_service_file();
    let action = if enable { "enable" } else { "disable" };
    Command::new("systemctl")
        .args(["--user", action, "micctl"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
