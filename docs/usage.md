# Kullanım ve yedekleme

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

## Yedek ve dışa aktarma

Ayarlar → **JSON yedeği oluştur**: dersler, oturumlar, süre defteri ve ayarların sürümlü yedeği. **Yedekten geri yükle** tüm verileri atomik transaction içinde değiştirir; önce veri dizinine `before-restore-*.json` güvenlik yedeği oluşturur. Aktif oturum varken geri yükleme engellenir. Yedekteki aktif oturum duraklatılır. Sahipsiz/çakışan parçalar, geçersiz süreler ve toplam uyuşmazlığı reddedilir. JSON yedeği 100 MB ile sınırlıdır.

CSV tüm kayıtları (molalar dahil) içerir; tarih alanları saat dilimi ofsetiyle ISO biçimindedir. Süre saniyedir; duraklamalar dahil duvar saati aralığı değildir. CSV geri yükleme biçimi değildir. Dosyalar aynı dizinde geçici dosya + fsync + rename ile yazılır. Canlı SQLite dosyasını elle kopyalamak yerine JSON yedeğini kullanın.

Android dosya seçimi ve cihazlar arası aktarım: [Android kılavuzu](../android/README.md#bilgisayar--telefon-yedeği).

## Arayüz dili

**Ayarlar → Dil / Language** alanından **English** veya **Türkçe** seçin ve **Ayarları kaydet / Save settings** düğmesine basın. Dil seçimi SQLite'ta saklanır ve uygulama yeniden açıldığında korunur. Ders adları ve çalışma kayıtları çevrilmez. Dil cihaz bazındadır; Wi-Fi eşitlemesi diğer cihazın dilini değiştirmez. JSON yedeği dil tercihini içerir; eski yedeklerde varsayılan Türkçedir.

English: open **Settings → Dil / Language**, select **English**, then choose **Save settings**. Your selection is saved locally and survives restarts. Your course names and study records remain unchanged.
