# FocusFlow

Android ve CachyOS / Arch Linux için Türkçe, çevrimdışı Pomodoro, kronometre ve ders bazlı çalışma takibi. İki mod aynı Rust/SQLite çalışma defterini kullanır; süreler ikinci kez sayılmaz. Yeni kurulum boş bir çalışma geçmişiyle başlar.

## Android telefona indir

**[FocusFlow.apk indir](https://github.com/izzettinmr/focusflow/releases/latest/download/FocusFlow.apk)** · [Tüm sürümler](https://github.com/izzettinmr/focusflow/releases)

1. Yukarıdaki APK bağlantısını telefonunda aç ve dosyayı indir.
2. İndirilen `FocusFlow.apk` dosyasına dokun. Android sorarsa indirmeyi açtığın tarayıcıya veya dosya yöneticisine **bu kaynaktan yükleme** izni ver.
3. **Yükle → Aç** seç. Çalışma başlatınca bildirim iznini ver.
4. Dersini seç, Pomodoro veya kronometre başlat.

Android **8.0+**, **ARM64 veya x86_64** gerekir; 32 bit ARM desteklenmez. APK yaklaşık **5 MB** boyutundadır. Uygulama internetsiz çalışır ve hesap istemez. Play Store üzerinden dağıtılmaz. Android System WebView güncel olmalıdır.

Telefon ekranı kapalıyken çalışma hizmeti devam eder. Üreticinin pil yönetimi uygulamayı kapatıyorsa FocusFlow'un Android uygulama ayarlarından pil kullanımını **Sınırsız** yap. Güncellemeden önce JSON yedeği al; kayıtları korumak için uygulamayı kaldırmadan yeni APK'yı üzerine yükle.

[Android kurulum, derleme ve yedek kılavuzu](android/README.md) · [Android test sonuçları](android/VALIDATION.md)

Telefon ve bilgisayarın ayrı yerel veritabanları vardır. Otomatik eşitleme yoktur; JSON yedeğiyle taşıma hedef cihazın geçmişini değiştirir, geçmişleri birleştirmez.

## Kaynak kodunu indir

```bash
git clone https://github.com/izzettinmr/focusflow.git
cd focusflow
```

Arayüz React + TypeScript'tir. Linux'ta Tauri 2; Android'de Java/WebView + JNI kabuğu aynı Rust/SQLite motoruna bağlanır. `npm run dev` yalnızca tarayıcı varlıklarını açar; masaüstü SQLite ve zamanlayıcı için `npm run desktop` kullanılır. İlk derleme bağımlılıkları indirir; derlenmiş uygulama internet istemez.

## Yeni bir CachyOS / Arch kurulumu

```bash
sudo pacman -S --needed base-devel curl wget file openssl gtk3 webkit2gtk-4.1 appmenu-gtk-module libappindicator-gtk3 librsvg xdotool nodejs npm patchelf
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/focusflow-rustup.sh
sh /tmp/focusflow-rustup.sh -y --profile minimal
source ~/.cargo/env
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

## Kullanım

- **Genel bakış:** Bugünün gerçek çalışma toplamı, hedef yüzdesi, seri, son 7 gün, ders dağılımı ve son kayıtlar.
- **Odak zamanı:** Pomodoro veya kronometre seçin, ders belirleyin, başlatın. Pomodoro süresini doğrudan veya Ayarlar'dan değiştirin. Aynı anda tek oturum bulunabilir; mod değiştirmeden önce aktif oturumu bitirin veya iptal edin. Duraklatılmış oturum da aktif sayılır.
- **Bitir:** O ana kadar gerçekten çalışılan süreyi `erken bitirildi` olarak tutar. Pomodoro hedefi dolunca oturum otomatik tamamlanır. Tamamlanan oturum yeniden kaydedilmez.
- **İptal:** Onaydan sonra aktif oturum ve onun tüm süre parçalarını siler.
- **Molalar:** Kısa/uzun mola elle başlatılır; çalışma toplamlarına ve Pomodoro sayısına dahil edilmez. Ayarlanan her N Pomodoro için mola hatırlatma göstergesi vardır; otomatik geçiş yoktur.
- **İstatistikler:** Bugün, Pazartesi başlayan hafta, takvim ayı; 7/30 gün, son 8 hafta; derslere göre bugün/hafta/ay; tüm zamanların Pomodoro-kronometre karşılaştırması, en uzun oturum ve seri. Seri, bugün veya dünden geriye doğru en az 1 saniyelik çalışması olan ardışık günlerdir.
- **Geçmiş:** Ders, mod, durum ve başlangıç/bitiş tarihleriyle filtreleme. Tarih filtresi oturumun başlangıç gününe bakar; istatistikler gece yarısını bölen süre defterini kullanır. Molalar da geçmişte açıkça işaretlenir. Bitmiş kayıtların dersini, zaman aralığını ve gerçek süresini düzenleyebilir veya kaydı silebilirsiniz. Tarih/süre düzenlemesi, girilen toplamı tarih aralığına orantılı dağıtır; yalnızca ders değişikliği orijinal duraklamaları korur. Çakışan zaman aralıkları reddedilir.
- **Derslerim:** Ekleme, ad/renk düzenleme ve silme. Geçmişin ders adını korumak için silme arşivleme şeklindedir. Aktif oturumun dersi ve son kalan ders silinemez.
- **Ayarlar:** Süreler, günlük hedef, açılışta başlatma, CSV/JSON ve tam çıkış.

Kısayollar: **Odak zamanı sayfasında Space** başlatır/duraklatır/devam ettirir; **F** odak modunu açar/kapatır; **Esc** odak modundan çıkar. Bunlar pencere içi kısayollardır, sistem genelinde tuş yakalanmaz.

Pencerenin **X** düğmesi sistem tepsisine gizler; Rust zamanlayıcısı çalışır. Tepsi menüsünden açabilir, duraklatabilir veya **Kaydet ve çık** yapabilirsiniz. Tam çıkış oturumu duraklatır. Tepsi simgesi masaüstünün StatusNotifier/AppIndicator desteğine bağlıdır; KDE bunu destekler. Masaüstünün Rahatsız Etmeyin ayarı bildirimleri gizleyebilir. Bildirimler Rust tarafından Pomodoro/mola bitişinde ve günlük hedef geçişinde gönderilir; web görünümü gizliyken de çalışır.

## Veri ve süre güvenilirliği

- Kalıcı veritabanı: `~/.local/share/com.focusflow.desktop/focusflow.db` (veya sistemin XDG veri dizini). Kesin konum Ayarlar'da gösterilir.
- `sessions`, `segments`, `categories`, `settings` tabloları. Şema sürümü `PRAGMA user_version=1`. Kısmi benzersiz indeks, iki aktif kaydı veritabanında engeller. Tek uygulama örneği eklentisi ikinci pencere sürecinin başlamasını önler.
- Linux `CLOCK_MONOTONIC` zamanlayıcısı, askıda geçen zamanı içermez. Kabul edilen süre parçaları en geç yaklaşık her saniye **tek SQLite transaction'ında** oturum toplamıyla birlikte yazılır. WAL + `synchronous=FULL` kullanılır. Aralıksız süre parçaları birleştirilir; her saniye yeni satır üretilmez.
- Normal duraklatma/bitirme/çıkış son kısmi saniyeyi de kaydeder. Elektrik kesilmesi veya zorla öldürmede **son kontrol noktasından sonraki yaklaşık 1 saniye kaybolabilir**; henüz diske yazılmamış zaman için sıfır kayıp garantisi verilmez. Kaydedilmiş süre geri açılışta duraklatılarak korunur; uygulama kapalıyken çalışma eklenmez.
- Sistem saati önceki kayıtlardan geriye alınmışsa saat düzeltilene kadar yeni çalışma/devam etme reddedilir. Duvar saati ile monoton saatin farkı 2,5 saniyeyi aşarsa veya işlem 10 saniyeden uzun durursa belirsiz aralık eklenmez, oturum duraklatılır. Bu tercih, uyku/yeniden başlatma sırasında hatalı saatler yazmamak içindir. Arka planda normal çalışan pencere etkilenmez.
- İstatistikler `segments` kayıtlarından hesaplanır; `sessions.elapsed_ms` aynı transaction'da tutulur ve yedek alımında tutarlılığı test edilir. Molalar bu defterin çalışma toplamından dışlanır. Aynı dakika kronometreye yeniden aktarılmaz.
- Günlük dağılım sistemin **yerel saat dilimine** göredir. Saat dilimini sonradan değiştirirseniz geçmiş de o saat dilimine göre yeniden gruplanır. Gece yarısı bölünmesinde tamsayı yuvarlaması toplam milisaniyeyi korur.
- Uygulama kişisel, tek cihaz kullanımı içindir. Bulut veya ağ servisi yoktur. Disk hataları arayüzde gösterilir; kayıt hatası varsa çalışmayı duraklatıp disk alanını kontrol edin.

## Yedek ve dışa aktarma

Ayarlar → **JSON yedeği oluştur**: dersler, oturumlar, süre defteri ve ayarların sürümlü yedeği. **Yedekten geri yükle** tüm verileri atomik transaction içinde değiştirir; önce veri dizinine `before-restore-*.json` güvenlik yedeği oluşturur. Aktif oturum varken geri yükleme engellenir. Yedekteki aktif oturum duraklatılır. Sahipsiz/çakışan parçalar, geçersiz süreler ve toplam uyuşmazlığı reddedilir. JSON yedeği 100 MB ile sınırlıdır.

CSV tüm kayıtları (molalar dahil) içerir; tarih alanları saat dilimi ofsetiyle ISO biçimindedir. Süre saniyedir; duraklamalar dahil duvar saati aralığı değildir. CSV geri yükleme biçimi değildir. Dosyalar aynı dizinde geçici dosya + fsync + rename ile yazılır. Canlı SQLite dosyasını elle kopyalamak yerine JSON yedeğini kullanın.

## Mimari

```text
src/
  App.tsx              Sayfalar, kontroller, gerçek IPC çağrıları
  types.ts             Tipler ve yerel tarih/süre biçimleri
  style.css            Koyu tema ve duyarlı arayüz
  platform.ts          Tauri / Android yerel komut köprüsü
android/
  java/                Yerel Android pencere, hizmet ve alarm
  rust/                Aynı kayıt motoruna JNI köprüsü
  AndroidManifest.xml  Android izinleri ve bileşenler
src-tauri/
  src/core.rs          Ortak zamanlayıcı, SQLite, yedek doğrulama, testler
  src/desktop.rs       Tauri komutları, arka plan işçisi, tepsi, bildirimler
  src/lib.rs           Çekirdek / masaüstü özellik ayrımı
  examples/ui_bridge.rs  Yalnızca UI testinin gerçek çekirdeğe taşıyıcısı
  capabilities/        En az izinli masaüstü eklenti izinleri
scripts/
  install-local.sh     Kullanıcı dizinine masaüstü kurulumu
  ui-test.mjs          Geçici gerçek SQLite ile tarayıcı etkileşim testi
  native-smoke.py      AT-SPI ile gerçek Tauri/WebKit pencere testi
```

React süre üretmez veya kalıcı kayıt yazmaz; Rust'tan saniyede bir anlık görüntü alır. İş mantığı pencereden bağımsızdır. Test taşıyıcısı yalnızca `--example ui_bridge` ile derlenir; üretim uygulamasında HTTP sunucusu veya test verisi bulunmaz.

## Doğrulama

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

Ekran görüntüleri `test-results/` dizinine yazılır. Test geçici SQLite kullanır; kişisel veri dizinine dokunmaz. Gerçek bildirim ve tepsi normal masaüstü oturumunda ayrıca doğrulandı (VALIDATION.md). Otomatik başlatmanın yeniden giriş davranışı ayrıca denenmelidir. Süre testleri sentetik saatlerle çalışır; 125 dakika bekletmez.

Resmi başvuru: [Tauri sistem tepsisi](https://v2.tauri.app/learn/system-tray/), [bildirimler](https://v2.tauri.app/plugin/notification/), [otomatik başlatma](https://v2.tauri.app/plugin/autostart/).

Gerçek Linux penceresi testi (`python-gobject` ve `at-spi2-core` gerekir; masaüstünde geçici pencere açar):

```bash
python3 scripts/native-smoke.py
# Gerçek 1 dakikalık Pomodoro + bildirim (önce ui_bridge örneğini derleyin):
python3 scripts/native-smoke.py --notification
```

Bu test de geçici XDG veri dizinini kullanır; kişisel kayıtlarınızı değiştirmez.

## Güvenlik ve bellek denetimi

22 Ekim 2026 test kapsamı, doğrulanan bulgular, düzeltmeler ve kalan üçüncü taraf uyarıları: [ayrıntılı rapor](audit/2026-10-22/RAPOR.md).

Dosya seçimi ve geri yükleme onayı artık Rust tarafından açılan yerel diyaloglarda yapılır. Uygulamanın iç veri dizinine dışa aktarma engellenir. Unix verileri `0700` dizin ve `0600` dosya izinleri kullanır. İçe aktarma normal, sembolik bağ olmayan JSON dosyalarıyla ve okuma sırasında da 100 MB sınırıyla çalışır.
# Android telefon sürümü

Kurulum, yedek aktarımı, derleme ve test adımları: [Android kılavuzu](android/README.md). Hazır paket [GitHub Releases](https://github.com/izzettinmr/focusflow/releases/latest) sayfasındadır; yerel derleme çıktısı `android/output/FocusFlow.apk` olur. APK ve SHA-256 sağlama toplamı sürüme eklenir; APK kaynak kodu deposunda tutulmaz.
