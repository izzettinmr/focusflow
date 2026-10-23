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

- [Linux doğrulaması](validation/desktop.md)
- [Android doğrulaması](../android/VALIDATION.md)
- [Güvenlik ve bellek raporu](../audit/2026-10-22/RAPOR.md)

Raporlar tarihli test sonuçlarıdır; tüm cihazlar ve olası girdiler için garanti değildir. Linux GTK/GLib bağımlılıkları için raporda belirtilen uyarılar sürmektedir.
