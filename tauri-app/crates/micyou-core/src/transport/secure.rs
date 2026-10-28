//! Persistent, user-owned TLS identity. Pairing codes are credentials: never log
//! this type or serialize it into ordinary server/UI preferences.
use rcgen::{CertificateParams, KeyPair};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use subtle::ConstantTimeEq;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const AUTH_MAGIC: &[u8; 8] = b"YMICv1\0\0";
pub const AUTH_OK: &[u8; 4] = b"YMOK";
const MAX_IDENTITY_BYTES: u64 = 64 * 1024;
static IDENTITY_LOCK: Mutex<()> = Mutex::new(());

pub struct Identity {
    pub(crate) tls: Arc<rustls::ServerConfig>,
    pub(crate) certificate: CertificateDer<'static>,
    token: [u8; 32],
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedIdentity { version: u8, cert: String, key: String, token: String }

pub fn hex(bytes: &[u8]) -> String { bytes.iter().map(|v| format!("{v:02x}")).collect() }

fn unhex(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) { return None; }
    let mut out = [0; 32];
    for (i, byte) in out.iter_mut().enumerate() { *byte = u8::from_str_radix(&value[i * 2..i * 2 + 2], 16).ok()?; }
    Some(out)
}

impl SavedIdentity {
    fn generate() -> Result<Self, String> {
        let key = KeyPair::generate().map_err(|_| "Could not generate TLS key")?;
        let cert = CertificateParams::new(vec!["localhost".into()]).map_err(|_| "Could not create TLS parameters")?
            .self_signed(&key).map_err(|_| "Could not create TLS identity")?;
        let mut token = [0; 32];
        getrandom::fill(&mut token).map_err(|_| "Secure random generator unavailable")?;
        Ok(Self { version: 1, cert: cert.pem(), key: key.serialize_pem(), token: hex(&token) })
    }

    fn identity(&self) -> Result<Identity, String> {
        if self.version != 1 { return Err("Unsupported pairing identity version".into()); }
        let token = unhex(&self.token).ok_or("Invalid saved pairing credential")?;
        let chain = CertificateDer::pem_slice_iter(self.cert.as_bytes()).collect::<Result<Vec<_>, _>>()
            .map_err(|_| "Invalid saved TLS certificate")?;
        if chain.len() != 1 { return Err("Expected one saved TLS certificate".into()); }
        let certificate = chain[0].clone();
        let key = PrivateKeyDer::from_pem_slice(self.key.as_bytes()).map_err(|_| "Invalid saved TLS key")?;
        let config = rustls::ServerConfig::builder().with_no_client_auth().with_single_cert(chain, key)
            .map_err(|_| "Saved TLS identity is inconsistent")?;
        Ok(Identity { tls: Arc::new(config), certificate, token })
    }
}

impl Identity {
    #[cfg(test)]
    pub(crate) fn ephemeral() -> Arc<Self> {
        Arc::new(SavedIdentity::generate().unwrap().identity().unwrap())
    }
    pub fn load() -> Result<Arc<Self>, String> {
        let dir = crate::config::config_dir().join("pairing");
        let _lease = lease_at(&dir, false)?;
        Self::load_at(&dir)
    }

    pub(crate) fn session_lease() -> Result<fs::File, String> {
        lease_at(&crate::config::config_dir().join("pairing"), false)
    }

    pub fn rotate() -> Result<String, String> {
        Self::rotate_at(&crate::config::config_dir().join("pairing"))
    }

    fn rotate_at(dir: &Path) -> Result<String, String> {
        let _guard = IDENTITY_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        // A live Wi-Fi listener holds a shared OS lock until every client task
        // exits. Rotation needs the exclusive lock, even across CLI/GUI processes.
        let _lease = lease_at(dir, true)?;
        let saved = SavedIdentity::generate()?;
        let identity = saved.identity()?;
        save(dir, &saved, true)?;
        Ok(identity.pairing_code())
    }

    pub(crate) fn load_at(dir: &Path) -> Result<Arc<Self>, String> {
        let _guard = IDENTITY_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let path = dir.join("identity.bin");
        match fs::symlink_metadata(&path) {
            Ok(meta) => {
                if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > MAX_IDENTITY_BYTES {
                    return Err("Invalid pairing identity file; explicit reset required".into());
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if meta.permissions().mode() & 0o077 != 0 { return Err("Pairing identity permissions must be 0600".into()); }
                }
                let file = fs::File::open(&path).map_err(|_| "Could not open pairing identity")?;
                let mut sealed = Vec::new();
                file.take(MAX_IDENTITY_BYTES + 1).read_to_end(&mut sealed).map_err(|_| "Could not read pairing identity")?;
                if sealed.len() as u64 > MAX_IDENTITY_BYTES { return Err("Pairing identity too large".into()); }
                let bytes = unprotect(&sealed)?;
                let saved: SavedIdentity = serde_json::from_slice(&bytes).map_err(|_| "Pairing identity is corrupt; explicit reset required")?;
                saved.identity().map(Arc::new)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let saved = SavedIdentity::generate()?;
                let identity = saved.identity()?;
                save(dir, &saved, false)?;
                Ok(Arc::new(identity))
            }
            Err(_) => Err("Could not inspect pairing identity; refusing to regenerate it".into()),
        }
    }

    pub fn pairing_code(&self) -> String {
        format!("ymic1:{}:{}", hex(&Sha256::digest(self.certificate.as_ref())), hex(&self.token))
    }

    pub(crate) fn accepts(&self, candidate: &[u8; 32]) -> bool { bool::from(self.token.ct_eq(candidate)) }

    /// Called only after the TLS handshake has authenticated our pinned key to
    /// the phone. Bounds/timeouts are applied by the listener before this call.
    pub(crate) async fn authorize<S: AsyncRead + AsyncWrite + Unpin>(&self, socket: &mut S) -> std::io::Result<()> {
        let mut magic = [0; 8]; let mut token = [0; 32];
        socket.read_exact(&mut magic).await?;
        socket.read_exact(&mut token).await?;
        if &magic != AUTH_MAGIC || !self.accepts(&token) {
            return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Pairing authentication failed"));
        }
        socket.write_all(AUTH_OK).await?;
        socket.flush().await
    }
}

fn lease_at(dir: &Path, exclusive: bool) -> Result<fs::File, String> {
    fs::create_dir_all(dir).map_err(|_| "Could not create pairing directory")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).map_err(|_| "Could not protect pairing directory")?;
    }
    let mut options = OpenOptions::new(); options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let file = options.open(dir.join("session.lock")).map_err(|_| "Could not open pairing lifecycle lock")?;
    let result = if exclusive { file.try_lock() } else { file.try_lock_shared() };
    result.map_err(|_| "Stop all Wi-Fi servers before renewing pairing; another process is using the identity")?;
    // The OS releases this lock on File drop or process exit; stale PID files
    // cannot block recovery and there is no check-then-start race.
    Ok(file)
}

fn save(dir: &Path, saved: &SavedIdentity, replace: bool) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|_| "Could not create pairing directory")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).map_err(|_| "Could not protect pairing directory")?;
    }
    let bytes = serde_json::to_vec(saved).map_err(|_| "Could not serialize pairing identity")?;
    let sealed = protect(&bytes)?;
    if sealed.len() as u64 > MAX_IDENTITY_BYTES { return Err("Pairing identity too large".into()); }
    let mut nonce = [0; 16]; getrandom::fill(&mut nonce).map_err(|_| "Secure random generator unavailable")?;
    let temp = dir.join(format!(".{}.tmp", hex(&nonce)));
    let mut options = OpenOptions::new(); options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let result = (|| {
        let mut file = options.open(&temp).map_err(|_| "Could not create private pairing file")?;
        file.write_all(&sealed).and_then(|_| file.sync_all()).map_err(|_| "Could not persist pairing identity")?;
        drop(file);
        let target = dir.join("identity.bin");
        if replace { fs::rename(&temp, target) } else { fs::hard_link(&temp, target) }
            .map_err(|_| "Could not atomically publish pairing identity; retry without resetting".to_string())
    })();
    // Only remove the exact temporary file created by this invocation.
    let _ = fs::remove_file(temp);
    result
}

#[cfg(windows)]
fn protect(bytes: &[u8]) -> Result<Vec<u8>, String> { dpapi(bytes, true) }
#[cfg(windows)]
fn unprotect(bytes: &[u8]) -> Result<Vec<u8>, String> { dpapi(bytes, false) }

#[cfg(windows)]
fn dpapi(bytes: &[u8], seal: bool) -> Result<Vec<u8>, String> {
    use std::ptr::null_mut;
    use winapi::um::dpapi::{CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN};
    use winapi::um::winbase::LocalFree;
    use winapi::um::wincrypt::DATA_BLOB;
    if bytes.len() as u64 > MAX_IDENTITY_BYTES { return Err("Pairing identity too large".into()); }
    let mut input = DATA_BLOB { cbData: bytes.len() as u32, pbData: bytes.as_ptr() as *mut u8 };
    let mut output: DATA_BLOB = unsafe { std::mem::zeroed() };
    // DPAPI CurrentUser binds the ciphertext to this Windows user's credentials.
    // Input is borrowed for the call; the returned allocation belongs to LocalFree.
    let ok = unsafe {
        if seal { CryptProtectData(&mut input, null_mut(), null_mut(), null_mut(), null_mut(), CRYPTPROTECT_UI_FORBIDDEN, &mut output) }
        else { CryptUnprotectData(&mut input, null_mut(), null_mut(), null_mut(), null_mut(), CRYPTPROTECT_UI_FORBIDDEN, &mut output) }
    };
    if ok == 0 { return Err("Windows could not protect/unprotect pairing identity; explicit reset may be required".into()); }
    let result = if output.pbData.is_null() || output.cbData == 0 || output.cbData as u64 > MAX_IDENTITY_BYTES {
        Err("Invalid protected pairing identity".into())
    } else {
        // Valid only until LocalFree, so copy before releasing the OS allocation.
        Ok(unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec())
    };
    unsafe { LocalFree(output.pbData as *mut _); }
    result
}

#[cfg(not(windows))]
fn protect(bytes: &[u8]) -> Result<Vec<u8>, String> { Ok(bytes.to_vec()) }
#[cfg(not(windows))]
fn unprotect(bytes: &[u8]) -> Result<Vec<u8>, String> { Ok(bytes.to_vec()) }

#[cfg(test)]
mod tests {
    use super::*;
    struct Root(std::path::PathBuf);
    impl Root {
        fn new() -> Self {
            let mut nonce = [0;16]; getrandom::fill(&mut nonce).unwrap();
            let dir = std::env::temp_dir().join(format!("ymic-pairing-test-{}", hex(&nonce)));
            fs::create_dir(&dir).unwrap(); Self(dir)
        }
    }
    impl Drop for Root { fn drop(&mut self) { assert_eq!(self.0.parent(), Some(std::env::temp_dir().as_path())); let _ = fs::remove_dir_all(&self.0); } }

    #[test]
    fn persistent_identity_keeps_pin_and_credential_across_restarts() {
        let root = Root::new(); let first = Identity::load_at(&root.0).unwrap();
        let second = Identity::load_at(&root.0).unwrap();
        assert_eq!(first.pairing_code(), second.pairing_code());
        assert!(first.accepts(&second.token));
        assert_eq!(first.pairing_code().len(), 135);
        #[cfg(windows)]
        assert!(!fs::read(root.0.join("identity.bin")).unwrap().windows(16).any(|w| w == b"PRIVATE KEY-----"));
    }

    #[test]
    fn corrupt_or_oversized_identity_is_not_silently_replaced() {
        let root = Root::new(); Identity::load_at(&root.0).unwrap();
        let path = root.0.join("identity.bin");
        fs::write(&path, b"corrupt").unwrap(); assert!(Identity::load_at(&root.0).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"corrupt");
        fs::write(&path, vec![0; MAX_IDENTITY_BYTES as usize + 1]).unwrap();
        assert!(Identity::load_at(&root.0).is_err());
    }

    #[test]
    fn replacing_identity_invalidates_the_previous_credential_and_pin() {
        let root = Root::new(); let before = Identity::load_at(&root.0).unwrap();
        let saved = SavedIdentity::generate().unwrap(); save(&root.0, &saved, true).unwrap();
        let after = Identity::load_at(&root.0).unwrap();
        assert!(!after.accepts(&before.token)); assert_ne!(before.pairing_code(), after.pairing_code());
    }

    #[test]
    fn active_server_lease_prevents_rotation_and_is_released_on_drop() {
        let root = Root::new(); let before = Identity::load_at(&root.0).unwrap();
        let session = lease_at(&root.0, false).unwrap();
        assert!(Identity::rotate_at(&root.0).is_err());
        assert_eq!(before.pairing_code(), Identity::load_at(&root.0).unwrap().pairing_code());
        drop(session);
        let after = Identity::rotate_at(&root.0).unwrap();
        assert_ne!(before.pairing_code(), after);
        let exclusive = lease_at(&root.0, true).unwrap();
        assert!(lease_at(&root.0, false).is_err());
        drop(exclusive);
        assert!(lease_at(&root.0, false).is_ok());
    }

    #[tokio::test]
    async fn wrong_token_is_denied_before_an_acknowledgement() {
        let identity = SavedIdentity::generate().unwrap().identity().unwrap();
        let (mut client, mut server) = tokio::io::duplex(128);
        client.write_all(AUTH_MAGIC).await.unwrap(); client.write_all(&[0;32]).await.unwrap();
        assert!(identity.authorize(&mut server).await.is_err());
        drop(server); let mut ack=[0;4]; assert!(client.read_exact(&mut ack).await.is_err());
    }
}
