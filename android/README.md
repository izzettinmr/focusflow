# FocusFlow · Android

Android 8.0 ve üstü, ARM64 telefonlar için çevrimdışı sürüm. Aynı React arayüzünü ve masaüstündeki gerçek Rust/SQLite kayıt motorunu kullanır. Masaüstü Tauri kabuğuna dokunmadan Android için küçük bir yerel Java/WebView + JNI kabuğu eklenmiştir. Tarayıcı sitesi, uzaktaki bilgisayara bağlantı veya localStorage demosu değildir.

## Telefona kurma

1. Telefonunda **[FocusFlow.apk indir](https://github.com/izzettinmr/focusflow/releases/latest/download/FocusFlow.apk)** bağlantısını aç. [Sürüm sayfasında](https://github.com/izzettinmr/focusflow/releases/latest) paket bilgileri ve SHA-256 bulunur. Yerel derleme kullanıyorsan `android/output/FocusFlow.apk` dosyasını telefonuna kopyala.
2. Telefonda indirilen APK'yı aç. Android sorarsa tarayıcı veya dosya yöneticisi için **bilinmeyen uygulama yükleme** iznini ver.
3. **Yükle → Aç** seç. İlk çalışma başlatıldığında bildirim iznini ver.
4. Dersini seçip Pomodoro veya kronometre başlat. Ekranı kapattığında çalışma sürer; aktif oturum bildirim alanında görünür.
5. Telefon üreticisi arka plan uygulamalarını kapatıyorsa FocusFlow → **Ayarlar → Telefon → Android uygulama ayarları** bölümünden pil kullanımını **Sınırsız** seç. Android'in zorla durdurma işlemi çalışan uygulamayı ve alarmları durdurur.

USB hata ayıklama etkin ve bilgisayara yetki verilmiş bir cihazda alternatif kurulum:

```bash
adb devices
adb -s TELEFONUN_SERI_NUMARASI install -r android/output/FocusFlow.apk
```

`FocusFlow-audit.apk` yalnızca emülatör test paketidir; günlük kullanım için `FocusFlow.apk` kullan. APK yerel kalıcı bir anahtarla imzalanır; Play Store yayını yapılmamıştır. Güncelleme yüklerken uygulamayı kaldırma: aynı imzayla `install -r` veya APK üzerinden güncelleme kayıtları korur. Uygulamayı kaldırmak/“verileri temizle” yerel çalışma geçmişini siler; önceden JSON yedeği al.

## Kayıtlar ve zamanlayıcı

- Pomodoro ve kronometre tek SQLite defterine yazılır. Aynı anda yalnızca bir aktif oturum bulunabilir. Molalar hedef ve çalışma toplamına eklenmez.
- Android'de `CLOCK_BOOTTIME` ekran kapalıyken geçen süreyi de kapsar. Aynı süreç hayattayken uzun ekran kapalı aralığı sayılır; saat sıçramasında güvenli duraklama uygulanır.
- Çalışırken yerel bir **foreground service** görünür bildirim gösterir. SQLite kontrol noktası uyanıkken yaklaşık beş saniyede bir ve kullanıcı işlemlerinde yazılır. Sürekli wake lock tutulmaz; bildirimdeki sayaç Android tarafından çizilir. Pomodoro için yerel bitiş alarmı kurulur.
- Android 13+ gerçek zamanlayıcı uygulamalarına verilen `USE_EXACT_ALARM` izni kullanılır. Android 12'de kesin alarm yetkisi verilmemişse izin isteyen sayfa zorla açılmaz; sistemin gecikebilen alarmı kullanılır. Süre hesabı bitiş hedefinde sınırlandırılır.
- Android süreci öldürürse, telefon yeniden başlarsa veya uygulama zorla durdurulursa son kalıcı kontrol noktası korunur. Sonraki açılışta oturum **duraklatılmış** gelir; uygulamanın kapalı olduğu süre eklenmez. Son kontrol noktası ile kapanış arasındaki kaydedilememiş aralık geri kazanıldığı iddia edilmez.
- Ana ekrana dönmek veya son uygulamalar ekranından pencereyi kaydırmak aktif hizmeti normalde durdurmaz. **Kaydet ve uygulamadan çık** oturumu duraklatıp hizmeti ve alarmı durdurur.

## Bilgisayar ↔ telefon yedeği

Her cihazın kendi yerel veritabanı vardır; otomatik bulut veya LAN eşitlemesi yoktur.

Bilgisayarda **Ayarlar → JSON yedeği oluştur**, dosyayı telefona aktar, telefonda **Ayarlar → Yedekten geri yükle** seç. Ters yönde de aynı akış çalışır. Geri yükleme hedef cihazın kayıtlarını **değiştirir**, iki cihazın geçmişini birleştirmez. İşlem öncesinde uygulamanın özel veri dizininde otomatik güvenlik yedeği alınır. JSON/CSV dosyaları Android'in kendi dosya seçicisi üzerinden okunur/yazılır; genel depolama izni istenmez. Yedek seçerken yerel bir dosya sağlayıcısı kullanılmalıdır; seçtiğin bulut sağlayıcısının internet ihtiyacı FocusFlow'dan ayrıdır.

## CachyOS üzerinde derleme

Node.js, npm, Python 3 ve Rust kurulu olmalı. İlk araç kurulumu Java 21, resmi Android SDK/NDK r29 ve Rust hedeflerini yalnızca `~/.local/share/focusflow-android-tools` altına/standart Rust hedef dizinine indirir; sistem paketleri veya shell profili değiştirilmez. SDK lisans çıktısı araç dizinindeki `licenses.log` dosyasında saklanır. İndirmeler birkaç GB disk alanı gerektirir; emülatör ayrıca isteğe bağlıdır.

```bash
# Klonladığın focusflow dizininde:
npm ci
bash scripts/setup-android.sh
python3 scripts/build-android.py
```

Çıktı: `android/output/FocusFlow.apk` ve SHA-256 içeren `FocusFlow.json`. Varsayılan APK ARM64 ve x86_64 içerir. Yalnızca ARM64 APK için `python3 scripts/build-android.py --abi arm64-v8a` kullanılabilir. 32 bit ARM telefonlar ve Android 8 öncesi bu sürümün hedefi değildir.

Özel araç konumu için `FOCUSFLOW_ANDROID_TOOLS`; mevcut araçlar için `ANDROID_HOME`, `JAVA_HOME`, `NDK_HOME` ayarlanabilir. Derleme Gradle/Android Studio gerektirmez: `javac`, `aapt2`, `d8`, `zipalign`, `apksigner` doğrudan ve hata kontrolüyle çalıştırılır. İnternet yalnızca geliştirme bağımlılıklarını indirirken gereklidir.

İmzalama anahtarı `~/.local/share/focusflow-android-signing/release.p12`, parola dosyası aynı özel dizindeki `password` dosyasıdır. Dizin `0700`, dosyalar `0600` kullanır. **İkisini güvenli biçimde yedekle**; yeni bilgisayarda aynı anahtar olmadan mevcut uygulamanın üzerine güncelleme kurulamaz. Bu dosyalar kaynak koduna veya APK'ya dahil edilmez.

## Testler

```bash
npm test
cargo test --locked --manifest-path android/rust/Cargo.toml
cargo audit --file android/rust/Cargo.lock
# Başsız tarayıcıda mobil düzen + gerçek Rust/SQLite test taşıyıcısı:
FOCUSFLOW_MOBILE_TEST=1 npm run test:ui
```

Yerel Playwright tarayıcısı bulunamıyorsa `FOCUSFLOW_BROWSER=/tam/yol/chrome-headless-shell` ayarla. Mobil düzen testi 320, 360, 390 ve 430 piksel genişliklerinde bütün sayfaları ve zamanlayıcı işlemlerini kontrol eder.

Gerçek Android testi için isteğe bağlı emülatör:

```bash
export JAVA_HOME="$HOME/.local/share/focusflow-android-tools/jdk"
"$HOME/.local/share/focusflow-android-tools/sdk/cmdline-tools/latest/bin/sdkmanager" \
  'emulator' 'system-images;android-36;google_apis;x86_64'
python3 scripts/build-android.py --audit
python3 scripts/start-android-test.py
node scripts/test-android.mjs
node scripts/test-android.mjs --files
python3 scripts/smoke-android-release.py
adb -s emulator-5556 emu kill
```

İlk iki test yalnızca `FocusFlowAudit20261022` adlı emülatör ve `com.focusflow.mobile.audit` paketini kabul eder; kişisel telefon/masaüstü veritabanını kullanmaz. Son smoke testi aynı emülatöre günlük kullanım APK'sını kurar ve gerçek dokunmalarla başlat/duraklat/bitir akışını kontrol eder. Test paketinde WebView hata ayıklama açıktır; günlük kullanım APK'sında kapalıdır. Gerçek JNI üzerinden ekran kapalı kronometre, duraklat/devam, bir dakikalık Pomodoro bitiş alarmı/bildirimi, tek aktif oturum, ortak toplam, mola hariç tutma, SQLite dosyası, mobil gezinme ve zorla kapanmadan sonra güvenli kurtarma denenir. `--files` gerçek Android dosya seçicisiyle JSON/CSV dışa aktarmayı, iptali ve onaylı geri yüklemeyi kontrol eder. Test çıktıları `android/test-results` altındadır. Gerçek telefon üreticisinin pil yönetimi ayrıca cihaz üzerinde denenmelidir.

## Güvenlik sınırı

Manifest internet, genel depolama, konum, hesap veya kişi erişimi istemez. WebView yalnızca APK içindeki sabit HTTPS kaynaklarını yükler; dış gezinme engellenir, CSP bağlantı/iframe/script kaynaklarını sınırlar ve dosya/content erişimi kapalıdır. Java köprüsü yalnızca açıkça izin verilen çalışma komutlarını kabul eder; iç yedek motoru komutları JS'ye açılmaz. Dışa/içe aktarma dosya seçimi ve geri yükleme onayı yereldir. Veritabanı uygulamanın Android sandbox'ında tutulur. Android sistem yedeği kapalıdır; kullanıcı kontrollü JSON yedeklemesi kullanılır.
