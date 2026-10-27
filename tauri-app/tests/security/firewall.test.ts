import { expect, test } from 'bun:test';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';

const powershell = resolve(process.env.SystemRoot ?? 'C:/Windows', 'System32/WindowsPowerShell/v1.0/powershell.exe');
const policy = process.platform === 'win32'
  ? spawnSync(powershell, ['-NoProfile', '-NonInteractive', '-Command', 'Get-ExecutionPolicy'], { encoding: 'utf8' }).stdout?.trim()
  : undefined;
// Respect host policy; hosted Windows CI runs these behavior tests with its
// normal script policy. Do not change ExecutionPolicy or use Bypass here.
const scriptsBlocked = !['RemoteSigned', 'Unrestricted', 'Bypass'].includes(policy ?? '');

test.skipIf(process.platform !== 'win32' || (scriptsBlocked && !process.env.CI))('firewall rules exclude public networks and preserve program paths as data (requires scripts permitted by host policy)', () => {
  const result = spawnSync(powershell, ['-NoProfile', '-NonInteractive', '-File', resolve('tests/security/firewall.ps1')], { encoding: 'utf8' });
  expect(result.error).toBeUndefined();
  expect(result.status, `Host policy: ${JSON.stringify(policy)}; stderr: ${result.stderr}`).toBe(0);
  expect(result.stdout).toContain('no firewall changes performed');
});

test.skipIf(process.platform !== 'win32')('PowerShell firewall descriptors have valid syntax without executing them', () => {
  const path = resolve('crates/micyou-core/src/platform/firewall_rules.ps1').replaceAll("'", "''");
  const command = `$tokens=$null; $errors=$null; [System.Management.Automation.Language.Parser]::ParseFile('${path}',[ref]$tokens,[ref]$errors) | Out-Null; if($errors.Count) { $errors | Out-String | Write-Error; exit 1 }; Write-Output 'syntax valid'`;
  const result = spawnSync(powershell, ['-NoProfile', '-NonInteractive', '-Command', command], { encoding: 'utf8' });
  expect(result.status).toBe(0);
  expect(result.stdout).toContain('syntax valid');
});
