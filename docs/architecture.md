# Mimari ve süre hesaplama

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

## Veri ve süre güvenilirliği

- Kalıcı veritabanı: `~/.local/share/com.focusflow.desktop/focusflow.db` (veya sistemin XDG veri dizini). Kesin konum Ayarlar'da gösterilir.
- `sessions`, `segments`, `categories`, `settings` tabloları. Şema sürümü `PRAGMA user_version=1`. Kısmi benzersiz indeks, iki aktif kaydı veritabanında engeller. Tek uygulama örneği eklentisi ikinci pencere sürecinin başlamasını önler.
- Linux `CLOCK_MONOTONIC` zamanlayıcısı, askıda geçen zamanı içermez. Kabul edilen süre parçaları en geç yaklaşık her saniye **tek SQLite transaction'ında** oturum toplamıyla birlikte yazılır. WAL + `synchronous=FULL` kullanılır. Aralıksız süre parçaları birleştirilir; her saniye yeni satır üretilmez.
- Normal duraklatma/bitirme/çıkış son kısmi saniyeyi de kaydeder. Elektrik kesilmesi veya zorla öldürmede **son kontrol noktasından sonraki yaklaşık 1 saniye kaybolabilir**; henüz diske yazılmamış zaman için sıfır kayıp garantisi verilmez. Kaydedilmiş süre geri açılışta duraklatılarak korunur; uygulama kapalıyken çalışma eklenmez.
- Sistem saati önceki kayıtlardan geriye alınmışsa saat düzeltilene kadar yeni çalışma/devam etme reddedilir. Duvar saati ile monoton saatin farkı 2,5 saniyeyi aşarsa veya işlem 10 saniyeden uzun durursa belirsiz aralık eklenmez, oturum duraklatılır. Bu tercih, uyku/yeniden başlatma sırasında hatalı saatler yazmamak içindir. Arka planda normal çalışan pencere etkilenmez.
- İstatistikler `segments` kayıtlarından hesaplanır; `sessions.elapsed_ms` aynı transaction'da tutulur ve yedek alımında tutarlılığı test edilir. Molalar bu defterin çalışma toplamından dışlanır. Aynı dakika kronometreye yeniden aktarılmaz.
- Günlük dağılım sistemin **yerel saat dilimine** göredir. Saat dilimini sonradan değiştirirseniz geçmiş de o saat dilimine göre yeniden gruplanır. Gece yarısı bölünmesinde tamsayı yuvarlaması toplam milisaniyeyi korur.
- Uygulama kişisel, tek cihaz kullanımı içindir. Bulut veya ağ servisi yoktur. Disk hataları arayüzde gösterilir; kayıt hatası varsa çalışmayı duraklatıp disk alanını kontrol edin.

Bu bölümdeki saat ve kontrol noktası ayrıntıları Linux içindir. Android hizmeti, ekran kapalı sayım ve güvenli kurtarma davranışı [Android kılavuzunda](../android/README.md#kayıtlar-ve-zamanlayıcı) açıklanır.
