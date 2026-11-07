use std::fs::{self, File};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

fn default_true() -> bool { true }
fn default_false() -> bool { false }
fn default_bass() -> f64 { 3.0 }
fn default_bass_freq() -> f64 { 120.0 }
fn default_mid() -> f64 { 0.0 }
fn default_mid_freq() -> f64 { 1800.0 }
fn default_treble() -> f64 { 1.5 }
fn default_treble_freq() -> f64 { 6000.0 }
fn default_gain() -> u32 { 100 }
fn default_preset() -> String { "default".to_string() }
fn default_fx() -> String { "none".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateBackup {
    pub bass: f64,
    pub mid: f64,
    pub treble: f64,
    pub gain: u32,
    pub lowcut: bool,
    pub preset: String,
    pub fx_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_bass")]
    pub bass: f64,
    #[serde(default = "default_bass_freq")]
    pub bass_freq: f64,
    #[serde(default = "default_mid")]
    pub mid: f64,
    #[serde(default = "default_mid_freq")]
    pub mid_freq: f64,
    #[serde(default = "default_treble")]
    pub treble: f64,
    #[serde(default = "default_treble_freq")]
    pub treble_freq: f64,
    #[serde(default = "default_true")]
    pub lowcut: bool,
    #[serde(default = "default_false")]
    pub noise_suppression: bool,
    #[serde(default = "default_gain")]
    pub gain: u32,
    #[serde(default = "default_false")]
    pub listen: bool,
    #[serde(default = "default_preset")]
    pub preset: String,
    #[serde(default = "default_fx")]
    pub fx_mode: String,
    #[serde(default)]
    pub backup_state: Option<Box<StateBackup>>,
    #[serde(default = "default_true")]
    pub notify: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            enabled: true,
            bass: 3.0,
            bass_freq: 120.0,
            mid: 0.0,
            mid_freq: 1800.0,
            treble: 1.5,
            treble_freq: 6000.0,
            lowcut: true,
            noise_suppression: false,
            gain: 100,
            listen: false,
            preset: "default".to_string(),
            fx_mode: "none".to_string(),
            backup_state: None,
            notify: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub bass: f64,
    pub mid: f64,
    pub treble: f64,
    pub lowcut: bool,
    pub noise_suppression: bool,
    pub gain: u32,
    pub fx_mode: String,
}

pub fn get_config_dir() -> PathBuf {
    if let Some(user_dirs) = directories::BaseDirs::new() {
        user_dirs.config_dir().join("micctl")
    } else {
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
            .join(".config")
            .join("micctl")
    }
}

pub fn get_config_file() -> PathBuf {
    get_config_dir().join("config.json")
}

pub fn get_presets_dir() -> PathBuf {
    get_config_dir().join("presets")
}

pub fn get_sounds_dir() -> PathBuf {
    get_config_dir().join("sounds")
}

pub fn get_filter_conf_file() -> PathBuf {
    get_config_dir().join("filter-chain.conf")
}

pub fn get_daemon_pid_file() -> PathBuf {
    get_config_dir().join("daemon.pid")
}

pub fn get_monitor_pid_file() -> PathBuf {
    get_config_dir().join("monitor.pid")
}

pub fn get_playback_pid_file() -> PathBuf {
    get_config_dir().join("playback.pid")
}

pub fn get_systemd_service_file() -> PathBuf {
    if let Some(user_dirs) = directories::BaseDirs::new() {
        user_dirs.config_dir().join("systemd").join("user").join("micctl.service")
    } else {
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
            .join(".config")
            .join("systemd")
            .join("user")
            .join("micctl.service")
    }
}

pub fn ensure_dirs() -> std::io::Result<()> {
    let cfg = get_config_dir();
    fs::create_dir_all(&cfg)?;
    let p_dir = get_presets_dir();
    fs::create_dir_all(&p_dir)?;
    let s_dir = get_sounds_dir();
    fs::create_dir_all(&s_dir)?;

    if let Some(parent) = get_systemd_service_file().parent() {
        fs::create_dir_all(parent)?;
    }

    // Default presets
    let builtin: [(&str, Preset); 12] = [
        ("podcast", Preset { bass: 5.0, mid: 1.5, treble: 2.5, lowcut: true, noise_suppression: false, gain: 100, fx_mode: "none".into() }),
        ("gaming", Preset { bass: 2.0, mid: 2.5, treble: 3.0, lowcut: true, noise_suppression: true, gain: 100, fx_mode: "none".into() }),
        ("natural", Preset { bass: 0.0, mid: 0.0, treble: 0.0, lowcut: true, noise_suppression: false, gain: 100, fx_mode: "none".into() }),
        ("deep-bass", Preset { bass: 12.0, mid: -1.0, treble: 1.0, lowcut: true, noise_suppression: false, gain: 105, fx_mode: "none".into() }),
        ("extreme-bass", Preset { bass: 24.0, mid: -2.0, treble: 0.0, lowcut: false, noise_suppression: false, gain: 120, fx_mode: "none".into() }),
        ("nuke", Preset { bass: 18.0, mid: 10.0, treble: 6.0, lowcut: false, noise_suppression: false, gain: 150, fx_mode: "nuke".into() }),
        ("ultra-nuke", Preset { bass: 30.0, mid: 15.0, treble: 10.0, lowcut: false, noise_suppression: false, gain: 200, fx_mode: "ultra_nuke".into() }),
        ("robot", Preset { bass: 0.0, mid: 12.0, treble: -4.0, lowcut: true, noise_suppression: false, gain: 115, fx_mode: "robot".into() }),
        ("rabbit", Preset { bass: -18.0, mid: 8.0, treble: 16.0, lowcut: true, noise_suppression: false, gain: 130, fx_mode: "rabbit".into() }),
        ("demon", Preset { bass: 22.0, mid: -6.0, treble: -14.0, lowcut: false, noise_suppression: false, gain: 125, fx_mode: "demon".into() }),
        ("walkie-talkie", Preset { bass: -12.0, mid: 9.0, treble: -8.0, lowcut: true, noise_suppression: false, gain: 115, fx_mode: "walkie_talkie".into() }),
        ("asmr", Preset { bass: 4.0, mid: 1.0, treble: 8.5, lowcut: true, noise_suppression: false, gain: 135, fx_mode: "none".into() }),
    ];

    for (name, preset) in builtin {
        let file_path = p_dir.join(format!("{}.json", name));
        if !file_path.exists() {
            if let Ok(mut f) = File::create(&file_path) {
                let _ = serde_json::to_writer_pretty(&mut f, &preset);
            }
        }
    }

    Ok(())
}

pub fn load_state() -> State {
    let _ = ensure_dirs();
    let path = get_config_file();
    if !path.exists() {
        let def = State::default();
        let _ = save_state(&def);
        return def;
    }

    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str::<State>(&content).unwrap_or_else(|_| {
            let def = State::default();
            let _ = save_state(&def);
            def
        }),
        Err(_) => {
            let def = State::default();
            let _ = save_state(&def);
            def
        }
    }
}

pub fn save_state(state: &State) -> std::io::Result<()> {
    ensure_dirs()?;
    let path = get_config_file();
    let json_bytes = serde_json::to_vec_pretty(state)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    fs::write(path, json_bytes)
}

pub fn reset_state() -> State {
    let _ = ensure_dirs();
    let def = State::default();
    let _ = save_state(&def);
    def
}

pub fn list_presets() -> Vec<String> {
    let _ = ensure_dirs();
    let dir = get_presets_dir();
    let mut names = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    names.push(stem.to_string());
                }
            }
        }
    }
    names.sort();
    names
}

pub fn load_preset(name: &str) -> Result<Preset, String> {
    let _ = ensure_dirs();
    let path = get_presets_dir().join(format!("{}.json", name));
    if !path.exists() {
        return Err(format!("Ön ayar bulunamadı: {}", name));
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str::<Preset>(&content).map_err(|e| e.to_string())
}

pub fn save_preset(name: &str, state: &State) -> std::io::Result<()> {
    ensure_dirs()?;
    let path = get_presets_dir().join(format!("{}.json", name));
    let preset = Preset {
        bass: state.bass,
        mid: state.mid,
        treble: state.treble,
        lowcut: state.lowcut,
        noise_suppression: state.noise_suppression,
        gain: state.gain,
        fx_mode: state.fx_mode.clone(),
    };
    let json_bytes = serde_json::to_vec_pretty(&preset)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    fs::write(path, json_bytes)
}
