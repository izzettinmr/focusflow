# FocusFlow 1.3.0 — başlangıç ve bitiş sesleri

İki özgün zil sesi uygulamaya dahil edildi: 0,85 saniyelik kısa başlangıç melodisi ve 2,7 saniyelik üç vuruşlu bitiş zili. Kaynak ses örneği veya üçüncü taraf indirmesi kullanılmadı; üretim betiği `scripts/generate-sounds.py` içindedir.

## Davranış

- Pomodoro çalışması veya molası başarıyla başlatılınca başlangıç sesi çalar.
- Sayaç doğal olarak bitince bitiş sesi bir kez çalar. Tekrarlanan kontrol, aynı sesi yeniden üretmez.
- Duraklatma/devam, iptal, erken bitirme ve kronometre için bitiş zili üretilmez.
- Ayarlar → Sesli uyarılar alanından aç/kapat ve yüzde 0–100 ses düzeyi seçilebilir. Değişiklikler Ayarları kaydet düğmesiyle SQLite'a kaydedilir; önizleme düğmeleri kaydetmeden dinlenebilir.
- Eski ayar/yedeklerde ses açık ve düzey yüzde 80 varsayılır. JSON yedeği tercihi korur; Wi-Fi eşitlemesinde diğer cihazın ayarını değiştirmez.

## Yerel oynatma

Linux sesleri Rust tarafında, pencere görünürlüğünden bağımsız bir işçi çalar. WAV dosyaları binary'ye gömülüdür; özel uygulama veri dizinine atomik yazılır. `pw-play`, başarısız olursa `paplay` kullanılır. Komut kabuk üzerinden çalıştırılmaz; dosya yolu ve seçenekler uygulama tarafından belirlenir. Kuyruk ve oynatma süresi sınırlıdır. Sistemde PipeWire veya PulseAudio ses çıkışı bulunmalıdır. Önizlemede bağlantı/oynatıcı sorunu görünür hata döndürür.

Android, paketlenmiş `res/raw` WAV dosyalarını yerel `MediaPlayer` ile çalar. Sesler WebView autoplay politikasına veya görünür pencereye bağlı değildir. Telefonun **alarm ses düzeyi** kullanılır; uygulama düzeyi bunun üzerine uygulanır. Telefonun sessiz alarm düzeyi veya Rahatsız Etmeyin politikası aşılmaz. Medya oynatıcısı tamamlanınca serbest bırakılır; sonsuz tekrar yoktur. Tamamlama bildirimleri özel sessiz kanaldan gönderilerek aynı uyarının iki ses çalması önlenir. Aktif foreground servis, bitiş sesi sürerken erken kapatılmaz.

## Doğrulama

- Dört yeni Rust testi: eski ayar varsayılanları ve geçersiz düzeyin atomik reddi; tek başlangıç/tek bitiş; duraklatılmış zaman ve erken bitirişin bitiş sesi üretmemesi; mola, kronometre, iptal ve geri yüklemede tekrar oynatılmama.
- 24 mevcut zamanlayıcı/kayıt testi, dört dil testi ve bir Android RPC testi geçti.
- Linux'ta `cargo run --manifest-path src-tauri/Cargo.toml --example sound_preview` gerçek PipeWire/PulseAudio bağlantısıyla iki sesi de oynattı.
- Gerçek Android WebView → Java → JNI testinde **11 kontrol grubu geçti**: iki sesin MediaPlayer ile önizlenmesi, ekran kapalı tamamlanmada tek başlangıç/tek bitiş çalınması, kapalı ses ayarının sessiz kalması; mevcut zamanlayıcı, duraklatma, alarm/bildirim, mola hariç tutma, İngilizce ve kurtarma kontrolleri.
- Türkçe/İngilizce React → gerçek SQLite arayüz testi geçti. Ses aç/kapat ve yüzde 35 düzeyi SQLite’a kaydedildi; dil seçimi yeniden yüklemede korundu. 390/800/1365 piksel görünümde yatay taşma olmadı.

Android test emülatörü fiziksel hoparlör değerlendirmesi değildir. Sanal Android'de gerçek MediaPlayer tamamlanması ve kayıtları doğrulanır; kullanıcının telefonundaki alarm düzeyi ayrıca açık olmalıdır.

Bu sürüm, [önceki güvenlik raporunda](security-2026-10-23.md) belirtilen eşitleme sorunlarını gidermez.

Linux üretim derlemesi tamamlandı ve yerel uygulama atomik olarak kuruldu. Günlük kullanım APK: v1.3.0 / versionCode 4, 9012359 bayt. SHA-256: `477b82b9caa6b480357c1c02b1c8aa12fb9506d26f97ff6645465479bcf1c0ca`. İmzalı üretim APK ile denetim APK’sının x86_64 kütüphaneleri ve üretim web varlıkları eşleşiyor. Test emülatörü kapatıldı.
