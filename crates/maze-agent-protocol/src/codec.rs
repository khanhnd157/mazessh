use crate::error::AgentError;
use crate::message::*;

/// Maximum allowed SSH agent message size (256 KB). Messages larger than this
/// are rejected to prevent DoS via memory exhaustion.
const MAX_MESSAGE_SIZE: usize = 262_144;

// ─── Wire format helpers ─────────────────────────────────────────

/// Read a 4-byte big-endian u32 from a buffer at the given offset.
fn read_u32(buf: &[u8], offset: usize) -> Result<(u32, usize), AgentError> {
    if buf.len() < offset + 4 {
        return Err(AgentError::TooShort {
            need: offset + 4,
            got: buf.len(),
        });
    }
    let val = u32::from_be_bytes([buf[offset], buf[offset + 1], buf[offset + 2], buf[offset + 3]]);
    Ok((val, offset + 4))
}

/// Read an SSH "string" (4-byte length + data) from a buffer at the given offset.
fn read_string(buf: &[u8], offset: usize) -> Result<(Vec<u8>, usize), AgentError> {
    let (len, pos) = read_u32(buf, offset)?;
    let len = len as usize;
    if len > MAX_MESSAGE_SIZE {
        return Err(AgentError::InvalidFormat(
            format!("string too large: {} bytes", len),
        ));
    }
    if buf.len() < pos + len {
        return Err(AgentError::TooShort {
            need: pos + len,
            got: buf.len(),
        });
    }
    let data = buf[pos..pos + len].to_vec();
    Ok((data, pos + len))
}

/// Write a 4-byte big-endian u32 to a buffer.
fn write_u32(buf: &mut Vec<u8>, val: u32) {
    buf.extend_from_slice(&val.to_be_bytes());
}

/// Write an SSH "string" (4-byte length + data) to a buffer.
fn write_string(buf: &mut Vec<u8>, data: &[u8]) {
    write_u32(buf, data.len() as u32);
    buf.extend_from_slice(data);
}

// ─── Decode ──────────────────────────────────────────────────────

/// Decode a single SSH agent message from raw bytes.
///
/// Input: the FULL wire message INCLUDING the 4-byte length prefix.
/// Returns the parsed message.
pub fn decode_message(buf: &[u8]) -> Result<AgentMessage, AgentError> {
    if buf.len() < 5 {
        return Err(AgentError::TooShort {
            need: 5,
            got: buf.len(),
        });
    }

    let (msg_len, _) = read_u32(buf, 0)?;
    let msg_len = msg_len as usize;
    if msg_len > MAX_MESSAGE_SIZE {
        return Err(AgentError::InvalidFormat(
            format!("message too large: {} bytes (max {})", msg_len, MAX_MESSAGE_SIZE),
        ));
    }
    let total_len = 4 + msg_len;
    if buf.len() < total_len {
        return Err(AgentError::TooShort {
            need: total_len,
            got: buf.len(),
        });
    }

    let msg_type = buf[4];
    let payload = &buf[5..total_len];

    match msg_type {
        SSH_AGENTC_REQUEST_IDENTITIES => Ok(AgentMessage::RequestIdentities),

        SSH_AGENTC_SIGN_REQUEST => {
            let (key_blob, pos) = read_string(payload, 0)?;
            let (data, pos) = read_string(payload, pos)?;
            let (flags, _) = if pos + 4 <= payload.len() {
                read_u32(payload, pos)?
            } else {
                (0u32, pos)
            };
            Ok(AgentMessage::SignRequest {
                key_blob,
                data,
                flags,
            })
        }

        SSH_AGENTC_ADD_IDENTITY | SSH_AGENTC_ADD_ID_CONSTRAINED => {
            Ok(AgentMessage::AddIdentity {
                raw: payload.to_vec(),
            })
        }

        SSH_AGENTC_REMOVE_IDENTITY => {
            let (key_blob, _) = read_string(payload, 0)?;
            Ok(AgentMessage::RemoveIdentity { key_blob })
        }

        SSH_AGENTC_REMOVE_ALL_IDENTITIES => Ok(AgentMessage::RemoveAllIdentities),

        SSH_AGENTC_EXTENSION => {
            let (name_bytes, pos) = read_string(payload, 0)?;
            let name = String::from_utf8(name_bytes)
                .map_err(|e| AgentError::InvalidFormat(format!("extension name: {e}")))?;
            let data = payload[pos..].to_vec();
            Ok(AgentMessage::Extension { name, data })
        }

        _ => Ok(AgentMessage::Unknown {
            msg_type,
            payload: payload.to_vec(),
        }),
    }
}

// ─── Encode ──────────────────────────────────────────────────────

/// Encode an SSH agent response into wire format bytes.
///
/// Returns the FULL wire message INCLUDING the 4-byte length prefix.
pub fn encode_message(response: &AgentResponse) -> Vec<u8> {
    let mut payload = Vec::new();

    match response {
        AgentResponse::Failure => {
            payload.push(SSH_AGENT_FAILURE);
        }

        AgentResponse::Success => {
            payload.push(SSH_AGENT_SUCCESS);
        }

        AgentResponse::IdentitiesAnswer { identities } => {
            payload.push(SSH_AGENT_IDENTITIES_ANSWER);
            write_u32(&mut payload, identities.len() as u32);
            for (key_blob, comment) in identities {
                write_string(&mut payload, key_blob);
                write_string(&mut payload, comment.as_bytes());
            }
        }

        AgentResponse::SignResponse { signature_blob } => {
            payload.push(SSH_AGENT_SIGN_RESPONSE);
            write_string(&mut payload, signature_blob);
        }
    }

    // Prepend length
    let mut msg = Vec::with_capacity(4 + payload.len());
    write_u32(&mut msg, payload.len() as u32);
    msg.extend_from_slice(&payload);
    msg
}

/// Read a framed message from a byte stream.
/// Returns `(message_bytes, remaining_bytes)`.
/// The returned `message_bytes` includes the 4-byte length prefix.
/// Returns `None` if there aren't enough bytes for a complete message.
pub fn try_read_frame(buf: &[u8]) -> Option<(Vec<u8>, usize)> {
    if buf.len() < 4 {
        return None;
    }
    let msg_len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    // Reject oversized messages to prevent DoS / integer overflow
    if msg_len > MAX_MESSAGE_SIZE {
        return None;
    }
    let total = 4 + msg_len; // safe: msg_len <= MAX_MESSAGE_SIZE, no overflow
    if buf.len() < total {
        return None;
    }
    Some((buf[..total].to_vec(), total))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request_identities() -> Vec<u8> {
        // length=1 (just the type byte), type=11
        vec![0, 0, 0, 1, SSH_AGENTC_REQUEST_IDENTITIES]
    }

    fn make_sign_request(key: &[u8], data: &[u8], flags: u32) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.push(SSH_AGENTC_SIGN_REQUEST);
        write_string(&mut payload, key);
        write_string(&mut payload, data);
        write_u32(&mut payload, flags);

        let mut msg = Vec::new();
        write_u32(&mut msg, payload.len() as u32);
        msg.extend_from_slice(&payload);
        msg
    }

    #[test]
    fn decode_request_identities() {
        let buf = make_request_identities();
        let msg = decode_message(&buf).unwrap();
        assert!(matches!(msg, AgentMessage::RequestIdentities));
    }

    #[test]
    fn decode_sign_request() {
        let key = b"fake-key-blob";
        let data = b"data-to-sign";
        let buf = make_sign_request(key, data, SSH_AGENT_RSA_SHA2_256);
        let msg = decode_message(&buf).unwrap();
        match msg {
            AgentMessage::SignRequest {
                key_blob,
                data: d,
                flags,
            } => {
                assert_eq!(key_blob, key);
                assert_eq!(d, data);
                assert_eq!(flags, SSH_AGENT_RSA_SHA2_256);
            }
            _ => panic!("expected SignRequest"),
        }
    }

    #[test]
    fn decode_too_short() {
        let buf = vec![0, 0, 0];
        assert!(decode_message(&buf).is_err());
    }

    #[test]
    fn encode_failure() {
        let msg = encode_message(&AgentResponse::Failure);
        assert_eq!(msg, vec![0, 0, 0, 1, SSH_AGENT_FAILURE]);
    }

    #[test]
    fn encode_success() {
        let msg = encode_message(&AgentResponse::Success);
        assert_eq!(msg, vec![0, 0, 0, 1, SSH_AGENT_SUCCESS]);
    }

    #[test]
    fn encode_identities_answer() {
        let msg = encode_message(&AgentResponse::IdentitiesAnswer {
            identities: vec![
                (b"key1".to_vec(), "comment1".to_string()),
            ],
        });
        // Decode: len=4bytes, type=12, count=1, string("key1"), string("comment1")
        assert_eq!(msg[4], SSH_AGENT_IDENTITIES_ANSWER);
        // Verify count
        let count = u32::from_be_bytes([msg[5], msg[6], msg[7], msg[8]]);
        assert_eq!(count, 1);
    }

    #[test]
    fn encode_sign_response() {
        let sig = b"signature-data";
        let msg = encode_message(&AgentResponse::SignResponse {
            signature_blob: sig.to_vec(),
        });
        assert_eq!(msg[4], SSH_AGENT_SIGN_RESPONSE);
    }

    #[test]
    fn round_trip_identities() {
        let response = AgentResponse::IdentitiesAnswer {
            identities: vec![
                (b"key-blob-1".to_vec(), "test key".to_string()),
                (b"key-blob-2".to_vec(), "other key".to_string()),
            ],
        };
        let encoded = encode_message(&response);
        // Should be a valid frame
        let (frame, consumed) = try_read_frame(&encoded).unwrap();
        assert_eq!(consumed, encoded.len());
        assert_eq!(frame, encoded);
    }

    #[test]
    fn try_read_frame_incomplete() {
        assert!(try_read_frame(&[0, 0, 0]).is_none());
        assert!(try_read_frame(&[0, 0, 0, 10, 1]).is_none()); // says 10 bytes but only 1
    }

    #[test]
    fn try_read_frame_exact() {
        let buf = vec![0, 0, 0, 2, 11, 0]; // 2-byte payload
        let (frame, consumed) = try_read_frame(&buf).unwrap();
        assert_eq!(consumed, 6);
        assert_eq!(frame, buf);
    }

    #[test]
    fn decode_remove_all() {
        let buf = vec![0, 0, 0, 1, SSH_AGENTC_REMOVE_ALL_IDENTITIES];
        let msg = decode_message(&buf).unwrap();
        assert!(matches!(msg, AgentMessage::RemoveAllIdentities));
    }

    #[test]
    fn decode_unknown_type() {
        let buf = vec![0, 0, 0, 3, 99, 0xAB, 0xCD]; // type 99
        let msg = decode_message(&buf).unwrap();
        match msg {
            AgentMessage::Unknown { msg_type, payload } => {
                assert_eq!(msg_type, 99);
                assert_eq!(payload, vec![0xAB, 0xCD]);
            }
            _ => panic!("expected Unknown"),
        }
    }
}
