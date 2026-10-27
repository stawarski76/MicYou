# Security hardening progress

Plan: docs/superpowers/plans/2026-10-27-security-baseline.md

Base: fork master 692ecbb9cee1ac45da590cadb59b645f23c1ea65. The initial audit was on v2.1.1; relevant plugin and installer paths remain vulnerable on this base.

Ruling: use the existing dedicated task checkout and a new branch, preserving the untracked audit artifacts. No additional worktree is needed for this single implementation.

Ruling: quarantine all external plugins in the first patch rather than ship an incomplete permission redesign. Restoring plugins requires independently stored artifact-bound approvals and a separate writable data root.

Status: first patch implemented for a draft PR. No claim of overall security or production readiness.

Implemented:

- External plugin enable/import/preview/download/update operations fail at backend entry points before effects. Startup disables saved enabled flags. CLI install, enable and dev use the same guard. Disable/uninstall remain available.
- The settings panel mounts only a quarantine notice. Remote README content uses Vue text escaping; the custom regex Markdown/HTML renderer is removed.
- Installer and runtime firewall rules allow TCP/UDP only on Private networks from LocalSubnet. Runtime migration removes legacy broad rules matching binaries in the same installation folder. PowerShell invocation escapes literal paths and preserves interactive elevation.
- English is the first installer language; this is not a Polish translation.
- A read-only, SHA-pinned Windows security workflow runs frontend checks and backend Rust tests/clippy. The external AI workflow is restricted to the upstream repository.

Local verification (2026-10-27):

- The README exploit regression failed against the previous renderer (3 failures), then passed with literal rendering.
- Bun security suite: 5 passed, 1 skipped, 0 failed. The skip is the PowerShell behavior test because the host disallows script files; CI requires this test instead of silently skipping it. PowerShell AST syntax validation passed without executing firewall changes.
- Vue type checking passed. No dependency install hooks were executed.
- Rust tests and clippy could not run: cargo is absent and the user declined toolchain installation. Added Rust tests are unverified until CI passes.
- The full frontend build cannot finish because license generation requires cargo. GUI/installer packaging, elevation/path quoting at runtime, firewall migration behavior and real audio tests remain unverified.
- Independent code review identified CLI quarantine bypasses and old broad firewall rules retained by the runtime helper. Both paths were corrected before publication.

Do not merge or release until CI and required Windows/device validation are complete. Quarantine deliberately removes third-party plugin functionality. It does not fix network authentication/encryption or validate existing release binaries.

Outstanding: authenticated pairing/encryption for TCP/UDP/WebSocket; verified driver downloads; Android quick-start and updater hardening; dependency upgrades; release binary/signature checks; real phone and Windows runtime testing.
