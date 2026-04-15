/// SSH Agent Protocol message types per draft-miller-ssh-agent.
///
/// Wire format: [4-byte big-endian length][1-byte type][payload...]
/// Length includes the type byte but NOT the 4-byte length itself.

// ─── Message type constants ──────────────────────────────────────

// Requests (client → agent)
pub const SSH_AGENTC_REQUEST_IDENTITIES: u8 = 11;
pub const SSH_AGENTC_SIGN_REQUEST: u8 = 13;
pub const SSH_AGENTC_ADD_IDENTITY: u8 = 17;
pub const SSH_AGENTC_REMOVE_IDENTITY: u8 = 18;
pub const SSH_AGENTC_REMOVE_ALL_IDENTITIES: u8 = 19;
pub const SSH_AGENTC_ADD_ID_CONSTRAINED: u8 = 25;
pub const SSH_AGENTC_EXTENSION: u8 = 27;

// Responses (agent → client)
pub const SSH_AGENT_FAILURE: u8 = 5;
pub const SSH_AGENT_SUCCESS: u8 = 6;
pub const SSH_AGENT_IDENTITIES_ANSWER: u8 = 12;
pub const SSH_AGENT_SIGN_RESPONSE: u8 = 14;
pub const SSH_AGENT_EXTENSION_FAILURE: u8 = 28;

// Sign flags
pub const SSH_AGENT_RSA_SHA2_256: u32 = 2;
pub const SSH_AGENT_RSA_SHA2_512: u32 = 4;

// ─── Parsed message types ────────────────────────────────────────

/// A parsed SSH agent protocol message.
#[derive(Debug, Clone)]
pub enum AgentMessage {
    /// Client requests the list of public keys (type 11)
    RequestIdentities,

    /// Client requests a signature (type 13)
    SignRequest {
        /// The public key blob to identify which key to use
        key_blob: Vec<u8>,
        /// The data to sign
        data: Vec<u8>,
        /// Flags (e.g. RSA_SHA2_256, RSA_SHA2_512)
        flags: u32,
    },

    /// Client wants to add an identity (type 17) — we reject this
    AddIdentity { raw: Vec<u8> },

    /// Client wants to remove a specific identity (type 18)
    RemoveIdentity { key_blob: Vec<u8> },

    /// Client wants to remove all identities (type 19)
    RemoveAllIdentities,

    /// Extension message (type 27)
    Extension { name: String, data: Vec<u8> },

    /// Any other message type we don't handle
    Unknown { msg_type: u8, payload: Vec<u8> },
}

/// A response message to send back to the client.
#[derive(Debug, Clone)]
pub enum AgentResponse {
    /// Generic failure (type 5)
    Failure,

    /// Generic success (type 6)
    Success,

    /// List of identities (type 12)
    IdentitiesAnswer {
        /// List of (public_key_blob, comment) pairs
        identities: Vec<(Vec<u8>, String)>,
    },

    /// Signature response (type 14)
    SignResponse {
        /// The full signature blob (algorithm string + signature data, SSH-encoded)
        signature_blob: Vec<u8>,
    },
}

/// An SSH public key identity for the agent to serve.
#[derive(Debug, Clone)]
pub struct AgentIdentity {
    /// The SSH wire-format public key blob
    pub key_blob: Vec<u8>,
    /// Human-readable comment
    pub comment: String,
}
