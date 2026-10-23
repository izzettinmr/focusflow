# Wi-Fi eşitleme

FocusFlow 1.1.0, aynı yerel ağdaki Android telefon ve Linux bilgisayar arasında dersleri ve bitmiş çalışma kayıtlarını birleştirir. Bulut, hesap veya internet bağlantısı gerekmez. Bilgisayar kabloyla aynı yönlendiriciye bağlı da olabilir.

## İlk bağlantı

1. İki cihazda da **1.1.0 veya üzerini** kur. Android'de uygulamayı kaldırmadan yeni APK'yı üzerine yükle. Linux'ta yeni sürümü derleyip yerel kurulumu güncelle.
2. Bilgisayarda **Ayarlar → Wi-Fi eşitleme → Bu bilgisayarda eşitlemeyi aç** seç.
3. Birden fazla adres varsa telefonunun bağlı olduğu yerel ağın adresini seç. **Kodu seç** düğmesine basıp Ctrl+C ile kopyala. Kodu telefonuna aktar; ortak pano kullanıyorsan oradan yapıştırabilirsin.
4. Telefonda **Ayarlar → Wi-Fi eşitleme** bölümüne kodun tamamını yapıştır ve **Eşleştir ve eşitle** seç.
5. **Cihazlar eşitlendi** durumunu ve son eşitleme saatini kontrol et.

Kodda bilgisayarın yerel adresi, sertifika parmak izi ve rastgele bağlantı anahtarı bulunur. Kodu yalnızca kendi cihazlarınla paylaş. Kodun bir kısmı eksikse veya değişmişse bağlantı kurulmaz.

## Günlük kullanım

Bilgisayarda FocusFlow açık kalmalı; sistem tepsisinde durabilir. Telefon bağlıyken yaklaşık **30 saniyede bir** kontrol eder. **Şimdi eşitle** ile beklemeden deneyebilirsin. Eşleştirme cihazda saklanır; her açılışta kod girmen gerekmez.

Telefon uygulaması Android tarafından tamamen kapatılmışsa arka planda eşitleme garantisi yoktur; sonraki açılışta yeniden bağlanır. Ağ bağlantısı kesilse de yerel çalışma ve SQLite kaydı devam eder. Aynı ağa dönünce biriken bitmiş kayıtlar aktarılır.

| Eşitlenen                                | Cihazda kalan                                |
| ---------------------------------------- | -------------------------------------------- |
| Ders adları, renkleri ve arşiv durumu    | Aktif veya duraklatılmış sayaç               |
| Bitmiş Pomodoro ve kronometre oturumları | Pomodoro/mola süre ayarları ve günlük hedef  |
| Geçmiş düzenlemeleri ve oturum silmeleri | Bildirim, pil ve otomatik başlatma ayarları  |
| Molaların geçmiş kaydı                   | Eşleştirme anahtarı ve özel bağlantı dosyası |

Molalar çalışma toplamına dahil edilmez. Tekrar aktarılan bir oturum yeni kayıt oluşturmaz. Gece yarısı dağılımı ve grafikler, birleştirilen aynı süre parçalarından hesaplanır.

## Çakışma ve düzenleme

Farklı oturumların çalışma süreleri üst üste binerse birleştirme uygulanmaz. Her cihazdaki mevcut çalışma kayıtları korunur; arayüz nedenini gösterir. **Geçmiş** ekranından hatalı kaydı düzelt veya sil; sonra yeniden eşitle. İki cihazda aynı anda çalışan sayaçlar ağ üzerinden kilitlenmez; bunlardan doğan çakışma birleştirme sırasında yakalanır.

Aynı kayıt iki cihazda düzenlenirse daha yüksek değişiklik sürümü seçilir. Sürüm cihaz saatinden ve monoton artan yerel değişiklik sayacından üretilir; eşitlikte cihaz kimliği belirleyicidir. Cihaz saatlerini doğru tut. Eşzamanlı düzenlemeleri azaltmak için bir kaydı düzenledikten sonra eşitlemenin tamamlanmasını bekle.

Gelen içerik farklıysa, işlem öncesi cihazın özel veri dizininde `before-wifi-sync.json` güvenlik yedeği tutulur. Bu dosya son eşitleme öncesi durumu içerir; uzun süreli arşiv için ayrıca normal JSON yedeği oluştur. Başarısız birleştirme çalışma tablolarını değiştirmez.

## Bağlantıyı kaldırma

Telefonda **Cihaz bağlantısını kaldır** otomatik bağlantıyı durdurur. Bilgisayarda **Wi-Fi eşitlemesini kapat** yerel bağlantıyı kapatır. Bilgisayarda tekrar açmak yeni kimlik ve anahtar üretir; önceki kod geçersiz olur. Yeni kodla yeniden eşleştir.

Bilgisayarın yerel IP adresi değişirse yeni adresli kodu telefona yeniden gir. Eşitleme kendiliğinden ağ taraması yapmaz.

## Bağlanmıyorsa

- İki cihazın aynı yerel ağa bağlı ve bilgisayarda FocusFlow'un açık olduğunu kontrol et.
- Konuk ağı veya cihaz yalıtımı, aynı Wi-Fi adında olsalar bile cihazların birbirine ulaşmasını engelleyebilir.
- Güvenlik duvarı kullanıyorsan bilgisayardaki **TCP 45873** portuna kendi yerel ağından erişim sağla. FocusFlow güvenlik duvarını otomatik değiştirmez.
- VPN, farklı ağ arayüzü veya değişen IP nedeniyle yanlış adres seçilmiş olabilir. Bilgisayardaki adres seçimini ve kodun tamamını kontrol et.
- Kod yenilenmişse eski kodla tekrar bağlanılmaz. Yeni kodu kullan.

## Veri ve bağlantı sınırları

Bağlantı Rustls TLS ile şifrelenir. Telefon, kodda verilen bilgisayar sertifikasının SHA-256 parmak izini ve TLS imzasını doğrular. Bilgisayar 256 bit bağlantı anahtarını doğrulamadan çalışma verisi okumaz. Yalnızca özel IPv4 adresleri ve yerel loopback kabul edilir; genel IP adresleri, alan adları ve IPv6 bu sürümde desteklenmez.

Eşitleme varsayılan olarak kapalıdır. Bağlantı ayarları `wifi-sync.json` dosyasında cihazın özel veri dizininde saklanır; Unix'te dosya izni `0600` olur. Bu dosya JSON çalışma yedeğine veya GitHub deposuna dahil edilmez. Tek aktarım 32 MB, gelen kayıt listesi 200.000 kayıt ile sınırlıdır.

Android'in `INTERNET` adlı sistem izni yerel soket bağlantısı için gerekir. WebView dış ağ erişimi kapalı kalır; eşitleme yerel Rust katmanında yapılır. Hedef SDK 36 için izin davranışı [Android'in resmi yerel ağ belgesinde](https://developer.android.com/privacy-and-security/local-network-permission) açıklanır.

JSON yedeği geri yüklemek tüm yerel çalışma geçmişini değiştirir. Önce Wi-Fi eşitlemesini kapat; geri yükleme ile iki cihazın geçmişini birleştirmeye çalışma.

[Ana sayfa](../README.md) · [Android kılavuzu](../android/README.md) · [Testler](testing.md)
