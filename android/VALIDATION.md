# Android sürümü doğrulaması

23 Ekim 2026. Günlük kullanım paketi: `FocusFlow.apk`; [GitHub Releases](https://github.com/izzettinmr/focusflow/releases/latest) üzerinden dağıtılır. Yerel derleme çıktısı `android/output/FocusFlow.apk` olur.

- **47 Rust testi başarılı:** 24 ortak çekirdek, 22 güvenlik, 1 Android RPC girdi doğrulama testi. Sayısal güvenlik testi 20.000 mutasyon içerir.
- Ortak motorun mobil testleri ekran kapalı uzun aralığı yalnızca bir kez saymayı, duraklama hariç tutmayı, Pomodoro hedefinde sınırlandırmayı, saat sıçramasını ve yeniden açılışta güvenli duraklamayı doğrular.
- Gerçek Android 16 WebView → Java → JNI → SQLite testleri başarılı: ekran kapalı kronometre, duraklat/devam, ekran kapalı bir dakikalık Pomodoro alarmı ve bildirimi, tek aktif oturum, ortak toplam, mola hariç tutma, gerçek SQLite dosyası, zorla kapatıp yeniden açınca duraklatılmış kurtarma.
- Android'in gerçek dosya seçicisinde JSON/CSV kaydetme, iptal, yerel geri yükleme onayı, otomatik güvenlik yedeği ve aynı verilerin geri yüklenmesi doğrulandı.
- İmzalı **release APK**, hata ayıklama kapalıyken emülatöre kuruldu. Android'in erişilebilirlik ağacı üzerinden gerçek dokunmalarla **başlat → duraklat → bitir** geçti. Bitiş sonrası ekran `test-results/release-phone.png` dosyasında.
- React/gerçek SQLite taşıyıcısı ile 320, 360, 390, 430 × 800 px düzenleri test edildi. Başlat düğmesinin merkezine dokunmayı engelleyen katman olmadığı kontrol edildi. Gerçek APK testinde bulunan kısa ekran çakışması; ders ve dakika ayarlarını tek satıra alarak, kısa ekranda halkayı küçülterek ve durum mesajlarının dokunmaları engellemesini kaldırarak düzeltildi.
- Tauri masaüstü hedefi `cargo check --offline --locked --manifest-path src-tauri/Cargo.toml` başarılı. Masaüstü arayüz regresyon testi de geçti (Pomodoro/kronometre, duraklama, ortak toplam, ders, geçmiş düzenleme ve grafikler). Ortak kayıt motorunun masaüstü uyku davranışı korunur.
- Android `Cargo.lock` için mevcut RustSec veritabanıyla `cargo audit --no-fetch` bildirilen açık/uyarı üretmedi. Bu sonuç, Linux GTK bağımlılıkları hakkındaki önceki raporu değiştirmez ve tam güvenlik garantisi değildir.
- APK imzası v2/v3 doğrulandı; 16 KB hizalama kontrolü geçti. Manifestte internet izni ve debug bayrağı yok. Paket içinde kullanıcı veritabanı, yedek veya özel imzalama anahtarı bulunmadığı ve web kaynaklarının son `dist` çıktısıyla aynı olduğu kontrol edildi.

## Paket

Boyut: **5,276,589 bayt**. Android 8.0/API 26+, ARM64 veya x86_64; hedef API 36. JavaScript arayüzü için güncel Android System WebView kullanılmalı.

SHA-256: `f98fc67dc838f0fb1f995121225fb0f6d3c82cd4b90062a0ebd6644722ef556a`

## Kapsam ve kanıtlar

Testler fiziksel kullanıcının telefonu üzerinde yapılmadı. Yalnızca `FocusFlowAudit20261022` adlı emülatör ve geçici/audit veritabanları kullanıldı. Kişisel Linux çalışma geçmişi test girdisi yapılmadı. Emülatör test sonunda kapatıldı.

İlk emülatör denemelerinde aynı anda derleme ve sanal cihaz çalışırken RAM/swap baskısı oluştu; Pixel Launcher ve System UI yanıt vermeme pencereleri görüldü. Bu pencereler FocusFlow çökmesi olarak sayılmadı. Kontroller sıraya alındı ve test cihazındaki gereksiz Google uygulamaları kapatıldı. GTK/Android otomasyonundaki bağlantı, ekran görüntüsü ve dil farkı hataları başarılı uygulama testi olarak raporlanmadı; yukarıdaki sonuçlar tamamlanan tekrar koşularıdır. Son release dokunma testi ve ekran görüntüsü esas alınır.

Ham sonuçlar: `test-results/native-results.json`, `native-file-results.json`, `release-smoke.json`, `delivery.json`. İlgili tekrar komutları [Android kılavuzunda](README.md).

Bilgisayar ile telefon arasında otomatik eşitleme yoktur. JSON geri yükleme hedef geçmişi değiştirir, geçmişleri birleştirmez. Telefon üreticisinin pil kısıtlamaları fiziksel cihaz üzerinde ayrıca kontrol edilmelidir. Beklenmedik süreç kapanışında son kalıcı kontrol noktası korunur; kapanış sonrası belirsiz süre eklenmez.
