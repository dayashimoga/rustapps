//! Standalone Desktop GUI Host & Loopback IPC Bridge
//!
//! Provides a 100% offline, loopback-only (`127.0.0.1`) embedded asset server,
//! JSON-RPC/Tauri-compatible IPC dispatcher (`/__ipc/<command>`), native chromeless
//! app-window launcher (Edge WebView2 / Chromium `--app` mode with isolated profile),
//! and automated `--self-test` verification harness for all 5 desktop applications.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Child;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::{AppPaths, PlatformError};

/// Drive or quick-access folder entry exposed to desktop UIs via `/__fs/drives`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsLocationEntry {
    pub name: String,
    pub path: String,
    pub kind: String,
    pub available: bool,
}

/// Normalize a Windows or POSIX path by stripping verbatim prefixes (`\\?\` or `\\?\UNC\`).
pub fn normalize_display_path(raw: &str) -> String {
    if let Some(rest) = raw.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{}", rest)
    } else if let Some(rest) = raw.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        raw.to_string()
    }
}

/// Enumerate mounted drives (`A:\`–`Z:\` on Windows, `/` and `/Volumes` on POSIX) and standard user folders.
pub fn enumerate_drives_and_places() -> Vec<FsLocationEntry> {
    let mut entries = Vec::new();

    #[cfg(target_os = "windows")]
    {
        for letter in b'A'..=b'Z' {
            let root = format!("{}:\\", letter as char);
            if Path::new(&root).exists() && std::fs::read_dir(&root).is_ok() {
                entries.push(FsLocationEntry {
                    name: format!("Local Disk ({}:)", letter as char),
                    path: root,
                    kind: "drive".to_string(),
                    available: true,
                });
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        entries.push(FsLocationEntry {
            name: "Root (/)".to_string(),
            path: "/".to_string(),
            kind: "drive".to_string(),
            available: true,
        });
        if Path::new("/Volumes").exists() {
            if let Ok(rd) = std::fs::read_dir("/Volumes") {
                for entry in rd.flatten() {
                    let p = entry.path();
                    if p.is_dir() {
                        entries.push(FsLocationEntry {
                            name: entry.file_name().to_string_lossy().to_string(),
                            path: p.to_string_lossy().to_string(),
                            kind: "drive".to_string(),
                            available: true,
                        });
                    }
                }
            }
        }
        if Path::new("/mnt").exists() {
            if let Ok(rd) = std::fs::read_dir("/mnt") {
                for entry in rd.flatten() {
                    let p = entry.path();
                    if p.is_dir() {
                        entries.push(FsLocationEntry {
                            name: format!("/mnt/{}", entry.file_name().to_string_lossy()),
                            path: p.to_string_lossy().to_string(),
                            kind: "drive".to_string(),
                            available: true,
                        });
                    }
                }
            }
        }
    }

    // Standard user quick-access directories
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from);

    if let Some(home_dir) = home {
        if home_dir.exists() {
            entries.push(FsLocationEntry {
                name: "Home".to_string(),
                path: normalize_display_path(&home_dir.to_string_lossy()),
                kind: "quick_access".to_string(),
                available: true,
            });
        }
        for (label, folder) in [
            ("Desktop", "Desktop"),
            ("Downloads", "Downloads"),
            ("Documents", "Documents"),
            ("Pictures", "Pictures"),
            ("Music", "Music"),
            ("Videos", "Videos"),
        ] {
            let candidate = home_dir.join(folder);
            if candidate.exists() {
                entries.push(FsLocationEntry {
                    name: label.to_string(),
                    path: normalize_display_path(&candidate.to_string_lossy()),
                    kind: "quick_access".to_string(),
                    available: true,
                });
            }
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        entries.push(FsLocationEntry {
            name: "Workspace".to_string(),
            path: normalize_display_path(&cwd.to_string_lossy()),
            kind: "quick_access".to_string(),
            available: true,
        });
    }

    entries
}

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

    /// Dispatch a named binary fast-path command with raw byte payload and return a binary response (ADR-003).
    /// Default implementation provides echo capability for test_ping and ping.
    fn handle_binary_command(
        &self,
        command: &str,
        _headers: Value,
        payload: &[u8],
    ) -> Result<Vec<u8>, String> {
        if command == "ping" || command == "test_ping" {
            Ok(payload.to_vec())
        } else {
            Err(format!("Binary IPC command '{}' not implemented", command))
        }
    }
}

/// Thread-safe capability allowlist tracking filesystem paths explicitly granted by the user
/// via file dialogs, workspace directories, or application settings (SEC-03).
#[derive(Debug, Default)]
pub struct ScopedFileRegistry {
    allowed_exact_paths: RwLock<HashSet<PathBuf>>,
    allowed_roots: RwLock<HashSet<PathBuf>>,
}

impl ScopedFileRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Allow an exact file or directory path
    pub fn allow_path(&self, path: &Path) {
        if let Ok(mut set) = self.allowed_exact_paths.write() {
            set.insert(path.to_path_buf());
            if let Ok(canonical) = path.canonicalize() {
                set.insert(canonical);
            }
        }
    }

    /// Allow an entire directory root and all its subdirectories/files
    pub fn allow_root(&self, root: &Path) {
        if let Ok(mut set) = self.allowed_roots.write() {
            set.insert(root.to_path_buf());
            if let Ok(canonical) = root.canonicalize() {
                set.insert(canonical);
            }
        }
    }

    /// Check if a path is explicitly allowed by an exact match or a root prefix
    pub fn is_allowed(&self, path: &Path) -> bool {
        let canonical = path.canonicalize().ok();
        if let Ok(set) = self.allowed_exact_paths.read() {
            if set.contains(path) {
                return true;
            }
            if let Some(ref c) = canonical {
                if set.contains(c) {
                    return true;
                }
            }
        }
        if let Ok(roots) = self.allowed_roots.read() {
            for root in roots.iter() {
                if path.starts_with(root) {
                    return true;
                }
                if let Some(ref c) = canonical {
                    if c.starts_with(root) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

fn generate_bridge_script(session_token: &str) -> String {
    format!(
        r#"<script>
(function() {{
  // Strip one-time bootstrap nonce from window location and history immediately
  if (window.history && window.history.replaceState && window.location.search.includes('bootstrap=')) {{
    window.history.replaceState({{}}, document.title, window.location.pathname);
  }}

  const __AUTH_TOKEN__ = "{}";
  window.__ANTIGRAVITY_TOKEN__ = __AUTH_TOKEN__;

  async function ipcInvoke(cmd, args) {{
    const response = await fetch('/__ipc/' + encodeURIComponent(cmd), {{
      method: 'POST',
      headers: {{
        'Content-Type': 'application/json',
        'X-Antigravity-Token': __AUTH_TOKEN__
      }},
      body: JSON.stringify(args || {{}})
    }});
    const payload = await response.json();
    if (!response.ok || (payload && payload.__ipc_error)) {{
      throw new Error((payload && payload.__ipc_error) || ('IPC HTTP ' + response.status));
    }}
    return payload ? payload.__ipc_data : null;
  }}

  async function dialogOpen(options) {{
    const response = await fetch('/__dialog/open', {{
      method: 'POST',
      headers: {{
        'Content-Type': 'application/json',
        'X-Antigravity-Token': __AUTH_TOKEN__
      }},
      body: JSON.stringify(options || {{}})
    }});
    const payload = await response.json();
    return payload ? payload.path : null;
  }}

  async function dialogSave(options) {{
    const response = await fetch('/__dialog/save', {{
      method: 'POST',
      headers: {{
        'Content-Type': 'application/json',
        'X-Antigravity-Token': __AUTH_TOKEN__
      }},
      body: JSON.stringify(options || {{}})
    }});
    const payload = await response.json();
    return payload ? payload.path : null;
  }}

  async function getDrivesAndPlaces() {{
    const response = await fetch('/__fs/drives', {{
      headers: {{ 'X-Antigravity-Token': __AUTH_TOKEN__ }}
    }});
    if (!response.ok) return [];
    const payload = await response.json();
    return payload && payload.entries ? payload.entries : [];
  }}

  async function ipcInvokeBin(cmd, headers, binaryPayload) {{
    const response = await fetch('/__ipc_bin/' + encodeURIComponent(cmd), {{
      method: 'POST',
      headers: {{
        'Content-Type': 'application/octet-stream',
        'X-Antigravity-Token': __AUTH_TOKEN__,
        'X-Antigravity-Header': JSON.stringify(headers || {{}})
      }},
      body: binaryPayload || new Uint8Array()
    }});
    if (!response.ok) {{
      throw new Error('Binary IPC HTTP ' + response.status);
    }}
    return await response.arrayBuffer();
  }}

  function fileUrl(localPath) {{
    if (!localPath) return '';
    return '/__file?path=' + encodeURIComponent(localPath) + '&token=' + encodeURIComponent(__AUTH_TOKEN__);
  }}

  function connectEvents(onMessage) {{
    const es = new EventSource('/__events?token=' + encodeURIComponent(__AUTH_TOKEN__));
    if (onMessage) es.onmessage = onMessage;
    return es;
  }}

  window.__TAURI__ = window.__TAURI__ || {{}};
  window.__TAURI__.invoke = ipcInvoke;
  window.__TAURI__.invokeBinary = ipcInvokeBin;
  window.__TAURI__.core = {{ invoke: ipcInvoke, invokeBinary: ipcInvokeBin }};
  window.__TAURI__.dialog = {{ open: dialogOpen, save: dialogSave }};
  window.__TAURI__.fs = {{ getDrivesAndPlaces: getDrivesAndPlaces, fileUrl: fileUrl }};
  window.__TAURI__.events = {{ connect: connectEvents }};
  window.__TAURI_INTERNALS__ = {{ invoke: ipcInvoke }};

  setInterval(function() {{
    fetch('/__heartbeat', {{
      method: 'POST',
      headers: {{ 'X-Antigravity-Token': __AUTH_TOKEN__ }},
      keepalive: true
    }}).catch(function() {{}});
  }}, 2000);
}})();
</script>"#,
        session_token
    )
}

fn current_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_millis() as u64
}

fn inject_bridge_into_html(index_html: &str, app_title: &str, session_token: &str) -> String {
    let bridge_script = generate_bridge_script(session_token);
    let mut html = index_html.to_string();
    if !html.contains("<title>") && html.contains("<head>") {
        html = html.replacen("<head>", &format!("<head><title>{}</title>", app_title), 1);
    }
    if html.contains("</head>") {
        html.replacen("</head>", &format!("{}\n</head>", bridge_script), 1)
    } else {
        format!("{}\n{}", bridge_script, html)
    }
}

struct Worker {
    _id: usize,
    thread: Option<std::thread::JoinHandle<()>>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<std::sync::mpsc::Sender<Job>>,
}

impl ThreadPool {
    fn new(size: usize) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel::<Job>();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            let rx = Arc::clone(&receiver);
            let thread = std::thread::Builder::new()
                .name(format!("ag-desktop-worker-{}", id))
                .spawn(move || loop {
                    let job = {
                        let Ok(lock) = rx.lock() else { break };
                        match lock.recv() {
                            Ok(job) => job,
                            Err(_) => break,
                        }
                    };
                    job();
                })
                .ok();
            workers.push(Worker { _id: id, thread });
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        if let Some(ref sender) = self.sender {
            let _ = sender.send(Box::new(f));
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());
        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

fn unblock_server_listener(port: u16) {
    if port > 0 {
        use std::net::SocketAddr;
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let _ = TcpStream::connect_timeout(&addr, Duration::from_millis(200));
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

    // Cryptographically secure 256-bit session token for authenticated loopback IPC
    let session_token: Arc<str> = {
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
            .into()
    };

    // One-time cryptographically secure bootstrap nonce exchanged on first launch for HttpOnly cookie (SEC-04)
    let bootstrap_nonce: Arc<std::sync::Mutex<Option<String>>> = {
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        Arc::new(std::sync::Mutex::new(Some(
            bytes.iter().map(|b| format!("{:02x}", b)).collect(),
        )))
    };

    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let last_heartbeat_ms = Arc::new(AtomicU64::new(current_epoch_ms()));
    let client_connected = Arc::new(AtomicBool::new(false));
    let handler = Arc::new(handler);

    // Capability-based filesystem allowlist scoped to dialog grants and workspace roots (SEC-03)
    let scoped_registry = Arc::new(ScopedFileRegistry::new());
    if let Ok(cwd) = std::env::current_dir() {
        scoped_registry.allow_root(&cwd);
    }
    scoped_registry.allow_root(&std::env::temp_dir());

    let server_shutdown = Arc::clone(&shutdown_flag);
    let server_heartbeat = Arc::clone(&last_heartbeat_ms);
    let server_connected = Arc::clone(&client_connected);
    let server_handler = Arc::clone(&handler);
    let server_token = Arc::clone(&session_token);
    let server_registry = Arc::clone(&scoped_registry);
    let server_bootstrap = Arc::clone(&bootstrap_nonce);

    let index_with_bridge = Arc::new(inject_bridge_into_html(
        bundle.index_html,
        bundle.app_title,
        &session_token,
    ));

    let server_thread = std::thread::spawn(move || {
        let pool_size = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(8)
            .clamp(4, 16);
        let pool = ThreadPool::new(pool_size);

        while !server_shutdown.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _peer)) => {
                    if server_shutdown.load(Ordering::SeqCst) {
                        break;
                    }
                    let _ = stream.set_nonblocking(false);
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
                    let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));
                    let hb = Arc::clone(&server_heartbeat);
                    let conn = Arc::clone(&server_connected);
                    let sd = Arc::clone(&server_shutdown);
                    let h = Arc::clone(&server_handler);
                    let html = Arc::clone(&index_with_bridge);
                    let tok = Arc::clone(&server_token);
                    let reg = Arc::clone(&server_registry);
                    let boot = Arc::clone(&server_bootstrap);
                    pool.execute(move || {
                        handle_http_connection(
                            stream,
                            &bundle,
                            &html,
                            h.as_ref(),
                            &hb,
                            &conn,
                            &sd,
                            &tok,
                            &reg,
                            port,
                            &boot,
                        );
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {
                    continue;
                }
                Err(_) => {
                    if server_shutdown.load(Ordering::SeqCst) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(25));
                }
            }
        }
    });

    if is_self_test {
        let report = execute_self_test(
            port,
            &bundle,
            handler.as_ref(),
            &session_token,
            &scoped_registry,
        )?;
        shutdown_flag.store(true, Ordering::SeqCst);
        unblock_server_listener(port);
        let _ = server_thread.join();
        return Ok(Some(report));
    }

    if is_headless {
        println!("{} listening on {} (headless mode)", bundle.app_title, url);
        println!("Session Token: {}", session_token);
        while !shutdown_flag.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(100));
        }
        unblock_server_listener(port);
        let _ = server_thread.join();
        return Ok(None);
    }

    // Launch native chromeless desktop window with one-time bootstrap nonce (SEC-04)
    let initial_nonce = bootstrap_nonce.lock().ok().and_then(|g| g.clone());
    let window_url = match initial_nonce {
        Some(ref n) => format!("{}/?bootstrap={}", url, n),
        None => url.clone(),
    };
    let mut window_child = spawn_native_app_window(&window_url, &bundle);
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
    unblock_server_listener(port);
    let _ = server_thread.join();
    Ok(None)
}

#[allow(clippy::too_many_arguments)]
fn handle_http_connection(
    mut stream: TcpStream,
    bundle: &EmbeddedUiBundle,
    index_html: &str,
    handler: &dyn DesktopIpcHandler,
    last_heartbeat_ms: &AtomicU64,
    client_connected: &AtomicBool,
    shutdown_flag: &AtomicBool,
    session_token: &str,
    scoped_registry: &ScopedFileRegistry,
    port: u16,
    bootstrap_nonce: &std::sync::Mutex<Option<String>>,
) {
    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });

    loop {
        if shutdown_flag.load(Ordering::SeqCst) {
            break;
        }

        // Set read timeout for idle keep-alive requests (5 seconds timeout between requests)
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));

        let mut request_line = String::new();
        match reader.read_line(&mut request_line) {
            Ok(0) => break, // EOF, peer closed socket cleanly
            Ok(_) => {}
            Err(_) => break, // Timeout or socket read error, terminate keep-alive loop
        }

        let trimmed_req = request_line.trim();
        if trimmed_req.is_empty() {
            break;
        }

        let parts: Vec<&str> = trimmed_req.split_whitespace().collect();
        if parts.len() < 2 {
            break;
        }
        let method = parts[0];
        let raw_path = parts[1];
        let http_version = if parts.len() >= 3 {
            parts[2]
        } else {
            "HTTP/1.1"
        };
        let (path, query_string) = raw_path.split_once('?').unwrap_or((raw_path, ""));

        let mut content_length: usize = 0;
        let mut range_header: Option<String> = None;
        let mut auth_token_header: Option<String> = None;
        let mut origin_header: Option<String> = None;
        let mut custom_header: Option<String> = None;
        let mut connection_header: Option<String> = None;
        let mut cookie_header: Option<String> = None;

        let mut header_err = false;
        loop {
            let mut header_line = String::new();
            if reader.read_line(&mut header_line).is_err() {
                header_err = true;
                break;
            }
            let trimmed = header_line.trim();
            if trimmed.is_empty() {
                break;
            }
            if let Some((k, v)) = trimmed.split_once(':') {
                let key = k.trim();
                let val = v.trim();
                if key.eq_ignore_ascii_case("content-length") {
                    content_length = val.parse::<usize>().unwrap_or(0);
                } else if key.eq_ignore_ascii_case("range") {
                    range_header = Some(val.to_string());
                } else if key.eq_ignore_ascii_case("x-antigravity-token") {
                    auth_token_header = Some(val.to_string());
                } else if key.eq_ignore_ascii_case("authorization") {
                    if let Some(bearer) = val.strip_prefix("Bearer ") {
                        auth_token_header = Some(bearer.trim().to_string());
                    }
                } else if key.eq_ignore_ascii_case("origin") {
                    origin_header = Some(val.to_string());
                } else if key.eq_ignore_ascii_case("x-antigravity-header") {
                    custom_header = Some(val.to_string());
                } else if key.eq_ignore_ascii_case("connection") {
                    connection_header = Some(val.to_string());
                } else if key.eq_ignore_ascii_case("cookie") {
                    cookie_header = Some(val.to_string());
                }
            }
        }
        if header_err {
            break;
        }

        let is_http_10 = http_version.starts_with("HTTP/1.0");
        let client_wants_close = connection_header
            .as_deref()
            .map(|v| v.to_ascii_lowercase().contains("close"))
            .unwrap_or(false);
        let client_wants_keep_alive = connection_header
            .as_deref()
            .map(|v| v.to_ascii_lowercase().contains("keep-alive"))
            .unwrap_or(false);

        let mut keep_alive = if is_http_10 {
            client_wants_keep_alive
        } else {
            !client_wants_close
        };

        if shutdown_flag.load(Ordering::SeqCst) {
            keep_alive = false;
        }

        last_heartbeat_ms.store(current_epoch_ms(), Ordering::SeqCst);
        client_connected.store(true, Ordering::SeqCst);

        // Security Gate 1 (SEC-02): Block external cross-origin requests.
        let is_valid_local_origin = match origin_header.as_deref() {
            Some(orig) => {
                orig.starts_with("http://127.0.0.1:")
                    || orig.starts_with("http://localhost:")
                    || orig.starts_with("tauri://")
                    || orig.starts_with("app://")
                    || orig.starts_with("antigravity://")
            }
            None => true,
        };
        if !is_valid_local_origin {
            let _ = write_http_response(
                &mut stream,
                403,
                "application/json",
                b"{\"__ipc_ok\":false,\"__ipc_error\":\"Forbidden: Cross-origin request rejected\"}",
                None,
                false,
                None,
            );
            break;
        }

        if method == "OPTIONS" {
            let _ = write_http_response(
                &mut stream,
                200,
                "text/plain",
                b"OK",
                origin_header.as_deref(),
                keep_alive,
                None,
            );
            if !keep_alive {
                break;
            }
            continue;
        }

        let mut query_token: Option<String> = None;
        let mut query_bootstrap: Option<String> = None;
        for pair in query_string.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                if k == "token" {
                    query_token = Some(url_decode(v));
                } else if k == "bootstrap" {
                    query_bootstrap = Some(url_decode(v));
                }
            }
        }

        // Parse HttpOnly session cookie if provided
        let mut cookie_token: Option<String> = None;
        if let Some(ref cookies) = cookie_header {
            for cookie in cookies.split(';') {
                if let Some((k, v)) = cookie.trim().split_once('=') {
                    if k == "__ag_session" {
                        cookie_token = Some(v.to_string());
                        break;
                    }
                }
            }
        }

        // Security Gate: One-time bootstrap nonce verification & session cookie exchange (SEC-04)
        let mut session_cookie_to_set: Option<String> = None;
        if let Some(ref candidate_nonce) = query_bootstrap {
            let is_nonce_valid = {
                let mut guard = bootstrap_nonce.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(ref active_nonce) = *guard {
                    if constant_time_eq_token(candidate_nonce, active_nonce) {
                        *guard = None; // One-time use: consume nonce immediately!
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            };

            if !is_nonce_valid {
                let _ = write_http_response(
                    &mut stream,
                    403,
                    "application/json",
                    b"{\"__ipc_ok\":false,\"__ipc_error\":\"Forbidden: Invalid or expired bootstrap nonce\"}",
                    origin_header.as_deref(),
                    false,
                    None,
                );
                break;
            }

            session_cookie_to_set = Some(format!(
                "__ag_session={}; Path=/; HttpOnly; SameSite=Strict",
                session_token
            ));
        }

        // Security Gate 2 (SEC-04): Constant-time token verification (supports X-Antigravity-Token, Cookie, and ?token=)
        let is_authenticated = auth_token_header
            .as_deref()
            .map(|t| constant_time_eq_token(t, session_token))
            .or_else(|| {
                cookie_token
                    .as_deref()
                    .map(|t| constant_time_eq_token(t, session_token))
            })
            .or_else(|| {
                query_token
                    .as_deref()
                    .map(|t| constant_time_eq_token(t, session_token))
            })
            .unwrap_or(false);

        // Security Gate 3 (SEC-02 & SEC-05): Check authentication BEFORE allocating memory or reading body.
        if path.starts_with("/__") && !is_authenticated {
            let _ = write_http_response(
                &mut stream,
                403,
                "application/json",
                b"{\"__ipc_ok\":false,\"__ipc_error\":\"Forbidden: Invalid or missing X-Antigravity-Token\"}",
                origin_header.as_deref(),
                false,
                None,
            );
            break;
        }

        // Security Gate 4 (SEC-05): Prevent memory exhaustion DoS via oversized Content-Length.
        const MAX_IPC_BODY_SIZE: usize = 8 * 1024 * 1024; // 8 MB max for JSON IPC
        if content_length > MAX_IPC_BODY_SIZE {
            let _ = write_http_response(
                &mut stream,
                413,
                "application/json",
                b"{\"__ipc_ok\":false,\"__ipc_error\":\"Payload Too Large\"}",
                origin_header.as_deref(),
                false,
                None,
            );
            break;
        }

        let mut body_bytes = vec![0u8; content_length];
        if content_length > 0 && reader.read_exact(&mut body_bytes).is_err() {
            break;
        }

        if path == "/__heartbeat" {
            let _ = write_http_response(
                &mut stream,
                200,
                "application/json",
                b"{\"ok\":true}",
                origin_header.as_deref(),
                keep_alive,
                None,
            );
            if !keep_alive {
                break;
            }
            continue;
        }

        if path == "/__shutdown" {
            let _ = write_http_response(
                &mut stream,
                200,
                "application/json",
                b"{\"shutdown\":true}",
                origin_header.as_deref(),
                false,
                None,
            );
            shutdown_flag.store(true, Ordering::SeqCst);
            unblock_server_listener(port);
            break;
        }

        if path == "/__fs/drives" {
            let entries = enumerate_drives_and_places();
            let payload = serde_json::json!({ "entries": entries });
            let bytes = serde_json::to_vec(&payload).unwrap_or_default();
            let _ = write_http_response(
                &mut stream,
                200,
                "application/json",
                &bytes,
                origin_header.as_deref(),
                keep_alive,
                None,
            );
            if !keep_alive {
                break;
            }
            continue;
        }

        if path == "/__dialog/open" {
            let opts: Value = if body_bytes.is_empty() {
                Value::Object(serde_json::Map::new())
            } else {
                serde_json::from_slice(&body_bytes)
                    .unwrap_or_else(|_| Value::Object(serde_json::Map::new()))
            };
            let is_dir = opts
                .get("directory")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let multiple = opts
                .get("multiple")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let title = opts
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or(if is_dir { "Select Folder" } else { "Open File" });

            let selected =
                invoke_native_open_dialog(title, is_dir, multiple, &opts, scoped_registry);
            let payload = serde_json::json!({ "path": selected });
            let bytes = serde_json::to_vec(&payload).unwrap_or_default();
            let _ = write_http_response(
                &mut stream,
                200,
                "application/json",
                &bytes,
                origin_header.as_deref(),
                keep_alive,
                None,
            );
            if !keep_alive {
                break;
            }
            continue;
        }

        if path == "/__dialog/save" {
            let opts: Value = if body_bytes.is_empty() {
                Value::Object(serde_json::Map::new())
            } else {
                serde_json::from_slice(&body_bytes)
                    .unwrap_or_else(|_| Value::Object(serde_json::Map::new()))
            };
            let title = opts
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Save File");
            let default_path = opts
                .get("defaultPath")
                .or_else(|| opts.get("default_path"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let selected = invoke_native_save_dialog(title, default_path, &opts, scoped_registry);
            let payload = serde_json::json!({ "path": selected });
            let bytes = serde_json::to_vec(&payload).unwrap_or_default();
            let _ = write_http_response(
                &mut stream,
                200,
                "application/json",
                &bytes,
                origin_header.as_deref(),
                keep_alive,
                None,
            );
            if !keep_alive {
                break;
            }
            continue;
        }

        if path == "/__events" {
            let mut sse_headers = String::from(
                "HTTP/1.1 200 OK\r\n\
                 Content-Type: text/event-stream\r\n\
                 Cache-Control: no-cache\r\n\
                 Connection: keep-alive\r\n",
            );
            if let Some(orig) = origin_header.as_deref() {
                if orig.starts_with("http://127.0.0.1:") || orig.starts_with("http://localhost:") {
                    sse_headers.push_str(&format!("Access-Control-Allow-Origin: {}\r\n", orig));
                }
            }
            sse_headers.push_str("\r\n");
            if stream.write_all(sse_headers.as_bytes()).is_ok() {
                let initial = b"event: connected\ndata: {\"status\":\"ready\"}\n\n";
                let _ = stream.write_all(initial);
                let _ = stream.flush();
            }
            if keep_alive {
                while !shutdown_flag.load(Ordering::SeqCst) {
                    std::thread::sleep(Duration::from_millis(2000));
                    if stream.write_all(b": ping\n\n").is_err() || stream.flush().is_err() {
                        break;
                    }
                }
            }
            break;
        }

        if let Some(cmd_encoded) = path.strip_prefix("/__ipc_bin/") {
            let cmd = url_decode(cmd_encoded);
            let header_val: Value = custom_header
                .as_deref()
                .and_then(|h| serde_json::from_str(h).ok())
                .unwrap_or_else(|| Value::Object(serde_json::Map::new()));

            match handler.handle_binary_command(&cmd, header_val, &body_bytes) {
                Ok(bin_data) => {
                    let _ = write_http_response(
                        &mut stream,
                        200,
                        "application/octet-stream",
                        &bin_data,
                        origin_header.as_deref(),
                        keep_alive,
                        None,
                    );
                }
                Err(err_msg) => {
                    let err_payload = serde_json::json!({
                        "__ipc_ok": false,
                        "__ipc_error": err_msg
                    });
                    let bytes = serde_json::to_vec(&err_payload).unwrap_or_default();
                    let _ = write_http_response(
                        &mut stream,
                        400,
                        "application/json",
                        &bytes,
                        origin_header.as_deref(),
                        keep_alive,
                        None,
                    );
                }
            }
            if !keep_alive {
                break;
            }
            continue;
        }

        if path == "/__file" {
            serve_local_file(
                &mut stream,
                query_string,
                range_header.as_deref(),
                origin_header.as_deref(),
                scoped_registry,
                keep_alive,
            );
            if !keep_alive {
                break;
            }
            continue;
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
                    let _ = write_http_response(
                        &mut stream,
                        200,
                        "application/json",
                        &bytes,
                        origin_header.as_deref(),
                        keep_alive,
                        None,
                    );
                }
                Err(err_msg) => {
                    let envelope = serde_json::json!({
                        "__ipc_ok": false,
                        "__ipc_error": err_msg
                    });
                    let bytes = serde_json::to_vec(&envelope).unwrap_or_default();
                    let _ = write_http_response(
                        &mut stream,
                        400,
                        "application/json",
                        &bytes,
                        origin_header.as_deref(),
                        keep_alive,
                        None,
                    );
                }
            }
            if !keep_alive {
                break;
            }
            continue;
        }

        match path {
            "/" | "/index.html" => {
                let _ = write_http_response(
                    &mut stream,
                    200,
                    "text/html; charset=utf-8",
                    index_html.as_bytes(),
                    origin_header.as_deref(),
                    keep_alive,
                    session_cookie_to_set.as_deref(),
                );
            }
            "/assets/app.js" => {
                let _ = write_http_response(
                    &mut stream,
                    200,
                    "application/javascript; charset=utf-8",
                    bundle.app_js,
                    origin_header.as_deref(),
                    keep_alive,
                    None,
                );
            }
            "/assets/app.css" => {
                let _ = write_http_response(
                    &mut stream,
                    200,
                    "text/css; charset=utf-8",
                    bundle.app_css,
                    origin_header.as_deref(),
                    keep_alive,
                    None,
                );
            }
            "/favicon.ico" => {
                let _ = write_http_response(
                    &mut stream,
                    204,
                    "image/x-icon",
                    b"",
                    origin_header.as_deref(),
                    keep_alive,
                    None,
                );
            }
            _ => {
                // SPA fallback to index.html
                let _ = write_http_response(
                    &mut stream,
                    200,
                    "text/html; charset=utf-8",
                    index_html.as_bytes(),
                    origin_header.as_deref(),
                    keep_alive,
                    session_cookie_to_set.as_deref(),
                );
            }
        }

        if !keep_alive {
            break;
        }
    }
}

fn constant_time_eq_token(a: &str, b: &str) -> bool {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    if a_bytes.len() != b_bytes.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a_bytes.iter().zip(b_bytes.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn is_path_safe_for_serving(clean_path: &str) -> bool {
    if clean_path.contains("..") || clean_path.contains('\0') {
        return false;
    }
    let lower = clean_path.to_ascii_lowercase().replace('\\', "/");
    if lower.contains("/.ssh/")
        || lower.ends_with("/.ssh")
        || lower.contains("/.gnupg/")
        || lower.ends_with("/.gnupg")
        || lower.contains("/.aws/")
        || lower.contains("/.azure/")
        || lower.contains("/.kube/")
        || lower.contains("/.docker/")
        || lower.contains("/.env")
        || lower.contains("/etc/shadow")
        || lower.contains("/etc/passwd")
        || lower.contains("/etc/sudoers")
        || lower.contains("/system32/config")
        || lower.ends_with("/sam")
        || lower.ends_with("/system")
        || lower.ends_with("/security")
        || lower.ends_with(".key")
        || lower.ends_with(".pem")
        || lower.ends_with(".pfx")
        || lower.ends_with(".p12")
    {
        return false;
    }
    if lower.ends_with(".exe")
        || lower.ends_with(".dll")
        || lower.ends_with(".sys")
        || lower.ends_with(".bat")
        || lower.ends_with(".cmd")
        || lower.ends_with(".ps1")
        || lower.ends_with(".vbs")
        || lower.ends_with(".scr")
    {
        return false;
    }
    true
}

fn write_http_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
    origin: Option<&str>,
    keep_alive: bool,
    set_cookie: Option<&str>,
) -> std::io::Result<()> {
    let status_text = match status {
        200 => "OK",
        204 => "No Content",
        206 => "Partial Content",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        413 => "Payload Too Large",
        416 => "Range Not Satisfiable",
        _ => "Internal Server Error",
    };

    let mut cors_headers = String::new();
    if let Some(orig) = origin {
        if orig.starts_with("http://127.0.0.1:") || orig.starts_with("http://localhost:") {
            cors_headers = format!(
                "Access-Control-Allow-Origin: {}\r\nVary: Origin\r\nAccess-Control-Allow-Headers: Content-Type, X-Antigravity-Token, Authorization, Range\r\nAccess-Control-Expose-Headers: Content-Range, Accept-Ranges, Content-Length\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\n",
                orig
            );
        }
    }

    let conn_header = if keep_alive {
        "Connection: keep-alive\r\nKeep-Alive: timeout=5, max=1000\r\n"
    } else {
        "Connection: close\r\n"
    };

    let cookie_header = match set_cookie {
        Some(cookie) => format!("Set-Cookie: {}\r\n", cookie),
        None => String::new(),
    };

    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\n{}{}{}Cache-Control: no-store\r\n\r\n",
        status,
        status_text,
        content_type,
        body.len(),
        cors_headers,
        conn_header,
        cookie_header
    );
    stream.write_all(header.as_bytes())?;
    if !body.is_empty() {
        stream.write_all(body)?;
    }
    stream.flush()
}

fn url_decode(input: &str) -> String {
    let mut out = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (
                (bytes[i + 1] as char).to_digit(16),
                (bytes[i + 2] as char).to_digit(16),
            ) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        } else if bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).to_string())
}

fn url_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

fn parse_byte_range(range_header: &str, total_size: u64) -> Option<Result<(u64, u64), ()>> {
    let spec = range_header.strip_prefix("bytes=")?;
    let first = spec.split(',').next()?.trim();
    let (start_str, end_str) = first.split_once('-')?;
    let start_str = start_str.trim();
    let end_str = end_str.trim();

    if start_str.is_empty() {
        // Suffix byte range: bytes=-suffix (e.g. -500 means last 500 bytes)
        let suffix = end_str.parse::<u64>().ok()?;
        if suffix == 0 || total_size == 0 {
            return Some(Err(()));
        }
        let len = suffix.min(total_size);
        let start = total_size - len;
        let end = total_size - 1;
        Some(Ok((start, end)))
    } else if end_str.is_empty() {
        // Open-ended range: bytes=start-
        let start = start_str.parse::<u64>().ok()?;
        if start >= total_size {
            return Some(Err(()));
        }
        let end = total_size.saturating_sub(1);
        Some(Ok((start, end)))
    } else {
        // Explicit range: bytes=start-end
        let start = start_str.parse::<u64>().ok()?;
        let end = end_str.parse::<u64>().ok()?;
        if start <= end && start < total_size {
            let clamped_end = end.min(total_size.saturating_sub(1));
            Some(Ok((start, clamped_end)))
        } else {
            Some(Err(()))
        }
    }
}

fn apply_dialog_filters(mut dialog: rfd::FileDialog, opts: &Value) -> rfd::FileDialog {
    // If starting directory or defaultPath is specified
    if let Some(default_path) = opts
        .get("defaultPath")
        .or_else(|| opts.get("default_path"))
        .and_then(|v| v.as_str())
    {
        let p = Path::new(default_path);
        if p.is_dir() {
            dialog = dialog.set_directory(p);
        } else if let Some(parent) = p.parent() {
            if !parent.as_os_str().is_empty() {
                dialog = dialog.set_directory(parent);
            }
            if let Some(file_name) = p.file_name().and_then(|f| f.to_str()) {
                dialog = dialog.set_file_name(file_name);
            }
        }
    }

    // Single filter string: e.g. "png,jpg,webp" or "*.log;*.txt"
    if let Some(f_str) = opts.get("filter").and_then(|v| v.as_str()) {
        let clean = f_str.trim();
        if !clean.is_empty() && clean != "*.*" && clean != "*" {
            let exts: Vec<&str> = clean
                .split([';', ',', '|'])
                .map(|s| s.trim().trim_start_matches('*').trim_start_matches('.'))
                .filter(|s| !s.is_empty() && *s != "*")
                .collect();
            if !exts.is_empty() {
                dialog = dialog.add_filter("Supported Files", &exts);
            }
        }
    }

    // Structured filters array: e.g. [{ name: "Images", extensions: ["png", "jpg"] }]
    if let Some(filters) = opts.get("filters").and_then(|v| v.as_array()) {
        for f in filters {
            let name = f.get("name").and_then(|v| v.as_str()).unwrap_or("Files");
            if let Some(exts_arr) = f.get("extensions").and_then(|v| v.as_array()) {
                let exts: Vec<&str> = exts_arr
                    .iter()
                    .filter_map(|e| e.as_str())
                    .map(|e| e.trim().trim_start_matches('*').trim_start_matches('.'))
                    .filter(|e| !e.is_empty() && *e != "*")
                    .collect();
                if !exts.is_empty() {
                    dialog = dialog.add_filter(name, &exts);
                }
            }
        }
    }

    dialog
}

fn invoke_native_open_dialog(
    title: &str,
    is_dir: bool,
    multiple: bool,
    opts: &Value,
    scoped_registry: &ScopedFileRegistry,
) -> Value {
    let mut dialog = rfd::FileDialog::new().set_title(title);
    dialog = apply_dialog_filters(dialog, opts);

    if is_dir {
        if multiple {
            match dialog.pick_folders() {
                Some(paths) => {
                    let mut arr = Vec::with_capacity(paths.len());
                    for p in paths {
                        scoped_registry.allow_root(&p);
                        let clean = normalize_display_path(&p.to_string_lossy());
                        arr.push(Value::String(clean));
                    }
                    Value::Array(arr)
                }
                None => Value::Null,
            }
        } else {
            match dialog.pick_folder() {
                Some(p) => {
                    scoped_registry.allow_root(&p);
                    let clean = normalize_display_path(&p.to_string_lossy());
                    Value::String(clean)
                }
                None => Value::Null,
            }
        }
    } else if multiple {
        match dialog.pick_files() {
            Some(paths) => {
                let mut arr = Vec::with_capacity(paths.len());
                for p in paths {
                    scoped_registry.allow_path(&p);
                    let clean = normalize_display_path(&p.to_string_lossy());
                    arr.push(Value::String(clean));
                }
                Value::Array(arr)
            }
            None => Value::Null,
        }
    } else {
        match dialog.pick_file() {
            Some(p) => {
                scoped_registry.allow_path(&p);
                let clean = normalize_display_path(&p.to_string_lossy());
                Value::String(clean)
            }
            None => Value::Null,
        }
    }
}

fn invoke_native_save_dialog(
    title: &str,
    default_path: &str,
    opts: &Value,
    scoped_registry: &ScopedFileRegistry,
) -> Value {
    let mut dialog = rfd::FileDialog::new().set_title(title);
    dialog = apply_dialog_filters(dialog, opts);

    if !default_path.is_empty() {
        let p = Path::new(default_path);
        if let Some(parent) = p.parent() {
            if !parent.as_os_str().is_empty() {
                dialog = dialog.set_directory(parent);
            }
        }
        if let Some(file_name) = p.file_name().and_then(|f| f.to_str()) {
            dialog = dialog.set_file_name(file_name);
        }
    }

    match dialog.save_file() {
        Some(p) => {
            scoped_registry.allow_path(&p);
            let clean = normalize_display_path(&p.to_string_lossy());
            Value::String(clean)
        }
        None => Value::Null,
    }
}

fn mime_type_for_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("js") | Some("mjs") => "application/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        Some("ico") => "image/x-icon",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("ogg") => "audio/ogg",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("wasm") => "application/wasm",
        Some("pdf") => "application/pdf",
        Some("txt") | Some("log") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn serve_local_file(
    stream: &mut TcpStream,
    query_string: &str,
    range_header: Option<&str>,
    origin: Option<&str>,
    scoped_registry: &ScopedFileRegistry,
    keep_alive: bool,
) {
    let mut target_path: Option<String> = None;
    for pair in query_string.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == "path" {
                target_path = Some(url_decode(v));
                break;
            }
        }
    }

    let Some(raw_path) = target_path else {
        let _ = write_http_response(
            stream,
            400,
            "text/plain",
            b"Missing path query parameter",
            origin,
            keep_alive,
            None,
        );
        return;
    };

    let clean_path = normalize_display_path(&raw_path);

    // Defense-in-depth Check 1: Prohibit sensitive system, credential, and executable file paths
    if !is_path_safe_for_serving(&clean_path) {
        let _ = write_http_response(
            stream,
            403,
            "text/plain",
            b"Access denied: protected or sensitive file path",
            origin,
            keep_alive,
            None,
        );
        return;
    }

    let path_obj = Path::new(&clean_path);

    // Defense-in-depth Check 2 (SEC-03): Capability allowlist verification
    if !scoped_registry.is_allowed(path_obj) {
        let _ = write_http_response(
            stream,
            403,
            "text/plain",
            b"Access denied: path not granted via dialog or workspace scope",
            origin,
            keep_alive,
            None,
        );
        return;
    }
    if !path_obj.exists() || !path_obj.is_file() {
        let _ = write_http_response(
            stream,
            404,
            "text/plain",
            b"File not found",
            origin,
            keep_alive,
            None,
        );
        return;
    }

    let mut file = match File::open(path_obj) {
        Ok(f) => f,
        Err(_) => {
            let _ = write_http_response(
                stream,
                404,
                "text/plain",
                b"Cannot open file",
                origin,
                keep_alive,
                None,
            );
            return;
        }
    };

    let total_size = match file.metadata() {
        Ok(m) => m.len(),
        Err(_) => {
            let _ = write_http_response(
                stream,
                500,
                "text/plain",
                b"Metadata error",
                origin,
                keep_alive,
                None,
            );
            return;
        }
    };

    let content_type = mime_type_for_path(path_obj);

    let mut cors_headers = String::new();
    if let Some(orig) = origin {
        if orig.starts_with("http://127.0.0.1:") || orig.starts_with("http://localhost:") {
            cors_headers = format!(
                "Access-Control-Allow-Origin: {}\r\nVary: Origin\r\nAccess-Control-Allow-Headers: Content-Type, X-Antigravity-Token, Authorization, Range\r\nAccess-Control-Expose-Headers: Content-Range, Accept-Ranges, Content-Length\r\n",
                orig
            );
        }
    }

    let conn_header = if keep_alive {
        "Connection: keep-alive\r\nKeep-Alive: timeout=5, max=1000\r\n"
    } else {
        "Connection: close\r\n"
    };

    if let Some(range_str) = range_header {
        match parse_byte_range(range_str, total_size) {
            Some(Ok((start, end))) => {
                let length = end - start + 1;
                if file.seek(SeekFrom::Start(start)).is_err() {
                    let _ = write_http_response(
                        stream,
                        500,
                        "text/plain",
                        b"Seek error",
                        origin,
                        keep_alive,
                        None,
                    );
                    return;
                }

                let header = format!(
                    "HTTP/1.1 206 Partial Content\r\nContent-Type: {}\r\nContent-Length: {}\r\nContent-Range: bytes {}-{}/{}\r\nAccept-Ranges: bytes\r\n{}{}Cache-Control: no-store\r\n\r\n",
                    content_type, length, start, end, total_size, cors_headers, conn_header
                );
                if stream.write_all(header.as_bytes()).is_err() {
                    return;
                }

                let mut remaining = length;
                let mut buffer = [0u8; 65536];
                while remaining > 0 {
                    let to_read = (remaining as usize).min(buffer.len());
                    match file.read(&mut buffer[..to_read]) {
                        Ok(0) => break,
                        Ok(n) => {
                            if stream.write_all(&buffer[..n]).is_err() {
                                return;
                            }
                            remaining -= n as u64;
                        }
                        Err(_) => return,
                    }
                }
                let _ = stream.flush();
                return;
            }
            Some(Err(())) => {
                let header = format!(
                    "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{}\r\nContent-Type: text/plain\r\nContent-Length: 0\r\nAccept-Ranges: bytes\r\n{}{}Cache-Control: no-store\r\n\r\n",
                    total_size, cors_headers, conn_header
                );
                let _ = stream.write_all(header.as_bytes());
                let _ = stream.flush();
                return;
            }
            None => {
                // Invalid or unsupported Range header syntax: fall through to 200 OK stream
            }
        }
    }

    // Standard streaming full-file response using 64 KB stack buffer
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\n{}{}Cache-Control: no-store\r\n\r\n",
        content_type, total_size, cors_headers, conn_header
    );
    if stream.write_all(header.as_bytes()).is_err() {
        return;
    }

    let mut remaining = total_size;
    let mut buffer = [0u8; 65536];
    while remaining > 0 {
        let to_read = (remaining as usize).min(buffer.len());
        match file.read(&mut buffer[..to_read]) {
            Ok(0) => break,
            Ok(n) => {
                if stream.write_all(&buffer[..n]).is_err() {
                    return;
                }
                remaining -= n as u64;
            }
            Err(_) => return,
        }
    }
    let _ = stream.flush();
}

fn execute_self_test(
    port: u16,
    bundle: &EmbeddedUiBundle,
    handler: &dyn DesktopIpcHandler,
    session_token: &str,
    scoped_registry: &ScopedFileRegistry,
) -> Result<SelfTestReport, PlatformError> {
    // 1. Verify HTML endpoint over loopback TCP
    let (status_html, html_body) = loopback_http_request(port, "GET", "/", b"", None, &[])?;
    if status_html != 200 || html_body.len() < 200 {
        return Err(PlatformError::Unsupported(format!(
            "Self-test HTML check failed: status={} len={}",
            status_html,
            html_body.len()
        )));
    }

    // 2. Verify JS & CSS asset bundles over loopback TCP
    let (status_js, js_body) =
        loopback_http_request(port, "GET", "/assets/app.js", b"", None, &[])?;
    if status_js != 200 || js_body.len() < 1024 {
        return Err(PlatformError::Unsupported(format!(
            "Self-test JS bundle check failed: status={} len={}",
            status_js,
            js_body.len()
        )));
    }

    let (status_css, css_body) =
        loopback_http_request(port, "GET", "/assets/app.css", b"", None, &[])?;
    if status_css != 200 || css_body.is_empty() {
        return Err(PlatformError::Unsupported(format!(
            "Self-test CSS bundle check failed: status={} len={}",
            status_css,
            css_body.len()
        )));
    }

    // 3. Negative Security Gate: Verify unauthenticated and bad-token calls to /__* return 403 Forbidden
    let (status_unauth, _) = loopback_http_request(port, "POST", "/__fs/drives", b"", None, &[])?;
    if status_unauth != 403 {
        return Err(PlatformError::Unsupported(format!(
            "Loopback security gate check failed: unauthenticated /__fs/drives returned HTTP {} instead of expected 403 Forbidden",
            status_unauth
        )));
    }

    // Negative Security Gate: Invalid or expired bootstrap nonce rejection
    let (status_bad_nonce, _) = loopback_http_request(
        port,
        "GET",
        "/?bootstrap=invalid_or_consumed_nonce",
        b"",
        None,
        &[],
    )?;
    if status_bad_nonce != 403 {
        return Err(PlatformError::Unsupported(format!(
            "Negative bootstrap nonce check failed: candidate nonce returned status={} (expected 403)",
            status_bad_nonce
        )));
    }

    // Security Gate Assertion: Cookie-based session authentication succeeds for IPC
    let session_cookie = format!("__ag_session={}", session_token);
    let (status_cookie_auth, _) = loopback_http_request(
        port,
        "POST",
        "/__fs/drives",
        b"",
        None,
        &[("Cookie", &session_cookie)],
    )?;
    if status_cookie_auth != 200 {
        return Err(PlatformError::Unsupported(format!(
            "Cookie-based authentication check failed: status={} (expected 200)",
            status_cookie_auth
        )));
    }

    let (status_bad_token, _) = loopback_http_request(
        port,
        "POST",
        "/__fs/drives",
        b"",
        Some("invalid_token_12345"),
        &[],
    )?;
    if status_bad_token != 403 {
        return Err(PlatformError::Unsupported(format!(
            "Loopback security gate check failed: invalid token /__fs/drives returned HTTP {} instead of expected 403 Forbidden",
            status_bad_token
        )));
    }

    // Negative Security Gate: Cross-origin request rejection (SEC-02)
    let (status_cross_origin, _) = loopback_http_request(
        port,
        "POST",
        "/__fs/drives",
        b"",
        Some(session_token),
        &[("Origin", "https://malicious-website.com")],
    )?;
    if status_cross_origin != 403 {
        return Err(PlatformError::Unsupported(format!(
            "Cross-origin security gate failed: Origin https://malicious-website.com returned HTTP {} instead of expected 403 Forbidden",
            status_cross_origin
        )));
    }

    // Negative Security Gate: Sensitive path traversal rejection (SEC-03)
    let bad_path = format!(
        "/__file?path={}",
        url_encode("C:\\Windows\\System32\\config\\SAM")
    );
    let (status_sensitive, _) =
        loopback_http_request(port, "GET", &bad_path, b"", Some(session_token), &[])?;
    if status_sensitive != 403 {
        return Err(PlatformError::Unsupported(format!(
            "Sensitive path protection check failed: accessing sensitive path returned HTTP {} instead of expected 403 Forbidden",
            status_sensitive
        )));
    }

    // Negative Security Gate: Scoped capability allowlist rejection (SEC-03)
    let bad_unscoped_path = format!(
        "/__file?path={}",
        url_encode("C:\\arbitrary_forbidden_unscoped.txt")
    );
    let (status_unscoped, _) = loopback_http_request(
        port,
        "GET",
        &bad_unscoped_path,
        b"",
        Some(session_token),
        &[],
    )?;
    if status_unscoped != 403 {
        return Err(PlatformError::Unsupported(format!(
            "ScopedFileRegistry capability gate check failed: unscoped path returned HTTP {} instead of expected 403 Forbidden",
            status_unscoped
        )));
    }

    // 4. Positive Security Gate: Authenticated drive check
    let (status_auth_fs, _) =
        loopback_http_request(port, "POST", "/__fs/drives", b"", Some(session_token), &[])?;
    if status_auth_fs != 200 {
        return Err(PlatformError::Unsupported(format!(
            "Authenticated loopback check failed: valid token /__fs/drives returned HTTP {}",
            status_auth_fs
        )));
    }

    // 5. Positive Security Gate: Binary fast-path echo check (ADR-003)
    let test_bin_payload = b"AG_BINARY_FASTPATH_ECHO_TEST";
    let (status_bin, bin_response) = loopback_http_request(
        port,
        "POST",
        "/__ipc_bin/test_ping",
        test_bin_payload,
        Some(session_token),
        &[("Content-Type", "application/octet-stream")],
    )?;
    if status_bin != 200 || bin_response != test_bin_payload {
        return Err(PlatformError::Unsupported(format!(
            "Binary fast-path IPC check failed: status={} len={}",
            status_bin,
            bin_response.len()
        )));
    }

    // 6. Positive Security Gate: SSE Event stream handshake check (ADR-001)
    let (status_events, events_body) =
        loopback_http_request(port, "GET", "/__events", b"", Some(session_token), &[])?;
    if status_events != 200 || !events_body.starts_with(b"event: connected") {
        return Err(PlatformError::Unsupported(format!(
            "Event stream check failed: status={} body={}",
            status_events,
            String::from_utf8_lossy(&events_body)
        )));
    }

    // 7. Verify HTTP Range Streaming (206 Partial Content & 416 Range Not Satisfiable)
    let temp_range_file = std::env::temp_dir().join(format!("ag_range_test_{}.dat", port));
    let sample_payload = vec![0x42u8; 4096];
    if std::fs::write(&temp_range_file, &sample_payload).is_ok() {
        scoped_registry.allow_path(&temp_range_file);
        let test_path = temp_range_file.to_string_lossy().to_string();
        let range_path = format!("/__file?path={}", url_encode(&test_path));

        // Exact range check: bytes=1024-2047 (exactly 1024 bytes)
        let (status_range, range_bytes) = loopback_http_request(
            port,
            "GET",
            &range_path,
            b"",
            Some(session_token),
            &[("Range", "bytes=1024-2047")],
        )?;
        if status_range != 206 || range_bytes.len() != 1024 {
            let _ = std::fs::remove_file(&temp_range_file);
            return Err(PlatformError::Unsupported(format!(
                "Range streaming check failed: status={} len={} (expected 206 and 1024 bytes)",
                status_range,
                range_bytes.len()
            )));
        }

        // Out-of-bounds range check: bytes=10000-20000 -> 416
        let (status_oob, _) = loopback_http_request(
            port,
            "GET",
            &range_path,
            b"",
            Some(session_token),
            &[("Range", "bytes=10000-20000")],
        )?;
        if status_oob != 416 {
            let _ = std::fs::remove_file(&temp_range_file);
            return Err(PlatformError::Unsupported(format!(
                "Range streaming out-of-bounds check failed: status={} (expected 416)",
                status_oob
            )));
        }

        let _ = std::fs::remove_file(&temp_range_file);
    }

    // 6. Execute live IPC checks over HTTP POST /__ipc/<command> with session token
    let checks = handler.self_test_checks();
    let total = checks.len();
    let mut passed = 0usize;

    for check in checks {
        let path = format!("/__ipc/{}", check.command);
        let body = serde_json::to_vec(&check.payload).unwrap_or_default();
        let (status, resp_bytes) =
            loopback_http_request(port, "POST", &path, &body, Some(session_token), &[])?;
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
    auth_token: Option<&str>,
    extra_headers: &[(&str, &str)],
) -> Result<(u16, Vec<u8>), PlatformError> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;

    let auth_header = match auth_token {
        Some(tok) => format!("X-Antigravity-Token: {}\r\n", tok),
        None => String::new(),
    };

    let mut extra_str = String::new();
    for (name, val) in extra_headers {
        extra_str.push_str(&format!("{}: {}\r\n", name, val));
    }

    let req = format!(
        "{} {} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n{}{}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        method,
        path,
        port,
        auth_header,
        extra_str,
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

    // Security Gate Assertion (SEC-02): Loopback responses must never emit wildcard CORS
    if header_str.contains("Access-Control-Allow-Origin: *") {
        return Err(PlatformError::Unsupported(
            "CORS policy violation: server returned wildcard Access-Control-Allow-Origin: *"
                .to_string(),
        ));
    }
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

/// Seed clean, air-gapped browser preferences to strictly suppress First-Run Experience (FRE) prompts,
/// automatic Microsoft/Google account sign-in popups, and cloud data synchronization.
fn seed_airgapped_profile_preferences(profile_dir: &Path) {
    let default_dir = profile_dir.join("Default");
    let _ = std::fs::create_dir_all(&default_dir);

    // 1. Sanitize Default/Preferences: purge any attached Microsoft/Google accounts
    // and explicitly disable First-Run Experience (FRE), sync, and browser sign-in.
    let pref_path = default_dir.join("Preferences");
    let mut prefs: Value = if pref_path.exists() {
        std::fs::read_to_string(&pref_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if let Some(obj) = prefs.as_object_mut() {
        // Strip any cached or automatically linked MSA/Azure AD account tokens
        obj.remove("account_info");
        obj.remove("account_tracker_service_last_update");

        obj.insert("has_seen_signin_fre".to_string(), serde_json::json!(true));
        obj.insert("sync_consent_recorded".to_string(), serde_json::json!(true));
        obj.insert(
            "signin".to_string(),
            serde_json::json!({
                "allowed": false,
                "signin_with_explicit_browser_signin_on": false
            }),
        );
        obj.insert(
            "sync".to_string(),
            serde_json::json!({
                "has_setup_completed": true,
                "has_been_enabled": false,
                "keep_everything_synced": false,
                "apps": false,
                "autofill": false,
                "bookmarks": false,
                "history": false,
                "passwords": false,
                "preferences": false,
                "tabs": false
            }),
        );
        obj.insert(
            "profile".to_string(),
            serde_json::json!({
                "has_seen_signin_fre": true,
                "edge_san_is_option_explicitly_selectedby_user": true,
                "edge_account_type": 0
            }),
        );
    }

    if let Ok(serialized) = serde_json::to_string_pretty(&prefs) {
        let _ = std::fs::write(&pref_path, serialized);
    }

    // 2. Pre-seed Local State to mark initial onboarding, telemetry, and FRE as complete
    let local_state_path = profile_dir.join("Local State");
    let mut local_state: Value = if local_state_path.exists() {
        std::fs::read_to_string(&local_state_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if let Some(obj) = local_state.as_object_mut() {
        obj.insert(
            "fre".to_string(),
            serde_json::json!({ "has_user_seen_fre": true }),
        );
        obj.insert(
            "edge".to_string(),
            serde_json::json!({
                "fre": { "has_user_seen_fre": true }
            }),
        );
    }

    if let Ok(serialized) = serde_json::to_string_pretty(&local_state) {
        let _ = std::fs::write(&local_state_path, serialized);
    }
}

fn spawn_native_app_window(url: &str, bundle: &EmbeddedUiBundle) -> Option<Child> {
    let profile_dir = AppPaths::from_app_name(bundle.app_id)
        .map(|p| p.cache_dir.join("desktop_window_profile"))
        .unwrap_or_else(|_| std::env::temp_dir().join(format!("{}_window_profile", bundle.app_id)));
    let _ = std::fs::create_dir_all(&profile_dir);

    // Pre-seed clean air-gapped preferences into the isolated profile
    seed_airgapped_profile_preferences(&profile_dir);

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
                let is_edge = bin.to_ascii_lowercase().contains("edge");
                let privacy_flag = if is_edge {
                    "--inprivate"
                } else {
                    "--incognito"
                };

                if let Ok(child) = Command::new(bin)
                    .args([
                        &app_arg,
                        &size_arg,
                        &profile_arg,
                        privacy_flag,
                        "--no-first-run",
                        "--no-default-browser-check",
                        "--disable-sync",
                        "--disable-background-networking",
                        "--disable-component-update",
                        "--disable-default-apps",
                        "--disable-features=Translate,msEdgeSync,msEdgeIdentityCredential,msEdgeAccountConsistency,EdgeAccountConsistency,EdgeFre,msEdgeFre,ImplicitSignIn",
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
                let is_edge = bin.to_ascii_lowercase().contains("edge");
                let privacy_flag = if is_edge {
                    "--inprivate"
                } else {
                    "--incognito"
                };

                if let Ok(child) = Command::new(bin)
                    .args([
                        &app_arg,
                        &size_arg,
                        &profile_arg,
                        privacy_flag,
                        "--no-first-run",
                        "--no-default-browser-check",
                        "--disable-sync",
                        "--disable-background-networking",
                        "--disable-component-update",
                        "--disable-default-apps",
                        "--disable-features=Translate,msEdgeSync,msEdgeIdentityCredential,msEdgeAccountConsistency,EdgeAccountConsistency,EdgeFre,msEdgeFre,ImplicitSignIn",
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
            let is_edge = bin.to_ascii_lowercase().contains("edge");
            let privacy_flag = if is_edge {
                "--inprivate"
            } else {
                "--incognito"
            };

            if let Ok(child) = Command::new(bin)
                .args([
                    &app_arg,
                    &size_arg,
                    &profile_arg,
                    privacy_flag,
                    "--no-first-run",
                    "--no-default-browser-check",
                    "--disable-sync",
                    "--disable-background-networking",
                    "--disable-component-update",
                    "--disable-default-apps",
                    "--disable-features=Translate,msEdgeSync,msEdgeIdentityCredential,msEdgeAccountConsistency,EdgeAccountConsistency,EdgeFre,msEdgeFre,ImplicitSignIn",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_decode_utf8_and_plus() {
        assert_eq!(url_decode("hello%20world+test"), "hello world test");
        assert_eq!(
            url_decode("H%3A%5Crustapps%5C%E6%97%A5%E6%9C%AC"),
            r"H:\rustapps\日本"
        );
    }

    #[test]
    fn test_normalize_display_path_strips_verbatim_prefix() {
        assert_eq!(normalize_display_path(r"\\?\H:\rustapps"), r"H:\rustapps");
        assert_eq!(normalize_display_path(r"\\?\H:\"), r"H:\");
        assert_eq!(
            normalize_display_path(r"\\?\UNC\server\share\folder"),
            r"\\server\share\folder"
        );
        assert_eq!(normalize_display_path("/home/user/docs"), "/home/user/docs");
    }

    #[test]
    fn test_enumerate_drives_and_places_returns_non_empty() {
        let places = enumerate_drives_and_places();
        assert!(!places.is_empty());
        assert!(places
            .iter()
            .any(|p| p.kind == "drive" || p.kind == "quick_access"));
    }

    #[test]
    fn test_mime_type_detection() {
        assert_eq!(mime_type_for_path(Path::new("clip.mp4")), "video/mp4");
        assert_eq!(mime_type_for_path(Path::new("voice.wav")), "audio/wav");
        assert_eq!(mime_type_for_path(Path::new("frame.png")), "image/png");
    }

    #[test]
    fn test_bridge_injection_includes_dialog_and_fs() {
        let html = inject_bridge_into_html(
            "<html><head></head><body></body></html>",
            "TestApp",
            "test_session_token_12345",
        );
        assert!(html.contains("window.__TAURI__.dialog"));
        assert!(html.contains("window.__TAURI__.fs"));
        assert!(html.contains("<title>TestApp</title>"));
        assert!(html.contains(r#"const __AUTH_TOKEN__ = "test_session_token_12345";"#));
        assert!(html.contains("window.__ANTIGRAVITY_TOKEN__ = __AUTH_TOKEN__;"));
    }

    #[test]
    fn test_url_encode_and_decode_roundtrip() {
        let original = "H:\\rustapps\\test file & folder [v1.0].mp4";
        let encoded = url_encode(original);
        assert_eq!(url_decode(&encoded), original);
    }

    #[test]
    fn test_parse_byte_range_variants() {
        let total = 4096u64;

        // Explicit range
        assert_eq!(
            parse_byte_range("bytes=1024-2047", total),
            Some(Ok((1024, 2047)))
        );

        // Clamped explicit range
        assert_eq!(
            parse_byte_range("bytes=4000-5000", total),
            Some(Ok((4000, 4095)))
        );

        // Open-ended range
        assert_eq!(
            parse_byte_range("bytes=2048-", total),
            Some(Ok((2048, 4095)))
        );

        // Suffix range (last 512 bytes)
        assert_eq!(
            parse_byte_range("bytes=-512", total),
            Some(Ok((3584, 4095)))
        );

        // Out-of-bounds start
        assert_eq!(parse_byte_range("bytes=5000-", total), Some(Err(())));

        // Inverted range
        assert_eq!(parse_byte_range("bytes=2000-1000", total), Some(Err(())));

        // Non-range header
        assert_eq!(parse_byte_range("no-range", total), None);
    }

    #[test]
    fn test_thread_pool_bounded_execution() {
        let pool = ThreadPool::new(4);
        let counter = Arc::new(AtomicU64::new(0));

        for _ in 0..16 {
            let c = Arc::clone(&counter);
            pool.execute(move || {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }

        // Drop pool to join workers and verify all jobs ran
        drop(pool);
        assert_eq!(counter.load(Ordering::SeqCst), 16);
    }

    #[test]
    fn test_seed_airgapped_profile_preferences() {
        let temp_dir = std::env::temp_dir().join("test_airgapped_profile_prefs");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::create_dir_all(&temp_dir);

        // Pre-populate with a dirty state simulating an attached Microsoft account
        let default_dir = temp_dir.join("Default");
        let _ = std::fs::create_dir_all(&default_dir);
        let dirty_prefs = serde_json::json!({
            "account_info": [{"email": "test.user@microsoft.com", "full_name": "Test User"}],
            "has_seen_signin_fre": false,
            "signin": { "allowed": true }
        });
        let _ = std::fs::write(
            default_dir.join("Preferences"),
            serde_json::to_string(&dirty_prefs).unwrap(),
        );

        // Run air-gapped seed sanitizer
        seed_airgapped_profile_preferences(&temp_dir);

        // Verify Preferences sanitization
        let cleaned_str = std::fs::read_to_string(default_dir.join("Preferences")).unwrap();
        let cleaned: Value = serde_json::from_str(&cleaned_str).unwrap();
        assert!(
            cleaned.get("account_info").is_none(),
            "Stale account_info must be purged"
        );
        assert_eq!(cleaned["has_seen_signin_fre"], true);
        assert_eq!(cleaned["signin"]["allowed"], false);
        assert_eq!(cleaned["sync"]["has_been_enabled"], false);

        // Verify Local State FRE suppression
        let local_state_str = std::fs::read_to_string(temp_dir.join("Local State")).unwrap();
        let local_state: Value = serde_json::from_str(&local_state_str).unwrap();
        assert_eq!(local_state["fre"]["has_user_seen_fre"], true);
        assert_eq!(local_state["edge"]["fre"]["has_user_seen_fre"], true);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
    #[test]
    fn test_keep_alive_multiple_requests_on_single_tcp_stream() {
        use std::io::{BufRead, BufReader, Write};

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let shutdown = Arc::new(AtomicBool::new(false));
        let server_shutdown = Arc::clone(&shutdown);

        let bundle = EmbeddedUiBundle {
            app_id: "test-keepalive",
            app_title: "Test KeepAlive",
            version: "1.0.0",
            index_html: "<html><body>Hello KeepAlive</body></html>",
            app_js: b"console.log('js');",
            app_css: b"body { margin: 0; }",
            window_width: 1200,
            window_height: 800,
        };

        struct DummyHandler;
        impl DesktopIpcHandler for DummyHandler {
            fn handle_command(&self, _cmd: &str, _args: Value) -> Result<Value, String> {
                Ok(serde_json::json!({"status": "ok"}))
            }
            fn self_test_checks(&self) -> Vec<SelfTestCheck> {
                Vec::new()
            }
        }

        let last_hb = Arc::new(AtomicU64::new(0));
        let connected = Arc::new(AtomicBool::new(false));
        let reg = Arc::new(ScopedFileRegistry::new());
        let token = "test_token_12345678";
        let html = inject_bridge_into_html(bundle.index_html, bundle.app_title, token);

        let server_handle = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            handle_http_connection(
                stream,
                &bundle,
                &html,
                &DummyHandler,
                &last_hb,
                &connected,
                &server_shutdown,
                token,
                &reg,
                port,
                &std::sync::Mutex::new(None),
            );
        });

        // Client connects over a SINGLE persistent TcpStream
        let mut client = TcpStream::connect(("127.0.0.1", port)).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();

        // Request 1: keep-alive
        let req1 = format!(
            "GET /assets/app.css HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: keep-alive\r\n\r\n",
            port
        );
        client.write_all(req1.as_bytes()).unwrap();
        client.flush().unwrap();

        let mut reader = BufReader::new(client.try_clone().unwrap());
        let mut status_line1 = String::new();
        reader.read_line(&mut status_line1).unwrap();
        assert!(
            status_line1.contains("200 OK"),
            "Expected 200 OK: {}",
            status_line1
        );

        let mut content_len1 = 0usize;
        let mut has_keep_alive1 = false;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" || line.is_empty() {
                break;
            }
            if line.to_ascii_lowercase().starts_with("content-length:") {
                content_len1 = line.split_once(':').unwrap().1.trim().parse().unwrap();
            }
            if line.to_ascii_lowercase().contains("connection: keep-alive") {
                has_keep_alive1 = true;
            }
        }
        assert!(
            has_keep_alive1,
            "Response 1 must specify Connection: keep-alive"
        );
        let mut body1 = vec![0u8; content_len1];
        reader.read_exact(&mut body1).unwrap();
        assert_eq!(&body1, b"body { margin: 0; }");

        // Request 2: on the SAME client stream with Connection: close
        let req2 = format!(
            "GET /assets/app.js HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
            port
        );
        client.write_all(req2.as_bytes()).unwrap();
        client.flush().unwrap();

        let mut status_line2 = String::new();
        reader.read_line(&mut status_line2).unwrap();
        assert!(
            status_line2.contains("200 OK"),
            "Expected 200 OK: {}",
            status_line2
        );

        let mut content_len2 = 0usize;
        let mut has_close2 = false;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" || line.is_empty() {
                break;
            }
            if line.to_ascii_lowercase().starts_with("content-length:") {
                content_len2 = line.split_once(':').unwrap().1.trim().parse().unwrap();
            }
            if line.to_ascii_lowercase().contains("connection: close") {
                has_close2 = true;
            }
        }
        assert!(has_close2, "Response 2 must specify Connection: close");
        let mut body2 = vec![0u8; content_len2];
        reader.read_exact(&mut body2).unwrap();
        assert_eq!(&body2, b"console.log('js');");

        // Request 3: Confirm stream is closed by server (EOF or ConnectionReset)
        let mut eof_buf = [0u8; 1];
        match reader.read(&mut eof_buf) {
            Ok(0) => {}
            Err(ref e) if e.kind() == std::io::ErrorKind::ConnectionReset => {}
            other => panic!("Expected EOF or connection close, got {:?}", other),
        }

        shutdown.store(true, Ordering::SeqCst);
        let _ = server_handle.join();
    }

    #[test]
    fn test_unblock_server_listener_terminates_blocking_accept() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let shutdown = Arc::new(AtomicBool::new(false));
        let server_shutdown = Arc::clone(&shutdown);

        let handle = std::thread::spawn(move || {
            while !server_shutdown.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((_stream, _)) => {
                        if server_shutdown.load(Ordering::SeqCst) {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Initially server is blocked on listener.accept()
        std::thread::sleep(Duration::from_millis(50));
        assert!(!handle.is_finished());

        // Signal shutdown and unblock
        shutdown.store(true, Ordering::SeqCst);
        unblock_server_listener(port);

        // Server thread must exit promptly within 500ms
        let start = Instant::now();
        while !handle.is_finished() && start.elapsed() < Duration::from_millis(500) {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            handle.is_finished(),
            "Server thread must exit promptly upon unblocking"
        );
        let _ = handle.join();
    }
    #[test]
    fn test_one_time_bootstrap_nonce_exchange_sets_cookie_and_consumes_nonce() {
        use std::io::{BufRead, BufReader, Write};

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let shutdown = Arc::new(AtomicBool::new(false));
        let server_shutdown = Arc::clone(&shutdown);

        let bundle = EmbeddedUiBundle {
            app_id: "test-bootstrap",
            app_title: "Test Bootstrap",
            version: "1.0.0",
            index_html: "<html><body>Hello Bootstrap</body></html>",
            app_js: b"console.log('js');",
            app_css: b"body { margin: 0; }",
            window_width: 1200,
            window_height: 800,
        };

        struct DummyHandler;
        impl DesktopIpcHandler for DummyHandler {
            fn handle_command(&self, _cmd: &str, _args: Value) -> Result<Value, String> {
                Ok(serde_json::json!({"status": "ok"}))
            }
            fn self_test_checks(&self) -> Vec<SelfTestCheck> {
                Vec::new()
            }
        }

        let last_hb = Arc::new(AtomicU64::new(0));
        let connected = Arc::new(AtomicBool::new(false));
        let reg = Arc::new(ScopedFileRegistry::new());
        let token = "test_bootstrap_session_token_xyz987";
        let html = inject_bridge_into_html(bundle.index_html, bundle.app_title, token);

        let valid_nonce = "initial_launch_nonce_1234567890abcdef".to_string();
        let bootstrap_nonce = Arc::new(std::sync::Mutex::new(Some(valid_nonce.clone())));
        let server_boot = Arc::clone(&bootstrap_nonce);

        let server_handle = std::thread::spawn(move || {
            // Service 2 connections: first for valid nonce, second for replay attempt
            for _ in 0..2 {
                if let Ok((stream, _)) = listener.accept() {
                    let sd = Arc::clone(&server_shutdown);
                    handle_http_connection(
                        stream,
                        &bundle,
                        &html,
                        &DummyHandler,
                        &last_hb,
                        &connected,
                        &sd,
                        token,
                        &reg,
                        port,
                        &server_boot,
                    );
                }
            }
        });

        // 1. First connection: send valid bootstrap nonce
        let mut client1 = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let req1 = format!(
            "GET /?bootstrap={} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
            valid_nonce, port
        );
        client1.write_all(req1.as_bytes()).unwrap();
        client1.flush().unwrap();

        let mut reader1 = BufReader::new(client1);
        let mut status_line1 = String::new();
        reader1.read_line(&mut status_line1).unwrap();
        assert!(
            status_line1.contains("200 OK"),
            "Expected 200 OK: {}",
            status_line1
        );

        let mut set_cookie_found = false;
        loop {
            let mut line = String::new();
            reader1.read_line(&mut line).unwrap();
            if line == "\r\n" || line.is_empty() {
                break;
            }
            if line.to_ascii_lowercase().starts_with("set-cookie:") {
                assert!(
                    line.contains("__ag_session="),
                    "Set-Cookie must contain __ag_session"
                );
                assert!(
                    line.contains(token),
                    "Set-Cookie must contain session token"
                );
                assert!(line.contains("HttpOnly"), "Set-Cookie must be HttpOnly");
                assert!(
                    line.contains("SameSite=Strict"),
                    "Set-Cookie must be SameSite=Strict"
                );
                set_cookie_found = true;
            }
        }
        assert!(
            set_cookie_found,
            "Server must emit Set-Cookie header upon bootstrap exchange"
        );

        // Verify nonce was consumed and is now None
        assert!(
            bootstrap_nonce.lock().unwrap().is_none(),
            "Bootstrap nonce must be consumed immediately"
        );

        // 2. Second connection: replay the consumed bootstrap nonce (must be rejected 403 Forbidden)
        let mut client2 = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let req2 = format!(
            "GET /?bootstrap={} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
            valid_nonce, port
        );
        client2.write_all(req2.as_bytes()).unwrap();
        client2.flush().unwrap();

        let mut reader2 = BufReader::new(client2);
        let mut status_line2 = String::new();
        reader2.read_line(&mut status_line2).unwrap();
        assert!(
            status_line2.contains("403 Forbidden"),
            "Replayed bootstrap nonce must return 403: {}",
            status_line2
        );

        shutdown.store(true, Ordering::SeqCst);
        let _ = server_handle.join();
    }
    #[test]
    fn test_apply_dialog_filters_configures_extensions_and_directory() {
        let opts = serde_json::json!({
            "defaultPath": "C:\\temp\\subfolder\\sample.txt",
            "filters": [
                { "name": "Text Documents", "extensions": ["txt", ".md"] },
                { "name": "All Media", "extensions": ["mp4", "png", "*"] }
            ]
        });
        let dialog = rfd::FileDialog::new();
        let _configured = apply_dialog_filters(dialog, &opts);
    }

    #[test]
    fn test_scoped_file_registry_allow_and_isolation() {
        let registry = ScopedFileRegistry::new();
        let temp_dir = std::env::temp_dir();
        let allowed_folder = temp_dir.join("ag_scoped_test_folder");
        let allowed_file = allowed_folder.join("allowed.txt");
        let foreign_folder = temp_dir.join("ag_foreign_folder");
        let foreign_file = foreign_folder.join("forbidden.txt");

        let _ = std::fs::create_dir_all(&allowed_folder);
        let _ = std::fs::create_dir_all(&foreign_folder);
        let _ = std::fs::write(&allowed_file, b"ok");
        let _ = std::fs::write(&foreign_file, b"forbidden");

        registry.allow_root(&allowed_folder);

        assert!(registry.is_allowed(&allowed_folder));
        assert!(registry.is_allowed(&allowed_file));
        assert!(!registry.is_allowed(&foreign_file));

        registry.allow_path(&foreign_file);
        assert!(registry.is_allowed(&foreign_file));

        let _ = std::fs::remove_dir_all(&allowed_folder);
        let _ = std::fs::remove_dir_all(&foreign_folder);
    }
}
