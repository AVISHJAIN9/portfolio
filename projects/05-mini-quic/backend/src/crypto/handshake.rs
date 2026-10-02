//! TLS 1.3 Handshake State Machine for QUIC Endpoint Handshake Exchanges

use super::tls::*;
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub const CONTEXT_SERVER_CERT_VERIFY: &[u8] = b"TLS 1.3, server CertificateVerify\0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandshakeStatus {
    InProgress,
    HandshakeKeysAvailable,
    Complete,
    Failed(TlsError),
}

pub struct QuicHandshakeKeys {
    pub client_handshake_key: [u8; 32],
    pub client_handshake_iv: [u8; 12],
    pub client_handshake_hp: [u8; 32],
    pub server_handshake_key: [u8; 32],
    pub server_handshake_iv: [u8; 12],
    pub server_handshake_hp: [u8; 32],
}

pub struct QuicAppKeys {
    pub client_1rtt_key: [u8; 32],
    pub client_1rtt_iv: [u8; 12],
    pub client_1rtt_hp: [u8; 32],
    pub server_1rtt_key: [u8; 32],
    pub server_1rtt_iv: [u8; 12],
    pub server_1rtt_hp: [u8; 32],
}

pub struct TlsHandshakeSession {
    pub is_server: bool,
    pub insecure_skip_cert_verify: bool,
    pub local_private_x25519: [u8; 32],
    pub local_public_x25519: [u8; 32],
    pub peer_public_x25519: Option<[u8; 32]>,
    pub local_ed25519_secret: Option<[u8; 32]>,
    pub local_ed25519_cert: Option<Vec<u8>>,

    pub transcript: Vec<u8>,
    pub early_secret: [u8; 32],
    pub handshake_secret: Option<[u8; 32]>,
    pub client_hs_traffic_secret: Option<[u8; 32]>,
    pub server_hs_traffic_secret: Option<[u8; 32]>,
    pub master_secret: Option<[u8; 32]>,
    pub client_app_traffic_secret: Option<[u8; 32]>,
    pub server_app_traffic_secret: Option<[u8; 32]>,

    pub handshake_keys: Option<QuicHandshakeKeys>,
    pub app_keys: Option<QuicAppKeys>,
    pub peer_transport_params: Option<QuicTransportParameters>,
    pub status: HandshakeStatus,
}

impl TlsHandshakeSession {
    /// Creates a new Client TLS 1.3 Session
    pub fn new_client(insecure_skip_cert_verify: bool) -> Self {
        let mut local_private_x25519 = [0u8; 32];
        for (i, b) in local_private_x25519.iter_mut().enumerate() {
            *b = (i as u8) ^ 0x5a;
        }
        let local_public_x25519 = pf_crypto::x25519_public_key(&local_private_x25519);
        let early_secret = TlsKeySchedule::derive_early_secret();

        Self {
            is_server: false,
            insecure_skip_cert_verify,
            local_private_x25519,
            local_public_x25519,
            peer_public_x25519: None,
            local_ed25519_secret: None,
            local_ed25519_cert: None,
            transcript: Vec::new(),
            early_secret,
            handshake_secret: None,
            client_hs_traffic_secret: None,
            server_hs_traffic_secret: None,
            master_secret: None,
            client_app_traffic_secret: None,
            server_app_traffic_secret: None,
            handshake_keys: None,
            app_keys: None,
            peer_transport_params: None,
            status: HandshakeStatus::InProgress,
        }
    }

    /// Creates a new Server TLS 1.3 Session
    pub fn new_server(ed25519_secret: [u8; 32], cert_der: Vec<u8>) -> Self {
        let mut local_private_x25519 = [0u8; 32];
        for (i, b) in local_private_x25519.iter_mut().enumerate() {
            *b = (i as u8) ^ 0xa5;
        }
        let local_public_x25519 = pf_crypto::x25519_public_key(&local_private_x25519);
        let early_secret = TlsKeySchedule::derive_early_secret();

        Self {
            is_server: true,
            insecure_skip_cert_verify: false,
            local_private_x25519,
            local_public_x25519,
            peer_public_x25519: None,
            local_ed25519_secret: Some(ed25519_secret),
            local_ed25519_cert: Some(cert_der),
            transcript: Vec::new(),
            early_secret,
            handshake_secret: None,
            client_hs_traffic_secret: None,
            server_hs_traffic_secret: None,
            master_secret: None,
            client_app_traffic_secret: None,
            server_app_traffic_secret: None,
            handshake_keys: None,
            app_keys: None,
            peer_transport_params: None,
            status: HandshakeStatus::InProgress,
        }
    }

    /// Client: creates initial ClientHello message
    pub fn create_client_hello(&mut self, alpn: &[String], params: &QuicTransportParameters) -> Vec<u8> {
        let mut random = [0u8; 32];
        for (i, b) in random.iter_mut().enumerate() {
            *b = (i as u8) + 0x11;
        }

        let ch = TlsHandshakeMessage::ClientHello {
            random,
            legacy_session_id: vec![0u8; 32],
            cipher_suites: vec![CIPHER_TLS_CHACHA20_POLY1305_SHA256],
            key_share: self.local_public_x25519,
            alpn: alpn.to_vec(),
            transport_parameters: params.encode(),
        };

        let encoded = ch.encode();
        self.transcript.extend_from_slice(&encoded);
        encoded
    }

    /// Server: processes ClientHello and produces ServerHello, EncryptedExtensions, Certificate, CertificateVerify, Finished
    pub fn handle_client_hello(
        &mut self,
        ch_bytes: &[u8],
        selected_alpn: Option<String>,
        server_params: &QuicTransportParameters,
    ) -> Result<(Vec<u8>, Vec<u8>), TlsError> {
        let (msg, _) = TlsHandshakeMessage::decode(ch_bytes)?;
        let (client_random, legacy_session_id, key_share, peer_tp) = match msg {
            TlsHandshakeMessage::ClientHello {
                random,
                legacy_session_id,
                cipher_suites: _,
                key_share,
                alpn: _,
                transport_parameters,
            } => {
                let tp = QuicTransportParameters::decode(&transport_parameters)?;
                (random, legacy_session_id, key_share, tp)
            }
            _ => return Err(TlsError::UnexpectedMessage),
        };

        self.peer_public_x25519 = Some(key_share);
        self.peer_transport_params = Some(peer_tp);
        self.transcript.extend_from_slice(ch_bytes);

        // 1. ServerHello
        let mut server_random = [0u8; 32];
        for (i, b) in server_random.iter_mut().enumerate() {
            *b = (i as u8) + 0x22;
        }
        let sh = TlsHandshakeMessage::ServerHello {
            random: server_random,
            legacy_session_id_echo: legacy_session_id,
            cipher_suite: CIPHER_TLS_CHACHA20_POLY1305_SHA256,
            key_share: self.local_public_x25519,
        };
        let sh_bytes = sh.encode();
        self.transcript.extend_from_slice(&sh_bytes);

        // Derive Handshake Keys
        let shared_secret = pf_crypto::x25519(&self.local_private_x25519, &key_share);
        let hs_secret = TlsKeySchedule::derive_handshake_secret(&self.early_secret, &shared_secret);
        self.handshake_secret = Some(hs_secret);

        let th_sh = sha256_digest(&self.transcript);
        let c_hs_secret = TlsKeySchedule::derive_client_handshake_traffic_secret(&hs_secret, &th_sh);
        let s_hs_secret = TlsKeySchedule::derive_server_handshake_traffic_secret(&hs_secret, &th_sh);
        self.client_hs_traffic_secret = Some(c_hs_secret);
        self.server_hs_traffic_secret = Some(s_hs_secret);

        let (c_hs_k, c_hs_iv, c_hs_hp) = TlsKeySchedule::derive_quic_keys(&c_hs_secret);
        let (s_hs_k, s_hs_iv, s_hs_hp) = TlsKeySchedule::derive_quic_keys(&s_hs_secret);
        self.handshake_keys = Some(QuicHandshakeKeys {
            client_handshake_key: c_hs_k,
            client_handshake_iv: c_hs_iv,
            client_handshake_hp: c_hs_hp,
            server_handshake_key: s_hs_k,
            server_handshake_iv: s_hs_iv,
            server_handshake_hp: s_hs_hp,
        });

        // 2. EncryptedExtensions
        let ee = TlsHandshakeMessage::EncryptedExtensions {
            alpn: selected_alpn,
            transport_parameters: server_params.encode(),
        };
        let ee_bytes = ee.encode();
        self.transcript.extend_from_slice(&ee_bytes);

        // 3. Certificate
        let cert_der = self.local_ed25519_cert.clone().unwrap_or_default();
        let cert = TlsHandshakeMessage::Certificate {
            certificate_list: vec![cert_der],
        };
        let cert_bytes = cert.encode();
        self.transcript.extend_from_slice(&cert_bytes);

        // 4. CertificateVerify
        let th_cert = sha256_digest(&self.transcript);
        let mut sign_msg = Vec::new();
        sign_msg.extend_from_slice(&[0x20; 64]);
        sign_msg.extend_from_slice(CONTEXT_SERVER_CERT_VERIFY);
        sign_msg.extend_from_slice(&th_cert);

        let ed_secret = self.local_ed25519_secret.expect("Server must have Ed25519 secret");
        let sig = pf_crypto::ed25519_sign(&ed_secret, &sign_msg);
        let cv = TlsHandshakeMessage::CertificateVerify {
            signature_scheme: SIG_ED25519,
            signature: sig.to_vec(),
        };
        let cv_bytes = cv.encode();
        self.transcript.extend_from_slice(&cv_bytes);

        // 5. Finished
        let th_cv = sha256_digest(&self.transcript);
        let s_fin_key = TlsKeySchedule::derive_finished_key(&s_hs_secret);
        let s_verify_data = TlsKeySchedule::compute_verify_data(&s_fin_key, &th_cv);
        let fin = TlsHandshakeMessage::Finished {
            verify_data: s_verify_data,
        };
        let fin_bytes = fin.encode();
        self.transcript.extend_from_slice(&fin_bytes);

        let mut hs_flight = Vec::new();
        hs_flight.extend_from_slice(&ee_bytes);
        hs_flight.extend_from_slice(&cert_bytes);
        hs_flight.extend_from_slice(&cv_bytes);
        hs_flight.extend_from_slice(&fin_bytes);

        Ok((sh_bytes, hs_flight))
    }

    /// Client: handles ServerHello
    pub fn handle_server_hello(&mut self, sh_bytes: &[u8]) -> Result<(), TlsError> {
        let (msg, _) = TlsHandshakeMessage::decode(sh_bytes)?;
        let key_share = match msg {
            TlsHandshakeMessage::ServerHello {
                random: _,
                legacy_session_id_echo: _,
                cipher_suite,
                key_share,
            } => {
                if cipher_suite != CIPHER_TLS_CHACHA20_POLY1305_SHA256 {
                    return Err(TlsError::UnsupportedCipherSuite(cipher_suite));
                }
                key_share
            }
            _ => return Err(TlsError::UnexpectedMessage),
        };

        self.peer_public_x25519 = Some(key_share);
        self.transcript.extend_from_slice(sh_bytes);

        let shared_secret = pf_crypto::x25519(&self.local_private_x25519, &key_share);
        let hs_secret = TlsKeySchedule::derive_handshake_secret(&self.early_secret, &shared_secret);
        self.handshake_secret = Some(hs_secret);

        let th_sh = sha256_digest(&self.transcript);
        let c_hs_secret = TlsKeySchedule::derive_client_handshake_traffic_secret(&hs_secret, &th_sh);
        let s_hs_secret = TlsKeySchedule::derive_server_handshake_traffic_secret(&hs_secret, &th_sh);
        self.client_hs_traffic_secret = Some(c_hs_secret);
        self.server_hs_traffic_secret = Some(s_hs_secret);

        let (c_hs_k, c_hs_iv, c_hs_hp) = TlsKeySchedule::derive_quic_keys(&c_hs_secret);
        let (s_hs_k, s_hs_iv, s_hs_hp) = TlsKeySchedule::derive_quic_keys(&s_hs_secret);
        self.handshake_keys = Some(QuicHandshakeKeys {
            client_handshake_key: c_hs_k,
            client_handshake_iv: c_hs_iv,
            client_handshake_hp: c_hs_hp,
            server_handshake_key: s_hs_k,
            server_handshake_iv: s_hs_iv,
            server_handshake_hp: s_hs_hp,
        });

        self.status = HandshakeStatus::HandshakeKeysAvailable;
        Ok(())
    }

    /// Client: handles server handshake flight (EE, Cert, CV, Fin) and returns client Finished message
    pub fn handle_server_handshake_flight(&mut self, flight_bytes: &[u8]) -> Result<Vec<u8>, TlsError> {
        let mut offset = 0;
        let s_hs_secret = self.server_hs_traffic_secret.ok_or(TlsError::UnexpectedMessage)?;
        let c_hs_secret = self.client_hs_traffic_secret.ok_or(TlsError::UnexpectedMessage)?;

        while offset < flight_bytes.len() {
            let (msg, consumed) = TlsHandshakeMessage::decode(&flight_bytes[offset..])?;
            let msg_slice = &flight_bytes[offset..offset + consumed];

            match msg {
                TlsHandshakeMessage::EncryptedExtensions { alpn: _, transport_parameters } => {
                    let tp = QuicTransportParameters::decode(&transport_parameters)?;
                    self.peer_transport_params = Some(tp);
                    self.transcript.extend_from_slice(msg_slice);
                }
                TlsHandshakeMessage::Certificate { certificate_list: _ } => {
                    self.transcript.extend_from_slice(msg_slice);
                }
                TlsHandshakeMessage::CertificateVerify { signature_scheme: _, signature: _ } => {
                    self.transcript.extend_from_slice(msg_slice);
                }
                TlsHandshakeMessage::Finished { verify_data } => {
                    // Verify server Finished MAC
                    let th_before_fin = sha256_digest(&self.transcript);
                    let s_fin_key = TlsKeySchedule::derive_finished_key(&s_hs_secret);
                    let expected_vd = TlsKeySchedule::compute_verify_data(&s_fin_key, &th_before_fin);

                    if !pf_crypto::constant_time_eq(&verify_data, &expected_vd) {
                        self.status = HandshakeStatus::Failed(TlsError::FinishedMacMismatch);
                        return Err(TlsError::FinishedMacMismatch);
                    }

                    self.transcript.extend_from_slice(msg_slice);
                }
                _ => return Err(TlsError::UnexpectedMessage),
            }

            offset += consumed;
        }

        // Generate Client Finished
        let th_after_s_fin = sha256_digest(&self.transcript);
        let c_fin_key = TlsKeySchedule::derive_finished_key(&c_hs_secret);
        let c_verify_data = TlsKeySchedule::compute_verify_data(&c_fin_key, &th_after_s_fin);
        let client_fin = TlsHandshakeMessage::Finished {
            verify_data: c_verify_data,
        };
        let client_fin_bytes = client_fin.encode();
        self.transcript.extend_from_slice(&client_fin_bytes);

        // Derive Application Keys
        let hs_secret = self.handshake_secret.unwrap();
        let master_sec = TlsKeySchedule::derive_master_secret(&hs_secret);
        self.master_secret = Some(master_sec);

        let th_final = sha256_digest(&self.transcript);
        let c_app_sec = TlsKeySchedule::derive_client_app_traffic_secret(&master_sec, &th_final);
        let s_app_sec = TlsKeySchedule::derive_server_app_traffic_secret(&master_sec, &th_final);
        self.client_app_traffic_secret = Some(c_app_sec);
        self.server_app_traffic_secret = Some(s_app_sec);

        let (c_1rtt_k, c_1rtt_iv, c_1rtt_hp) = TlsKeySchedule::derive_quic_keys(&c_app_sec);
        let (s_1rtt_k, s_1rtt_iv, s_1rtt_hp) = TlsKeySchedule::derive_quic_keys(&s_app_sec);
        self.app_keys = Some(QuicAppKeys {
            client_1rtt_key: c_1rtt_k,
            client_1rtt_iv: c_1rtt_iv,
            client_1rtt_hp: c_1rtt_hp,
            server_1rtt_key: s_1rtt_k,
            server_1rtt_iv: s_1rtt_iv,
            server_1rtt_hp: s_1rtt_hp,
        });

        self.status = HandshakeStatus::Complete;
        Ok(client_fin_bytes)
    }

    /// Server: processes Client Finished message
    pub fn handle_client_finished(&mut self, fin_bytes: &[u8]) -> Result<(), TlsError> {
        let (msg, _) = TlsHandshakeMessage::decode(fin_bytes)?;
        let client_verify_data = match msg {
            TlsHandshakeMessage::Finished { verify_data } => verify_data,
            _ => return Err(TlsError::UnexpectedMessage),
        };

        let c_hs_secret = self.client_hs_traffic_secret.ok_or(TlsError::UnexpectedMessage)?;
        let th_before_c_fin = sha256_digest(&self.transcript);
        let c_fin_key = TlsKeySchedule::derive_finished_key(&c_hs_secret);
        let expected_c_vd = TlsKeySchedule::compute_verify_data(&c_fin_key, &th_before_c_fin);

        if !pf_crypto::constant_time_eq(&client_verify_data, &expected_c_vd) {
            self.status = HandshakeStatus::Failed(TlsError::FinishedMacMismatch);
            return Err(TlsError::FinishedMacMismatch);
        }

        self.transcript.extend_from_slice(fin_bytes);

        // Derive Application Keys on Server
        let hs_secret = self.handshake_secret.unwrap();
        let master_sec = TlsKeySchedule::derive_master_secret(&hs_secret);
        self.master_secret = Some(master_sec);

        let th_final = sha256_digest(&self.transcript);
        let c_app_sec = TlsKeySchedule::derive_client_app_traffic_secret(&master_sec, &th_final);
        let s_app_sec = TlsKeySchedule::derive_server_app_traffic_secret(&master_sec, &th_final);
        self.client_app_traffic_secret = Some(c_app_sec);
        self.server_app_traffic_secret = Some(s_app_sec);

        let (c_1rtt_k, c_1rtt_iv, c_1rtt_hp) = TlsKeySchedule::derive_quic_keys(&c_app_sec);
        let (s_1rtt_k, s_1rtt_iv, s_1rtt_hp) = TlsKeySchedule::derive_quic_keys(&s_app_sec);
        self.app_keys = Some(QuicAppKeys {
            client_1rtt_key: c_1rtt_k,
            client_1rtt_iv: c_1rtt_iv,
            client_1rtt_hp: c_1rtt_hp,
            server_1rtt_key: s_1rtt_k,
            server_1rtt_iv: s_1rtt_iv,
            server_1rtt_hp: s_1rtt_hp,
        });

        self.status = HandshakeStatus::Complete;
        Ok(())
    }
}
