---
name: zero-mock-audit
description: >-
  Use this skill when auditing or verifying any crate, module, UI component, or mobile screen
  to guarantee zero mock implementations, zero simulated outputs, zero hardcoded metrics,
  and full end-to-end integration across Rust, Tauri IPC, and Flutter FFI.
---

# Zero-Mock & Production Readiness Audit Skill

This skill enforces the **Zero-Mock Engineering Contract** defined in `AGENTS.md`. Run this procedure before marking any phase or stage complete.

## Step 1: Automated Static Scan for Stubs, Mocks & Placeholders

Run the following PowerShell audit commands from the workspace root (`H:\rustapps`):

```powershell
# 1. Scan non-test Rust source code for mock/stub/simulated/todo markers
Get-ChildItem -Path platform, shared, apps -Include *.rs -Recurse |
  Select-String -Pattern "todo!|unimplemented!|InMemoryKeyring|MockBiometric|simulated|placeholder|FIXME|HACK" |
  Where-Object { $_.Line -notmatch "#\[cfg\(test\)\]" }

# 2. Scan for hardcoded fake metrics or dummy returns
Get-ChildItem -Path platform, shared, apps -Include *.rs -Recurse |
  Select-String -Pattern "64 \* 1024 \* 1024|cpu_percentage = 0\.5"

# 3. Verify embedded desktop GUI host & IPC dispatch in all desktop app entrypoints
Get-ChildItem -Path apps/*/src/main.rs |
  Select-String -Pattern "run_desktop_app|EmbeddedUiBundle|dispatch_ipc_command"
```

## Step 2: Subsystem Authenticity Verification Checklist

Verify each subsystem against these concrete requirements:
1. **OS Keyring & Biometrics (`platform/src/keyring.rs`):**
   - Windows: Calls Win32 `CredReadW` / `CredWriteW` / `CredDeleteW` and `Windows.Security.Credentials.UI.UserConsentVerifier`.
   - macOS: Calls Security.framework (`SecItemAdd`, `SecItemCopyMatching`) and `LocalAuthentication` (`LAContext`).
   - Linux: Connects to `org.freedesktop.secrets` via D-Bus or `libsecret` and `org.freedesktop.PolicyKit1`.
   - `InMemoryKeyring` is strictly restricted to `#[cfg(test)]`.
2. **Process Resource Telemetry (`platform/src/lib.rs`):**
   - Queries actual OS process metrics (`GetProcessMemoryInfo` / `GetProcessTimes` on Windows, `/proc/self/statm` & `/proc/self/stat` on Linux, `proc_pid_rusage` / `mach_task_basic_info` on macOS).
3. **System Clipboard (`apps/devops-secret-vault/src/clipboard.rs`):**
   - Reads and writes the real OS clipboard buffer and overwrites/clears the OS clipboard when the TTL expires.
4. **Image Decoding & OCR (`apps/ai-file-explorer/src/ocr.rs`):**
   - Decodes PNG (IHDR + IDAT zlib + row filters), JPEG (DCT + Huffman), WebP, and BMP into true luminance pixels before running `compute_dhash` or OCR inference.
   - Uses real ONNX / pure-Rust neural or template recognition models rather than synthetic character loops.
5. **Desktop Loopback IPC & Flutter FFI Wiring:**
   - Every desktop binary `apps/*/src/main.rs` embeds `../ui/dist/{index.html,assets/app.js,assets/app.css}` via `platform_common::run_desktop_app` and routes `/__ipc/<cmd>` to `commands::dispatch_ipc_command`.
   - Every UI action invokes a real IPC command or `extern "C"` FFI symbol in production mode; Flutter services must throw `StateError` if native library loading fails when `mockMode == false`.
   - Every desktop binary must pass `<binary> --self-test` with `"status": "PASSED"`.

## Step 3: Runtime Verification Gate

```powershell
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
npm run typecheck
npm run test:unit
```

