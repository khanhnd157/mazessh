use std::sync::Arc;

use maze_agent_protocol::{
    decode_message, encode_message, try_read_frame, AgentMessage, AgentResponse,
};
use maze_vault::SshKeyVault;
use ssh_key::PublicKey;
use tauri::{Emitter, Manager};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Notify;

use crate::services::{audit_service, policy_service};
use crate::state::{AppState, ConsentDecision, PendingConsent};

/// The named pipe path for the MazeSSH agent on Windows.
pub const PIPE_NAME: &str = r"\\.\pipe\maze-ssh-agent";

/// Start the SSH agent daemon as a background task.
/// Listens on a Windows named pipe and handles SSH agent protocol messages.
/// Uses the Tauri AppHandle to access the shared AppState.
pub fn start_agent_daemon(app_handle: tauri::AppHandle) -> Arc<Notify> {
    let shutdown = Arc::new(Notify::new());
    let shutdown_clone = shutdown.clone();

    tauri::async_runtime::spawn(async move {
        if let Err(e) = run_agent_loop(app_handle, shutdown_clone).await {
            eprintln!("[maze-agent] daemon error: {e}");
        }
    });

    shutdown
}

#[cfg(windows)]
async fn run_agent_loop(
    app_handle: tauri::AppHandle,
    shutdown: Arc<Notify>,
) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::net::windows::named_pipe::ServerOptions;

    // Limit concurrent connections to prevent resource exhaustion
    let semaphore = Arc::new(tokio::sync::Semaphore::new(100));

    loop {
        // Create a new pipe server instance
        let server = match ServerOptions::new()
            .first_pipe_instance(false)
            .create(PIPE_NAME)
        {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[maze-agent] failed to create pipe: {e}");
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                continue;
            }
        };

        tokio::select! {
            _ = shutdown.notified() => {
                return Ok(());
            }
            result = server.connect() => {
                if let Err(e) = result {
                    eprintln!("[maze-agent] connect error: {e}");
                    continue;
                }
                let handle = app_handle.clone();
                let permit = match semaphore.clone().try_acquire_owned() {
                    Ok(p) => p,
                    Err(_) => {
                        eprintln!("[maze-agent] max connections reached, rejecting client");
                        drop(server);
                        continue;
                    }
                };
                tokio::spawn(async move {
                    let _permit = permit; // held until task completes
                    if let Err(e) = handle_connection(server, handle).await {
                        let msg = e.to_string();
                        if !msg.contains("broken pipe") && !msg.contains("end of file") {
                            eprintln!("[maze-agent] connection error: {e}");
                        }
                    }
                });
            }
        }
    }
}

#[cfg(not(windows))]
async fn run_agent_loop(
    _app_handle: tauri::AppHandle,
    shutdown: Arc<Notify>,
) -> Result<(), Box<dyn std::error::Error>> {
    // On non-Windows, just wait for shutdown (named pipes are Windows-only)
    shutdown.notified().await;
    Ok(())
}

#[cfg(windows)]
async fn handle_connection(
    mut pipe: tokio::net::windows::named_pipe::NamedPipeServer,
    app_handle: tauri::AppHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    // Identify the connecting process
    let client_info = get_pipe_client_info(&pipe);

    const MAX_PENDING: usize = 1_048_576; // 1MB max pending buffer

    let mut buf = vec![0u8; 65536];
    let mut pending = Vec::new();

    loop {
        let n = pipe.read(&mut buf).await?;
        if n == 0 {
            break; // client disconnected
        }

        pending.extend_from_slice(&buf[..n]);

        // Reject clients that accumulate too much unprocessed data (DoS protection)
        if pending.len() > MAX_PENDING {
            eprintln!("[maze-agent] pending buffer overflow ({} bytes), disconnecting client", pending.len());
            break;
        }

        // Process all complete frames in the buffer
        while let Some((frame, consumed)) = try_read_frame(&pending) {
            let state = app_handle.state::<AppState>();
            let response = match decode_message(&frame) {
                Ok(msg) => handle_message(msg, &state, &app_handle, &client_info).await,
                Err(e) => {
                    eprintln!("[maze-agent] decode error: {e}");
                    AgentResponse::Failure
                }
            };

            let response_bytes = encode_message(&response);
            pipe.write_all(&response_bytes).await?;

            pending = pending[consumed..].to_vec();
        }
    }

    Ok(())
}

/// Information about the process that connected to the pipe.
#[derive(Debug, Clone, Default)]
struct ClientInfo {
    pub process_name: String,
    pub process_path: String,
    pub pid: u32,
}

#[cfg(windows)]
fn get_pipe_client_info(pipe: &tokio::net::windows::named_pipe::NamedPipeServer) -> ClientInfo {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::Pipes::GetNamedPipeClientProcessId;

    let mut pid: u32 = 0;
    let handle = pipe.as_raw_handle() as windows_sys::Win32::Foundation::HANDLE;

    unsafe {
        if GetNamedPipeClientProcessId(handle, &mut pid) == 0 {
            return ClientInfo::default();
        }
    }

    // Get process name/path from PID
    get_process_info(pid)
}

#[cfg(windows)]
fn get_process_info(pid: u32) -> ClientInfo {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::ProcessStatus::GetModuleFileNameExW;
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};

    let handle = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid) };
    if handle.is_null() {
        return ClientInfo {
            pid,
            process_name: format!("PID {pid}"),
            process_path: String::new(),
        };
    }

    let mut buf = [0u16; 512];
    let len = unsafe { GetModuleFileNameExW(handle, std::ptr::null_mut(), buf.as_mut_ptr(), buf.len() as u32) };
    unsafe { CloseHandle(handle) };

    if len == 0 {
        return ClientInfo {
            pid,
            process_name: format!("PID {pid}"),
            process_path: String::new(),
        };
    }

    let path = OsString::from_wide(&buf[..len as usize])
        .to_string_lossy()
        .to_string();
    let name = path.rsplit(['\\', '/']).next().unwrap_or(&path).to_string();

    ClientInfo {
        pid,
        process_name: name,
        process_path: path,
    }
}

/// Handle a single parsed SSH agent message and return a response.
async fn handle_message(
    msg: AgentMessage,
    app_state: &AppState,
    app_handle: &tauri::AppHandle,
    client_info: &ClientInfo,
) -> AgentResponse {
    match msg {
        AgentMessage::RequestIdentities => handle_request_identities(app_state),
        AgentMessage::SignRequest {
            key_blob,
            data,
            flags: _,
        } => handle_sign_request(app_state, app_handle, &key_blob, &data, client_info).await,
        AgentMessage::RemoveAllIdentities => AgentResponse::Success,
        AgentMessage::AddIdentity { .. } => {
            // We don't accept add-identity — keys are managed via vault UI
            AgentResponse::Failure
        }
        AgentMessage::RemoveIdentity { .. } => AgentResponse::Success,
        AgentMessage::Extension { .. } => AgentResponse::Failure,
        AgentMessage::Unknown { .. } => AgentResponse::Failure,
    }
}

/// Return the list of public keys from the vault.
fn handle_request_identities(app_state: &AppState) -> AgentResponse {
    // Check if vault is unlocked
    let session_guard = match app_state.vault_session.lock() {
        Ok(g) => g,
        Err(_) => return AgentResponse::IdentitiesAnswer { identities: vec![] },
    };

    if session_guard.is_none() {
        return AgentResponse::IdentitiesAnswer { identities: vec![] };
    }
    drop(session_guard);

    // List all active keys from vault metadata
    let keys = match SshKeyVault::list_keys(&app_state.vault_dir) {
        Ok(k) => k,
        Err(_) => return AgentResponse::IdentitiesAnswer { identities: vec![] },
    };

    let mut identities = Vec::new();
    for key_summary in &keys {
        if key_summary.state != maze_vault::KeyState::Active {
            continue;
        }
        if let Ok(key_item) = SshKeyVault::get_key(&key_summary.id, &app_state.vault_dir) {
            // Parse the OpenSSH public key to get the wire-format blob
            if let Ok(pub_key) = PublicKey::from_openssh(&key_item.public_key_openssh) {
                if let Ok(key_data) = pub_key.to_bytes() {
                    identities.push((key_data.to_vec(), key_item.name.clone()));
                }
            }
        }
    }

    AgentResponse::IdentitiesAnswer { identities }
}

/// Sign data with the key matching the given key_blob.
/// Checks policy rules first; if no rule matches, opens consent popup (60s timeout).
async fn handle_sign_request(
    app_state: &AppState,
    app_handle: &tauri::AppHandle,
    key_blob: &[u8],
    data: &[u8],
    client_info: &ClientInfo,
) -> AgentResponse {
    // Find which vault key matches the key_blob
    let key_id = match find_key_by_blob(app_state, key_blob) {
        Some(id) => id,
        None => return AgentResponse::Failure,
    };

    let key_item = match SshKeyVault::get_key(&key_id, &app_state.vault_dir) {
        Ok(k) => k,
        Err(_) => return AgentResponse::Failure,
    };

    // ── Policy check: skip consent if rule matches ──
    // Check "always" persistent rule
    if policy_service::has_always_rule(&app_state.vault_dir, &key_id) {
        return perform_sign(app_state, &key_id, data);
    }
    // Check "session" rule
    if app_state.session_rules.is_allowed(&key_id) {
        return perform_sign(app_state, &key_id, data);
    }

    // ── No rule — show consent popup ──
    let consent_id = uuid::Uuid::new_v4().to_string();
    let (tx, rx) = tokio::sync::oneshot::channel::<ConsentDecision>();

    // Store pending consent
    {
        let mut consents = match app_state.pending_consents.lock() {
            Ok(c) => c,
            Err(_) => return AgentResponse::Failure,
        };
        consents.insert(
            consent_id.clone(),
            PendingConsent {
                key_id: key_id.clone(),
                key_name: key_item.name.clone(),
                process_name: client_info.process_name.clone(),
                host: "unknown".to_string(),
                tx,
            },
        );
    }

    // Open consent popup window and emit event
    open_consent_window(app_handle);
    let _ = app_handle.emit(
        "consent-request",
        serde_json::json!({
            "consent_id": consent_id,
            "key_name": key_item.name,
            "key_fingerprint": key_item.fingerprint,
            "process_name": client_info.process_name,
            "process_path": client_info.process_path,
            "pid": client_info.pid,
        }),
    );

    // Wait for user response with 60s timeout
    let decision = match tokio::time::timeout(
        std::time::Duration::from_secs(60),
        rx,
    )
    .await
    {
        Ok(Ok(d)) => d,
        Ok(Err(_)) => {
            // Channel closed without response
            audit_service::log_action("consent_dropped", Some(&key_item.name), "channel closed");
            cleanup_consent(app_state, &consent_id);
            return AgentResponse::Failure;
        }
        Err(_) => {
            // Timeout — 60s elapsed without user response
            audit_service::log_action("consent_timeout", Some(&key_item.name), "auto-denied after 60s");
            cleanup_consent(app_state, &consent_id);
            return AgentResponse::Failure;
        }
    };

    if !decision.approved {
        return AgentResponse::Failure;
    }

    // Use the key_id from the decision (user may have selected a different key)
    let sign_key_id = if decision.selected_key_id.is_empty() {
        key_id
    } else {
        decision.selected_key_id
    };

    // Save policy rule based on user's choice
    match decision.allow_mode.as_str() {
        "session" => {
            app_state.session_rules.allow(&sign_key_id);
        }
        "always" => {
            let key_name = SshKeyVault::get_key(&sign_key_id, &app_state.vault_dir)
                .map(|k| k.name)
                .unwrap_or_default();
            let _ = policy_service::add_always_rule(&app_state.vault_dir, &sign_key_id, &key_name);
        }
        _ => {} // "once" — no caching
    }

    perform_sign(app_state, &sign_key_id, data)
}

/// Perform the actual signing operation (shared between policy-cached and consent flows).
fn perform_sign(app_state: &AppState, key_id: &str, data: &[u8]) -> AgentResponse {
    let guard = match app_state.vault_session.lock() {
        Ok(g) => g,
        Err(_) => return AgentResponse::Failure,
    };
    let session = match guard.as_ref() {
        Some(s) => s,
        None => return AgentResponse::Failure,
    };

    match SshKeyVault::sign(session, key_id, data, &app_state.vault_dir) {
        Ok(signature_bytes) => {
            let signed_key = match SshKeyVault::get_key(key_id, &app_state.vault_dir) {
                Ok(k) => k,
                Err(_) => return AgentResponse::Failure,
            };

            let alg_name = match signed_key.algorithm {
                maze_vault::KeyAlgorithm::Ed25519 => "ssh-ed25519",
                maze_vault::KeyAlgorithm::Rsa4096 => "rsa-sha2-512",
            };

            let mut sig_blob = Vec::new();
            let alg_bytes = alg_name.as_bytes();
            sig_blob.extend_from_slice(&(alg_bytes.len() as u32).to_be_bytes());
            sig_blob.extend_from_slice(alg_bytes);
            sig_blob.extend_from_slice(&(signature_bytes.len() as u32).to_be_bytes());
            sig_blob.extend_from_slice(&signature_bytes);

            audit_service::log_action("agent_sign", Some(&signed_key.name), "success");
            AgentResponse::SignResponse {
                signature_blob: sig_blob,
            }
        }
        Err(e) => {
            eprintln!("[maze-agent] sign error: {e}");
            audit_service::log_action("agent_sign", Some(key_id), &format!("failed: {e}"));
            AgentResponse::Failure
        }
    }
}

/// Open the consent popup as a Tauri window (or focus it if already open).
fn open_consent_window(app_handle: &tauri::AppHandle) {
    use tauri::WebviewWindowBuilder;

    // If the consent window already exists, just focus it
    if let Some(window) = app_handle.get_webview_window("consent") {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }

    // Create new consent popup window
    let _ = WebviewWindowBuilder::new(
        app_handle,
        "consent",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("SSH Signing Consent")
    .inner_size(420.0, 520.0)
    .resizable(false)
    .decorations(false)
    .always_on_top(true)
    .center()
    .build();
}

fn cleanup_consent(app_state: &AppState, consent_id: &str) {
    if let Ok(mut consents) = app_state.pending_consents.lock() {
        consents.remove(consent_id);
    }
}

/// Find a vault key whose public key wire-format blob matches the request.
/// Uses constant-time comparison to prevent timing side-channel attacks.
fn find_key_by_blob(app_state: &AppState, target_blob: &[u8]) -> Option<String> {
    use subtle::ConstantTimeEq;

    let keys = SshKeyVault::list_keys(&app_state.vault_dir).ok()?;

    for key_summary in &keys {
        if key_summary.state != maze_vault::KeyState::Active {
            continue;
        }
        if let Ok(key_item) = SshKeyVault::get_key(&key_summary.id, &app_state.vault_dir) {
            if let Ok(pub_key) = PublicKey::from_openssh(&key_item.public_key_openssh) {
                if let Ok(blob) = pub_key.to_bytes() {
                    if blob.len() == target_blob.len()
                        && bool::from(blob.ct_eq(target_blob))
                    {
                        return Some(key_item.id);
                    }
                }
            }
        }
    }

    None
}
