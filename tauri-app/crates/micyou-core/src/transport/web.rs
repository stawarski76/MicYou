/*
 * MicYou — Turns your Android device into a high-quality PC microphone.
 * Copyright (C) 2026 LanRhyme <https://github.com/MicYou-Dev/MicYou>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version, with the MicYou Plugin Exception.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 */

use rcgen::{CertificateParams, KeyPair, SanType};
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub const DEFAULT_WEB_PORT: u16 = 8443;

pub struct WebServer {
    cancel_token: std::sync::Mutex<CancellationToken>,
    client_count: Arc<AtomicUsize>,
    running: Arc<AtomicBool>,
    task: std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
    /// Best-effort IPv6 companion listener task (see `start`). The IPv4
    /// listener in `task` remains the primary one for lifecycle state.
    task_v6: std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
}

struct GeneratedCert {
    cert_pem: String,
    key_pem: String,
}

/// Where the self-signed certificate lives. It sits next to the shared config
/// rather than in the world-readable temp directory because it holds the
/// TLS private key.
fn cert_cache_dir() -> PathBuf {
    crate::config::config_dir().join("web_cert")
}

/// LAN IPv4 addresses a browser may use to reach the web mode.
pub fn get_lan_ips() -> Vec<IpAddr> {
    let Ok(interfaces) = local_ip_address::list_afinet_netifas() else {
        return Vec::new();
    };
    interfaces
        .into_iter()
        .map(|(_, ip)| ip)
        .filter(|ip| match ip {
            IpAddr::V4(v4) => !v4.is_loopback() && !v4.is_link_local() && !is_benchmark_v4(v4),
            IpAddr::V6(_) => false,
        })
        .collect()
}

/// 198.18.0.0/15 is used by proxy tools (Clash TUN) for fake IPs.
fn is_benchmark_v4(ip: &std::net::Ipv4Addr) -> bool {
    let [a, b, ..] = ip.octets();
    a == 198 && (b & 0xfe) == 18
}

/// Bindable LAN IPv6 addresses (ULA/GUA, best first). Additive companion to
/// `get_lan_ips`, used for the WebSocket origin check and certificate SANs.
pub fn get_lan_ipv6s() -> Vec<IpAddr> {
    crate::transport::net_bind::collect_ipv6_interfaces(&[])
        .into_iter()
        .map(|(ip, _)| IpAddr::V6(ip))
        .collect()
}

/// Every address the certificate must cover, loopback first.
fn certificate_ips() -> Vec<IpAddr> {
    let mut ips = vec![
        IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
        IpAddr::V6(std::net::Ipv6Addr::LOCALHOST),
    ];
    ips.extend(get_lan_ips());
    ips.extend(get_lan_ipv6s());
    ips
}

fn generate_self_signed_cert_pem(ips: &[IpAddr]) -> Result<GeneratedCert, String> {
    let mut params = CertificateParams::new(vec!["localhost".to_string()])
        .map_err(|e| format!("Failed to create cert params: {}", e))?;
    params
        .subject_alt_names
        .extend(ips.iter().copied().map(SanType::IpAddress));

    let key_pair =
        KeyPair::generate().map_err(|e| format!("Failed to generate key pair: {}", e))?;
    let cert = params
        .self_signed(&key_pair)
        .map_err(|e| format!("Failed to sign certificate: {}", e))?;

    Ok(GeneratedCert {
        cert_pem: cert.pem(),
        key_pem: key_pair.serialize_pem(),
    })
}

/// Write a file readable by the current user only.
fn write_private(path: &std::path::Path, contents: &str) -> std::io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut file = options.open(path)?;
    // `mode` only applies on creation; tighten a key file left by an older build.
    #[cfg(unix)]
    file.set_permissions(std::os::unix::fs::PermissionsExt::from_mode(0o600))?;
    std::io::Write::write_all(&mut file, contents.as_bytes())
}

/// Older versions kept the certificate and its private key in the temp
/// directory; nothing reads them any more.
fn remove_legacy_cert_dir() {
    let legacy = std::env::temp_dir().join("micyou_web_cert");
    if let Err(e) = std::fs::remove_dir_all(&legacy) {
        if e.kind() != std::io::ErrorKind::NotFound {
            log::warn!("[Web] could not remove {}: {e}", legacy.display());
        }
    }
}

/// Reuse the cached certificate while it still covers the current LAN
/// addresses, so browsers only re-accept it when the network changed.
fn load_or_generate_cert_pem() -> Result<GeneratedCert, String> {
    let cache_dir = cert_cache_dir();
    let cert_path = cache_dir.join("cert.pem");
    let key_path = cache_dir.join("key.pem");
    let sans_path = cache_dir.join("sans.txt");
    remove_legacy_cert_dir();
    let ips = certificate_ips();
    // Sorted, so a different interface enumeration order alone does not make
    // browsers accept a new certificate.
    let mut sans = ips.iter().map(IpAddr::to_string).collect::<Vec<_>>();
    sans.sort_unstable();
    sans.dedup();
    let sans = sans.join("\n");

    let cached = (
        std::fs::read_to_string(&cert_path),
        std::fs::read_to_string(&key_path),
        std::fs::read_to_string(&sans_path),
    );
    if let (Ok(cert_pem), Ok(key_pem), Ok(cached_sans)) = cached {
        if !cert_pem.is_empty() && !key_pem.is_empty() && cached_sans == sans {
            return Ok(GeneratedCert { cert_pem, key_pem });
        }
    }

    let cert = generate_self_signed_cert_pem(&ips)?;
    let persisted = std::fs::create_dir_all(&cache_dir)
        .and_then(|()| write_private(&key_path, &cert.key_pem))
        .and_then(|()| std::fs::write(&cert_path, &cert.cert_pem))
        .and_then(|()| std::fs::write(&sans_path, &sans));
    if let Err(e) = persisted {
        log::warn!("[Web] could not cache the TLS certificate: {e}");
    }
    Ok(cert)
}

fn float32_to_pcm16(float32_bytes: &[u8]) -> Vec<u8> {
    float32_bytes
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|chunk| {
            let sample = f32::from_le_bytes(*chunk).clamp(-1.0, 1.0);
            ((sample * 32767.0) as i16).to_le_bytes()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_self_signed_cert_pem() {
        let cert = generate_self_signed_cert_pem(&certificate_ips());
        assert!(
            cert.is_ok(),
            "Cert generation should succeed: {:?}",
            cert.err()
        );
        let c = cert.unwrap();
        assert!(c.cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(c.key_pem.contains("PRIVATE KEY"));
    }

    #[test]
    fn test_get_lan_ips() {
        for ip in get_lan_ips() {
            assert!(ip.is_ipv4() && !ip.is_loopback(), "Invalid IP: {}", ip);
        }
    }

    #[test]
    fn test_get_lan_ipv6s_are_bindable() {
        for ip in get_lan_ipv6s() {
            match ip {
                IpAddr::V6(v6) => {
                    assert!(crate::transport::net_bind::is_bindable_v6(&v6), "Not bindable: {}", ip)
                }
                IpAddr::V4(_) => panic!("get_lan_ipv6s returned IPv4: {}", ip),
            }
        }
    }

    fn origin_headers(origin: Option<&str>, host: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("host", host.parse().unwrap());
        if let Some(origin) = origin { headers.insert("origin", origin.parse().unwrap()); }
        headers
    }

    #[test]
    fn websocket_origin_requires_https_and_the_same_host_and_port() {
        for (origin, host) in [
            ("https://localhost:8443", "localhost:8443"),
            ("https://127.0.0.1:8443", "127.0.0.1:8443"),
            ("https://[::1]:8443", "[::1]:8443"),
            ("https://LOCALHOST", "localhost:443"),
        ] { assert!(is_valid_origin(&origin_headers(Some(origin), host)), "{origin}"); }
        for origin in [
            "http://localhost:8443", "https://localhost:8444", "https://127.0.0.1:8443",
            "https://localhost.evil.example:8443", "https://evil.example:8443", "null",
            "https://user@localhost:8443", "https://localhost:8443/", "https://localhost:8443/a",
            "https://localhost:8443?x=1", "https://localhost:8443#fragment",
            "https://localhost:8443:9999", "https://localhost:wrong",
        ] { assert!(!is_valid_origin(&origin_headers(Some(origin), "localhost:8443")), "{origin}"); }
    }

    #[test]
    fn websocket_origin_rejects_missing_duplicate_and_nonlocal_headers() {
        assert!(!is_valid_origin(&origin_headers(None, "localhost:8443")));
        let mut headers = origin_headers(Some("https://localhost:8443"), "localhost:8443");
        headers.append("origin", "https://localhost:8443".parse().unwrap());
        assert!(!is_valid_origin(&headers));
        let mut headers = origin_headers(Some("https://localhost:8443"), "localhost:8443");
        headers.append("host", "localhost:8443".parse().unwrap());
        assert!(!is_valid_origin(&headers));
        headers.remove("host");
        assert!(!is_valid_origin(&headers));
        assert!(!is_valid_origin(&origin_headers(Some("https://evil.example:8443"), "evil.example:8443")));
        for authority in ["localhost:", "localhost:wrong", "localhost:65536"] {
            assert!(!is_valid_origin(&origin_headers(Some("https://localhost"), authority)));
            assert!(!is_valid_origin(&origin_headers(Some(&format!("https://{authority}")), authority)));
        }
        let mut headers = origin_headers(None, "localhost:8443");
        headers.insert("origin", axum::http::HeaderValue::from_bytes(&[0xff]).unwrap());
        assert!(!is_valid_origin(&headers));
    }

    #[test]
    fn websocket_origin_accepts_this_servers_ipv4_and_ipv6_addresses() {
        for ip in get_lan_ips().into_iter().chain(get_lan_ipv6s()) {
            let authority = match ip {
                IpAddr::V4(_) => format!("{ip}:8443"),
                IpAddr::V6(_) => format!("[{ip}]:8443"),
            };
            assert!(is_valid_origin(&origin_headers(Some(&format!("https://{authority}")), &authority)));
        }
    }

    #[test]
    fn test_float32_to_pcm16_one() {
        let input = 1.0f32.to_le_bytes();
        let pcm = float32_to_pcm16(&input);
        assert_eq!(pcm.len(), 2);
        let sample = i16::from_le_bytes([pcm[0], pcm[1]]);
        assert_eq!(sample, 32767);
    }

    #[test]
    fn test_float32_to_pcm16_neg_one() {
        let input = (-1.0f32).to_le_bytes();
        let pcm = float32_to_pcm16(&input);
        let sample = i16::from_le_bytes([pcm[0], pcm[1]]);
        assert_eq!(sample, -32767);
    }

    #[test]
    fn test_float32_to_pcm16_zero() {
        let input = 0.0f32.to_le_bytes();
        let pcm = float32_to_pcm16(&input);
        let sample = i16::from_le_bytes([pcm[0], pcm[1]]);
        assert_eq!(sample, 0);
    }

    #[test]
    fn decrement_client_count_does_not_underflow_after_stop_reset() {
        let count = AtomicUsize::new(0);
        assert_eq!(decrement_client_count(&count), 0);
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn decrement_client_count_returns_the_remaining_clients() {
        let count = AtomicUsize::new(2);
        assert_eq!(decrement_client_count(&count), 1);
        assert_eq!(decrement_client_count(&count), 0);
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn busy_web_sender_is_rejected_without_cancelling_the_active_sender() {
        let senders = ActiveWebSender::default();
        let shutdown = CancellationToken::new();
        let (first, token) = senders.activate(&shutdown).unwrap();
        assert!(senders.activate(&shutdown).is_none());
        assert!(!token.is_cancelled());
        assert!(senders.is_current(first));
        assert!(!senders.deactivate(first + 1));
        assert!(senders.deactivate(first));
        assert!(!senders.is_current(first));
        let (second, _) = senders.activate(&shutdown).unwrap();
        assert!(second > first);
        assert!(!senders.deactivate(first));
        assert!(senders.is_current(second));
    }

    #[test]
    fn concurrent_web_candidates_cannot_replace_each_other() {
        let senders = Arc::new(ActiveWebSender::default());
        let shutdown = CancellationToken::new();
        let barrier = Arc::new(std::sync::Barrier::new(8));
        let tasks: Vec<_> = (0..8).map(|_| {
            let senders = senders.clone(); let shutdown = shutdown.clone(); let barrier = barrier.clone();
            std::thread::spawn(move || { barrier.wait(); senders.activate(&shutdown) })
        }).collect();
        let winners: Vec<_> = tasks.into_iter().filter_map(|t| t.join().unwrap()).collect();
        assert_eq!(winners.len(), 1);
        assert!(!winners[0].1.is_cancelled());
        assert!(senders.is_current(winners[0].0));
    }

    #[test]
    fn server_shutdown_closes_the_active_sender_and_rejects_new_senders() {
        let senders = ActiveWebSender::default();
        let shutdown = CancellationToken::new();
        let (generation, cancel) = senders.activate(&shutdown).unwrap();
        shutdown.cancel();
        assert!(cancel.is_cancelled());
        assert!(!senders.is_current(generation));
        assert!(senders.deactivate(generation));
        assert!(senders.activate(&shutdown).is_none());
    }

    #[tokio::test]
    async fn https_upgrade_rejects_a_second_sender_and_preserves_the_first() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let cert = generate_self_signed_cert_pem(&["127.0.0.1".parse().unwrap()]).unwrap();
        let chain = CertificateDer::pem_slice_iter(cert.cert_pem.as_bytes()).collect::<Result<Vec<_>, _>>().unwrap();
        let key = PrivateKeyDer::from_pem_slice(cert.key_pem.as_bytes()).unwrap();
        let mut roots = rustls::RootCertStore::empty();
        roots.add(chain[0].clone()).unwrap();
        let client = rustls::ClientConfig::builder().with_root_certificates(roots).with_no_client_auth();
        let connector = tokio_rustls::TlsConnector::from(Arc::new(client));
        let server = ServerConfig::builder().with_no_client_auth().with_single_cert(chain, key).unwrap();
        let tcp = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = tcp.local_addr().unwrap();
        let (completed, completed_rx) = tokio::sync::mpsc::channel(MAX_TLS_HANDSHAKES);
        let listener = TlsListener {
            tcp, acceptor: TlsAcceptor::from(Arc::new(server)),
            handshake_slots: Arc::new(Semaphore::new(MAX_TLS_HANDSHAKES)), completed, completed_rx,
        };
        let shutdown = CancellationToken::new();
        let count = Arc::new(AtomicUsize::new(0));
        let (audio_tx, _audio_rx) = tokio::sync::mpsc::channel(16);
        let state = WebServerState {
            events: Arc::new(crate::transport::tcp::admission_tests::Events::default()),
            audio_tx, client_count: count.clone(), active_sender: Arc::new(ActiveWebSender::default()),
            websocket_slots: Arc::new(Semaphore::new(MAX_WEBSOCKET_CONNECTIONS)), shutdown: shutdown.clone(),
        };
        let app = Router::new().route("/ws", get(handle_websocket)).with_state(state);
        let stop = shutdown.clone();
        let task = tokio::spawn(async move {
            axum::serve(listener, app).with_graceful_shutdown(stop.cancelled_owned()).await.unwrap();
        });

        async fn upgrade(connector: &tokio_rustls::TlsConnector, addr: SocketAddr, origin: bool) -> (tokio_rustls::client::TlsStream<TcpStream>, String) {
            let socket = TcpStream::connect(addr).await.unwrap();
            let mut socket = connector.connect(rustls::pki_types::ServerName::try_from("localhost").unwrap(), socket).await.unwrap();
            let origin = if origin { format!("Origin: https://localhost:{}\r\n", addr.port()) } else { String::new() };
            let request = format!("GET /ws HTTP/1.1\r\nHost: localhost:{}\r\n{origin}Connection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n", addr.port());
            socket.write_all(request.as_bytes()).await.unwrap();
            let mut response = Vec::new();
            loop {
                response.push(socket.read_u8().await.unwrap());
                if response.ends_with(b"\r\n\r\n") { break; }
                assert!(response.len() < 8192);
            }
            (socket, String::from_utf8(response).unwrap())
        }
        async fn ping(socket: &mut tokio_rustls::client::TlsStream<TcpStream>) {
            // A masked one-byte WebSocket ping; the server must return its pong.
            socket.write_all(&[0x89, 0x81, 1, 2, 3, 4, b'x' ^ 1]).await.unwrap();
            let mut pong = [0; 3]; socket.read_exact(&mut pong).await.unwrap();
            assert_eq!(pong, [0x8a, 1, b'x']);
        }
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            let (mut first, response) = upgrade(&connector, addr, true).await;
            assert!(response.starts_with("HTTP/1.1 101"), "{response}");
            ping(&mut first).await;
            assert_eq!(count.load(Ordering::SeqCst), 1);
            let (_second, response) = upgrade(&connector, addr, true).await;
            assert!(response.starts_with("HTTP/1.1 503"), "{response}");
            let (_missing_origin, response) = upgrade(&connector, addr, false).await;
            assert!(response.starts_with("HTTP/1.1 403"), "{response}");
            ping(&mut first).await;
            assert_eq!(count.load(Ordering::SeqCst), 1);
            drop(first);
            while count.load(Ordering::SeqCst) != 0 { tokio::task::yield_now().await; }
            let (mut third, response) = upgrade(&connector, addr, true).await;
            assert!(response.starts_with("HTTP/1.1 101"), "{response}");
            ping(&mut third).await;
            drop(third);
            while count.load(Ordering::SeqCst) != 0 { tokio::task::yield_now().await; }
        }).await.unwrap();
        shutdown.cancel();
        tokio::time::timeout(std::time::Duration::from_secs(3), task).await.unwrap().unwrap();
    }

}

use crate::events::SharedEvents;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::serve::Listener;
use axum::Router;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_rustls::{server::TlsStream, TlsAcceptor};

const MAX_TLS_HANDSHAKES: usize = 32;
// Keep the upgrade permit through teardown, so a new sender cannot race the
// previous handler's client-count and disconnect events.
const MAX_WEBSOCKET_CONNECTIONS: usize = 1;
const TLS_HANDSHAKE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

const WEB_CLIENT_HTML: &str = include_str!("../../assets/web_client.html");
const ALPINE_JS: &str = include_str!("../../assets/alpine.min.js");

/// Origin is browser cross-site protection, not client authentication: native
/// clients can forge it. Require the actual HTTPS page origin including its port.
fn is_valid_origin(headers: &HeaderMap) -> bool {
    if headers.get_all("origin").iter().count() != 1 || headers.get_all("host").iter().count() != 1 {
        return false;
    }
    let Some(origin) = headers.get("origin").and_then(|h| h.to_str().ok()) else { return false; };
    let Some(host) = headers.get("host").and_then(|h| h.to_str().ok()) else { return false; };
    let Ok(uri) = origin.parse::<axum::http::Uri>() else { return false; };
    let Some(authority) = uri.authority() else { return false; };
    let Ok(host) = host.parse::<axum::http::uri::Authority>() else { return false; };
    if uri.scheme_str() != Some("https")
        || origin != format!("https://{authority}")
        || authority.as_str().contains('@') || host.as_str().contains('@')
        || !authority.host().eq_ignore_ascii_case(host.host())
        || authority_port(&authority).is_none()
        || authority_port(&authority) != authority_port(&host)
    { return false; }
    let hostname = authority.host().trim_start_matches('[').trim_end_matches(']');
    if hostname.eq_ignore_ascii_case("localhost") { return true; }
    let Ok(ip) = hostname.parse::<IpAddr>() else { return false; };
    ip.is_loopback() || get_lan_ips().contains(&ip) || get_lan_ipv6s().contains(&ip)
}

// `port_u16() == None` alone does not distinguish an omitted port from an
// invalid explicit port. Never turn malformed input into the HTTPS default.
fn authority_port(authority: &axum::http::uri::Authority) -> Option<u16> {
    let suffix = authority.as_str().strip_prefix(authority.host())?;
    if suffix.is_empty() { Some(443) } else { suffix.strip_prefix(':')?.parse().ok() }
}

async fn handle_websocket(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    axum::extract::State(state): axum::extract::State<WebServerState>,
) -> impl IntoResponse {
    if !is_valid_origin(&headers) {
        return (StatusCode::FORBIDDEN, "Invalid origin").into_response();
    }
    let permit = match state.websocket_slots.clone().try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "Too many WebSocket connections",
            )
                .into_response()
        }
    };
    ws.on_upgrade(move |socket| handle_ws_socket(socket, state, permit))
}

async fn handle_ws_socket(
    mut socket: WebSocket,
    state: WebServerState,
    _permit: OwnedSemaphorePermit,
) {
    let Some((generation, cancel)) = state.active_sender.activate(&state.shutdown) else {
        let _ = socket.send(Message::Close(None)).await;
        return;
    };
    let count = state.client_count.fetch_add(1, Ordering::SeqCst) + 1;
    state.events.web_client_count(count as u32);
    log::info!("Web client connected (total: {})", count);

    if count == 1 {
        state
            .events
            .device_connected(crate::transport::tcp::DeviceInfo {
                name: "Web Browser".to_string(),
                ip: "browser".to_string(),
                latency: 0,
                platform: crate::transport::tcp::ClientPlatform::Unknown,
            });
    }

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                let _ = socket.send(Message::Close(None)).await;
                break;
            }
            message = socket.recv() => match message {
            Some(Ok(Message::Binary(data))) => {
                if !state.active_sender.is_current(generation) {
                    break;
                }
                // The bundled client never sends these; drop the peer
                // instead of logging every malformed frame.
                if data.len() > 64 * 1024 || !data.len().is_multiple_of(4) {
                    log::warn!("Web audio frame of {} bytes is malformed, closing", data.len());
                    break;
                }

                let pcm = float32_to_pcm16(&data);
                let packet = micyou_protocol::micyou::AudioPacketMessage {
                    buffer: pcm,
                    sample_rate: 48000,
                    channel_count: 1,
                    audio_format: 2,
                    codec: micyou_protocol::CODEC_PCM,
                };
                match state.audio_tx.try_send((generation, packet)) {
                    Ok(()) | Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {}
                    Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => break,
                }
            }
            Some(Ok(Message::Close(_))) | None => break,
            Some(Err(e)) => {
                log::warn!("WebSocket error: {}", e);
                break;
            }
            _ => {}
            }
        }
    }

    if state.active_sender.deactivate(generation) {
        let remaining = decrement_client_count(&state.client_count);
        state.events.web_client_count(remaining as u32);
        log::info!("Web client disconnected (remaining: {})", remaining);

        if remaining == 0 {
            state.events.device_disconnected();
        }
    } else {
        log::info!("Inactive Web client closed");
    }
}

/// Decrement without wrapping below zero; returns the new count.
fn decrement_client_count(client_count: &AtomicUsize) -> usize {
    client_count
        .try_update(Ordering::SeqCst, Ordering::SeqCst, |count| count.checked_sub(1))
        .map_or(0, |previous| previous - 1)
}

async fn serve_html() -> impl IntoResponse {
    Html(WEB_CLIENT_HTML)
}

async fn serve_alpine_js() -> impl IntoResponse {
    ([("Content-Type", "application/javascript")], ALPINE_JS)
}

#[derive(Default)]
struct ActiveWebSender {
    generation: AtomicU64,
    cancel: std::sync::Mutex<Option<(u64, CancellationToken)>>,
}

impl ActiveWebSender {
    /// `shutdown` is the server's token: stopping the server must also end
    /// upgraded sockets, which axum's graceful shutdown no longer tracks.
    fn activate(&self, shutdown: &CancellationToken) -> Option<(u64, CancellationToken)> {
        let mut active = self.cancel.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if shutdown.is_cancelled() || active.is_some() { return None; }
        let cancel = shutdown.child_token();
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        *active = Some((generation, cancel.clone()));
        Some((generation, cancel))
    }

    fn is_current(&self, generation: u64) -> bool {
        let active = self.cancel.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        active.as_ref().is_some_and(|(current, cancel)| *current == generation && !cancel.is_cancelled())
    }

    fn deactivate(&self, generation: u64) -> bool {
        let mut active = self.cancel.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if active.as_ref().is_some_and(|(current, _)| *current == generation) {
            active.take();
            true
        } else {
            false
        }
    }
}

#[derive(Clone)]
pub struct WebServerState {
    pub events: SharedEvents,
    pub audio_tx: tokio::sync::mpsc::Sender<(u64, micyou_protocol::micyou::AudioPacketMessage)>,
    pub client_count: Arc<AtomicUsize>,
    active_sender: Arc<ActiveWebSender>,
    pub websocket_slots: Arc<Semaphore>,
    shutdown: CancellationToken,
}

struct TlsListener {
    tcp: TcpListener,
    acceptor: TlsAcceptor,
    handshake_slots: Arc<Semaphore>,
    completed: tokio::sync::mpsc::Sender<(TlsStream<TcpStream>, SocketAddr)>,
    completed_rx: tokio::sync::mpsc::Receiver<(TlsStream<TcpStream>, SocketAddr)>,
}

impl Listener for TlsListener {
    type Io = TlsStream<TcpStream>;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            tokio::select! {
                Some(accepted) = self.completed_rx.recv() => return accepted,
                accept_result = self.tcp.accept() => {
                    match accept_result {
                        Ok((stream, addr)) => {
                            let permit = match self.handshake_slots.clone().try_acquire_owned() {
                                Ok(permit) => permit,
                                Err(_) => continue,
                            };
                            let acceptor = self.acceptor.clone();
                            let completed = self.completed.clone();
                            tokio::spawn(async move {
                                let _permit = permit;
                                match tokio::time::timeout(TLS_HANDSHAKE_TIMEOUT, acceptor.accept(stream)).await {
                                    Ok(Ok(tls)) => { let _ = completed.send((tls, addr)).await; }
                                    Ok(Err(e)) => log::debug!("TLS handshake failed: {}", e),
                                    Err(_) => log::debug!("TLS handshake timed out for {}", addr),
                                }
                            });
                        }
                        Err(e) => {
                            log::warn!("TCP accept error: {}", e);
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        }
                    }
                }
            }
        }
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.tcp.local_addr()
    }
}

impl Default for WebServer {
    fn default() -> Self {
        Self::new()
    }
}

impl WebServer {
    pub fn new() -> Self {
        Self {
            cancel_token: std::sync::Mutex::new(CancellationToken::new()),
            client_count: Arc::new(AtomicUsize::new(0)),
            running: Arc::new(AtomicBool::new(false)),
            task: std::sync::Mutex::new(None),
            task_v6: std::sync::Mutex::new(None),
        }
    }

    pub fn client_count(&self) -> usize {
        self.client_count.load(Ordering::SeqCst)
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub async fn start(
        &self,
        port: u16,
        bind_address: &str,
        events: SharedEvents,
        audio_tx: tokio::sync::mpsc::Sender<(u64, micyou_protocol::micyou::AudioPacketMessage)>,
    ) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            return Err("Web server is already running".to_string());
        }

        let new_token = CancellationToken::new();
        let state = WebServerState {
            events,
            audio_tx,
            client_count: self.client_count.clone(),
            active_sender: Arc::new(ActiveWebSender::default()),
            websocket_slots: Arc::new(Semaphore::new(MAX_WEBSOCKET_CONNECTIONS)),
            shutdown: new_token.clone(),
        };

        let app = Router::new()
            .route("/", get(serve_html))
            .route("/alpine.min.js", get(serve_alpine_js))
            .route("/ws", get(handle_websocket))
            .with_state(state);
        // Clone for the best-effort IPv6 listener started below; the IPv4
        // serve task keeps consuming the original exactly as before.
        let app_v6 = app.clone();

        // Load TLS certificate
        let cert = load_or_generate_cert_pem()?;
        let cert_chain = CertificateDer::pem_slice_iter(cert.cert_pem.as_bytes())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read certificate: {}", e))?;
        let private_key = PrivateKeyDer::from_pem_slice(cert.key_pem.as_bytes())
            .map_err(|e| format!("Failed to read private key: {}", e))?;

        let mut tls_config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain, private_key)
            .map_err(|e| format!("TLS config error: {}", e))?;

        tls_config.alpn_protocols = vec![b"http/1.1".to_vec()];

        let acceptor = TlsAcceptor::from(Arc::new(tls_config));
        let acceptor_v6 = acceptor.clone();

        let tcp = crate::transport::net_bind::bind_tcp_listener(bind_address, port)
            .await
            .map_err(|e| format!("Web server bind error: {}", e))?;

        let (completed, completed_rx) = tokio::sync::mpsc::channel(MAX_TLS_HANDSHAKES);
        let tls_listener = TlsListener {
            tcp,
            acceptor,
            handshake_slots: Arc::new(Semaphore::new(MAX_TLS_HANDSHAKES)),
            completed,
            completed_rx,
        };

        log::info!(
            "Web server listening on https://{}",
            crate::transport::net_bind::normalize_socket_addr(bind_address, port)
        );

        *self
            .cancel_token
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = new_token.clone();
        let cancel_v6 = new_token.clone();
        let cancel = new_token;
        let running = self.running.clone();
        let client_count = self.client_count.clone();

        running.store(true, Ordering::SeqCst);

        let task = tokio::spawn(async move {
            axum::serve(tls_listener, app)
                .with_graceful_shutdown(async move {
                    cancel.cancelled().await;
                })
                .await
                .ok();

            running.store(false, Ordering::SeqCst);
            client_count.store(0, Ordering::SeqCst);
        });
        *self.task.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(task);

        // Best-effort IPv6 companion listener so phones on IPv6-only or
        // IPv6-preferring networks can reach web mode. It is strictly
        // additive: the IPv4 listener above is untouched, `only_v6(true)`
        // keeps the two sockets from conflicting, and a failure here (no
        // IPv6 stack, port unavailable) only logs and leaves web mode
        // working exactly as before over IPv4.
        if !crate::transport::net_bind::wants_v6_companion(bind_address) {
            return Ok(());
        }
        match crate::transport::net_bind::bind_tcp_listener_v6only(port) {
            Ok(tcp_v6) => {
                let (completed_v6, completed_rx_v6) =
                    tokio::sync::mpsc::channel(MAX_TLS_HANDSHAKES);
                let tls_listener_v6 = TlsListener {
                    tcp: tcp_v6,
                    acceptor: acceptor_v6,
                    handshake_slots: Arc::new(Semaphore::new(MAX_TLS_HANDSHAKES)),
                    completed: completed_v6,
                    completed_rx: completed_rx_v6,
                };
                log::info!("Web server listening on https://[::]:{} (IPv6)", port);
                let task_v6 = tokio::spawn(async move {
                    axum::serve(tls_listener_v6, app_v6)
                        .with_graceful_shutdown(async move {
                            cancel_v6.cancelled().await;
                        })
                        .await
                        .ok();
                });
                *self
                    .task_v6
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(task_v6);
            }
            Err(e) => {
                log::warn!(
                    "IPv6 web listener not started: {}{}",
                    e,
                    crate::transport::net_bind::companion_failure_hint(&e)
                );
            }
        }

        Ok(())
    }

    pub async fn stop(&self) {
        self.cancel_token
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .cancel();
        // The IPv6 companion listener shares the cancel token.
        for slot in [&self.task, &self.task_v6] {
            let task = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take();
            if let Some(mut task) = task {
                if tokio::time::timeout(std::time::Duration::from_secs(3), &mut task)
                    .await
                    .is_err()
                {
                    task.abort();
                    let _ = task.await;
                }
            }
        }
        self.running.store(false, Ordering::SeqCst);
        self.client_count.store(0, Ordering::SeqCst);
    }
}
