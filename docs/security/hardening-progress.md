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

## Follow-up: CI and driver quarantine

GitHub Actions for commit f90ed40950f9392a2f1cd65696797e9ee8579aa9 confirmed Windows/Linux desktop builds and the Android debug build passed. Frontend security checks passed all 6 tests, including the PowerShell descriptor behavior test that could not run locally, and type checking passed. macOS jobs were cancelled; they are not passing evidence.

The backend test job compiled and passed the added plugin quarantine tests, then failed an existing terminal exact-case filename test (153 core tests passed, 1 failed). On Windows, writing a differently cased filename overwrites the same directory entry without changing its stored casing. The fixture now removes the wrong-case entry before creating the exact-name entry; production lookup behavior is unchanged. The test correction was published separately for an immediate CI rerun.

SEC-08 mitigation: automatic VB-CABLE download/extraction/elevated installation has been removed. The backend returns `manual_install_required` without performing installation work. Onboarding and settings offer manual download from https://vb-audio.com/Cable/ and rechecking detection. Existing virtual microphone use remains available. No driver was installed during this work. Restoring automation requires artifact authentication, bounded extraction and protection against replacement before elevation.

Follow-up plan: docs/superpowers/plans/2026-10-27-driver-quarantine.md. Current follow-up Rust and full-build results must be checked on the final PR head; successful earlier builds do not verify subsequent changes.

The test correction on commit 24be90bc983f31f8d1d84433d79a228f8d086123 passed all backend tests (including the exact-case regression). Clippy then exposed an existing denied `approx_constant` lint in micyou-infer: the LOG2_E numeric literal is replaced with Rust's standard f32 constant; no lint is suppressed.

The follow-up adds a real Vue component/event lifecycle regression: Windows welcome -> manual setup -> vendor URL -> recheck -> completion, with unexpected backend commands rejected by the test double. It failed because the welcome screen had no Next action. The Next control now makes the driver step reachable. CLI status text also directs users to the manual vendor download.

Follow-up local gates: 7 Bun tests passed, 1 PowerShell behavior test skipped under the existing host policy, 0 failures; Vue type checking and git diff --check passed. The independent static review found no remaining concrete blockers in this patch. New driver backend tests and the Clippy correction still need the final-head CI run. No local Rust installation or firewall/driver changes were performed.
