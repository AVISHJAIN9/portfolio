//! Minimal In-Tree TLS 1.3 Implementation for QUIC (RFC 8446, RFC 8448, RFC 9001)
//!
//! Scope (ADR-027):
//! - Protocol: TLS 1.3 only
//! - Cipher Suite: TLS_CHACHA20_POLY1305_SHA256 (0x1303)
//! - Key Exchange: X25519 (0x001d)
//! - Signature Scheme: ed25519 (0x0807)
//! - ALPN: "mini-quic-test"
//! - Extension: quic_transport_parameters (0x0039 / 57)
//! - Crates: pf-crypto (hkdf, x25519, chacha20poly1305, ed25519-dalek, sha2, hmac)

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::fmt;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TlsError {
    UnexpectedMessage,
    InvalidLength,
    InvalidEncoding(String),
    UnsupportedCipherSuite(u16),
    UnsupportedKeyShareGroup(u16),
    UnsupportedSignatureScheme(u16),
    FinishedMacMismatch,
    SignatureVerificationFailed,
    MissingExtension(u16),
}

impl fmt::Display for TlsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for TlsError {}

// Handshake Message Types (RFC 8446 Section 4)
pub const MSG_CLIENT_HELLO: u8 = 0x01;
pub const MSG_SERVER_HELLO: u8 = 0x02;
pub const MSG_ENCRYPTED_EXTENSIONS: u8 = 0x08;
pub const MSG_CERTIFICATE: u8 = 0x0b;
pub const MSG_CERTIFICATE_VERIFY: u8 = 0x0f;
pub const MSG_FINISHED: u8 = 0x14;

// TLS 1.3 Extension Types
pub const EXT_SERVER_NAME: u16 = 0x0000;
pub const EXT_ALPN: u16 = 0x0010;
pub const EXT_SUPPORTED_GROUPS: u16 = 0x000a;
pub const EXT_SIGNATURE_ALGORITHMS: u16 = 0x000d;
pub const EXT_KEY_SHARE: u16 = 0x0033;
pub const EXT_SUPPORTED_VERSIONS: u16 = 0x002b;
pub const EXT_QUIC_TRANSPORT_PARAMETERS: u16 = 0x0039;
pub const EXT_QUIC_TRANSPORT_PARAMETERS_DRAFT: u16 = 0xffa5;

// Named Groups & Cipher Suites
pub const GROUP_X25519: u16 = 0x001d;
pub const CIPHER_TLS_CHACHA20_POLY1305_SHA256: u16 = 0x1303;
pub const SIG_ED25519: u16 = 0x0807;
pub const TLS_VERSION_1_3: u16 = 0x0304;

/// Computes SHA-256 digest
pub fn sha256_digest(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let out = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&out);
    hash
}

/// RFC 8446 Section 7.1 Key Schedule
pub struct TlsKeySchedule;

impl TlsKeySchedule {
    /// Early Secret = HKDF-Extract(salt=0, ikm=0)
    pub fn derive_early_secret() -> [u8; 32] {
        let zero = [0u8; 32];
        pf_crypto::hkdf_extract(&zero, &zero)
    }

    /// Derived Secret = Derive-Secret(Secret, "derived", "")
    pub fn derive_secret(secret: &[u8; 32], label: &str, transcript_hash: &[u8]) -> [u8; 32] {
        let mut out = [0u8; 32];
        pf_crypto::hkdf_expand_label(secret, label, transcript_hash, &mut out);
        out
    }

    /// Handshake Secret = HKDF-Extract(salt=Derive-Secret(Early Secret, "derived", ""), ikm=ECDHE)
    pub fn derive_handshake_secret(early_secret: &[u8; 32], shared_secret: &[u8; 32]) -> [u8; 32] {
        let empty_hash = sha256_digest(b"");
        let derived_early = Self::derive_secret(early_secret, "derived", &empty_hash);
        pf_crypto::hkdf_extract(&derived_early, shared_secret)
    }

    /// Client / Server Handshake Traffic Secret
    pub fn derive_client_handshake_traffic_secret(handshake_secret: &[u8; 32], transcript_hash: &[u8; 32]) -> [u8; 32] {
        Self::derive_secret(handshake_secret, "c hs traffic", transcript_hash)
    }

    pub fn derive_server_handshake_traffic_secret(handshake_secret: &[u8; 32], transcript_hash: &[u8; 32]) -> [u8; 32] {
        Self::derive_secret(handshake_secret, "s hs traffic", transcript_hash)
    }

    /// Master Secret = HKDF-Extract(salt=Derive-Secret(Handshake Secret, "derived", ""), ikm=0)
    pub fn derive_master_secret(handshake_secret: &[u8; 32]) -> [u8; 32] {
        let empty_hash = sha256_digest(b"");
        let derived_hs = Self::derive_secret(handshake_secret, "derived", &empty_hash);
        let zero = [0u8; 32];
        pf_crypto::hkdf_extract(&derived_hs, &zero)
    }

    /// Client / Server Application Traffic Secret
    pub fn derive_client_app_traffic_secret(master_secret: &[u8; 32], transcript_hash: &[u8; 32]) -> [u8; 32] {
        Self::derive_secret(master_secret, "c ap traffic", transcript_hash)
    }

    pub fn derive_server_app_traffic_secret(master_secret: &[u8; 32], transcript_hash: &[u8; 32]) -> [u8; 32] {
        Self::derive_secret(master_secret, "s ap traffic", transcript_hash)
    }

    /// Finished Key = HKDF-Expand-Label(BaseKey, "finished", "", Hash.length)
    pub fn derive_finished_key(base_key: &[u8; 32]) -> [u8; 32] {
        let mut finished_key = [0u8; 32];
        pf_crypto::hkdf_expand_label(base_key, "finished", b"", &mut finished_key);
        finished_key
    }

    /// Computes verify_data = HMAC-SHA256(finished_key, transcript_hash)
    pub fn compute_verify_data(finished_key: &[u8; 32], transcript_hash: &[u8; 32]) -> [u8; 32] {
        let mut mac = HmacSha256::new_from_slice(finished_key).unwrap();
        mac.update(transcript_hash);
        let mut verify_data = [0u8; 32];
        verify_data.copy_from_slice(&mac.finalize().into_bytes());
        verify_data
    }

    /// QUIC Key, IV, and Header Protection derivation per RFC 9001 Section 5.1 (ChaCha20-Poly1305)
    pub fn derive_quic_keys(secret: &[u8; 32]) -> ([u8; 32], [u8; 12], [u8; 32]) {
        let mut key = [0u8; 32];
        let mut iv = [0u8; 12];
        let mut hp = [0u8; 32];
        pf_crypto::hkdf_expand_label(secret, "quic key", b"", &mut key);
        pf_crypto::hkdf_expand_label(secret, "quic iv", b"", &mut iv);
        pf_crypto::hkdf_expand_label(secret, "quic hp", b"", &mut hp);
        (key, iv, hp)
    }
}

/// QUIC Transport Parameters (RFC 9000 Section 18.2)
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QuicTransportParameters {
    pub original_destination_connection_id: Option<Vec<u8>>,
    pub initial_source_connection_id: Option<Vec<u8>>,
    pub max_idle_timeout: Option<u64>,
    pub initial_max_data: Option<u64>,
    pub initial_max_stream_data_bidi_local: Option<u64>,
    pub initial_max_stream_data_bidi_remote: Option<u64>,
    pub initial_max_stream_data_uni: Option<u64>,
    pub initial_max_streams_bidi: Option<u64>,
    pub initial_max_streams_uni: Option<u64>,
    pub max_packet_size: Option<u64>,
    pub active_connection_id_limit: Option<u64>,
    pub disable_active_migration: bool,
}

impl QuicTransportParameters {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        if let Some(ref cid) = self.original_destination_connection_id {
            push_bytes_param(&mut buf, 0x00, cid);
        }
        if let Some(val) = self.max_idle_timeout {
            push_varint_param(&mut buf, 0x01, val);
        }
        if let Some(val) = self.max_packet_size {
            push_varint_param(&mut buf, 0x03, val);
        }
        if let Some(val) = self.initial_max_data {
            push_varint_param(&mut buf, 0x04, val);
        }
        if let Some(val) = self.initial_max_stream_data_bidi_local {
            push_varint_param(&mut buf, 0x05, val);
        }
        if let Some(val) = self.initial_max_stream_data_bidi_remote {
            push_varint_param(&mut buf, 0x06, val);
        }
        if let Some(val) = self.initial_max_stream_data_uni {
            push_varint_param(&mut buf, 0x07, val);
        }
        if let Some(val) = self.initial_max_streams_bidi {
            push_varint_param(&mut buf, 0x08, val);
        }
        if let Some(val) = self.initial_max_streams_uni {
            push_varint_param(&mut buf, 0x09, val);
        }
        if self.disable_active_migration {
            encode_varint(0x0c, &mut buf);
            encode_varint(0, &mut buf);
        }
        if let Some(val) = self.active_connection_id_limit {
            push_varint_param(&mut buf, 0x0e, val);
        }
        if let Some(ref cid) = self.initial_source_connection_id {
            push_bytes_param(&mut buf, 0x0f, cid);
        }

        buf
    }

    pub fn decode(mut data: &[u8]) -> Result<Self, TlsError> {
        let mut params = QuicTransportParameters::default();

        while !data.is_empty() {
            let (param_id, consumed) = decode_varint(data).ok_or_else(|| TlsError::InvalidEncoding("Truncated param id".into()))?;
            data = &data[consumed..];

            let (len, consumed_len) = decode_varint(data).ok_or_else(|| TlsError::InvalidEncoding("Truncated param len".into()))?;
            data = &data[consumed_len..];

            if data.len() < len as usize {
                return Err(TlsError::InvalidEncoding("Truncated param val".into()));
            }

            let val_bytes = &data[..len as usize];
            data = &data[len as usize..];

            match param_id {
                0x00 => params.original_destination_connection_id = Some(val_bytes.to_vec()),
                0x01 => {
                    let (v, _) = decode_varint(val_bytes).ok_or_else(|| TlsError::InvalidEncoding("Bad varint val".into()))?;
                    params.max_idle_timeout = Some(v);
                }
                0x03 => {
                    let (v, _) = decode_varint(val_bytes).ok_or_else(|| TlsError::InvalidEncoding("Bad varint val".into()))?;
                    params.max_packet_size = Some(v);
                }
                0x04 => {
                    let (v, _) = decode_varint(val_bytes).ok_or_else(|| TlsError::InvalidEncoding("Bad varint val".into()))?;
                    params.initial_max_data = Some(v);
                }
                0x05 => {
                    let (v, _) = decode_varint(val_bytes).ok_or_else(|| TlsError::InvalidEncoding("Bad varint val".into()))?;
                    params.initial_max_stream_data_bidi_local = Some(v);
                }
                0x06 => {
                    let (v, _) = decode_varint(val_bytes).ok_or_else(|| TlsError::InvalidEncoding("Bad varint val".into()))?;
                    params.initial_max_stream_data_bidi_remote = Some(v);
                }
                0x07 => {
                    let (v, _) = decode_varint(val_bytes).ok_or_else(|| TlsError::InvalidEncoding("Bad varint val".into()))?;
                    params.initial_max_stream_data_uni = Some(v);
                }
                0x08 => {
                    let (v, _) = decode_varint(val_bytes).ok_or_else(|| TlsError::InvalidEncoding("Bad varint val".into()))?;
                    params.initial_max_streams_bidi = Some(v);
                }
                0x09 => {
                    let (v, _) = decode_varint(val_bytes).ok_or_else(|| TlsError::InvalidEncoding("Bad varint val".into()))?;
                    params.initial_max_streams_uni = Some(v);
                }
                0x0c => params.disable_active_migration = true,
                0x0e => {
                    let (v, _) = decode_varint(val_bytes).ok_or_else(|| TlsError::InvalidEncoding("Bad varint val".into()))?;
                    params.active_connection_id_limit = Some(v);
                }
                0x0f => params.initial_source_connection_id = Some(val_bytes.to_vec()),
                _ => {} // Ignore unknown parameters per RFC
            }
        }

        Ok(params)
    }
}

fn encode_varint(val: u64, buf: &mut Vec<u8>) {
    if val <= 63 {
        buf.push(val as u8);
    } else if val <= 16383 {
        let b = (val as u16) | 0x4000;
        buf.extend_from_slice(&b.to_be_bytes());
    } else if val <= 1073741823 {
        let b = (val as u32) | 0x80000000;
        buf.extend_from_slice(&b.to_be_bytes());
    } else {
        let b = val | 0xc000000000000000;
        buf.extend_from_slice(&b.to_be_bytes());
    }
}

fn decode_varint(buf: &[u8]) -> Option<(u64, usize)> {
    if buf.is_empty() {
        return None;
    }
    let first = buf[0];
    let prefix = first >> 6;
    let len = 1 << prefix;
    if buf.len() < len {
        return None;
    }
    let mut val = (first & 0x3f) as u64;
    for &b in &buf[1..len] {
        val = (val << 8) | (b as u64);
    }
    Some((val, len))
}

fn push_varint_param(buf: &mut Vec<u8>, id: u64, val: u64) {
    encode_varint(id, buf);
    let mut val_bytes = Vec::new();
    encode_varint(val, &mut val_bytes);
    encode_varint(val_bytes.len() as u64, buf);
    buf.extend_from_slice(&val_bytes);
}

fn push_bytes_param(buf: &mut Vec<u8>, id: u64, bytes: &[u8]) {
    encode_varint(id, buf);
    encode_varint(bytes.len() as u64, buf);
    buf.extend_from_slice(bytes);
}

/// TLS 1.3 Handshake Message Representation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TlsHandshakeMessage {
    ClientHello {
        random: [u8; 32],
        legacy_session_id: Vec<u8>,
        cipher_suites: Vec<u16>,
        key_share: [u8; 32],
        alpn: Vec<String>,
        transport_parameters: Vec<u8>,
    },
    ServerHello {
        random: [u8; 32],
        legacy_session_id_echo: Vec<u8>,
        cipher_suite: u16,
        key_share: [u8; 32],
    },
    EncryptedExtensions {
        alpn: Option<String>,
        transport_parameters: Vec<u8>,
    },
    Certificate {
        certificate_list: Vec<Vec<u8>>,
    },
    CertificateVerify {
        signature_scheme: u16,
        signature: Vec<u8>,
    },
    Finished {
        verify_data: [u8; 32],
    },
}

impl TlsHandshakeMessage {
    pub fn encode(&self) -> Vec<u8> {
        let mut payload = Vec::new();
        let msg_type = match self {
            TlsHandshakeMessage::ClientHello {
                random,
                legacy_session_id,
                cipher_suites,
                key_share,
                alpn,
                transport_parameters,
            } => {
                // ClientHello legacy header
                payload.extend_from_slice(&0x0303u16.to_be_bytes()); // legacy_version = TLS 1.2
                payload.extend_from_slice(random);
                payload.push(legacy_session_id.len() as u8);
                payload.extend_from_slice(legacy_session_id);

                // Cipher Suites
                payload.extend_from_slice(&((cipher_suites.len() * 2) as u16).to_be_bytes());
                for cs in cipher_suites {
                    payload.extend_from_slice(&cs.to_be_bytes());
                }

                // Compression Methods (null)
                payload.push(1);
                payload.push(0x00);

                // Extensions
                let mut ext_buf = Vec::new();

                // 1. supported_versions (0x002b)
                let mut sv = Vec::new();
                sv.push(2); // 1 version of 2 bytes
                sv.extend_from_slice(&TLS_VERSION_1_3.to_be_bytes());
                append_extension(EXT_SUPPORTED_VERSIONS, &sv, &mut ext_buf);

                // 2. supported_groups (0x000a)
                let mut sg = Vec::new();
                sg.extend_from_slice(&2u16.to_be_bytes());
                sg.extend_from_slice(&GROUP_X25519.to_be_bytes());
                append_extension(EXT_SUPPORTED_GROUPS, &sg, &mut ext_buf);

                // 3. signature_algorithms (0x000d)
                let mut sa = Vec::new();
                sa.extend_from_slice(&2u16.to_be_bytes());
                sa.extend_from_slice(&SIG_ED25519.to_be_bytes());
                append_extension(EXT_SIGNATURE_ALGORITHMS, &sa, &mut ext_buf);

                // 4. key_share (0x0033)
                let mut ks = Vec::new();
                let client_shares_len = 2 + 2 + key_share.len();
                ks.extend_from_slice(&(client_shares_len as u16).to_be_bytes());
                ks.extend_from_slice(&GROUP_X25519.to_be_bytes());
                ks.extend_from_slice(&(key_share.len() as u16).to_be_bytes());
                ks.extend_from_slice(key_share);
                append_extension(EXT_KEY_SHARE, &ks, &mut ext_buf);

                // 5. ALPN (0x0010)
                if !alpn.is_empty() {
                    let mut alpn_buf = Vec::new();
                    let mut alpn_list = Vec::new();
                    for proto in alpn {
                        alpn_list.push(proto.len() as u8);
                        alpn_list.extend_from_slice(proto.as_bytes());
                    }
                    alpn_buf.extend_from_slice(&(alpn_list.len() as u16).to_be_bytes());
                    alpn_buf.extend_from_slice(&alpn_list);
                    append_extension(EXT_ALPN, &alpn_buf, &mut ext_buf);
                }

                // 6. quic_transport_parameters (0x0039 and draft 0xffa5)
                append_extension(EXT_QUIC_TRANSPORT_PARAMETERS, transport_parameters, &mut ext_buf);

                payload.extend_from_slice(&(ext_buf.len() as u16).to_be_bytes());
                payload.extend_from_slice(&ext_buf);

                MSG_CLIENT_HELLO
            }
            TlsHandshakeMessage::ServerHello {
                random,
                legacy_session_id_echo,
                cipher_suite,
                key_share,
            } => {
                payload.extend_from_slice(&0x0303u16.to_be_bytes());
                payload.extend_from_slice(random);
                payload.push(legacy_session_id_echo.len() as u8);
                payload.extend_from_slice(legacy_session_id_echo);
                payload.extend_from_slice(&cipher_suite.to_be_bytes());
                payload.push(0x00); // compression

                let mut ext_buf = Vec::new();

                // supported_versions
                let mut sv = Vec::new();
                sv.extend_from_slice(&TLS_VERSION_1_3.to_be_bytes());
                append_extension(EXT_SUPPORTED_VERSIONS, &sv, &mut ext_buf);

                // key_share
                let mut ks = Vec::new();
                ks.extend_from_slice(&GROUP_X25519.to_be_bytes());
                ks.extend_from_slice(&(key_share.len() as u16).to_be_bytes());
                ks.extend_from_slice(key_share);
                append_extension(EXT_KEY_SHARE, &ks, &mut ext_buf);

                payload.extend_from_slice(&(ext_buf.len() as u16).to_be_bytes());
                payload.extend_from_slice(&ext_buf);

                MSG_SERVER_HELLO
            }
            TlsHandshakeMessage::EncryptedExtensions { alpn, transport_parameters } => {
                let mut ext_buf = Vec::new();

                if let Some(ref proto) = alpn {
                    let mut alpn_buf = Vec::new();
                    let mut alpn_list = Vec::new();
                    alpn_list.push(proto.len() as u8);
                    alpn_list.extend_from_slice(proto.as_bytes());
                    alpn_buf.extend_from_slice(&(alpn_list.len() as u16).to_be_bytes());
                    alpn_buf.extend_from_slice(&alpn_list);
                    append_extension(EXT_ALPN, &alpn_buf, &mut ext_buf);
                }

                append_extension(EXT_QUIC_TRANSPORT_PARAMETERS, transport_parameters, &mut ext_buf);

                payload.extend_from_slice(&(ext_buf.len() as u16).to_be_bytes());
                payload.extend_from_slice(&ext_buf);

                MSG_ENCRYPTED_EXTENSIONS
            }
            TlsHandshakeMessage::Certificate { certificate_list } => {
                payload.push(0x00); // certificate_request_context_len = 0
                let mut cert_data = Vec::new();
                for cert in certificate_list {
                    // 3 bytes cert len
                    let l = cert.len();
                    cert_data.push(((l >> 16) & 0xff) as u8);
                    cert_data.push(((l >> 8) & 0xff) as u8);
                    cert_data.push((l & 0xff) as u8);
                    cert_data.extend_from_slice(cert);
                    cert_data.extend_from_slice(&0u16.to_be_bytes()); // extensions len = 0
                }
                let total_l = cert_data.len();
                payload.push(((total_l >> 16) & 0xff) as u8);
                payload.push(((total_l >> 8) & 0xff) as u8);
                payload.push((total_l & 0xff) as u8);
                payload.extend_from_slice(&cert_data);

                MSG_CERTIFICATE
            }
            TlsHandshakeMessage::CertificateVerify { signature_scheme, signature } => {
                payload.extend_from_slice(&signature_scheme.to_be_bytes());
                payload.extend_from_slice(&(signature.len() as u16).to_be_bytes());
                payload.extend_from_slice(signature);

                MSG_CERTIFICATE_VERIFY
            }
            TlsHandshakeMessage::Finished { verify_data } => {
                payload.extend_from_slice(verify_data);

                MSG_FINISHED
            }
        };

        let mut msg = Vec::with_capacity(4 + payload.len());
        msg.push(msg_type);
        let len = payload.len();
        msg.push(((len >> 16) & 0xff) as u8);
        msg.push(((len >> 8) & 0xff) as u8);
        msg.push((len & 0xff) as u8);
        msg.extend_from_slice(&payload);
        msg
    }

    pub fn decode(data: &[u8]) -> Result<(Self, usize), TlsError> {
        if data.len() < 4 {
            return Err(TlsError::InvalidLength);
        }
        let msg_type = data[0];
        let msg_len = ((data[1] as usize) << 16) | ((data[2] as usize) << 8) | (data[3] as usize);
        if data.len() < 4 + msg_len {
            return Err(TlsError::InvalidLength);
        }

        let payload = &data[4..4 + msg_len];
        let total_consumed = 4 + msg_len;

        let msg = match msg_type {
            MSG_CLIENT_HELLO => {
                if payload.len() < 34 {
                    return Err(TlsError::InvalidEncoding("ClientHello too short".into()));
                }
                let mut random = [0u8; 32];
                random.copy_from_slice(&payload[2..34]);

                let sess_id_len = payload[34] as usize;
                if payload.len() < 35 + sess_id_len + 2 {
                    return Err(TlsError::InvalidEncoding("ClientHello session ID overflow".into()));
                }
                let legacy_session_id = payload[35..35 + sess_id_len].to_vec();
                let mut offset = 35 + sess_id_len;

                let cs_len = u16::from_be_bytes(payload[offset..offset + 2].try_into().unwrap()) as usize;
                offset += 2;
                if payload.len() < offset + cs_len + 2 {
                    return Err(TlsError::InvalidEncoding("ClientHello cipher suite overflow".into()));
                }
                let mut cipher_suites = Vec::new();
                for chunk in payload[offset..offset + cs_len].chunks_exact(2) {
                    cipher_suites.push(u16::from_be_bytes(chunk.try_into().unwrap()));
                }
                offset += cs_len;

                let comp_len = payload[offset] as usize;
                offset += 1 + comp_len;

                if payload.len() < offset + 2 {
                    return Err(TlsError::InvalidEncoding("ClientHello extensions length missing".into()));
                }
                let ext_len = u16::from_be_bytes(payload[offset..offset + 2].try_into().unwrap()) as usize;
                offset += 2;
                if payload.len() < offset + ext_len {
                    return Err(TlsError::InvalidEncoding("ClientHello extensions overflow".into()));
                }

                let ext_bytes = &payload[offset..offset + ext_len];
                let mut key_share = [0u8; 32];
                let mut alpn = Vec::new();
                let mut transport_parameters = Vec::new();

                parse_extensions(ext_bytes, |ext_type, ext_data| {
                    match ext_type {
                        EXT_KEY_SHARE => {
                            // ClientKeyShare: client_shares<0..2^16-1> -> KeyShareEntry
                            if ext_data.len() >= 6 {
                                let group = u16::from_be_bytes(ext_data[2..4].try_into().unwrap());
                                let key_len = u16::from_be_bytes(ext_data[4..6].try_into().unwrap()) as usize;
                                if group == GROUP_X25519 && key_len == 32 && ext_data.len() >= 6 + 32 {
                                    key_share.copy_from_slice(&ext_data[6..38]);
                                }
                            }
                        }
                        EXT_ALPN => {
                            if ext_data.len() >= 2 {
                                let mut p_off = 2;
                                while p_off < ext_data.len() {
                                    let l = ext_data[p_off] as usize;
                                    p_off += 1;
                                    if p_off + l <= ext_data.len() {
                                        if let Ok(s) = std::str::from_utf8(&ext_data[p_off..p_off + l]) {
                                            alpn.push(s.to_string());
                                        }
                                        p_off += l;
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }
                        EXT_QUIC_TRANSPORT_PARAMETERS | EXT_QUIC_TRANSPORT_PARAMETERS_DRAFT => {
                            transport_parameters = ext_data.to_vec();
                        }
                        _ => {}
                    }
                })?;

                TlsHandshakeMessage::ClientHello {
                    random,
                    legacy_session_id,
                    cipher_suites,
                    key_share,
                    alpn,
                    transport_parameters,
                }
            }
            MSG_SERVER_HELLO => {
                if payload.len() < 38 {
                    return Err(TlsError::InvalidEncoding("ServerHello too short".into()));
                }
                let mut random = [0u8; 32];
                random.copy_from_slice(&payload[2..34]);

                let sess_id_len = payload[34] as usize;
                if payload.len() < 35 + sess_id_len + 3 {
                    return Err(TlsError::InvalidEncoding("ServerHello session ID overflow".into()));
                }
                let legacy_session_id_echo = payload[35..35 + sess_id_len].to_vec();
                let mut offset = 35 + sess_id_len;

                let cipher_suite = u16::from_be_bytes(payload[offset..offset + 2].try_into().unwrap());
                offset += 3; // cipher_suite (2) + compression (1)

                if payload.len() < offset + 2 {
                    return Err(TlsError::InvalidEncoding("ServerHello extensions length missing".into()));
                }
                let ext_len = u16::from_be_bytes(payload[offset..offset + 2].try_into().unwrap()) as usize;
                offset += 2;
                let ext_bytes = &payload[offset..offset + ext_len];

                let mut key_share = [0u8; 32];
                parse_extensions(ext_bytes, |ext_type, ext_data| {
                    if ext_type == EXT_KEY_SHARE && ext_data.len() >= 36 {
                        let group = u16::from_be_bytes(ext_data[0..2].try_into().unwrap());
                        let k_len = u16::from_be_bytes(ext_data[2..4].try_into().unwrap()) as usize;
                        if group == GROUP_X25519 && k_len == 32 {
                            key_share.copy_from_slice(&ext_data[4..36]);
                        }
                    }
                })?;

                TlsHandshakeMessage::ServerHello {
                    random,
                    legacy_session_id_echo,
                    cipher_suite,
                    key_share,
                }
            }
            MSG_ENCRYPTED_EXTENSIONS => {
                if payload.len() < 2 {
                    return Err(TlsError::InvalidLength);
                }
                let ext_len = u16::from_be_bytes(payload[0..2].try_into().unwrap()) as usize;
                let ext_bytes = &payload[2..2 + ext_len];

                let mut alpn = None;
                let mut transport_parameters = Vec::new();

                parse_extensions(ext_bytes, |ext_type, ext_data| {
                    match ext_type {
                        EXT_ALPN => {
                            if ext_data.len() >= 3 {
                                let l = ext_data[2] as usize;
                                if ext_data.len() >= 3 + l {
                                    if let Ok(s) = std::str::from_utf8(&ext_data[3..3 + l]) {
                                        alpn = Some(s.to_string());
                                    }
                                }
                            }
                        }
                        EXT_QUIC_TRANSPORT_PARAMETERS | EXT_QUIC_TRANSPORT_PARAMETERS_DRAFT => {
                            transport_parameters = ext_data.to_vec();
                        }
                        _ => {}
                    }
                })?;

                TlsHandshakeMessage::EncryptedExtensions { alpn, transport_parameters }
            }
            MSG_CERTIFICATE => {
                if payload.len() < 4 {
                    return Err(TlsError::InvalidLength);
                }
                let req_ctx_len = payload[0] as usize;
                let mut offset = 1 + req_ctx_len;
                if payload.len() < offset + 3 {
                    return Err(TlsError::InvalidLength);
                }
                let total_cert_len = ((payload[offset] as usize) << 16) | ((payload[offset + 1] as usize) << 8) | (payload[offset + 2] as usize);
                offset += 3;
                let cert_end = offset + total_cert_len;
                if payload.len() < cert_end {
                    return Err(TlsError::InvalidLength);
                }

                let mut certificate_list = Vec::new();
                while offset < cert_end {
                    if offset + 3 > cert_end {
                        return Err(TlsError::InvalidLength);
                    }
                    let cert_len = ((payload[offset] as usize) << 16) | ((payload[offset + 1] as usize) << 8) | (payload[offset + 2] as usize);
                    offset += 3;
                    if offset + cert_len > cert_end {
                        return Err(TlsError::InvalidLength);
                    }
                    certificate_list.push(payload[offset..offset + cert_len].to_vec());
                    offset += cert_len;
                    // Skip extensions
                    if offset + 2 <= cert_end {
                        let ext_len = u16::from_be_bytes(payload[offset..offset + 2].try_into().unwrap()) as usize;
                        offset += 2 + ext_len;
                    }
                }

                TlsHandshakeMessage::Certificate { certificate_list }
            }
            MSG_CERTIFICATE_VERIFY => {
                if payload.len() < 4 {
                    return Err(TlsError::InvalidLength);
                }
                let signature_scheme = u16::from_be_bytes(payload[0..2].try_into().unwrap());
                let sig_len = u16::from_be_bytes(payload[2..4].try_into().unwrap()) as usize;
                if payload.len() < 4 + sig_len {
                    return Err(TlsError::InvalidLength);
                }
                let signature = payload[4..4 + sig_len].to_vec();

                TlsHandshakeMessage::CertificateVerify { signature_scheme, signature }
            }
            MSG_FINISHED => {
                if payload.len() < 32 {
                    return Err(TlsError::InvalidLength);
                }
                let mut verify_data = [0u8; 32];
                verify_data.copy_from_slice(&payload[..32]);

                TlsHandshakeMessage::Finished { verify_data }
            }
            _ => return Err(TlsError::UnexpectedMessage),
        };

        Ok((msg, total_consumed))
    }
}

fn append_extension(ext_type: u16, data: &[u8], buf: &mut Vec<u8>) {
    buf.extend_from_slice(&ext_type.to_be_bytes());
    buf.extend_from_slice(&(data.len() as u16).to_be_bytes());
    buf.extend_from_slice(data);
}

fn parse_extensions<F>(mut data: &[u8], mut f: F) -> Result<(), TlsError>
where
    F: FnMut(u16, &[u8]),
{
    while !data.is_empty() {
        if data.len() < 4 {
            return Err(TlsError::InvalidEncoding("Truncated extension header".into()));
        }
        let ext_type = u16::from_be_bytes(data[0..2].try_into().unwrap());
        let ext_len = u16::from_be_bytes(data[2..4].try_into().unwrap()) as usize;
        data = &data[4..];
        if data.len() < ext_len {
            return Err(TlsError::InvalidEncoding("Truncated extension data".into()));
        }
        f(ext_type, &data[..ext_len]);
        data = &data[ext_len..];
    }
    Ok(())
}
