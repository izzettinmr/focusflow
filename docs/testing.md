# Test ve doğrulama

```bash
source ~/.cargo/env
npm test
npm run build
cargo build --manifest-path src-tauri/Cargo.toml
```

Arayüz entegrasyon testi (masaüstü eklentilerini değil, React + gerçek Rust/SQLite akışını doğrular):

```bash
cargo build --manifest-path src-tauri/Cargo.toml --no-default-features --example ui_bridge
npx playwright install chromium
npm run build
node scripts/ui-test.mjs
```

Ekran görüntüleri `test-results/` dizinine yazılır. Test geçici SQLite kullanır; kişisel veri dizinine dokunmaz. Gerçek bildirim ve tepsi normal masaüstü oturumunda ayrıca doğrulandı ([masaüstü doğrulama raporu](validation/desktop.md)). Otomatik başlatmanın yeniden giriş davranışı ayrıca denenmelidir. Süre testleri sentetik saatlerle çalışır; 125 dakika bekletmez.

Resmi başvuru: [Tauri sistem tepsisi](https://v2.tauri.app/learn/system-tray/), [bildirimler](https://v2.tauri.app/plugin/notification/), [otomatik başlatma](https://v2.tauri.app/plugin/autostart/).

Gerçek Linux penceresi testi (`python-gobject` ve `at-spi2-core` gerekir; masaüstünde geçici pencere açar):

```bash
python3 scripts/native-smoke.py
# Gerçek 1 dakikalık Pomodoro + bildirim (önce ui_bridge örneğini derleyin):
python3 scripts/native-smoke.py --notification
```

Bu test de geçici XDG veri dizinini kullanır; kişisel kayıtlarınızı değiştirmez.

## Android

```bash
cargo test --locked --manifest-path android/rust/Cargo.toml
FOCUSFLOW_MOBILE_TEST=1 npm run test:ui
```

Mobil arayüz testinden önce yukarıdaki `ui_bridge`, Playwright ve üretim web derlemesi adımlarını tamamlayın. Yerel Android/emülatör test komutları [Android kılavuzunda](../android/README.md#testler) bulunur.

## Tamamlanan incelemeler

- [1.1.0 Wi-Fi ve Android paket doğrulaması](validation/wifi-sync.md)

- [Linux doğrulaması](validation/desktop.md)
- [Android doğrulaması](../android/VALIDATION.md)
- [Güvenlik ve bellek raporu](../audit/2026-10-22/RAPOR.md)

Raporlar tarihli test sonuçlarıdır; tüm cihazlar ve olası girdiler için garanti değildir. Linux GTK/GLib bağımlılıkları için raporda belirtilen uyarılar sürmektedir.

## Yerel Wi-Fi eşitleme

Çekirdek testleri; idempotent birleştirme, silme ve kimlik tekrar kullanımı, eski yedekler, aktif sayaç, ileri tarih ve çakışma reddini içerir. TCP/TLS testleri gerçek loopback bağlantısı açar; ağ erişimine izin veren bir ortam gerekir.

Gerçek Android JNI → TLS → masaüstü motoru testi, yalnızca adlandırılmış test emülatöründe:

```bash
cargo build --manifest-path src-tauri/Cargo.toml --no-default-features --example sync_host
python3 scripts/build-android.py --audit --abi x86_64
python3 scripts/start-android-test.py
node scripts/test-android-sync.mjs
adb -s emulator-5556 emu kill
```

Geçici anahtarlar ve veritabanları test dizininde kalır. Test kişisel çalışma geçmişini ve fiziksel telefonu kullanmaz.
