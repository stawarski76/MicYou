# Driver installation quarantine

Continuation of the authorized security audit and hardening. Keep PR #1 as a draft; no merge or release.

Decision: remove the unauthenticated ZIP-to-elevated-EXE automation entirely, instead of adding a partial signature check that leaves replacement/race risks. Manual installation remains available through the verified vendor landing page https://vb-audio.com/Cable/. This mitigates SEC-08 by removing the vulnerable automation; it is not a verified automatic installer.

1. Diagnose the failing Windows Rust gate. Its exact-case fixture overwrites a differently cased entry on case-insensitive filesystems. Remove that fixture before creating the exact-name fixture, preserving both assertions.
2. Keep the public VB-CABLE install boundary compatible but return `manual_install_required` immediately, with no download, extraction, temp cleanup, process launch or device configuration.
3. Replace automatic install controls in onboarding/settings with a shared localized manual-download/recheck component. Keep existing device detection and refresh paths; remove unused install state/listeners.
4. Add a backend regression rejecting installation before progress events and a Vue render test for manual-download/recheck controls. Run feasible tests/typecheck; verify Rust and full builds in GitHub Actions because local Rust installation was declined.
5. Publish to the same draft PR, document current CI evidence and unresolved pairing/encryption/Android/updater/dependency issues. Do not claim full security or successful manual device installation.
