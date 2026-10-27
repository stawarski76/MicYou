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

/// Ask Windows (UAC prompt) to allow inbound TCP/UDP for this executable.
/// Other platforms have no per-app firewall prompt to satisfy.
#[cfg(windows)]
pub fn allow_inbound() -> Result<(), String> {
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = exe_path.to_string_lossy();
    let script = format!(
        "{}\n$ErrorActionPreference = 'Stop'\nSet-MicYouFirewallRules -ProgramPath '{}'",
        include_str!("firewall_rules.ps1"),
        exe.replace('\'', "''"),
    );
    // Encode inside PowerShell from a single-quoted literal: neither the
    // program path nor the nested script is ever evaluated by cmd.exe or
    // interpolated as a second layer of PowerShell code.
    let elevation = format!(
        "$encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes('{}')); $child = Start-Process -FilePath (Join-Path $PSHOME 'powershell.exe') -ArgumentList @('-NoProfile','-NonInteractive','-EncodedCommand',$encoded) -Verb RunAs -WindowStyle Hidden -Wait -PassThru; exit $child.ExitCode",
        script.replace('\'', "''"),
    );
    let powershell = std::path::PathBuf::from(
        std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()),
    ).join("System32\\WindowsPowerShell\\v1.0\\powershell.exe");
    let status = std::process::Command::new(powershell)
        .args([
            "-NoProfile", "-NonInteractive", "-Command", &elevation,
        ])
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("Failed to execute firewall rule addition".to_string());
    }
    log::info!(target: "system", "Requested firewall permission on Windows");
    Ok(())
}

#[cfg(not(windows))]
pub fn allow_inbound() -> Result<(), String> {
    Ok(())
}
