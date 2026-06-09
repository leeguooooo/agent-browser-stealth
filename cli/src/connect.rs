//! `agent-browser connect` — zero-confirmation control of the user's real,
//! logged-in Chrome via the `ab-connect` MV3 extension over Chrome **native
//! messaging** (no localhost port, no token; Chrome authenticates the extension
//! to this host by id).
//!
//! Two pieces live here:
//! - `run_connect` — `--install` writes the native-messaging host manifest (and
//!   a tiny launcher) so Chrome will spawn us; with no flag it reports status.
//! - `run_nm_host` — the hidden `__nm-host` mode Chrome launches: it speaks the
//!   native-messaging stdio framing (4-byte little-endian length + JSON).
//!
//! This step wires the transport end-to-end (Chrome ⇄ host). Bridging the host
//! to the daemon's relay + CdpClient is layered on next.

use std::io::Write;
use std::path::PathBuf;

/// Native-messaging host name; must match `HOST_NAME` in the extension and the
/// manifest filename.
pub const HOST_NAME: &str = "com.agent_browser.connect";

/// Stable id of the `ab-connect` extension, pinned by the `key` in its
/// manifest.json. Chrome only lets that extension talk to this host.
pub const EXTENSION_ID: &str = "bdoiejojpjogcjojeladhioioijhgade";

/// `agent-browser extension <install|uninstall|status>` (local; no daemon).
/// `args` is the cleaned argv including the leading "extension".
pub fn run_connect(args: &[String], json: bool) {
    let install = args.iter().any(|a| a == "--install" || a == "install");
    let uninstall = args.iter().any(|a| a == "--uninstall" || a == "uninstall");

    if uninstall {
        let removed = remove_host_manifests();
        report(json, true, &format!("removed {removed} native-host manifest(s)"));
        return;
    }
    if install {
        match install_native_host() {
            Ok(paths) => {
                if json {
                    println!(
                        "{}",
                        serde_json::to_string(&serde_json::json!({
                            "success": true,
                            "data": { "installed": paths, "extensionId": EXTENSION_ID }
                        }))
                        .unwrap_or_default()
                    );
                } else {
                    println!("✓ native-messaging host installed:");
                    for p in &paths {
                        println!("  {p}");
                    }
                    println!(
                        "\nNext: load the ab-connect extension in Chrome (chrome://extensions →\n\
                         Developer mode → Load unpacked → extensions/ab-connect), then this host\n\
                         is reachable with no token and no per-use confirmation."
                    );
                }
            }
            Err(e) => report(json, false, &format!("install failed: {e}")),
        }
        return;
    }

    // Status.
    let manifest = host_manifest_path_for_chrome();
    let installed = manifest.as_ref().map(|p| p.exists()).unwrap_or(false);
    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "success": true,
                "data": {
                    "installed": installed,
                    "manifest": manifest.as_ref().map(|p| p.display().to_string()),
                    "extensionId": EXTENSION_ID,
                }
            }))
            .unwrap_or_default()
        );
    } else if installed {
        println!("✓ native-messaging host installed ({HOST_NAME}).");
        println!("  Load the ab-connect extension and it connects automatically.");
    } else {
        println!("✗ not installed. Run: agent-browser connect --install");
    }
}

/// Write the launcher script + native-messaging host manifest(s).
fn install_native_host() -> Result<Vec<String>, String> {
    let home = dirs::home_dir().ok_or("no home dir")?;
    let ab_dir = home.join(".agent-browser");
    std::fs::create_dir_all(&ab_dir).map_err(|e| e.to_string())?;

    // Chrome execs the manifest `path` directly with the calling extension's
    // origin as argv[1]; a launcher lets us run the binary in __nm-host mode
    // regardless of how/where agent-browser is installed.
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let launcher = ab_dir.join("nm-host.sh");
    let script = format!(
        "#!/bin/sh\n# agent-browser native-messaging host launcher (auto-generated)\nexec \"{}\" __nm-host \"$@\"\n",
        exe.display()
    );
    std::fs::write(&launcher, script).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o755));
    }

    let manifest = serde_json::json!({
        "name": HOST_NAME,
        "description": "agent-browser connect — native messaging host",
        "path": launcher.display().to_string(),
        "type": "stdio",
        "allowed_origins": [format!("chrome-extension://{EXTENSION_ID}/")],
    });
    let body = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;

    let mut written = Vec::new();
    for dir in native_messaging_dirs() {
        if let Some(parent) = dir.parent() {
            if !parent.exists() {
                continue; // that browser isn't installed
            }
        }
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join(format!("{HOST_NAME}.json"));
        std::fs::write(&path, &body).map_err(|e| e.to_string())?;
        written.push(path.display().to_string());
    }
    if written.is_empty() {
        return Err("no Chrome/Chromium NativeMessagingHosts directory found".into());
    }
    Ok(written)
}

fn remove_host_manifests() -> usize {
    let mut n = 0;
    for dir in native_messaging_dirs() {
        let path = dir.join(format!("{HOST_NAME}.json"));
        if path.exists() && std::fs::remove_file(&path).is_ok() {
            n += 1;
        }
    }
    n
}

/// Per-OS NativeMessagingHosts directories for Chrome + Chromium-family browsers.
fn native_messaging_dirs() -> Vec<PathBuf> {
    let mut dirs_out = Vec::new();
    #[cfg(target_os = "macos")]
    {
        if let Some(app_support) = dirs::config_dir() {
            for sub in [
                "Google/Chrome",
                "Google/Chrome Beta",
                "Google/Chrome Canary",
                "Chromium",
                "Microsoft Edge",
                "BraveSoftware/Brave-Browser",
            ] {
                dirs_out.push(app_support.join(sub).join("NativeMessagingHosts"));
            }
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(config) = dirs::config_dir() {
            for sub in ["google-chrome", "chromium", "microsoft-edge", "BraveSoftware/Brave-Browser"] {
                dirs_out.push(config.join(sub).join("NativeMessagingHosts"));
            }
        }
    }
    dirs_out
}

fn host_manifest_path_for_chrome() -> Option<PathBuf> {
    native_messaging_dirs()
        .into_iter()
        .map(|d| d.join(format!("{HOST_NAME}.json")))
        .find(|p| p.exists())
        .or_else(|| {
            native_messaging_dirs()
                .into_iter()
                .next()
                .map(|d| d.join(format!("{HOST_NAME}.json")))
        })
}

fn report(json: bool, ok: bool, msg: &str) {
    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({ "success": ok, "error": if ok { serde_json::Value::Null } else { serde_json::json!(msg) }, "message": msg }))
                .unwrap_or_default()
        );
    } else if ok {
        println!("✓ {msg}");
    } else {
        eprintln!("✗ {msg}");
    }
    if !ok {
        std::process::exit(1);
    }
}

// ---- native messaging host (`__nm-host`) ----------------------------------

fn nm_log(line: &str) {
    let path = dirs::home_dir()
        .map(|h| h.join(".agent-browser").join("nm-host.log"))
        .unwrap_or_else(|| PathBuf::from("/tmp/ab-nm-host.log"));
    if let Some(p) = path.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "{line}");
    }
}

fn random_guid() -> String {
    let mut b = [0u8; 16];
    let _ = getrandom::getrandom(&mut b);
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Where the daemon/CLI reads the relay's CDP WebSocket URL (perms 600).
fn relay_url_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".agent-browser").join("relay-cdp-url"))
        .unwrap_or_else(|| PathBuf::from("/tmp/ab-relay-cdp-url"))
}

/// The live relay CDP WebSocket URL, if the native-messaging host is running
/// (it writes the file on connect and removes it on exit). Used by
/// `agent-browser extension connect` to attach without the user copying a URL.
pub fn relay_url() -> Option<String> {
    let s = std::fs::read_to_string(relay_url_path()).ok()?;
    let s = s.trim().to_string();
    if s.starts_with("ws://") {
        Some(s)
    } else {
        None
    }
}

/// Hidden `__nm-host` mode: launched by Chrome for the ab-connect extension.
///
/// Bridges the extension (native-messaging stdio, envelope protocol) to a local
/// **CDP WebSocket endpoint** that agent-browser connects to like any Chrome.
/// `relay::RelayState` translates envelope ⇄ raw CDP and emulates browser-level
/// Target discovery. The ws URL carries an unguessable guid (written to a 600
/// file) so only this user's agent-browser — not arbitrary local processes —
/// can drive the browser. No token, no user interaction.
pub fn run_nm_host() {
    let rt = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(e) => {
            nm_log(&format!("[nm-host] runtime build failed: {e}"));
            return;
        }
    };
    rt.block_on(nm_host_main());
}

async fn nm_host_main() {
    use crate::native::relay::{RelayOut, RelayState};
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::sync::{broadcast, mpsc, Mutex};

    nm_log(&format!(
        "[nm-host] start argv={:?}",
        std::env::args().skip(1).collect::<Vec<_>>()
    ));

    let listener = match tokio::net::TcpListener::bind("127.0.0.1:0").await {
        Ok(l) => l,
        Err(e) => {
            nm_log(&format!("[nm-host] bind failed: {e}"));
            return;
        }
    };
    let port = listener.local_addr().map(|a| a.port()).unwrap_or(0);
    let guid = random_guid();
    let url = format!("ws://127.0.0.1:{port}/{guid}");
    let url_path = relay_url_path();
    if let Some(p) = url_path.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    if std::fs::write(&url_path, &url).is_ok() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&url_path, std::fs::Permissions::from_mode(0o600));
        }
    }
    nm_log(&format!("[nm-host] cdp endpoint {url}"));

    let state = Arc::new(Mutex::new(RelayState::new()));
    let (to_clients, _) = broadcast::channel::<String>(4096);
    let (to_ext, mut to_ext_rx) = mpsc::channel::<Vec<u8>>(4096);

    // Single writer to Chrome (extension) over stdout, native-messaging framed.
    tokio::spawn(async move {
        let mut out = tokio::io::stdout();
        while let Some(frame) = to_ext_rx.recv().await {
            let len = (frame.len() as u32).to_ne_bytes();
            if out.write_all(&len).await.is_err() || out.write_all(&frame).await.is_err() {
                break;
            }
            let _ = out.flush().await;
        }
    });

    // Accept agent-browser CDP clients on the guid-scoped ws endpoint.
    {
        let state = state.clone();
        let to_clients = to_clients.clone();
        let to_ext = to_ext.clone();
        let guid = guid.clone();
        tokio::spawn(async move {
            loop {
                let (stream, _) = match listener.accept().await {
                    Ok(x) => x,
                    Err(_) => break,
                };
                let st = state.clone();
                let rx = to_clients.subscribe();
                let tx = to_ext.clone();
                let g = guid.clone();
                tokio::spawn(async move {
                    handle_cdp_client(stream, g, st, rx, tx).await;
                });
            }
        });
    }

    // Extension → host frames.
    let mut stdin = tokio::io::stdin();
    loop {
        let mut len_buf = [0u8; 4];
        if stdin.read_exact(&mut len_buf).await.is_err() {
            break;
        }
        let len = u32::from_ne_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        if stdin.read_exact(&mut buf).await.is_err() {
            break;
        }
        let v: serde_json::Value = match serde_json::from_slice(&buf) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let outs = {
            let mut s = state.lock().await;
            s.handle_ext_message(&v, "")
        };
        for o in outs {
            match o {
                RelayOut::ToClient(m) => {
                    let _ = to_clients.send(m.to_string());
                }
                RelayOut::ToExt(m) => {
                    let _ = to_ext.send(m.to_string().into_bytes()).await;
                }
            }
        }
    }
    nm_log("[nm-host] stdin EOF — Chrome closed the port");
    let _ = std::fs::remove_file(relay_url_path());
}

async fn handle_cdp_client(
    stream: tokio::net::TcpStream,
    guid: String,
    state: std::sync::Arc<tokio::sync::Mutex<crate::native::relay::RelayState>>,
    mut from_relay: tokio::sync::broadcast::Receiver<String>,
    to_ext: tokio::sync::mpsc::Sender<Vec<u8>>,
) {
    use crate::native::relay::ClientRoute;
    use futures_util::{SinkExt, StreamExt};
    use tokio::sync::broadcast::error::RecvError;
    use tokio_tungstenite::tungstenite::Message;

    let want_path = format!("/{guid}");
    let cb = |req: &tokio_tungstenite::tungstenite::handshake::server::Request,
              resp: tokio_tungstenite::tungstenite::handshake::server::Response| {
        if req.uri().path() == want_path {
            Ok(resp)
        } else {
            let mut reject = tokio_tungstenite::tungstenite::handshake::server::ErrorResponse::new(
                Some("forbidden".to_string()),
            );
            *reject.status_mut() = tokio_tungstenite::tungstenite::http::StatusCode::FORBIDDEN;
            Err(reject)
        }
    };
    let ws = match tokio_tungstenite::accept_hdr_async(stream, cb).await {
        Ok(ws) => ws,
        Err(_) => return,
    };
    nm_log("[nm-host] cdp client connected");
    // Ask the extension to (re)attach + announce every tab so this client
    // discovers the user's existing tabs instead of racing an empty list.
    let _ = to_ext.send(br#"{"method":"attachAll"}"#.to_vec()).await;
    let (mut tx, mut rx) = ws.split();
    loop {
        tokio::select! {
            relayed = from_relay.recv() => match relayed {
                Ok(text) => { if tx.send(Message::Text(text)).await.is_err() { break } }
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            },
            incoming = rx.next() => match incoming {
                Some(Ok(Message::Text(text))) => {
                    let v: serde_json::Value = match serde_json::from_str(&text) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let route = { state.lock().await.route_client_command(&v) };
                    match route {
                        ClientRoute::Local(reply) => {
                            if tx.send(Message::Text(reply.to_string())).await.is_err() { break }
                        }
                        ClientRoute::Forward(env) => {
                            let _ = to_ext.send(env.to_string().into_bytes()).await;
                        }
                    }
                }
                Some(Ok(Message::Close(_))) | None => break,
                _ => {}
            },
        }
    }
    nm_log("[nm-host] cdp client disconnected");
}
