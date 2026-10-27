# Pure rule descriptors, shared by the runtime helper and regression tests.
# Merely loading this file never changes the Windows firewall.
function Get-MicYouFirewallRules {
    param([Parameter(Mandatory = $true)][string]$ProgramPath)
    $ruleName = switch ([System.IO.Path]::GetFileName($ProgramPath).ToLowerInvariant()) {
        'micyou-cli.exe' { 'MicYou CLI' }
        'micyou-tui.exe' { 'MicYou TUI' }
        default { 'MicYou App' }
    }
    foreach ($protocol in @('TCP', 'UDP')) {
        @{
            DisplayName = "$ruleName ($protocol-In)"
            Program = $ProgramPath
            Direction = 'Inbound'
            Action = 'Allow'
            Enabled = 'True'
            Profile = 'Private'
            RemoteAddress = 'LocalSubnet'
            Protocol = $protocol
        }
    }
}

function Set-MicYouFirewallRules {
    param([Parameter(Mandatory = $true)][string]$ProgramPath)
    $ErrorActionPreference = 'Stop'
    $folder = Split-Path -Path $ProgramPath -Parent
    $programs = @('micyou.exe', 'micyou-cli.exe', 'micyou-tui.exe') | ForEach-Object { Join-Path $folder $_ }
    # Migrate only this installation's old unrestricted rules, not a different
    # installation which happens to use the same display names.
    $legacyNames = @('MicYou', 'MicYou CLI', 'MicYou TUI')
    $migratePrograms = @($ProgramPath)
    foreach ($legacy in @(Get-NetFirewallRule -DisplayName $legacyNames -ErrorAction SilentlyContinue)) {
        $filter = $legacy | Get-NetFirewallApplicationFilter -ErrorAction Stop
        if ($programs -contains $filter.Program) {
            $migratePrograms += $filter.Program
            $legacy | Remove-NetFirewallRule -ErrorAction Stop
        }
    }
    foreach ($program in @($migratePrograms | Select-Object -Unique)) {
        foreach ($rule in @(Get-MicYouFirewallRules -ProgramPath $program)) {
            Get-NetFirewallRule -DisplayName $rule.DisplayName -ErrorAction SilentlyContinue | Remove-NetFirewallRule -ErrorAction Stop
            New-NetFirewallRule @rule -ErrorAction Stop | Out-Null
        }
    }
}
