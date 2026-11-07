# micctl — PipeWire Mikrofon DSP, Soundboard & FX Yöneticisi

PipeWire 1.6 altyapısı üzerinde çalışan, **sıfır gecikmeli (zero-lag kernel param update)**, yerel ve bağımsız mikrofon filtreleme ve ses çalma (soundboard) aracı.

## Öne Çıkan Özellikler
- **⚡ Sıfır Gecikmeli Canlı EQ (Zero-Lag Kernel Updates):** Bass, Mid ve Tiz kaydırıcıları değiştirilirken filtre servisi yeniden başlatılmaz; PipeWire SPA çekirdek parametreleri anlık (~5ms) güncellenir. TUI içinde tuşlara ne kadar hızlı basarsanız basın asla kasma veya donma yaşanmaz.
- **💥💥 Ultra Nuke / Kıyamet Modu (`micctl ultra`):** 5 aşamalı +50 dB bas doygunluğu, aşırı mid rezonansı ve %200 ses kazancı ile kulak zarı titreten ekstrem nuke modu.
- **🤖 Robot Modu (`micctl fx robot`):** Dar bantlı metalik rezonans filtreleri ile Cybernetic Dalek / Sci-Fi robot sesi.
- **🐰 Tavşan / Helyum Modu (`micctl fx rabbit`):** 550 Hz High-pass ve +16 dB tiz tepe filtresi ile çizgi film tavşanı / sincap / helyum gazı incelmiş ses efekti.
- **👹 Canavar / Demon Modu (`micctl fx demon`):** 75 Hz derin sub-bass boost ve 1100 Hz Low-pass filtresi ile karanlık, kalın canavar hırıltısı.
- **🔊 Mikrofona Ses Çalma (Soundboard):** `micctl play <dosya>` ile herhangi bir MP3, WAV veya OGG dosyasını hem mikrofondan Discord/oyunlara aktarır hem de kendi kulaklığından eş zamanlı dinletir.
- **📯 Hazır SFX Efektleri:** Tek komutla mikrofona telsiz cızırtısı (`roger`), nükleer siren (`nuke`), korna (`airhorn`) veya uyarı bip (`beep`) çalma.
- **🔄 Varsayılanlara Sıfırlama (`micctl reset`):** Tüm EQ, kazanç ve efektleri tek tuşla/komutla fabrika ayarlarına döndürür.
- **🎙️ Hızlı Ses Testi (`micctl test`):** 3 saniyelik ses kaydı alıp anında kulaklığa çalarak ses tonunu test etmeni sağlar.
- **📻 Telsiz / Megafon (Walkie-Talkie):** 500Hz - 3200Hz bant filtre + rezonans ile askeri telsiz efekti.
- **☎️ Eski Telefon (Telephone):** Sabit hat filtreleme efekti.
- **🔔 Masaüstü Bildirimleri:** Ayar veya mod değiştiğinde Hyprland üzerinde `notify-send` ile anlık görsel bildirim.
- **🐟 Fish Shell Desteği:** Tab tuşu ile tüm komutlar, parametreler, dosyalar ve hazır profiller otomatik tamamlanır.
- **🚀 Açılışta Otomatik Başlama:** `micctl autostart on` ile systemd servisini tek komutla açma/kapatma.
- **🔇 Mute & Bas-Konuş (PTT):** Tek tuşla susturma (`micctl mute toggle`) ve Hyprland için bas-konuş (`micctl ptt start/stop`).

---

## Genişletilmiş Parametre Aralıkları
- **Bass:** `-20.0 dB` .. `+30.0 dB` (Aşırı bas doygunluğu)
- **Mid:** `-18.0 dB` .. `+20.0 dB` (Geniş frekans açıklığı)
- **Tiz:** `-18.0 dB` .. `+20.0 dB` (Yüksek parlaklık)
- **Gain:** `%0` .. `%200` (Çıkış ses seviyesi)

---

## Hızlı Komut Tablosu

```bash
# Canlı Terminal Arayüzü (TUI)
micctl

# 💥 Ekstrem Nuke Modları
micctl ultra         # 💥💥 Ultra Nuke / Kıyamet Modu (+50dB Bas)
micctl nuke          # 💥 Standart Nuke Modu

# 🎭 Eğlenceli Ses Efektleri (Voice FX)
micctl fx robot      # 🤖 Metalik Robot
micctl fx rabbit     # 🐰 Tavşan / Helyum (İnce ses)
micctl fx demon      # 👹 Canavar / Demon (Karanlık kalın ses)
micctl fx walkie     # 📻 Telsiz / Megafon
micctl fx phone      # ☎️ Eski telefon
micctl fx none       # Normal stüdyo moduna dön

# 🔊 Mikrofona Ses Dosyası Çalma (Soundboard)
micctl play ~/Müzik/meme.mp3
micctl stop-play

# 📯 Hazır Ses Efektleri (SFX)
micctl sfx roger     # Telsiz bas-konuş kapanış sesi
micctl sfx nuke      # Nükleer siren & patlama
micctl sfx airhorn   # Meme korna sesi
micctl sfx beep      # UI bip sesi

# 🔄 Fabrika Ayarlarına Sıfırla
micctl reset

# 🎙️ Sesini Kaydet ve Dinle (3 saniye test)
micctl test

# Bass, Mid, Tiz Ayarları (Anlık 5ms gecikmesiz güncelleme)
micctl bass +10      # 10 dB artır (-20..+30 dB arası)
micctl bass set 25   # Ekstrem bas!
micctl treble +5.0
micctl mid +3.0
micctl gain 150%

# Bas-Konuş (PTT) & Hold-to-Nuke
micctl ptt start         # Mikrofonu aç
micctl ptt stop          # Mikrofonu sustur
micctl hold-nuke start   # Basılı tutulduğunda nuke patlatır
micctl hold-nuke stop    # Bırakıldığında normale döner
```

## TUI Kısayol Tuşları (`micctl`)
- `[↑/↓]` : Menüde gezin
- `[←/→]` : Seçili ayarın değerini anlık değiştir (Kasma/donma sıfır!)
- `[N]` : 💥 Nuke modunu aç/kapat
- `[U]` : 💥💥 Ultra Nuke (Kıyamet) modunu aç/kapat
- `[1-4]` : 📯 SFX Çal (1: Roger, 2: Nuke, 3: Airhorn, 4: Bip)
- `[S]` : ⏹️ Çalan sesi durdur
- `[R]` : 🔄 Fabrika ayarlarına sıfırla (Reset)
- `[T]` : 🎙️ 2 saniyelik hızlı ses testi yap ve dinle
- `[X]` : 🔇 Mikrofonu sustur / aç
- `[L]` : 🎧 Kulaklıktan dinlemeyi (Monitör) aç/kapat
- `[D]` : 🎙️ Sanal mikrofonu sistem varsayılanı yap
- `[Q]` : Çıkış
