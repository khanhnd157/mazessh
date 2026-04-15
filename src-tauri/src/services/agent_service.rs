use std::sync::Arc;

use maze_agent_protocol::{
    decode_message, encode_message, try_read_frame, AgentMessage, AgentResponse,
};
use maze_vault::SshKeyVault;
use ssh_key::PublicKey;
use tauri::{Emitter, Manager};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Notify;

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

    loop {
        // Create a new pipe server instance
        let server = match ServerOptions::new()
            .first_pipe_instance(false)
            .create(PIPE_NAME)
        {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[maze-agent] failed to create pipe: {e}");
                // Retry after a delay (pipe might be in use)
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
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(server, handle).await {
                        // Client disconnected is normal, only log real errors
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
    let mut buf = vec![0u8; 65536];
    let mut pending = Vec::new();

    loop {
        let n = pipe.read(&mut buf).await?;
        if n == 0 {
            break; // client disconnected
        }

        pending.extend_from_slice(&buf[..n]);

        // Process all complete frames in the buffer
        while let Some((frame, consumed)) = try_read_frame(&pending) {
            let state = app_handle.state::<AppState>();
            let response = match decode_message(&frame) {
                Ok(msg) => handle_message(msg, &state, &app_handle).await,
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

/// Handle a single parsed SSH agent message and return a response.
async fn handle_message(
    msg: AgentMessage,
    app_state: &AppState,
    app_handle: &tauri::AppHandle,
) -> AgentResponse {
    match msg {
        AgentMessage::RequestIdentities => handle_request_identities(app_state),
        AgentMessage::SignRequest {
            key_blob,
            data,
            flags: _,
        } => handle_sign_request(app_state, app_handle, &key_blob, &data).await,
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
/// Opens a consent popup and waits for user approval (60s timeout).
async fn handle_sign_request(
    app_state: &AppState,
    app_handle: &tauri::AppHandle,
    key_blob: &[u8],
    data: &[u8],
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

    // Create consent request with oneshot channel
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
                process_name: "ssh/git".to_string(),
                host: "unknown".to_string(),
                tx,
            },
        );
    }

    // Emit event to frontend so it can show the consent popup
    let _ = app_handle.emit(
        "consent-request",
        serde_json::json!({
            "consent_id": consent_id,
            "key_name": key_item.name,
            "key_fingerprint": key_item.fingerprint,
            "process_name": "ssh/git",
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
            cleanup_consent(app_state, &consent_id);
            return AgentResponse::Failure;
        }
        Err(_) => {
            // Timeout
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

    // Perform the actual signing
    let guard = match app_state.vault_session.lock() {
        Ok(g) => g,
        Err(_) => return AgentResponse::Failure,
    };
    let session = match guard.as_ref() {
        Some(s) => s,
        None => return AgentResponse::Failure,
    };

    match SshKeyVault::sign(session, &sign_key_id, data, &app_state.vault_dir) {
        Ok(signature_bytes) => {
            let signed_key = match SshKeyVault::get_key(&sign_key_id, &app_state.vault_dir) {
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

            AgentResponse::SignResponse {
                signature_blob: sig_blob,
            }
        }
        Err(e) => {
            eprintln!("[maze-agent] sign error: {e}");
            AgentResponse::Failure
        }
    }
}

fn cleanup_consent(app_state: &AppState, consent_id: &str) {
    if let Ok(mut consents) = app_state.pending_consents.lock() {
        consents.remove(consent_id);
    }
}

/// Find a vault key whose public key wire-format blob matches the request.
fn find_key_by_blob(app_state: &AppState, target_blob: &[u8]) -> Option<String> {
    let keys = SshKeyVault::list_keys(&app_state.vault_dir).ok()?;

    for key_summary in &keys {
        if key_summary.state != maze_vault::KeyState::Active {
            continue;
        }
        if let Ok(key_item) = SshKeyVault::get_key(&key_summary.id, &app_state.vault_dir) {
            if let Ok(pub_key) = PublicKey::from_openssh(&key_item.public_key_openssh) {
                if let Ok(blob) = pub_key.to_bytes() {
                    if &blob[..] == target_blob {
                        return Some(key_item.id);
                    }
                }
            }
        }
    }

    None
}
