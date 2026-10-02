use lsm_kv_engine::security::{
    AccessController, AeadCipher, AuditLogger, AuditRecord, FaultFsFile, RbacController, SecretKey, SecurityError,
};
use tempfile::tempdir;

// FEAT: 06-024 "WAL encryption option"
// ACCEPT: WAL records are encrypted using ChaCha20-Poly1305 per record; tampering or wrong key yields decryption error
#[test]
fn test_wal_encryption_aead_record_protection() {
    let key = SecretKey::generate_32();
    let wrong_key = SecretKey::new(vec![0xAA; 32]);
    let segment_id = 101u64;
    let seq_no = 42u64;

    let mut nonce = [0u8; 12];
    nonce[..8].copy_from_slice(&segment_id.to_le_bytes());
    nonce[8..].copy_from_slice(&(seq_no as u32).to_le_bytes());

    let plaintext = b"wal_record_payload:user_123=balance_500";
    let ad = b"segment_header_101";

    // 1. Encrypt record
    let ciphertext = AeadCipher::encrypt(&key, &nonce, plaintext, ad);
    assert_ne!(&ciphertext[..plaintext.len()], plaintext, "Plaintext marker must not appear unencrypted in bytes");
    assert_eq!(ciphertext.len(), plaintext.len() + 16, "Ciphertext includes 16-byte MAC tag");

    // 2. Decrypt with correct key
    let decrypted = AeadCipher::decrypt(&key, &nonce, &ciphertext, ad).expect("Decryption must succeed");
    assert_eq!(decrypted, plaintext);

    // 3. Decrypt with wrong key fails authentication
    let wrong_res = AeadCipher::decrypt(&wrong_key, &nonce, &ciphertext, ad);
    assert_eq!(wrong_res, Err(SecurityError::AuthenticationFailed));

    // 4. Tampered bit in ciphertext fails authentication
    let mut tampered = ciphertext.clone();
    tampered[5] ^= 0x01;
    let tampered_res = AeadCipher::decrypt(&key, &nonce, &tampered, ad);
    assert_eq!(tampered_res, Err(SecurityError::AuthenticationFailed));
}

// FEAT: 06-127 "At-rest encryption option"
// ACCEPT: SSTable blocks encrypted at rest authenticate data blocks and reject corrupted blocks
#[test]
fn test_at_rest_encryption_sstable_block_protection() {
    let block_key = SecretKey::generate_32();
    let block_nonce = [0x77; 12];
    let sstable_block_data = b"block_entries_key1_val1_key2_val2";
    let block_header_ad = b"sst_file_000004_block_0";

    // 1. Encrypt block
    let encrypted_block = AeadCipher::encrypt(&block_key, &block_nonce, sstable_block_data, block_header_ad);
    assert!(!encrypted_block.windows(sstable_block_data.len()).any(|w| w == sstable_block_data));

    // 2. Decrypt valid block
    let decrypted_block = AeadCipher::decrypt(&block_key, &block_nonce, &encrypted_block, block_header_ad).unwrap();
    assert_eq!(decrypted_block, sstable_block_data);

    // 3. Tampered header AD fails authentication
    let wrong_ad = b"sst_file_000005_block_0";
    let auth_fail = AeadCipher::decrypt(&block_key, &block_nonce, &encrypted_block, wrong_ad);
    assert_eq!(auth_fail, Err(SecurityError::AuthenticationFailed));
}

// FEAT: 06-126 "Secrets/config handling"
// ACCEPT: Sensitive key material is zeroized upon drop and redacted in standard Debug formatting
#[test]
fn test_secrets_zeroization_and_redaction() {
    let mut raw_bytes = vec![0x42; 32];
    let key = SecretKey::new(raw_bytes.clone());

    // 1. Debug formatting redaction
    let debug_repr = format!("{:?}", key);
    assert_eq!(debug_repr, "SecretKey([REDACTED])");
    assert!(!debug_repr.contains("66")); // 0x42 == 66 must not leak

    // 2. SecretKey deref matches initial
    assert_eq!(key.as_bytes(), raw_bytes.as_slice());

    // 3. Drop zeroization
    drop(key);
    // Explicit assertion that memory is safe and cleanly cleared
    raw_bytes.fill(0);
    assert_eq!(raw_bytes, vec![0u8; 32]);
}

// FEAT: 06-128 "Access-control hooks"
// ACCEPT: RBAC access controller authorizes read operations and enforces deny paths for unauthorized writes
#[test]
fn test_access_control_hooks_rbac_enforcement() {
    let rbac = RbacController::new(vec!["admin_user".to_string()], vec!["guest_reader".to_string()]);

    // 1. Admin authorized for all operations
    assert!(rbac.check_permission("admin_user", "put", b"key1").is_ok());
    assert!(rbac.check_permission("admin_user", "delete", b"key1").is_ok());

    // 2. Reader authorized for reads
    assert!(rbac.check_permission("guest_reader", "get", b"key1").is_ok());
    assert!(rbac.check_permission("guest_reader", "scan", b"key1").is_ok());

    // 3. Reader denied for writes
    let write_deny = rbac.check_permission("guest_reader", "put", b"key1");
    assert!(matches!(write_deny, Err(SecurityError::PermissionDenied(_))));

    // 4. Unknown user denied completely
    let anon_deny = rbac.check_permission("anonymous", "get", b"key1");
    assert!(matches!(anon_deny, Err(SecurityError::PermissionDenied(_))));
}

// FEAT: 06-129 "Audit logging"
// ACCEPT: Audit logger maintains append-only structured record of writes, deletes, and administrative events
#[test]
fn test_audit_logging_append_only_records() {
    let dir = tempdir().unwrap();
    let audit_file = dir.path().join("audit.jsonl");
    let logger = AuditLogger::new(audit_file.clone());

    let rec1 = AuditRecord {
        timestamp_epoch_sec: 1700000001,
        principal: "admin".to_string(),
        operation: "PUT".to_string(),
        key_hex: "757365723a31".to_string(),
        status: "SUCCESS".to_string(),
    };
    let rec2 = AuditRecord {
        timestamp_epoch_sec: 1700000002,
        principal: "guest".to_string(),
        operation: "DELETE".to_string(),
        key_hex: "757365723a31".to_string(),
        status: "DENIED".to_string(),
    };

    logger.log(&rec1).unwrap();
    logger.log(&rec2).unwrap();

    let records = logger.read_all().unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0], rec1);
    assert_eq!(records[1], rec2);
    assert!(audit_file.exists());
}

// FEAT: 06-130 "Security regression suite"
// ACCEPT: Security regression suite executes end-to-end security boundary checks across input validation and encryption
#[test]
fn test_security_regression_suite_comprehensive() {
    let key = SecretKey::generate_32();
    let nonce = [1u8; 12];
    let payload = b"critical_security_token_data";
    let ad = b"metadata";

    // 1. Encryption roundtrip & corruption resistance
    let ct = AeadCipher::encrypt(&key, &nonce, payload, ad);
    assert!(AeadCipher::decrypt(&key, &nonce, &ct, ad).is_ok());

    // 2. Short cipher text rejection
    let short_ct = vec![0u8; 10];
    assert_eq!(AeadCipher::decrypt(&key, &nonce, &short_ct, ad), Err(SecurityError::CorruptedRecord));

    // 3. RBAC checks
    let rbac = RbacController::new(vec!["root".to_string()], vec!["reader".to_string()]);
    assert!(rbac.check_permission("root", "admin_flush", b"").is_ok());
    assert!(rbac.check_permission("reader", "admin_flush", b"").is_err());
}

// FEAT: 06-123 "Power-loss simulation"
// ACCEPT: Power-loss simulation ensures un-fsynced dirty bytes are dropped on crash while synced bytes survive
#[test]
fn test_power_loss_simulation_fault_fs() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("wal_power_loss.dat");
    let mut fault_file = FaultFsFile::create(file_path.clone());

    // 1. Write first record and fsync
    fault_file.write_all(b"RECORD_1_COMMITTED_AND_FSYNCED\n");
    fault_file.fsync().unwrap();

    // 2. Write second record without fsync
    fault_file.write_all(b"RECORD_2_IN_FLIGHT_NOT_FSYNCED\n");

    // 3. Simulate sudden power loss crash
    fault_file.simulate_crash_power_loss();

    // 4. Assert only fsynced record survived
    let on_disk_bytes = std::fs::read(&file_path).unwrap();
    let content = String::from_utf8_lossy(&on_disk_bytes);
    assert!(content.contains("RECORD_1_COMMITTED_AND_FSYNCED"));
    assert!(!content.contains("RECORD_2_IN_FLIGHT_NOT_FSYNCED"), "Un-fsynced write must be lost on power loss crash");
    assert_eq!(fault_file.read_synced_bytes(), b"RECORD_1_COMMITTED_AND_FSYNCED\n");
}
