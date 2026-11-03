//! Standalone Desktop GUI Host & Loopback IPC Bridge
//!
//! Provides a 100% offline, loopback-only (`127.0.0.1`) embedded asset server,
//! JSON-RPC/Tauri-compatible IPC dispatcher (`/__ipc/<command>`), native chromeless
//! app-window launcher (Edge WebView2 / Chromium `--app` mode with isolated profile),
//! and automated `--self-test` verification harness for all 5 desktop applications.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::{AppPaths, PlatformError};

/// Static UI bundle embedded into an application binary via `include_str!` / `include_bytes!`.
#[derive(Debug, Clone, Copy)]
pub struct EmbeddedUiBundle {
    pub app_id: &'static str,
    pub app_title: &'static str,
    pub version: &'static str,
    pub index_html: &'static str,
    pub app_js: &'static [u8],
    pub app_css: &'static [u8],
    pub window_width: u32,
    pub window_height: u32,
}

/// Self-test command specification verified during `<app> --self-test`.
#[derive(Debug, Clone)]
pub struct SelfTestCheck {
    pub command: &'static str,
    pub payload: Value,
}

/// Result of running `<app> --self-test`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfTestReport {
    pub app_id: String,
    pub version: String,
    pub html_bytes: usize,
    pub js_bytes: usize,
    pub css_bytes: usize,
    pub ipc_checks_passed: usize,
    pub ipc_checks_total: usize,
    pub status: String,
}

/// Trait implemented by each desktop application to dispatch UI commands to its Rust backend.
pub trait DesktopIpcHandler: Send + Sync + 'static {
    /// Dispatch a named command with JSON arguments and return a JSON response or error string.
    fn handle_command(&self, command: &str, args: Value) -> Result<Value, String>;

    /// Return a list of IPC checks to execute when `--self-test` is invoked.
    fn self_test_checks(&self) -> Vec<SelfTestCheck>;
}

const TAURI_BRIDGE_SCRIPT: &str = r#"<script>
(function() {
  async function ipcInvoke(cmd, args) {
    const response = await fetch('/__ipc/' + encodeURIComponent(cmd), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(args || {})
    });
    const payload = await response.json();
    if (!response.ok || (payload && payload. __ipc_error)) {
      throw new Error((payload && payload.__ipc_error) || ('IPC HTTP ' + response.status));
    }
    return payload ? payload.__ipc_data : null;
  }
  window.__TAURI__ = window.__TAURI__ || {};
  window.__TAURI__.invoke = ipcInvoke;
  window.__TAURI__.core = { invoke: ipcInvoke };
  window.__TAURI_INTERNALS__ = { invoke: ipcInvoke };

  setInterval(function() {
    fetch('/__heartbeat', { method: 'POST', keepalive: true }).catch(function() {});
  }, 2000);
})();
</script>"#;

fn current_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_millis() as u64
}

fn inject_bridge_into_html(index_html: &str, app_title: &str) -> String {
    let mut html = index_html.to_string();
    if !html.contains("<title>") && html.contains("<head>") {
        html = html.replacen("<head>", &format!("<head><title>{}</title>", app_title), 1);
    }
    if html.contains("</head>") {
        html.replacen("</head>", &format!("{}\n</head>", TAURI_BRIDGE_SCRIPT), 1)
    } else {
        format!("{}\n{}", TAURI_BRIDGE_SCRIPT, html)
    }
}

/// Launch the desktop application runtime or execute `--self-test` / `--headless-server`.
pub fn run_desktop_app<H: DesktopIpcHandler>(
    bundle: EmbeddedUiBundle,
    handler: H,
) -> Result<Option<SelfTestReport>, PlatformError> {
    let args: Vec<String> = std::env::args().collect();
    let is_self_test = args.iter().any(|a| a == "--self-test");
    let is_headless = args.iter().any(|a| a == "--headless-server");

    let mut requested_port: u16 = 0;
    for i in 0..args.len() {
        if args[i] == "--port" {
            if let Some(p) = args.get(i + 1).and_then(|s| s.parse::<u16>().ok()) {
                requested_port = p;
            }
        }
    }

    let listener = TcpListener::bind(("127.0.0.1", requested_port))?;
    let local_addr = listener.local_addr()?;
    let port = local_addr.port();
    let url = format!("http://127.0.0.1:{}", port);

    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let last_heartbeat_ms = Arc::new(AtomicU64::new(current_epoch_ms()));
    let client_connected = Arc::new(AtomicBool::new(false));
    let handler = Arc::new(handler);

    let server_shutdown = Arc::clone(&shutdown_flag);
    let server_heartbeat = Arc::clone(&last_heartbeat_ms);
    let server_connected = Arc::clone(&client_connected);
    let server_handler = Arc::clone(&handler);

    let index_with_bridge = Arc::new(inject_bridge_into_html(bundle.index_html, bundle.app_title));

    let server_thread = std::thread::spawn(move || {
        let _ = listener.set_nonblocking(true);
        while !server_shutdown.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _peer)) => {
                    let _ = stream.set_nonblocking(false);
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
                    let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));
                    let hb = Arc::clone(&server_heartbeat);
                    let conn = Arc::clone(&server_connected);
                    let sd = Arc::clone(&server_shutdown);
                    let h = Arc::clone(&server_handler);
                    let html = Arc::clone(&index_with_bridge);
                    std::thread::spawn(move || {
                        handle_http_connection(stream, &bundle, &html, h.as_ref(), &hb, &conn, &sd);
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(12));
                }
                Err(_) => {
                    std::thread::sleep(Duration::from_millis(25));
                }
            }
        }
    });

    if is_self_test {
        let report = execute_self_test(port, &bundle, handler.as_ref())?;
        shutdown_flag.store(true, Ordering::SeqCst);
        let _ = server_thread.join();
        return Ok(Some(report));
    }

    if is_headless {
        println!("{} listening on {} (headless mode)", bundle.app_title, url);
        while !shutdown_flag.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = server_thread.join();
        return Ok(None);
    }

    // Launch native chromeless desktop window
    let mut window_child = spawn_native_app_window(&url, &bundle);
    let startup_instant = Instant::now();

    loop {
        if shutdown_flag.load(Ordering::SeqCst) {
            break;
        }

        // If we have a dedicated child window process, check if it exited
        if let Some(ref mut child) = window_child {
            if let Ok(Some(_status)) = child.try_wait() {
                // Give brief grace period in case browser handed off to another process,
                // then exit once heartbeats cease.
                let since_hb =
                    current_epoch_ms().saturating_sub(last_heartbeat_ms.load(Ordering::SeqCst));
                if since_hb > 4500 && startup_instant.elapsed() > Duration::from_secs(5) {
                    break;
                }
            }
        }

        // If client connected and heartbeats stopped for > 8 seconds, close backend
        if client_connected.load(Ordering::SeqCst) {
            let since_hb =
                current_epoch_ms().saturating_sub(last_heartbeat_ms.load(Ordering::SeqCst));
            if since_hb > 8000 {
                break;
            }
        } else if startup_instant.elapsed() > Duration::from_secs(45) {
            // Safety timeout if no UI window ever connected
            break;
        }

        std::thread::sleep(Duration::from_millis(200));
    }

    shutdown_flag.store(true, Ordering::SeqCst);
    if let Some(mut child) = window_child {
        let _ = child.kill();
    }
    let _ = server_thread.join();
    Ok(None)
}

fn handle_http_connection(
    mut stream: TcpStream,
    bundle: &EmbeddedUiBundle,
    index_html: &str,
    handler: &dyn DesktopIpcHandler,
    last_heartbeat_ms: &AtomicU64,
    client_connected: &AtomicBool,
    shutdown_flag: &AtomicBool,
) {
    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() || request_line.is_empty() {
        return;
    }

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }
    let method = parts[0];
    let raw_path = parts[1];
    let path = raw_path.split('?').next().unwrap_or("/");

    let mut content_length: usize = 0;
    loop {
        let mut header_line = String::new();
        if reader.read_line(&mut header_line).is_err() {
            break;
        }
        let trimmed = header_line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                content_length = v.trim().parse::<usize>().unwrap_or(0);
            }
        }
    }

    let mut body_bytes = vec![0u8; content_length.min(32 * 1024 * 1024)];
    if content_length > 0 {
        let _ = reader.read_exact(&mut body_bytes);
    }

    last_heartbeat_ms.store(current_epoch_ms(), Ordering::SeqCst);
    client_connected.store(true, Ordering::SeqCst);

    if method == "OPTIONS" {
        let _ = write_http_response(&mut stream, 200, "text/plain", b"OK");
        return;
    }

    if path == "/__heartbeat" {
        let _ = write_http_response(&mut stream, 200, "application/json", b"{\"ok\":true}");
        return;
    }

    if path == "/__shutdown" {
        let _ = write_http_response(&mut stream, 200, "application/json", b"{\"shutdown\":true}");
        shutdown_flag.store(true, Ordering::SeqCst);
        return;
    }

    if let Some(cmd_encoded) = path.strip_prefix("/__ipc/") {
        let cmd = url_decode(cmd_encoded);
        let args: Value = if body_bytes.is_empty() {
            Value::Object(serde_json::Map::new())
        } else {
            serde_json::from_slice(&body_bytes)
                .unwrap_or_else(|_| Value::Object(serde_json::Map::new()))
        };

        match handler.handle_command(&cmd, args) {
            Ok(data) => {
                let envelope = serde_json::json!({
                    "__ipc_ok": true,
                    "__ipc_data": data
                });
                let bytes = serde_json::to_vec(&envelope).unwrap_or_default();
                let _ = write_http_response(&mut stream, 200, "application/json", &bytes);
            }
            Err(err_msg) => {
                let envelope = serde_json::json!({
                    "__ipc_ok": false,
                    "__ipc_error": err_msg
                });
                let bytes = serde_json::to_vec(&envelope).unwrap_or_default();
                let _ = write_http_response(&mut stream, 400, "application/json", &bytes);
            }
        }
        return;
    }

    match path {
        "/" | "/index.html" => {
            let _ = write_http_response(
                &mut stream,
                200,
                "text/html; charset=utf-8",
                index_html.as_bytes(),
            );
        }
        "/assets/app.js" => {
            let _ = write_http_response(
                &mut stream,
                200,
                "application/javascript; charset=utf-8",
                bundle.app_js,
            );
        }
        "/assets/app.css" => {
            let _ =
                write_http_response(&mut stream, 200, "text/css; charset=utf-8", bundle.app_css);
        }
        "/favicon.ico" => {
            let _ = write_http_response(&mut stream, 204, "image/x-icon", b"");
        }
        _ => {
            // SPA fallback to index.html
            let _ = write_http_response(
                &mut stream,
                200,
                "text/html; charset=utf-8",
                index_html.as_bytes(),
            );
        }
    }
}

fn write_http_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let status_text = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Internal Server Error",
    };
    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        status,
        status_text,
        content_type,
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

fn url_decode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (
                (bytes[i + 1] as char).to_digit(16),
                (bytes[i + 2] as char).to_digit(16),
            ) {
                out.push(((h * 16 + l) as u8) as char);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn execute_self_test(
    port: u16,
    bundle: &EmbeddedUiBundle,
    handler: &dyn DesktopIpcHandler,
) -> Result<SelfTestReport, PlatformError> {
    // 1. Verify HTML endpoint over loopback TCP
    let (status_html, html_body) = loopback_http_request(port, "GET", "/", b"")?;
    if status_html != 200 || html_body.len() < 200 {
        return Err(PlatformError::Unsupported(format!(
            "Self-test HTML check failed: status={} len={}",
            status_html,
            html_body.len()
        )));
    }

    // 2. Verify JS & CSS asset bundles over loopback TCP
    let (status_js, js_body) = loopback_http_request(port, "GET", "/assets/app.js", b"")?;
    if status_js != 200 || js_body.len() < 1024 {
        return Err(PlatformError::Unsupported(format!(
            "Self-test JS bundle check failed: status={} len={}",
            status_js,
            js_body.len()
        )));
    }

    let (status_css, css_body) = loopback_http_request(port, "GET", "/assets/app.css", b"")?;
    if status_css != 200 || css_body.is_empty() {
        return Err(PlatformError::Unsupported(format!(
            "Self-test CSS bundle check failed: status={} len={}",
            status_css,
            css_body.len()
        )));
    }

    // 3. Execute live IPC checks over HTTP POST /__ipc/<command>
    let checks = handler.self_test_checks();
    let total = checks.len();
    let mut passed = 0usize;

    for check in checks {
        let path = format!("/__ipc/{}", check.command);
        let body = serde_json::to_vec(&check.payload).unwrap_or_default();
        let (status, resp_bytes) = loopback_http_request(port, "POST", &path, &body)?;
        if status == 200 {
            if let Ok(val) = serde_json::from_slice::<Value>(&resp_bytes) {
                if val.get("__ipc_ok").and_then(|v| v.as_bool()) == Some(true) {
                    passed += 1;
                } else {
                    return Err(PlatformError::Unsupported(format!(
                        "Self-test IPC command '{}' returned error: {}",
                        check.command,
                        String::from_utf8_lossy(&resp_bytes)
                    )));
                }
            }
        } else {
            return Err(PlatformError::Unsupported(format!(
                "Self-test IPC command '{}' failed HTTP {}: {}",
                check.command,
                status,
                String::from_utf8_lossy(&resp_bytes)
            )));
        }
    }

    let report = SelfTestReport {
        app_id: bundle.app_id.to_string(),
        version: bundle.version.to_string(),
        html_bytes: html_body.len(),
        js_bytes: js_body.len(),
        css_bytes: css_body.len(),
        ipc_checks_passed: passed,
        ipc_checks_total: total,
        status: "PASSED".to_string(),
    };

    if let Ok(json) = serde_json::to_string_pretty(&report) {
        println!("{}", json);
    }

    Ok(report)
}

fn loopback_http_request(
    port: u16,
    method: &str,
    path: &str,
    body: &[u8],
) -> Result<(u16, Vec<u8>), PlatformError> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;

    let req = format!(
        "{} {} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        method,
        path,
        port,
        body.len()
    );
    stream.write_all(req.as_bytes())?;
    if !body.is_empty() {
        stream.write_all(body)?;
    }
    stream.flush()?;

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw)?;

    let header_end = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap_or(0);
    let header_str = String::from_utf8_lossy(&raw[..header_end]);
    let status = header_str
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(500);

    let payload = if header_end + 4 <= raw.len() {
        raw[header_end + 4..].to_vec()
    } else {
        Vec::new()
    };

    Ok((status, payload))
}

fn spawn_native_app_window(url: &str, bundle: &EmbeddedUiBundle) -> Option<Child> {
    let profile_dir = AppPaths::from_app_name(bundle.app_id)
        .map(|p| p.cache_dir.join("desktop_window_profile"))
        .unwrap_or_else(|_| std::env::temp_dir().join(format!("{}_window_profile", bundle.app_id)));
    let _ = std::fs::create_dir_all(&profile_dir);

    let app_arg = format!("--app={}", url);
    let size_arg = format!(
        "--window-size={},{}",
        bundle.window_width, bundle.window_height
    );
    let profile_arg = format!("--user-data-dir={}", profile_dir.display());

    #[cfg(target_os = "windows")]
    {
        let candidates = [
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        ];

        for bin in candidates {
            if PathBuf::from(bin).exists() {
                if let Ok(child) = Command::new(bin)
                    .args([
                        &app_arg,
                        &size_arg,
                        &profile_arg,
                        "--no-first-run",
                        "--no-default-browser-check",
                        "--disable-features=Translate",
                    ])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                {
                    return Some(child);
                }
            }
        }

        Command::new("cmd")
            .args(["/C", "start", "", url])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()
    }

    #[cfg(target_os = "macos")]
    {
        let candidates = [
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
        ];
        for bin in candidates {
            if PathBuf::from(bin).exists() {
                if let Ok(child) = Command::new(bin)
                    .args([
                        &app_arg,
                        &size_arg,
                        &profile_arg,
                        "--no-first-run",
                        "--no-default-browser-check",
                    ])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                {
                    return Some(child);
                }
            }
        }

        Command::new("open")
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()
    }

    #[cfg(target_os = "linux")]
    {
        let candidates = [
            "chromium",
            "chromium-browser",
            "google-chrome",
            "microsoft-edge",
        ];
        for bin in candidates {
            if let Ok(child) = Command::new(bin)
                .args([
                    &app_arg,
                    &size_arg,
                    &profile_arg,
                    "--no-first-run",
                    "--no-default-browser-check",
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                return Some(child);
            }
        }

        Command::new("xdg-open")
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = (app_arg, size_arg, profile_arg);
        None
    }
}
