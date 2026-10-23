# FocusFlow 1.2.0 — Türkçe ve İngilizce

Dil seçimi Ayarlar → Dil / Language alanındadır; kaydet düğmesiyle uygulanır. Türkçe varsayılandır. React ekranları, erişilebilirlik etiketleri, tarih/süre gösterimi, uygulama hata mesajları, Linux tepsi menüsü ve yerel bildirim/dosya diyalogları için İngilizce karşılıklar eklendi. İşletim sisteminin kendi dosya seçicisi ve izin ekranları sistem dilini kullanır. Ders adları ve kullanıcı verileri çevrilmez.

Çeviri dosyaları `src/locales/en.json` ve yerel mesajlar için `src/locales/messages.en.json` içindedir. React, Rust ve Android mesaj kataloğunu çevrimdışı paketten okur; çeviri servisi veya ağ isteği yoktur. Eksik anahtar özgün metne döner; sistem kütüphanelerinden gelen teknik hatalar kendi dillerinde kalabilir.

`Settings.language` SQLite'taki mevcut ayar JSON'una eklenir. Eski ayar/yedeklerde Serde varsayılanı `tr` olur. Geçersiz dil değerleri reddedilir. Dil, Wi-Fi kayıt birleştirmesine dahil edilmez; JSON yedeği ve geri yüklemesi içinde korunur.

## Doğrulama

- Dört yeni Rust testi: eski yedek uyumluluğu; SQLite/yedek kalıcılığı ve eşitlemede dilin yerel kalması; geçersiz dilin atomik reddi; yerel bildirim/birleşik hata çevirisi.
- Arayüz testine TR → EN → yeniden yükleme → tüm sayfalar → İngilizce eşitleme hatası → TR akışı ve 390/800/1365 piksel taşma kontrolleri eklendi.
- Android testine gerçek Java/JNI üzerinden dil kaydetme ve uygulama zorla kapatılıp açıldığında İngilizcenin korunması kontrolleri eklendi.

Dil desteği [önceki sızma testinde](security-2026-10-23.md) bildirilen eşitleme açıklarını gidermez. Bu sürüm bir güvenlik düzeltmesi olarak sunulmaz.

23 Ekim 2026 çalıştırma sonuçları: 76 ortak Rust testi ve 1 Android RPC testi tamamlandı; bunlara önceki güvenlik raporundaki iki açık yeniden üretimi de dahildir, bu sayı güvenlik onayı değildir. Dil için dört yeni test geçti. Mobil ve masaüstü React → gerçek SQLite akışları geçti. Masaüstü sunucu açma kontrolü 45873 portu kullanımda olduğu için `FOCUSFLOW_SKIP_HOST_TEST=1` ile son koşuda kapsam dışı bırakıldı; ilk denemede alınan `Address already in use` hatası gizlenmedi ve mevcut bağlantıya müdahale edilmedi. Dil akışı, diğer sayfalar ve istemci hata çevirisi test edildi.

Linux üretim derlemesi tamamlandı ve yerel uygulama atomik olarak güncellendi. APK v1.2.0 / versionCode 3, ARM64+x86_64; SHA-256: `0305dc7aa90a9584bab8198508751b117f828dc7196e7303946efe9e495d70d3`. Üretim ve denetim APK'larının x86_64 yerel kütüphaneleri aynı; üretim web varlıkları son derlemeyle birebir doğrulandı.
