# micctl — PipeWire Mikrofon DSP, Soundboard & FX Yöneticisi

Arch Linux üzerinde PipeWire ile çalışan mikrofon filtresi, canlı EQ, ses efektleri ve soundboard aracı. CLI ve TUI olarak kullanılır.

<p align="center">
  <img src="assets/screenshot_cli.png" alt="micctl CLI" width="850">
</p>

<p align="center">
  <img src="assets/screenshot_tui.png" alt="micctl TUI" width="850">
</p>

## Özellikler

- **Canlı EQ:** Bass, Mid ve Tiz değerleri çalışan filtre zincirine `pw-cli` ile anında uygulanır; filtre yeniden başlatılmaz.
- **Low-cut ve RNNoise:** 80 Hz masa vuruntusu kesici; isteğe bağlı RNNoise gürültü engelleme.
- **Nuke / Ultra Nuke:** Bas ve mid ağırlıklı agresif EQ ve %150 / %200 kazanç.
- **Ses efektleri:** `robot`, `rabbit`, `demon`, `walkie`, `phone`.
- **Soundboard:** `micctl play <dosya>` ile MP3/WAV/OGG dosyasını hem mikrofona hem kendi kulaklığına gönderir (`pw-link` ile çift yönlü yönlendirme).
- **Gömülü SFX:** `roger`, `nuke`, `airhorn`, `beep` sesleri binary içine gömülüdür.
- **Hazır profiller:** `podcast`, `gaming`, `deep-bass`, `asmr` ve diğerleri.
- **Mute, PTT, monitör:** `micctl mute`, `micctl ptt`, `micctl listen`.
- **Entegrasyon:** Waybar JSON çıktısı (`micctl waybar`), masaüstü bildirimleri (`notify-send`), systemd user servisi ile açılışta başlatma.

## Parametreler

| Parametre | Aralık | Varsayılan | Açıklama |
| :--- | :--- | :--- | :--- |
| **Bass** | `-20.0 dB` .. `+30.0 dB` | `+3.0 dB` | 120 Hz low-shelf |
| **Mid** | `-18.0 dB` .. `+20.0 dB` | `0.0 dB` | 1800 Hz peaking (Q=1.0) |
| **Tiz** | `-18.0 dB` .. `+20.0 dB` | `+1.5 dB` | 6000 Hz high-shelf |
| **Gain** | `%0` .. `%200` | `%100` | Çıkış seviyesi |
| **Low-cut** | Açık / Kapalı | Açık | 80 Hz high-pass |

## CLI

```bash
micctl                   # TUI (terminalde)
micctl status            # Durum özeti

# Canlı EQ
micctl bass +5.0         # 5 dB artır
micctl bass set 20       # -20..+30 dB aralığında mutlak değer
micctl mid +1.5
micctl treble +2.0
micctl gain 125%

# Nuke modları
micctl nuke              # Aç/kapat
micctl ultra             # Ultra Nuke (aç/kapat)

# Ses efektleri
micctl fx robot
micctl fx rabbit
micctl fx demon
micctl fx walkie
micctl fx phone
micctl fx none           # Normal moda dön

# Soundboard
micctl play ~/Müzik/siren.wav 120    # İkinci argüman ses yüzdesi (10-150)
micctl stop-play

# Hazır SFX
micctl sfx roger | nuke | airhorn | beep

# Test kaydı (1-10 sn, varsayılan 3)
micctl test 3

# Mute / PTT / monitör / gürültü engelleme
micctl mute toggle
micctl ptt start | stop
micctl hold-nuke start | stop
micctl listen on | off
micctl noise on          # RNNoise gerekir (aşağıya bak)

# Profil ve sistem
micctl preset list
micctl preset load podcast
micctl preset save benim-profilim
micctl default           # micctl_source'u varsayılan mikrofon yap
micctl autostart on      # systemd user servisi
micctl waybar            # Waybar JSON
micctl reset             # Fabrika ayarları
```

## TUI kısayolları

| Tuş | İşlev |
| :--- | :--- |
| `↑/↓` veya `k/j` | Menüde gezin |
| `←/→` | Değeri değiştir (canlı uygulanır) |
| `Enter` | Low-cut, RNNoise, monitör, mute gibi öğeleri aç/kapat |
| `1`–`4` | SFX çal (1 Roger, 2 Nuke, 3 Airhorn, 4 Bip) |
| `S` | Çalan sesi durdur |
| `N` / `U` | Nuke / Ultra Nuke aç/kapat |
| `X` | Mute aç/kapat |
| `L` | Monitörü aç/kapat |
| `T` | 2 saniyelik ses testi |
| `D` | Varsayılan mikrofon yap |
| `R` | Fabrika ayarları |
| `Q` / `Esc` | Çıkış |

## Derleme ve kurulum

```bash
# Bağımlılıklar (Arch)
sudo pacman -S pipewire libpulse mpv libnotify
# İsteğe bağlı: RNNoise (micctl noise on için)
sudo pacman -S noise-suppression-for-voice

cargo build --release
ln -sf "$(pwd)/target/release/micctl" ~/.local/bin/micctl
```

`pactl` (`libpulse`) ve `mpv` soundboard için, `notify-send` (`libnotify`) bildirimler için gereklidir.

## Dosyalar

Yapılandırma `$XDG_CONFIG_HOME/micctl` altında tutulur (genelde `~/.config/micctl`):

- `config.json`: ayarlar ve aktif durum
- `filter-chain.conf`: otomatik üretilen PipeWire filtre zinciri
- `presets/*.json`: hazır profiller
- `sounds/*.wav`: gömülü seslerin kopyaları
- `~/.config/systemd/user/micctl.service`: `micctl autostart on` ile oluşturulur
