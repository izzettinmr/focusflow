# Doğrulama — 20 Ekim 2026, CachyOS / Linux

- TypeScript kontrolü ve Vite üretim derlemesi başarılı.
- Rust/SQLite çekirdeğinde **21 test** başarılı.
- Gerçek Rust Engine ve geçici disk SQLite kullanan React/Chromium entegrasyon testi başarılı: Pomodoro, duraklatma, kronometre, ortak toplam, ders ekleme, geçmiş ders düzenleme, grafik dönemleri ve 800 px görünüm.
- Gerçek Tauri/WebKit penceresinde AT-SPI testi başarılı: başlat, duraklat, devam et, bitir ve temiz çıkış. Duraklatılan süre artmadı; gerçek SQLite kontrol edildi.
- KDE StatusNotifier üzerinde uygulamanın süreç kimliğiyle tepsi kaydı doğrulandı. Gizlenen pencere ikinci çalıştırmayla açıldı; ikinci süreç temiz kapandı.
- Hızlandırılmadan gerçek 1 dakikalık Pomodoro tamamlandı; `org.freedesktop.Notifications.Notify` çağrısı, FocusFlow başlığı ve Pomodoro tamamlanma metniyle gözlendi. Bildirimin kullanıcıya gösterilmesi masaüstünün bildirim ayarına bağlıdır.

Çekirdek testleri: 40 + 35 + 50 = 125 dakika; tek aktif oturum; duraklama; bitir/iptal; hedefte tam durma; molaların dışlanması; gün sınırı ve yuvarlama; kapanma sonrası duraklatılmış kurtarma; disk üzerinde kalıcılık; uyku, saat geri alınması ve uzun işlem kesintisi; küçük saat sapmaları; yinelenen kontrol noktası; transaction hatası ve güvenli tekrar; ders silme; kayıt düzenleme çakışması; günlük hedef bildirimi; doğrulanan yedek/geri yükleme.

Testler yalnızca geçici SQLite/XDG dizinlerinde çalıştı. Kullanıcının kişisel geçmişine test oturumu eklenmedi. Otomatik başlatmanın gerçek yeniden giriş davranışı ve güç kesintisi fiziksel olarak denenmedi. Çökme sonrası kurtarma bağlantının kapatılıp yeniden açılmasıyla test edildi.

Tekrar çalıştırma komutları [test kılavuzundadır](../testing.md).

Optimize edilmiş bağımsız sürüm de doğrulandı: `npm run tauri -- build --no-bundle` başarılı (12 MiB). Geliştirme sunucusu kapatıldıktan sonra release dosyasında yerel test yeniden geçti. Pencere sistem tepsisinde gizliyken 60.000 ms Pomodoro otomatik tamamlandı ve masaüstü bildirim çağrısı gönderildi. `~/.local/bin/focusflow` kopyasının SHA-256 değeri derlenmiş sürümle eşleşiyor; uygulama menüsü girdisi kuruldu. Çalıştırılabilir dosyanın eksik dinamik kütüphanesi yok.
