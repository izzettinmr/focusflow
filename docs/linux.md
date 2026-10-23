# Linux kurulumu ve geliştirme

CachyOS / Arch Linux üzerinde kaynak kodundan kurulum. Hazır Android paketi için [ana sayfaya](../README.md#kurulum) bakın.

## Yeni bir CachyOS / Arch kurulumu

```bash
sudo pacman -S --needed base-devel git curl wget file openssl gtk3 webkit2gtk-4.1 appmenu-gtk-module libappindicator-gtk3 librsvg xdotool nodejs npm patchelf
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/focusflow-rustup.sh
sh /tmp/focusflow-rustup.sh -y --profile minimal
source ~/.cargo/env
git clone https://github.com/izzettinmr/focusflow.git
cd focusflow
npm ci
npm run desktop
```

SQLite `bundled` özelliğiyle uygulamaya derlenir. Arch paket adları sistem sürümüne göre değişirse Tauri'nin resmi Linux önkoşullarını kontrol edin: <https://v2.tauri.app/start/prerequisites/#linux>.

## Tarayıcısız günlük kullanım için kurulum

```bash
# Klonladığınız focusflow dizininde:
source ~/.cargo/env
npm run tauri -- build --no-bundle
./scripts/install-local.sh
```

Uygulama menüsünden **FocusFlow**'u açın veya `~/.local/bin/focusflow` çalıştırın. Kurulum root istemez. Geliştirme sunucusu veya Node.js günlük kullanımda gerekmez. Otomatik başlatmayı bu sabit kurulumdan sonra Ayarlar'dan açın; geliştirme çalıştırılabilir dosyasını otomatik başlatmaya kaydetmeyin.

Dağıtılabilir AppImage:

```bash
npm run tauri -- build --bundles appimage
# Çıktı: src-tauri/target/release/bundle/appimage/
```

Tauri yapılandırması ayrıca DEB ve RPM hedeflerini içerir. CachyOS'ta önerilen yöntem yerel kurulum veya AppImage'dır. FUSE bulunmayan sistemde AppImage `--appimage-extract-and-run` ile açılabilir. Masaüstü paketi oluşturma ilk seferde ek araçlar indirebilir.

## Kurulu sürümü güncelleme

Klonladığın proje dizininde:

```bash
git pull --ff-only
npm ci
npm run tauri -- build --no-bundle
./scripts/install-local.sh
```

Kurulum, çalıştırılabilir dosyayı atomik olarak değiştirir; çalışma veritabanına dokunmaz. Açık olan eski süreç eski sürümle devam eder. Tepsiden **Kaydet ve çık** seçip FocusFlow'u yeniden aç. Pencerenin X düğmesi yalnızca tepsiye gizler. Wi-Fi eşitlemesi için iki cihazda da 1.1.0 veya üzeri gerekir.
