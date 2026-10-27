use super::*;
use crate::host::{HostIntegration, HotkeyCallback};
use crate::server::{lifecycle::ServerLifecycleState, output::AudioOutputHandle};
use crate::stats::NetworkStats;
use crate::transport::udp::ActiveAudioSession;
use micyou_plugin::{PluginManager, PluginManifest, PluginState, HOST_API_VERSION};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

struct TestHost;
impl HostIntegration for TestHost {
    fn open_url(&self, _: &str) -> Result<(), String> { panic!("unexpected URL opening") }
    fn notify(&self, _: &str, _: &str) -> Result<(), String> { panic!("unexpected notification") }
    fn register_hotkey(&self, _: &str, _: HotkeyCallback) -> Result<(), String> { panic!("unexpected hotkey") }
    fn open_plugin_panel(&self, _: &str, _: &str, _: &str) -> Result<(), String> { panic!("unexpected panel") }
}

struct TestRoot(PathBuf);
impl TestRoot {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!("micyou-quarantine-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for TestRoot {
    fn drop(&mut self) {
        // Only remove the unique directory this test created directly in TEMP.
        assert_eq!(self.0.parent(), Some(std::env::temp_dir().as_path()));
        assert!(self.0.file_name().unwrap().to_string_lossy().starts_with("micyou-quarantine-"));
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn host(root: &Path) -> Arc<PluginHost> {
    let stats = Arc::new(NetworkStats::default());
    let host = PluginHost::new(ServerHandles {
        audio_output: AudioOutputHandle::spawn_with_mute_flag(stats.mute_flag()),
        network_stats: stats,
        active_connection: Arc::new(tokio::sync::Mutex::new(None)),
        active_audio_session: Arc::new(RwLock::new(ActiveAudioSession::Inactive)),
        lifecycle: Arc::new(tokio::sync::Mutex::new(ServerLifecycleState::default())),
        #[cfg(feature = "web-server")]
        web_server: Arc::new(tokio::sync::Mutex::new(None)),
    }, Arc::new(TestHost));
    *host.manager.lock().unwrap() = PluginManager::new(root.join("plugins"), root.join("state.json"));
    host
}

fn fixture(root: &Path, runtime: RuntimeKind) -> PathBuf {
    let dir = root.join("dev.test.plugin");
    std::fs::create_dir_all(&dir).unwrap();
    let manifest = PluginManifest {
        id: "dev.test.plugin".into(), name: "Test".into(), version: "1.0.0".into(),
        runtime, entry: "payload".into(), api_version: HOST_API_VERSION,
        ..PluginManifest::default()
    };
    std::fs::write(dir.join("plugin.json"), serde_json::to_vec(&manifest).unwrap()).unwrap();
    // Never provide executable code, even if a regression attempts a load.
    std::fs::write(dir.join("payload"), b"not executable").unwrap();
    dir
}

#[test]
fn quarantine_rejects_native_and_wasm_enable_before_loading_artifacts() {
    for runtime in [RuntimeKind::Native, RuntimeKind::Wasm] {
        let root = TestRoot::new();
        let host = host(&root.0);
        let dir = fixture(&root.0, runtime);
        host.manager.lock().unwrap().discover_plugin(dir).unwrap();
        assert!(matches!(host.enable_plugin("dev.test.plugin"), Err(PluginError::PermissionDenied(_))));
        assert!(host.manager.lock().unwrap().loaded_ids().is_empty());
    }
}

#[test]
fn quarantine_rejects_import_before_copying_plugin_files() {
    let root = TestRoot::new();
    let host = host(&root.0);
    let source = fixture(&root.0.join("source"), RuntimeKind::Wasm);
    assert!(host.import(&source).unwrap_err().contains("security quarantine"));
    assert!(!root.0.join("plugins").exists());
}

#[test]
fn quarantine_rejects_market_and_preview_before_network_access() {
    let root = TestRoot::new();
    let host = host(&root.0);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/plugin.zip", listener.local_addr().unwrap());
    assert!(host.install_from_url("dev.test.plugin", &url).unwrap_err().contains("security quarantine"));
    assert!(install::preview_url(&url).err().unwrap().contains("security quarantine"));
    assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    assert!(!root.0.join("plugins").exists());
}

#[test]
fn quarantine_rejects_archive_preview_before_reading_an_archive() {
    assert!(install::preview_zip(Path::new("nonexistent.zip")).err().unwrap().contains("security quarantine"));
}

#[test]
fn quarantine_disables_saved_plugins_but_retains_uninstall() {
    let root = TestRoot::new();
    let host = host(&root.0);
    let dir = fixture(&root.0.join("plugins"), RuntimeKind::Native);
    {
        let mut manager = host.manager.lock().unwrap();
        manager.discover_plugin(dir.clone()).unwrap();
        manager.set_enabled("dev.test.plugin", true).unwrap();
    }
    host.load_saved_plugins();
    {
        let manager = host.manager.lock().unwrap();
        assert_eq!(manager.entry("dev.test.plugin").unwrap().unwrap().state, PluginState::Disabled);
        assert!(manager.loaded_ids().is_empty());
    }
    assert!(host.update("dev.test.plugin").unwrap_err().contains("security quarantine"));
    host.uninstall("dev.test.plugin").unwrap();
    assert!(!dir.exists());
}
