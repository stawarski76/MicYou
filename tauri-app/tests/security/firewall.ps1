$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot '../../crates/micyou-core/src/platform/firewall_rules.ps1')

# Exercise real rule generation only; never install or remove firewall rules.
$paths = @('C:\Program Files\MicYou\micyou.exe', 'C:\Users\O''Brien\$(& echo nope) & MicYou\micyou.exe')
foreach ($program in $paths) {
    $rules = @(Get-MicYouFirewallRules -ProgramPath $program)
    if ($rules.Count -ne 2) { throw 'Expected separate TCP and UDP rules' }
    foreach ($rule in $rules) {
        if ($rule.Program -cne $program) { throw 'Program path changed or was interpreted as code' }
        if ($rule.Profile -cne 'Private') { throw 'Public/Domain profile must not be allowed' }
        if ($rule.RemoteAddress -cne 'LocalSubnet') { throw 'Peers outside the local subnet must not be allowed' }
        if ($rule.Direction -cne 'Inbound' -or $rule.Action -cne 'Allow') { throw 'Incorrect rule direction or action' }
    }
    if ($rules[0].Protocol -cne 'TCP' -or $rules[1].Protocol -cne 'UDP') { throw 'Incorrect transport rules' }
}
Write-Output 'Firewall rule generation passed; no firewall changes performed.'
