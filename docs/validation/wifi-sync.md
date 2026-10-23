# FocusFlow 1.1.0 · Wi-Fi eşitleme doğrulaması

23 Ekim 2026. Aynı yerel ağdaki Android istemci ve Linux sunucu için isteğe bağlı eşitleme.

**Sonraki güvenlik incelemesi:** [23 Ekim sızma testi](security-2026-10-23.md) iki açık eşitleme sorunu buldu. Aşağıdaki işlev testlerinin başarılı olması güvenlik onayı değildir.

## Otomatik kontroller

- **67 Rust testi başarılı:** 24 mevcut zamanlayıcı/SQLite testi, 22 güvenlik testi, 6 yerel TLS/kimlik/çerçeve testi, 14 kayıt birleştirme testi ve 1 Android RPC doğrulama testi.
- İki cihazın 40 + 35 dakikalık çalışması tek 75 dakikalık toplam oluşturdu; yinelenen aktarım toplamı değiştirmedi. Eski JSON yedeğiyle iki cihaza taşınmış aynı kayıt çoğalmadı.
- Geçmiş düzenleme/silme, SQLite kimliğinin yeniden kullanılması, ilk eşitlemede özelleştirilmiş dersin korunması, yanıt kaybından sonra tekrar deneme, aktif oturuma müdahale ve ileri tarihli kayıt reddi doğrulandı.
- Üst üste binen sürelerde birleştirme reddedildi; çalışma verileri işlem öncesiyle aynı kaldı. Kabul edilen bütün oturum alanlarının SQLite'a tutarlı yazıldığı ayrıca doğrulandı.
- Yanlış sertifika parmak izi ve yanlış eşleştirme anahtarı gerçek TCP/TLS bağlantısında reddedildi. Genel IP, alan adı, IPv6 ve bozuk kodlar kabul edilmedi. Büyük çerçeve gövde ayrılmadan reddedildi; özel bağlantı dosyasının `0600` izni doğrulandı.
- Masaüstü ve mobil React → gerçek Rust/SQLite testleri geçti. Wi-Fi aç/kapat, kod seçimi ve genel IP reddi; 320–430 px sayfalar, Pomodoro/kronometre, ortak toplam ve geçmiş akışları kontrol edildi.
- Android ekran kapalı kronometre, duraklat/devam, gerçek bir dakikalık Pomodoro alarmı/bildirimi, mola hariç tutma, zorla kapanmadan sonra duraklatılmış kurtarma ve gerçek SQLite dosyası yeniden doğrulandı.
- Android'in yerel dosya seçicisinde JSON/CSV kaydetme, iptal, geri yükleme onayı, otomatik güvenlik yedeği ve aynı verilerin geri yüklenmesi geçti.

- Gerçek Android WebView → Java → JNI → TLS → yerel Rust sunucusu testinde iki yönlü tek toplam, yinelenen eşitleme, ders değişikliği, genel adres/Android sunucu reddi, yeniden açılışta bağlantının korunması ve bağlantı kaldırma geçti.
- İmzalı günlük kullanım APK'sı Android emülatörüne güncelleme olarak kuruldu. Gerçek dokunmalarla başlat/duraklat/bitir geçti; hata ayıklama kapalı, yerel ağ izni mevcut olarak doğrulandı. Test emülatörü sonunda kapatıldı.

## Paket

Android **8.0/API 26+**, ARM64 ve x86_64; hedef API 36, sürüm kodu 2. Günlük kullanım paketi `FocusFlow.apk`; test paketinin adı `FocusFlow-audit.apk` olur.

Boyut: **8,639,405 bayt**. SHA-256: `c091424cc5486b7738ccb3689dc03e4e1a27c7f464f1d1650347a44ff8f5cb16`.

APK v2/v3 imzası ve 16 KB hizalama kontrolü geçti. Test APK'sı ve günlük kullanım APK'sındaki x86_64 JNI kütüphanesi birebir eşleşti; web kaynakları son üretim derlemesiyle aynı. Kullanıcı veritabanı veya özel imzalama anahtarı APK'ya dahil edilmedi. Wi-Fi için Android `INTERNET` izni eklendi; WebView'ın dış ağ erişimi kapalı kaldı.

## Sınırlar

Testler fiziksel telefon yerine `FocusFlowAudit20261022` adlı Android 16 emülatörü ve geçici veritabanlarıyla yapıldı. Kişisel masaüstü geçmişi test girdisi olmadı. Emülatörün System UI yanıt vermeme pencereleri ve geç süreç açılışı, FocusFlow başarısı/çökmesi olarak sayılmadı. Dosya seçicide liste görünümü ve süreç kimliği bekleme kullanılarak otomasyonun dokunma/zamanlama varsayımları düzeltildi; tamamlanan koşular raporlandı.

Aktif sayaç taşınmaz; bitmiş çalışmalar eşitlenir. Cihazlar aynı anda çevrimdışı çalışabilir; çakışma aktarımda reddedilir. Süre ayarları ve günlük hedef yerel kalır. Android süreci öldürürse eşitleme sonraki açılışta yeniden denenir. Fiziksel telefonun pil ve yönlendiricinin cihaz yalıtımı davranışı ayrıca kontrol edilmelidir.

Android'in kilitli 137 bağımlılığı mevcut 1349 duyurulu RustSec veritabanında `cargo audit --no-fetch` ile tarandı; bulgu bildirilmedi. Bu, tam güvenlik garantisi değildir. Linux GTK/GLib için [önceki incelemedeki](../../audit/2026-10-22/RAPOR.md) uyarılar sürmektedir.

[Tekrar çalıştırma](../testing.md) · [Kullanım](../wifi-sync.md)
