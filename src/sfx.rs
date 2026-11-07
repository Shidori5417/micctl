use std::fs;
use std::path::PathBuf;
use crate::state::get_sounds_dir;

pub const AIRHORN_WAV: &[u8] = include_bytes!("../sounds/airhorn.wav");
pub const BEEP_WAV: &[u8] = include_bytes!("../sounds/beep.wav");
pub const NUKE_ALARM_WAV: &[u8] = include_bytes!("../sounds/nuke_alarm.wav");
pub const ROGER_BEEP_WAV: &[u8] = include_bytes!("../sounds/roger_beep.wav");

pub fn ensure_sounds() -> std::io::Result<()> {
    let s_dir = get_sounds_dir();
    fs::create_dir_all(&s_dir)?;

    let airhorn = s_dir.join("airhorn.wav");
    let beep = s_dir.join("beep.wav");
    let nuke = s_dir.join("nuke_alarm.wav");
    let roger = s_dir.join("roger_beep.wav");

    if !airhorn.exists() || airhorn.metadata().map(|m| m.len() == 0).unwrap_or(true) {
        let _ = fs::write(&airhorn, AIRHORN_WAV);
    }
    if !beep.exists() || beep.metadata().map(|m| m.len() == 0).unwrap_or(true) {
        let _ = fs::write(&beep, BEEP_WAV);
    }
    if !nuke.exists() || nuke.metadata().map(|m| m.len() == 0).unwrap_or(true) {
        let _ = fs::write(&nuke, NUKE_ALARM_WAV);
    }
    if !roger.exists() || roger.metadata().map(|m| m.len() == 0).unwrap_or(true) {
        let _ = fs::write(&roger, ROGER_BEEP_WAV);
    }

    Ok(())
}

pub fn get_sound_path(name: &str) -> Option<PathBuf> {
    let _ = ensure_sounds();
    let s_dir = get_sounds_dir();
    match name.to_lowercase().as_str() {
        "roger" | "roger_beep" | "roger-beep" | "radio" => Some(s_dir.join("roger_beep.wav")),
        "nuke" | "nuke_alarm" | "nuke-alarm" | "alarm" | "siren" => Some(s_dir.join("nuke_alarm.wav")),
        "airhorn" | "horn" | "korna" => Some(s_dir.join("airhorn.wav")),
        "beep" | "bip" => Some(s_dir.join("beep.wav")),
        _ => None,
    }
}
