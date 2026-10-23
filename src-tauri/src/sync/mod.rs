//! Opt-in, private-IPv4-only TLS transport. The WebView never gets network access.
pub mod ledger;
use crate::{core::Engine, files::atomic_write};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ledger::{Bundle, Result};
use rustls::{
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime},
    ClientConfig, ClientConnection, DigitallySignedStruct, ServerConfig, ServerConnection,
    SignatureScheme, StreamOwned,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;

const PORT: u16 = 45873;
const MAX_FRAME: usize = 32 * 1024 * 1024;
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    host: Option<Host>,
    peer: Option<Peer>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Host {
    cert: Vec<u8>,
    key: Vec<u8>,
    token: [u8; 32],
    port: u16,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Peer {
    address: SocketAddr,
    pin: [u8; 32],
    token: [u8; 32],
}
#[derive(Clone, Default, Serialize)]
pub struct Status {
    pub mode: String,
    pub ready: bool,
    pub busy: bool,
    pub peer: Option<String>,
    pub codes: Vec<String>,
    pub last_success: Option<i64>,
    pub changed: usize,
    pub error: Option<String>,
}
struct Runtime {
    config: Config,
    status: Status,
    generation: u64,
    request: u64,
}
pub struct LanSync {
    engine: Arc<Mutex<Engine>>,
    dir: PathBuf,
    state: Mutex<Runtime>,
    can_host: bool,
}

impl LanSync {
    pub fn new(engine: Arc<Mutex<Engine>>, dir: PathBuf, can_host: bool) -> Arc<Self> {
        let loaded = (|| -> Result<Config> {
            let file = dir.join("wifi-sync.json");
            if !file.exists() {
                return Ok(Config::default());
            }
            let data = crate::files::read_regular(&file, 32_768)?;
            let config: Config = serde_json::from_slice(&data).map_err(err)?;
            if config.host.is_some() && config.peer.is_some() {
                return Err("Geçersiz Wi-Fi ayarları".into());
            }
            if config
                .host
                .as_ref()
                .is_some_and(|h| h.cert.len() > 8192 || h.key.len() > 8192 || h.port == 0)
                || config
                    .peer
                    .as_ref()
                    .is_some_and(|p| !allowed(p.address.ip()))
            {
                return Err("Geçersiz Wi-Fi adresi".into());
            }
            Ok(config)
        })();
        let (mut config, error) = match loaded {
            Ok(c) => (c, None),
            Err(e) => (
                Config::default(),
                Some(format!("Wi-Fi ayarları açılamadı: {e}")),
            ),
        };
        if !can_host {
            config.host = None;
        }
        let this = Arc::new(Self {
            engine,
            dir,
            state: Mutex::new(Runtime {
                config,
                status: Status {
                    mode: "off".into(),
                    error,
                    ..Status::default()
                },
                generation: 1,
                request: 0,
            }),
            can_host,
        });
        let weak = Arc::downgrade(&this);
        thread::spawn(move || {
            let mut listener = None;
            let mut tls = None;
            let mut generation = 0;
            let mut request = 0;
            let mut next = Instant::now();
            loop {
                let Some(manager) = weak.upgrade() else {
                    break;
                };
                let (config, g, r) = {
                    let state = manager.state.lock().unwrap();
                    (state.config.clone(), state.generation, state.request)
                };
                if generation != g {
                    listener = None;
                    tls = None;
                    generation = g;
                    next = Instant::now();
                }
                if let Some(host) = config.host {
                    if listener.is_none() && Instant::now() >= next {
                        match TcpListener::bind((Ipv4Addr::UNSPECIFIED, host.port))
                            .map_err(err)
                            .and_then(|l| {
                                l.set_nonblocking(true).map_err(err)?;
                                Ok(l)
                            }) {
                            Ok(l) => match server_config(&host) {
                                Ok(c) => {
                                    listener = Some(l);
                                    tls = Some(c);
                                    manager.update(g, false, true, None, None);
                                }
                                Err(e) => manager.update(g, false, false, None, Some(e)),
                            },
                            Err(e) => manager.update(
                                g,
                                false,
                                false,
                                None,
                                Some(format!("Wi-Fi portu açılamadı: {e}")),
                            ),
                        }
                        next = Instant::now() + Duration::from_secs(5);
                    }
                    if let Some(l) = &listener {
                        if let Ok((socket, addr)) = l.accept() {
                            if allowed(addr.ip()) {
                                manager.update(g, true, true, None, None);
                                let result =
                                    manager.serve(socket, tls.as_ref().unwrap().clone(), &host);
                                match result {
                                    Ok(n) => manager.update(g, false, true, Some(n), None),
                                    Err(e) => manager.update(g, false, true, None, Some(e)),
                                }
                            }
                        }
                    }
                } else if let Some(peer) = config.peer {
                    if Instant::now() >= next || request != r {
                        request = r;
                        manager.update(g, true, false, None, None);
                        let result = manager.exchange(&peer);
                        match result {
                            Ok(n) => manager.update(g, false, true, Some(n), None),
                            Err(e) => manager.update(
                                g,
                                false,
                                false,
                                None,
                                Some(format!("Bağlantı bekleniyor: {e}")),
                            ),
                        }
                        next = Instant::now() + Duration::from_secs(30);
                    }
                }
                drop(manager);
                thread::sleep(Duration::from_millis(300));
            }
        });
        this
    }
    fn update(
        &self,
        generation: u64,
        busy: bool,
        ready: bool,
        changed: Option<usize>,
        error: Option<String>,
    ) {
        if let Ok(mut s) = self.state.lock() {
            if s.generation != generation {
                return;
            }
            s.status.busy = busy;
            s.status.ready = ready;
            s.status.error = error;
            if let Some(n) = changed {
                s.status.changed = n;
                s.status.last_success = Some(chrono::Local::now().timestamp_millis());
            }
        }
    }
    pub fn status(&self) -> Result<Status> {
        let s = self.state.lock().map_err(err)?;
        let mut status = s.status.clone();
        status.mode = if s.config.host.is_some() {
            "host"
        } else if s.config.peer.is_some() {
            "client"
        } else {
            "off"
        }
        .into();
        status.peer = s.config.peer.as_ref().map(|p| p.address.to_string());
        if let Some(h) = &s.config.host {
            status.codes = local_addresses()
                .into_iter()
                .map(|ip| pairing_code(h, SocketAddr::new(IpAddr::V4(ip), h.port)))
                .collect();
        }
        Ok(status)
    }
    pub fn enabled(&self) -> bool {
        self.state
            .lock()
            .is_ok_and(|s| s.config.host.is_some() || s.config.peer.is_some())
    }
    pub fn configure(&self, action: &str, code: Option<String>) -> Result<Status> {
        if action == "now" {
            let mut s = self.state.lock().map_err(err)?;
            s.request += 1;
            drop(s);
            return self.status();
        }
        let config = match action {
            "host" if self.can_host => {
                let cert = rcgen::generate_simple_self_signed(vec!["focusflow.local".into()])
                    .map_err(err)?;
                let mut token = [0; 32];
                getrandom::fill(&mut token).map_err(err)?;
                Config {
                    host: Some(Host {
                        cert: cert.cert.der().to_vec(),
                        key: cert.signing_key.serialize_der(),
                        token,
                        port: PORT,
                    }),
                    peer: None,
                }
            }
            "connect" => Config {
                host: None,
                peer: Some(parse_code(&code.ok_or("Bağlantı kodunu girin")?)?),
            },
            "off" => Config::default(),
            _ => return Err("Geçersiz eşitleme işlemi".into()),
        };
        atomic_write(
            &self.dir.join("wifi-sync.json"),
            &serde_json::to_vec(&config).map_err(err)?,
        )?;
        let mut state = self.state.lock().map_err(err)?;
        state.config = config;
        state.generation += 1;
        state.status = Status::default();
        drop(state);
        self.status()
    }
    fn prepare(&self) -> Result<Bundle> {
        let mut e = self.engine.lock().map_err(err)?;
        e.checkpoint(crate::core::Stamp::now())?;
        ledger::prepare(&mut e)
    }
    fn apply(&self, b: Bundle) -> Result<usize> {
        let mut e = self.engine.lock().map_err(err)?;
        e.checkpoint(crate::core::Stamp::now())?;
        // Retain a private pre-sync study backup whenever incoming content differs.
        let local = ledger::prepare(&mut e)?;
        let local_content: std::collections::BTreeMap<_, _> =
            local.records.iter().map(|r| (&r.uid, &r.content)).collect();
        let different = b
            .records
            .iter()
            .any(|r| local_content.get(&r.uid).is_none_or(|c| **c != r.content));
        if different {
            atomic_write(
                &self.dir.join("before-wifi-sync.json"),
                &serde_json::to_vec(&e.backup()?).map_err(err)?,
            )?;
        }
        ledger::apply(&mut e, b)
    }
    fn serve(&self, socket: TcpStream, config: Arc<ServerConfig>, host: &Host) -> Result<usize> {
        let mut stream = StreamOwned::new(
            ServerConnection::new(config).map_err(err)?,
            TimedTcp::new(socket),
        );
        let mut token = [0; 32];
        stream.read_exact(&mut token).map_err(err)?;
        if !bool::from(token.ct_eq(&host.token)) {
            return Err("Eşleştirme kodu geçersiz. Bağlantı reddedildi.".into());
        }
        let incoming: Bundle = read_frame(&mut stream)?;
        let gate = self.state.lock().map_err(err)?;
        if !gate
            .config
            .host
            .as_ref()
            .is_some_and(|h| bool::from(h.token.ct_eq(&host.token)))
        {
            return Err("Eşitleme kapatıldı; bağlantı iptal edildi".into());
        }
        let applied = self.apply(incoming);
        drop(gate);
        let response = match applied {
            Ok(n) => {
                let bundle = self.prepare()?;
                write_frame(
                    &mut stream,
                    &Reply {
                        bundle: Some(bundle),
                        error: None,
                    },
                )?;
                return Ok(n);
            }
            Err(e) => Reply {
                bundle: None,
                error: Some(e),
            },
        };
        write_frame(&mut stream, &response)?;
        Err(response.error.unwrap())
    }
    fn exchange(&self, peer: &Peer) -> Result<usize> {
        let outgoing = self.prepare()?;
        let socket =
            TcpStream::connect_timeout(&peer.address, Duration::from_secs(3)).map_err(|_| {
                "Bilgisayar açık, aynı Wi-Fi ağına bağlı ve port 45873 erişilebilir olmalı."
            })?;
        let mut config =
            ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()
                .map_err(err)?
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(Pin(peer.pin)))
                .with_no_client_auth();
        config.resumption = rustls::client::Resumption::disabled();
        let mut stream = StreamOwned::new(
            ClientConnection::new(
                Arc::new(config),
                ServerName::try_from("focusflow.local").map_err(err)?,
            )
            .map_err(err)?,
            TimedTcp::new(socket),
        );
        stream.write_all(&peer.token).map_err(err)?;
        write_frame(&mut stream, &outgoing)?;
        let response: Reply = read_frame(&mut stream)?;
        if let Some(error) = response.error {
            return Err(error);
        }
        let gate = self.state.lock().map_err(err)?;
        if !gate.config.peer.as_ref().is_some_and(|p| {
            p.address == peer.address && p.pin == peer.pin && bool::from(p.token.ct_eq(&peer.token))
        }) {
            return Err("Cihaz bağlantısı kaldırıldı".into());
        }
        let applied = self.apply(response.bundle.ok_or("Eksik eşitleme yanıtı")?);
        drop(gate);
        applied
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reply {
    bundle: Option<Bundle>,
    error: Option<String>,
}
fn allowed(ip: IpAddr) -> bool {
    matches!(ip,IpAddr::V4(v) if v.is_private()||v.is_loopback())
}
fn pairing_code(h: &Host, addr: SocketAddr) -> String {
    format!(
        "FF1|{addr}|{}|{}",
        URL_SAFE_NO_PAD.encode(Sha256::digest(&h.cert)),
        URL_SAFE_NO_PAD.encode(h.token)
    )
}
fn parse_code(code: &str) -> Result<Peer> {
    if code.len() > 200 {
        return Err("Bağlantı kodu çok uzun".into());
    }
    let parts: Vec<_> = code.trim().split('|').collect();
    if parts.len() != 4 || parts[0] != "FF1" {
        return Err("Bilgisayardaki bağlantı kodunun tamamını yapıştırın".into());
    }
    let address: SocketAddr = parts[1]
        .parse()
        .map_err(|_| "Kodda yerel IPv4 adresi gerekli")?;
    if !allowed(address.ip()) || address.port() == 0 {
        return Err("Yalnızca yerel IPv4 ağına bağlanılabilir".into());
    }
    let decode = |s: &str| -> Result<[u8; 32]> {
        URL_SAFE_NO_PAD
            .decode(s)
            .map_err(err)?
            .try_into()
            .map_err(|_| "Bağlantı kodu eksik veya bozuk".into())
    };
    Ok(Peer {
        address,
        pin: decode(parts[2])?,
        token: decode(parts[3])?,
    })
}
fn server_config(h: &Host) -> Result<Arc<ServerConfig>> {
    ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .map_err(err)?
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from(h.cert.clone())],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(h.key.clone())),
        )
        .map(Arc::new)
        .map_err(err)
}
#[derive(Debug)]
struct Pin([u8; 32]);
impl ServerCertVerifier for Pin {
    fn verify_server_cert(
        &self,
        cert: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: &ServerName<'_>,
        _: &[u8],
        _: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        if bool::from(Sha256::digest(cert.as_ref()).as_slice().ct_eq(&self.0)) {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::General(
                "Bilgisayarın kimliği doğrulanamadı. Yeni eşleştirme kodunu kullanın.".into(),
            ))
        }
    }
    fn verify_tls12_signature(
        &self,
        m: &[u8],
        c: &CertificateDer<'_>,
        d: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            m,
            c,
            d,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }
    fn verify_tls13_signature(
        &self,
        m: &[u8],
        c: &CertificateDer<'_>,
        d: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            m,
            c,
            d,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}
struct TimedTcp {
    stream: TcpStream,
    deadline: Instant,
}
impl TimedTcp {
    fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            deadline: Instant::now() + Duration::from_secs(15),
        }
    }
    fn remaining(&self) -> std::io::Result<Duration> {
        self.deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Eşitleme zaman aşımı",
            ))
    }
}
impl Read for TimedTcp {
    fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
        self.stream.set_read_timeout(Some(self.remaining()?))?;
        self.stream.read(b)
    }
}
impl Write for TimedTcp {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.stream.set_write_timeout(Some(self.remaining()?))?;
        self.stream.write(b)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.flush()
    }
}
fn write_frame<T: Serialize>(stream: &mut impl Write, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec(value).map_err(err)?;
    if bytes.len() > MAX_FRAME {
        return Err("Eşitleme 32 MB sınırını aşıyor".into());
    }
    stream
        .write_all(&(bytes.len() as u32).to_be_bytes())
        .map_err(err)?;
    stream.write_all(&bytes).map_err(err)?;
    stream.flush().map_err(err)
}
fn read_frame<T: serde::de::DeserializeOwned>(stream: &mut impl Read) -> Result<T> {
    let mut n = [0; 4];
    stream.read_exact(&mut n).map_err(err)?;
    let length = u32::from_be_bytes(n) as usize;
    if length > MAX_FRAME {
        return Err("Eşitleme 32 MB sınırını aşıyor".into());
    }
    let mut bytes = vec![0; length];
    stream.read_exact(&mut bytes).map_err(err)?;
    serde_json::from_slice(&bytes).map_err(err)
}
fn local_addresses() -> Vec<Ipv4Addr> {
    let mut result = Vec::new();
    #[cfg(unix)]
    unsafe {
        let mut list = std::ptr::null_mut();
        if libc::getifaddrs(&mut list) == 0 {
            let mut p = list;
            while !p.is_null() {
                let a = &*p;
                if !a.ifa_addr.is_null() && (*a.ifa_addr).sa_family as i32 == libc::AF_INET {
                    let addr = &*(a.ifa_addr as *const libc::sockaddr_in);
                    let ip = Ipv4Addr::from(addr.sin_addr.s_addr.to_ne_bytes());
                    if ip.is_private() && !result.contains(&ip) {
                        result.push(ip);
                    }
                }
                p = a.ifa_next;
            }
            libc::freeifaddrs(list);
        }
    }
    result.sort();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Stamp, TimingPolicy};
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    fn manager(offset: i64) -> Arc<LanSync> {
        let mut e = Engine::from_connection(
            rusqlite::Connection::open_in_memory().unwrap(),
            Stamp::now(),
        )
        .unwrap();
        e.set_timing_policy(TimingPolicy::Mobile);
        let wall = 1_792_670_400_000 + offset;
        e.start("stopwatch", "work", 1, 0, Stamp { wall, mono: 0 })
            .unwrap();
        e.action(
            "finish",
            Stamp {
                wall: wall + 1000,
                mono: 1000,
            },
        )
        .unwrap();
        let dir = std::env::temp_dir().join(format!(
            "focusflow-sync-tls-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Arc::new(LanSync {
            engine: Arc::new(Mutex::new(e)),
            dir,
            state: Mutex::new(Runtime {
                config: Config::default(),
                status: Status::default(),
                generation: 0,
                request: 0,
            }),
            can_host: true,
        })
    }
    fn host() -> Host {
        let c = rcgen::generate_simple_self_signed(vec!["focusflow.local".into()]).unwrap();
        let mut token = [0; 32];
        getrandom::fill(&mut token).unwrap();
        Host {
            cert: c.cert.der().to_vec(),
            key: c.signing_key.serialize_der(),
            token,
            port: PORT,
        }
    }
    fn transfer(
        wrong_pin: bool,
        wrong_token: bool,
    ) -> (Result<usize>, Result<usize>, Arc<LanSync>, Arc<LanSync>) {
        let a = manager(0);
        let b = manager(5000);
        let h = host();
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        a.state.lock().unwrap().config.host = Some(h.clone());
        let mut peer = parse_code(&pairing_code(&h, address)).unwrap();
        if wrong_pin {
            peer.pin[0] ^= 1;
        }
        if wrong_token {
            peer.token[0] ^= 1;
        }
        b.state.lock().unwrap().config.peer = Some(peer.clone());
        let a2 = a.clone();
        let server = thread::spawn(move || {
            let (socket, _) = listener.accept().unwrap();
            a2.serve(socket, server_config(&h).unwrap(), &h)
        });
        let client = b.exchange(&peer);
        let server = server.join().unwrap();
        (client, server, a, b)
    }
    #[test]
    fn tls_exchange_is_authenticated_and_bidirectional() {
        let (client, server, a, b) = transfer(false, false);
        assert!(client.is_ok(), "{client:?}");
        assert!(server.is_ok(), "{server:?}");
        for m in [&a, &b] {
            let e = m.engine.lock().unwrap();
            assert_eq!(e.sessions().unwrap().len(), 2);
            assert_eq!(
                e.sessions()
                    .unwrap()
                    .iter()
                    .map(|s| s.elapsed_ms)
                    .sum::<i64>(),
                2000
            );
        }
    }
    #[test]
    fn wrong_certificate_is_rejected_before_study_transfer() {
        let (client, server, a, b) = transfer(true, false);
        assert!(client.is_err());
        assert!(server.is_err());
        assert_eq!(a.engine.lock().unwrap().sessions().unwrap().len(), 1);
        assert_eq!(b.engine.lock().unwrap().sessions().unwrap().len(), 1);
    }
    #[test]
    fn wrong_pairing_secret_cannot_read_or_write_records() {
        let (client, server, a, b) = transfer(false, true);
        assert!(client.is_err());
        assert!(server.unwrap_err().contains("reddedildi"));
        assert_eq!(a.engine.lock().unwrap().sessions().unwrap().len(), 1);
        assert_eq!(b.engine.lock().unwrap().sessions().unwrap().len(), 1);
    }
    #[test]
    fn public_addresses_dns_and_broken_pairing_codes_are_rejected() {
        let h = host();
        let code = pairing_code(&h, "192.168.1.20:45873".parse().unwrap());
        assert!(parse_code(&code).is_ok());
        for address in [
            "8.8.8.8:443",
            "example.com:443",
            "[::1]:45873",
            "192.168.1.2:0",
        ] {
            assert!(parse_code(&code.replace("192.168.1.20:45873", address)).is_err());
        }
        assert!(parse_code("FF1|192.168.1.2:45873|invalid|invalid").is_err());
        assert!(parse_code(&"x".repeat(201)).is_err());
    }
    #[test]
    fn oversized_wire_frame_is_rejected_before_body_allocation() {
        let bytes = ((MAX_FRAME + 1) as u32).to_be_bytes();
        let mut input = &bytes[..];
        assert!(read_frame::<Bundle>(&mut input)
            .unwrap_err()
            .contains("32 MB"));
    }
    #[test]
    fn turning_off_revokes_the_saved_pairing_and_new_host_has_new_identity() {
        let m = manager(0);
        m.configure("host", None).unwrap();
        let old = m.state.lock().unwrap().config.host.clone().unwrap();
        m.configure("off", None).unwrap();
        assert!(!m.enabled());
        m.configure("host", None).unwrap();
        assert_ne!(
            m.state.lock().unwrap().config.host.as_ref().unwrap().token,
            old.token
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(m.dir.join("wifi-sync.json"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
    }
}
