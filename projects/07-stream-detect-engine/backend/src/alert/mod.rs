//! Alerting & Detection Lifecycle (Features 07-066..07-078)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertStatus {
    Open,
    Acknowledged,
    Resolved,
    Suppressed,
}

/// 07-066..07-070: Alert representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Alert {
    pub alert_id: String,
    pub rule_id: String,
    pub tenant_id: String,
    pub correlation_id: String,
    pub severity: AlertSeverity,
    pub status: AlertStatus,
    pub triggered_at: DateTime<Utc>,
    pub matched_event_ids: Vec<String>,
    pub metadata: HashMap<String, String>,
}

pub struct AlertManager {
    alerts: HashMap<String, Alert>,
    suppression_windows: HashMap<String, DateTime<Utc>>, // GroupKey -> Active until
}

impl AlertManager {
    pub fn new() -> Self {
        Self {
            alerts: HashMap::new(),
            suppression_windows: HashMap::new(),
        }
    }

    /// 07-069: Alert Generation with Suppression & Grouping
    pub fn emit_alert(
        &mut self,
        rule_id: &str,
        tenant_id: &str,
        severity: AlertSeverity,
        matched_event_ids: Vec<String>,
        suppress_duration_s: Option<i64>,
    ) -> Option<Alert> {
        let group_key = format!("{}:{}:{:?}", tenant_id, rule_id, severity);
        let now = Utc::now();

        if let Some(&until) = self.suppression_windows.get(&group_key) {
            if now < until {
                return None; // Suppressed
            }
        }

        if let Some(dur) = suppress_duration_s {
            self.suppression_windows
                .insert(group_key, now + chrono::Duration::seconds(dur));
        }

        let alert_id = format!("alt_{}_{}", rule_id, now.timestamp_micros());
        let correlation_id = format!("corr_{}", now.timestamp_millis());

        let alert = Alert {
            alert_id: alert_id.clone(),
            rule_id: rule_id.to_string(),
            tenant_id: tenant_id.to_string(),
            correlation_id,
            severity,
            status: AlertStatus::Open,
            triggered_at: now,
            matched_event_ids,
            metadata: HashMap::new(),
        };

        self.alerts.insert(alert_id, alert.clone());
        Some(alert)
    }

    /// 07-074: Acknowledgement state machine
    pub fn acknowledge(&mut self, alert_id: &str) -> Result<(), &'static str> {
        let alert = self.alerts.get_mut(alert_id).ok_or("Alert not found")?;
        if alert.status == AlertStatus::Resolved {
            return Err("Cannot acknowledge resolved alert");
        }
        alert.status = AlertStatus::Acknowledged;
        Ok(())
    }

    pub fn resolve(&mut self, alert_id: &str) -> Result<(), &'static str> {
        let alert = self.alerts.get_mut(alert_id).ok_or("Alert not found")?;
        alert.status = AlertStatus::Resolved;
        Ok(())
    }

    pub fn get_alert(&self, alert_id: &str) -> Option<&Alert> {
        self.alerts.get(alert_id)
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}
