//! Rule Hot Reload & Configuration (Features 07-079..07-091)

use crate::dsl::SigmaRule;
use arc_swap::ArcSwap;
use std::collections::HashMap;
use std::sync::Arc;

pub struct RuleSet {
    pub version: u64,
    pub rules: HashMap<String, SigmaRule>,
}

/// 07-079..07-084: Atomic Rule Registry with Rollback
pub struct RuleRegistry {
    current: ArcSwap<RuleSet>,
    previous: parking_lot::Mutex<Option<Arc<RuleSet>>>,
}

impl RuleRegistry {
    pub fn new() -> Self {
        let initial = Arc::new(RuleSet {
            version: 0,
            rules: HashMap::new(),
        });
        Self {
            current: ArcSwap::from(initial),
            previous: parking_lot::Mutex::new(None),
        }
    }

    /// 07-080: Atomic rule swap
    pub fn swap_rules(&self, new_rules: HashMap<String, SigmaRule>) -> u64 {
        let old_ptr = self.current.load_full();
        let next_version = old_ptr.version + 1;
        let new_set = Arc::new(RuleSet {
            version: next_version,
            rules: new_rules,
        });

        *self.previous.lock() = Some(old_ptr);
        self.current.store(new_set);
        next_version
    }

    /// 07-082: Atomic rollback to previous rule-set
    pub fn rollback(&self) -> Result<u64, &'static str> {
        let mut prev_guard = self.previous.lock();
        if let Some(prev) = prev_guard.take() {
            let ver = prev.version;
            self.current.store(prev);
            Ok(ver)
        } else {
            Err("No previous rule-set available for rollback")
        }
    }

    pub fn get_rules(&self) -> Arc<RuleSet> {
        self.current.load_full()
    }
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}
