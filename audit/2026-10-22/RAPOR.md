# FocusFlow — Güvenlik, veri sızıntısı ve bellek incelemesi

Tarih: 22 Ekim 2026. Kapsam: FocusFlow kaynakları, Linux/Tauri uygulaması, React arayüzü ve kilitli bağımlılıklar. Kişisel çalışma veritabanı test girdisi olarak kullanılmadı. Normal uygulamadan farklı `com.focusflow.audit20261022` kimliği, geçici XDG dizinleri ve sentetik çalışma kayıtları kullanıldı. Üçüncü taraf sistemlere sızma veya ağ taraması yapılmadı.

## Değerlendirme

Denediğimiz senaryolarda dışarı veri aktarımı veya açık bir sürekli bellek büyümesi doğrulanmadı. Bu, sınırsız süre ve tüm girdiler için güvenlik/bellek sızıntısı yokluğu garantisi değildir. Buna karşılık uygulama kaynaklı dosya ve girdi doğrulama sorunları ile bir yüksek önem dereceli bağımlılık duyurusu bulundu; aşağıdaki düzeltmeler uygulandı.

**Kalan konu:** Rust `glib 0.18.5` için `RUSTSEC-2024-0429` bellek güvenliği uyarısı ve dört bakım uyarısı sürüyor. Bu yüzden uygulama “tüm güvenlik uyarılarından arındırılmış” olarak değerlendirilmedi.

## Doğrulanan bulgular ve düzeltmeler

| Bulgu | Önem / koşul | Kanıt ve sonuç |
|---|---|---|
| Tokio 1.54.0 `Interval` yarış durumu | Upstream High; FocusFlow içinde istismar edildiği gösterilmedi | RustSec `RUSTSEC-2026-0384` / `GHSA-5wjr-9356-4wc8` bildirdi. Kilitli sürüm **1.54.1** yapıldı. Yeniden taramada bu bulgu ve yanked uyarısı kalktı. FocusFlow'un çalışma zamanlayıcısı `std::thread` kullanır; duyuru özellikle Tokio `Interval` içindir. |
| Dışa aktarmada iç veritabanı koruması aşılabiliyordu | Orta, yerel dosya bütünlüğü; seçilen/değiştirilmiş hedef yolu gerekir | İlk kod yalnızca birebir `focusflow.db` yolunu engelliyordu. `child/../focusflow.db`, üst dizin sembolik bağı ve SQLite `-wal`/`-shm`/`-journal` yollarında güvenlik testleri başarısız oldu. Artık çözülmüş hedef dizin kontrol edilir ve uygulamanın bütün iç veri dizinine dışa aktarma reddedilir. |
| WebView'dan serbest dosya yolu kabulü | Savunma sınırı; renderer zaten ele geçirilmişse dosya zararını artırır | Dışa aktarma/içe aktarma artık JS tarafından gönderilen yolu kullanmaz. Dosya seçimi Rust'ın açtığı yerel diyalogda yapılır. Geri yükleme onayı da yerel diyalogdadır. Bu, renderer ele geçirilmesine karşı tam izolasyon garantisi değildir; normal çalışma komutları ana pencereye açıktır. |
| FIFO yedeği okumayı süresiz bekletebiliyordu | Orta, yerel kullanılabilirlik | Eski gerçek okuma kodu geçici adlandırılmış boruda 2 saniyelik watchdog süresini aştı. Yeni kod `O_NONBLOCK`, `O_NOFOLLOW`, normal dosya denetimi ve okuma sırasında da 100 MB sınırı uygular; aynı girdiyi yaklaşık **16 ms** içinde reddetti. |
| Veritabanı ve yedek dosyaları varsayılan umask ile `0644` oluşuyordu | Düşük, yerel gizlilik; erişilebilir üst dizin gerekir | Artık Unix veritabanı/yan dosyalar ve atomik çıktılar `0600`, uygulama veri dizini `0700` kullanır. Eski HOME izinleri de koruma sağlıyor olabilir; önceki durum doğrudan bir internet sızıntısı olarak yorumlanmadı. |
| Aşırı sayısal girdiler debug derlemesinde panic üretebiliyordu | Düşük, doğrulama/dayanıklılık | Geçmiş düzenleme ve yedek süre çıkarımlarında `i64` taşması gösterildi. Üretim derlemesinde aynı çökme gösterilmedi. `checked_sub` ile hata dönüşü sağlandı; 20.000 sayısal mutasyon panic üretmedi. |
| Negatif tarih ve tutarsız mod/tür yedeği kabul ediliyordu | Düşük, veri bütünlüğü | Negatif tarih sonradan yedeklemeyi bozabiliyordu; `stopwatch/break` birleşimi başlatma kurallarıyla tutarsızdı. Kontroller eklendi ve başarısız geri yüklemenin mevcut verileri koruduğu doğrulandı. |
| Gereksiz genel Tauri izinleri / gelecekteki iframe yüzeyi | Savunma güçlendirmesi | Uygulama komutları `AppManifest` ile ACL'ye dahil edildi; yalnızca `main` penceresine gereken komutlar ve autostart izinleri verildi. JS diyalog izinleri ve `core:default` kaldırıldı. CSP'ye açık `script-src`, `object-src 'none'`, `frame-src 'none'`, `form-action 'none'`, `base-uri 'none'` eklendi. |

Ayrıca büyük geçmişlerde `Math.max(...liste)` argüman sınırına takılma riski taşıyan hesap `reduce` ile değiştirildi. Bu başlık altında büyük bir gerçek kullanıcı geçmişi bozulmadı veya silinmedi.

## Güvenlik testleri

- İlk 19 güvenlik testinde **10 başarılı, 9 başarısız** sonuç alındı. Birden fazla başarısız test aynı bulgunun farklı yollarını deniyordu; bunlar “9 bağımsız açık” sayılmadı.
- Düzeltmeler ve ek dayanıklılık denemelerinden sonra **21 mevcut çekirdek testi + 22 güvenlik testi = 43 test başarılı**. Sayısal yedek mutasyon testi ayrıca 20.000 girdiyi denedi.
- SQL enjeksiyon metni kategori adı olarak saklandı; tablo/şema değişmedi. Yabancı anahtar hatasında işlem geri alındı.
- Kategori içindeki `<img ... onerror=...>` gerçek React arayüzünde metin olarak kaldı; çalıştırılmadı ve ek bir `img` oluşmadı.
- CSP denemelerinde dış test adresine `fetch`, resim, script ve object üzerinden gidiş engellendi. Ayrı loopback portundaki canary sunucusu **0 istek** aldı. HTTP test taşıyıcısına yalnızca kendi IPC portu için ek izin verildi; dağıtılan uygulamada bu ek izin yoktur.
- Atomik çıktının hedef sembolik bağını takip edip kurban dosyayı değiştirmediği; korunan dizinin normalleştirilmiş yolları, SQLite yan dosyaları, `0600` izinleri, aşırı büyük ve derin JSON, bozuk yedek, CSV formül girdileri ve aktif oturum silme sınırı test edildi.
- Yaygın özel anahtar/token kalıpları kaynaklar, dağıtım JS/CSS ve test scriptlerinde tarandı: **0 eşleşme**. Bu bir tüm sırları tespit edebilme garantisi değildir.
- Yerel gerçek WebKit testinin süreç ağacında iki saniyelik socket örneklemesinde **INET socket gözlemi olmadı**. Bu paket yakalama değildir ve çok kısa bağlantıları kaçırabilir. Uygulama kaynaklarında dış HTTP istemcisi/telemetri bulunmadı. Bağımlılık tarayıcılarının kendi internet bağlantıları bu uygulama ölçümünden ayrıdır.

Yerel dosya diyaloğu regresyonu **başarılı**: gerçek GTK seçicisiyle JSON dışa aktarma, `0600` çıktı izni, yerel geri yükleme onayı, otomatik güvenlik yedeği, aynı oturum sayısıyla geri yükleme, bozuk JSON reddedildiğinde verilerin korunması ve dışa aktarmayı iptal etme doğrulandı. Test süreci temiz çıktı (`exit_code=0`). Sonuç: `native-files-summary.json`. Önceki otomasyon erişilebilirlik sorunları başarılı koşu olarak sayılmadı.

## Bellek ölçümleri

| Katman | Yük | Gözlem |
|---|---|---|
| Gerçek Rust/SQLite çekirdeği | 6.000 başlat–duraklat–devam–bitir–sil döngüsü | RSS **5.596–5.604 KiB**. Son kayıt ve süre parçası sayısı sıfır; SQLite `integrity_check=ok`. Sabit veri hacminde belirgin sürekli artış görülmedi. |
| React + gerçek Rust/SQLite test taşıyıcısı, Chromium | **5.500 ekran geçişi**, ilk 500 geçiş ısınma; her 500 geçişte GC | GC sonrası JS heap yaklaşık **3,56–3,93 MiB**, son değer **3,87 MiB**. DOM düğümleri bütün noktalarda **465**. Event listener sayısı 183/204 arasında değişti; son değer 183. |
| Gerçek Tauri/WebKit test sürümü | **649,8 saniye**, 1.000 mevcut geçici oturum, 400 ekran geçişi, 40 yeni oturum, görünür ve tepside boşta bekleme | Etkinlik sonrası son dakikada anonim PSS + SwapPss ortalaması **148,04 MiB**; tepside yaklaşık **146,71 MiB**. Anonim ayırmalarda belirgin kesintisiz artış gösterilmedi. Süreç temiz çıktı. |

WebKit karşılaştırması uygulamanın ana süreci ve iki WebKit alt sürecini içerir. Sistem eşzamanlı derleme sırasında swap kullandığından yalnızca RAM/PSS düşüşüne bakılarak “sızıntı yok” sonucu çıkarılmadı. Ek ölçümde `Pss_Anon + SwapPss` ve toplam `Pss + SwapPss` kaydedildi. Son yeniden açma/kapatma sıçraması tepsi karşılaştırmasından çıkarıldı.

Gerçek WebKit ölçümü **optimizasyonsuz ayrı test derlemesidir**; üretim sürümünün RAM/CPU bütçesi olarak alınmamalıdır. Görünür boşta son dakikada yaklaşık %12,5, tepside %7,25 tek çekirdek CPU örneklemesi görüldü; bu süreye derleme/swap baskısı da eşlik etti. Her saniye bütün geçmişi okuyan/yenileyen yapı, performans açısından ayrıca iyileştirilebilir. Bu CPU gözlemi kendi başına bellek sızıntısı değildir.

İlk WebKit koşusu yaklaşık 44. saniyede test sürecinin temiz çıkışıyla kesildi. Sebebi belirlenmedi; sıfır bellek değerleri karşılaştırmaya katılmadı. Başarıyla biten 649,8 saniyelik tekrar esas alındı; kesilen koşunun ham kayıtları ayrı saklandı.

## Bağımlılıklar ve kalan risk

- npm audit: **0 bildirilen açık**.
- Cargo audit ilk tarama: Tokio için 1 ana açık; dört `unmaintained`, bir `unsound` ve Tokio için bir `yanked` uyarısı.
- Tokio 1.54.1 sonrası: **ana açık sayacı 0**, dört bakım uyarısı ve **GLib unsound uyarısı devam ediyor**. Ana sayacın sıfır olması, GLib uyarısının çözüldüğü anlamına gelmez.
- Bakım uyarıları: `proc-macro-error 1.0.4`, `zbus_names 4.3.5`, `zvariant 5.15.1`, `zvariant_derive 5.15.1`.
- `glib 0.18.5`, GTK3/Tauri zincirinin bir parçasıdır. Yama `>=0.20.0` serisindedir; bunu zorla değiştirmek GTK tip/API uyumluluğunu kırabilir. Uygulama ve incelenen doğrudan Tauri/Wry/Tao/Muda kaynaklarında `VariantStrIter` çağrısı bulunmadı; bütün geçişli yolların ulaşılamazlığı kanıtlanmadı. Uyumlu upstream güncellemesi veya ayrıca denetlenmiş backport gerekir; uyarı bastırılmadı.
- Sistemde WebKitGTK **2.54.1**, GTK **3.24.52** ölçüldü. Kontrol edilen WebKitGTK resmi sayfası 2.54.1'i son kararlı düzeltme olarak listeliyordu; sistem paketlerine değişiklik yapılmadı.

Bu çalışma bir dış ekip sertifikasyonu, tam protokol fuzz testi, Valgrind/LSan çalışması veya günler süren dayanıklılık testi değildir. İşletim sisteminde aynı kullanıcı hesabında zaten kod çalıştırabilen saldırgana karşı tam izolasyon/şifreli depolama iddiası yoktur. Fiziksel güç kesintisi ve gerçek yeniden girişle autostart bu kapsamda denenmedi.

## Kanıtlar ve tekrar çalıştırma

Ham test çıktıları yerel denetim arşivinde saklandı; makine bilgileri, büyük test binary'leri ve ham kayıtlar GitHub deposuna dahil edilmedi. Bu rapor tamamlanan koşuların özetidir. Test scriptleri kaynak koduyla birlikte yayımlanır; aşağıdaki komutlarla yeni çıktılar üretilebilir.

```bash
cd ~/coding/focusflow
source ~/.cargo/env
npm test
cargo audit --file src-tauri/Cargo.lock
npm audit
cargo build --manifest-path src-tauri/Cargo.toml --no-default-features --example audit_probe
# Tamamen geçici bir dizinde sabit kayıt hacmiyle çekirdek testi:
probe_dir=$(mktemp -d /tmp/focusflow-soak.XXXXXX)
src-tauri/target/debug/examples/audit_probe --soak 6000 "$probe_dir/test.db"
```

Masaüstü ölçümünü tekrar etmek için normal uygulama yerine `tauri-audit-config.json` ile **ayrı** test binary'si oluşturulmalıdır. `scripts/audit-native-memory.py` ve `scripts/audit-native-files.py` bu audit binary'sini kullanır; GUI testleri masaüstünde geçici pencere açar. Üretim uygulamasına test HTTP sunucusu veya test kancası eklenmedi.

Resmi kaynaklar: [Tokio duyurusu ve yama](https://github.com/tokio-rs/tokio/security/advisories/GHSA-5wjr-9356-4wc8), [GLib uyarısı](https://rustsec.org/advisories/RUSTSEC-2024-0429.html), [Tauri komut izinleri](https://v2.tauri.app/security/capabilities/), [WebKitGTK 2.54.1](https://www.webkitgtk.org/2026/10/02/webkitgtk2.54.1-released.html).

## Kurulum doğrulaması

Güvenlik düzeltmelerini içeren üretim derlemesi `~/.local/bin/focusflow` konumuna atomik olarak kuruldu ve uygulama güvenli çıkışın ardından yeniden açıldı. Kurulu dosya ile test sonrası üretim derlemesinin SHA-256 değerleri aynı. Kişisel SQLite veritabanının son salt okunur bütünlük kontrolü `ok`; veri dizini `0700`, veritabanı `0600`. Testler kişisel kayıtları değiştirmedi. Makine tarafından okunabilir sonuç: `deployment-verification.json`.
