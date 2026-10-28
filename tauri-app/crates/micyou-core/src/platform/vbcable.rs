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

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct VBCableResult {
    pub success: bool,
    pub error_type: Option<String>,
    pub message: Option<String>,
}

/// Check if VB-CABLE is installed by scanning audio devices, registry, or driver files
#[cfg(target_os = "windows")]
pub fn is_installed() -> bool {
    use cpal::traits::{DeviceTrait, HostTrait};

    let is_vbcable_device = |name: &str| -> bool {
        let lower = name.to_lowercase();
        lower.contains("cable input")
            || lower.contains("cable output")
            || lower.contains("vb-audio")
            || (lower.contains("cable") && (lower.contains("virtual") || lower.contains("audio")))
    };

    // 1. Device check: check CPAL output and input devices
    let host = cpal::default_host();
    if let Ok(devices) = host.output_devices() {
        for dev in devices {
            if let Some(name) = micyou_audio::device_name(&dev) {
                if is_vbcable_device(&name) {
                    return true;
                }
            }
        }
    }
    if let Ok(devices) = host.input_devices() {
        for dev in devices {
            if let Some(name) = micyou_audio::device_name(&dev) {
                if is_vbcable_device(&name) {
                    return true;
                }
            }
        }
    }

    // 2. Registry verification: check services and software keys (64-bit and 32-bit views)
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};
    use winreg::RegKey;

    let registry_paths = [
        // Windows Services (service names created by official driver)
        "SYSTEM\\CurrentControlSet\\Services\\VBCABLE",
        "SYSTEM\\CurrentControlSet\\Services\\VBCABLEA",
        "SYSTEM\\CurrentControlSet\\Services\\VBCABLEB",
        "SYSTEM\\CurrentControlSet\\Services\\VB-Cable",
        // Software keys
        "SOFTWARE\\VB-Audio\\Cable",
        "SOFTWARE\\VB-Audio\\VB-Cable",
        "SOFTWARE\\WOW6432Node\\VB-Audio\\Cable",
        "SOFTWARE\\WOW6432Node\\VB-Audio\\VB-Cable",
    ];

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    for path in &registry_paths {
        if hklm.open_subkey_with_flags(path, KEY_READ).is_ok() {
            return true;
        }
    }

    // 3. File system verification: check driver and utility files
    let file_paths = [
        r"C:\Program Files\VB\CABLE\vbcable_control_panel.exe",
        r"C:\Program Files (x86)\VB\CABLE\vbcable_control_panel.exe",
        r"C:\Windows\System32\drivers\vbcable.sys",
        r"C:\Windows\System32\drivers\vbcable_win7_x64.sys",
    ];
    for path in &file_paths {
        if std::path::Path::new(path).exists() {
            return true;
        }
    }

    false
}

#[cfg(not(target_os = "windows"))]
pub fn is_installed() -> bool {
    false
}

/// Automatic installation is quarantined until downloaded artifacts can be
/// authenticated and protected from replacement before elevation. Keep this
/// boundary for GUI/CLI callers, without any network, filesystem or UAC work.
#[cfg(feature = "vbcable")]
pub async fn install(_events: crate::events::SharedEvents) -> VBCableResult {
    VBCableResult {
        success: false,
        error_type: Some("manual_install_required".to_string()),
        message: Some("Automatic VB-CABLE installation is disabled. Install manually from https://vb-audio.com/Cable/ and restart MicYou.".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_installed_runs_without_panic() {
        let _ = is_installed();
    }

    #[cfg(feature = "vbcable")]
    struct NoInstallationEvents;

    #[cfg(feature = "vbcable")]
    impl crate::events::ServerEvents for NoInstallationEvents {
        fn device_connected(&self, _: crate::transport::tcp::DeviceInfo) { panic!("unexpected event") }
        fn device_disconnected(&self) { panic!("unexpected event") }
        fn audio_metrics(&self, _: crate::stats::AudioMetrics) { panic!("unexpected event") }
        fn udp_audio_warning(&self) { panic!("unexpected event") }
        fn mute_state_changed(&self, _: bool) { panic!("unexpected event") }
        fn audio_level(&self, _: u32) { panic!("unexpected event") }
        fn audio_spectrum(&self, _: crate::events::SpectrumPayload) { panic!("unexpected event") }
        fn server_stopped(&self) { panic!("unexpected event") }
        fn web_client_count(&self, _: u32) { panic!("unexpected event") }
        fn install_progress(&self, _: String) { panic!("quarantine must not start installation") }
        fn aec_status_changed(&self, _: crate::events::AecStatus) { panic!("unexpected event") }
        fn monitoring_state_changed(&self, _: bool) { panic!("unexpected event") }
        fn plugin_download_progress(&self, _: crate::events::DownloadProgress) { panic!("unexpected event") }
    }

    #[cfg(feature = "vbcable")]
    #[tokio::test]
    async fn automatic_installation_returns_manual_required_without_starting_work() {
        let events = std::sync::Arc::new(NoInstallationEvents);
        let result = tokio::time::timeout(std::time::Duration::from_secs(1), install(events))
            .await
            .expect("quarantine must return immediately");
        assert!(!result.success);
        assert_eq!(result.error_type.as_deref(), Some("manual_install_required"));
        assert!(result.message.unwrap().contains("https://vb-audio.com/Cable/"));
    }
}
