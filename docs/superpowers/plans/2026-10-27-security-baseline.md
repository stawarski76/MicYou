# MicYou security baseline implementation plan

> **For agentic workers:** Use superpowers:executing-plans to implement this plan task by task. Track verification and limitations in docs/security/hardening-progress.md.

**Goal:** Publish a reviewable first security patch in stawarski76/MicYou, without claiming the remaining network or release risks are solved.

**Architecture:** Keep the fork's current master (692ecbb9cee1ac45da590cadb59b645f23c1ea65) and preserve phone/audio compatibility. Quarantine third-party plugin execution and installation at backend boundaries while replacing remote Markdown rendering with plain text. Restrict Windows inbound rules to Private/LocalSubnet. No custom cryptography or protocol change in this patch.

**Tech stack:** Rust, Vue 3, Bun/Node test runner, Inno Setup, GitHub Actions.

**Spec:** docs/security/2026-10-27-micyou-audit.md (audit baseline v2.1.1; relevant vulnerable paths checked again on fork master).

## Global constraints

- Work on codex/security-hardening; do not merge or publish release binaries.
- Keep GPL notices and upstream history. Communicate with this user in Polish.
- Plugins remain unavailable until artifact-bound approval and sandbox fixes are designed and verified. No environment-variable bypass.
- Audio transmission remains compatible and still lacks authenticated encryption; document the unresolved blocker prominently.
- Do not execute unreviewed repository install hooks or auto-download/install VB-CABLE.

## Review focus

- Previously enabled plugins must not execute at startup.
- Native/WASM imports, market downloads and plugin updates must fail before file/network side effects.
- Remote README markup must display as text, including the demonstrated onerror payload.
- UI must clearly communicate quarantine and must not start the market in the background.
- Windows Public-profile traffic and peers outside LocalSubnet must not be allowed by generated firewall rules.

## Tasks

### 1. Quarantine external plugins

Files: crates/micyou-core/src/plugins/{mod.rs,install.rs}, src/features/plugins/components/PluginsPanel.vue, src/shared/locales/*.json, optionally a shared policy module and its command mapping.

- [ ] Add regression tests exercising actual import/preview/enable entry points with temporary paths and a local HTTP listener; observe rejection is missing before implementation.
- [ ] Deny plugin execution, startup loading, imports and update/download operations before effects. Keep disable/uninstall available.
- [ ] Make the plugin settings page render a localized quarantine notice, without mounting market/import controls.
- [ ] Verify relevant Rust tests where toolchain permits; document exact unavailable gates.

### 2. Remove remote README HTML rendering

Files: src/features/plugins/components/{PluginMarketDialog.vue,PluginReadme.vue}, tests/security/*.test.ts.

- [ ] Write a Vue render regression for malicious Markdown/raw HTML; observe unsafe HTML in current rendering.
- [ ] Render README as plain text through a dedicated small component; remove custom regex Markdown/sanitizer pipeline.
- [ ] Check the raw README is passed to the safe component; run frontend type check and regression tests.

### 3. Restrict firewall exposure

Files: src-tauri/installer.iss, crates/micyou-core/src/platform/firewall.rs.

- [ ] Restrict installer rules to profile=private remoteip=LocalSubnet and explicit TCP/UDP protocols.
- [ ] Replace shell command interpolation in runtime firewall helper with a parameterized PowerShell script; add regression cases for shell metacharacters if extracting a testable builder is needed.
- [ ] Preserve interactive UAC; do not execute firewall changes during tests.

### 4. Verification and publication

Files: .github/workflows/security-baseline.yml, docs/security/hardening-progress.md.

- [ ] Add a read-only-permission CI workflow for regression tests, frontend type check, Rust tests and clippy on Windows. Never consume secrets or trigger releases.
- [ ] Run feasible local gates, request independent final code review and address substantive findings.
- [ ] Commit and publish the branch through the GitHub connector; open and attach a draft PR to the user's fork.
- [ ] Record unfinished SEC-04/05/06/08/09, dependency upgrades and release verification as next security work, not completed fixes.
