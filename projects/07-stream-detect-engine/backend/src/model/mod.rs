//! Ingestion & Event Model (Features 07-001..07-013)

use bytes::Bytes;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// 07-001..07-006: Event representation and normalization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub id: String,
    pub tenant_id: String,
    pub source: String,
    pub partition: u32,
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub fields: HashMap<String, String>,
    pub numeric_fields: HashMap<String, f64>,
    #[serde(skip)]
    pub raw_bytes: Option<Bytes>,
}

impl Event {
    pub fn new(
        tenant_id: impl Into<String>,
        source: impl Into<String>,
        partition: u32,
        event_type: impl Into<String>,
        timestamp: DateTime<Utc>,
    ) -> Self {
        let tenant_id = tenant_id.into();
        let source = source.into();
        let event_type = event_type.into();
        let mut hasher = Sha256::new();
        hasher.update(tenant_id.as_bytes());
        hasher.update(source.as_bytes());
        hasher.update(&partition.to_be_bytes());
        hasher.update(&timestamp.timestamp_nanos_opt().unwrap_or(0).to_be_bytes());
        hasher.update(event_type.as_bytes());
        let id = format!("{:x}", hasher.finalize());

        Self {
            id,
            tenant_id,
            source,
            partition,
            timestamp,
            event_type,
            fields: HashMap::new(),
            numeric_fields: HashMap::new(),
            raw_bytes: None,
        }
    }

    pub fn with_field(mut self, key: impl Into<String>, val: impl Into<String>) -> Self {
        self.fields.insert(key.into(), val.into());
        self
    }

    pub fn with_numeric(mut self, key: impl Into<String>, val: f64) -> Self {
        self.numeric_fields.insert(key.into(), val);
        self
    }

    /// 07-009: Event ID / Dedup key generation
    pub fn dedup_key(&self) -> String {
        format!("{}:{}:{}", self.tenant_id, self.event_type, self.id)
    }

    /// 07-007: Attach zero-copy buffer
    pub fn set_raw_buffer(&mut self, bytes: Bytes) {
        self.raw_bytes = Some(bytes);
    }
}

/// 07-004: Event Schema Validator
pub struct EventSchemaValidator {
    required_fields: Vec<String>,
}

impl EventSchemaValidator {
    pub fn new(required_fields: Vec<String>) -> Self {
        Self { required_fields }
    }

    pub fn validate(&self, event: &Event) -> Result<(), &'static str> {
        if event.tenant_id.is_empty() {
            return Err("Missing tenant_id");
        }
        if event.event_type.is_empty() {
            return Err("Missing event_type");
        }
        for req in &self.required_fields {
            if !event.fields.contains_key(req) && !event.numeric_fields.contains_key(req) {
                return Err("Missing required field");
            }
        }
        Ok(())
    }
}

/// 07-013: Quarantine Queue for Malformed Events
#[derive(Default)]
pub struct QuarantineStore {
    quarantined: parking_lot::Mutex<Vec<(String, &'static str)>>, // (raw_payload, reason)
}

impl QuarantineStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn quarantine(&self, raw: String, reason: &'static str) {
        self.quarantined.lock().push((raw, reason));
    }

    pub fn len(&self) -> usize {
        self.quarantined.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.quarantined.lock().is_empty()
    }
}

/// 07-012: Ingress Rate Metrics
#[derive(Default)]
pub struct IngressMetrics {
    total_events: AtomicU64,
    total_bytes: AtomicU64,
    malformed_events: AtomicU64,
}

impl IngressMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_ingest(&self, bytes: usize) {
        self.total_events.fetch_add(1, Ordering::Relaxed);
        self.total_bytes.fetch_add(bytes as u64, Ordering::Relaxed);
    }

    pub fn record_malformed(&self) {
        self.malformed_events.fetch_add(1, Ordering::Relaxed);
    }

    pub fn total_events(&self) -> u64 {
        self.total_events.load(Ordering::Relaxed)
    }

    pub fn total_bytes(&self) -> u64 {
        self.total_bytes.load(Ordering::Relaxed)
    }

    pub fn malformed_count(&self) -> u64 {
        self.malformed_events.load(Ordering::Relaxed)
    }
}
