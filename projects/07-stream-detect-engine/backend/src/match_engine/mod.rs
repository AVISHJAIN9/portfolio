//! Custom In-Tree Aho-Corasick Pattern Matcher & Predicate Engine (Features 07-027..07-039)

use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Default)]
struct TrieNode {
    children: HashMap<u8, usize>,
    failure_link: usize,
    output_patterns: Vec<usize>, // pattern IDs that end at this node
}

/// 07-027..07-030: Custom Aho-Corasick Multi-Pattern Matcher
pub struct AhoCorasickMatcher {
    nodes: Vec<TrieNode>,
    patterns: Vec<String>,
    case_insensitive: bool,
}

impl AhoCorasickMatcher {
    pub fn new(patterns: Vec<String>, case_insensitive: bool) -> Self {
        let mut matcher = Self {
            nodes: vec![TrieNode::default()],
            patterns: patterns.clone(),
            case_insensitive,
        };

        // 1. Build Trie
        for (pid, pattern) in patterns.iter().enumerate() {
            let bytes = if case_insensitive {
                pattern.to_lowercase().into_bytes()
            } else {
                pattern.as_bytes().to_vec()
            };

            let mut current = 0;
            for &b in &bytes {
                if !matcher.nodes[current].children.contains_key(&b) {
                    let next = matcher.nodes.len();
                    matcher.nodes.push(TrieNode::default());
                    matcher.nodes[current].children.insert(b, next);
                }
                current = *matcher.nodes[current].children.get(&b).unwrap();
            }
            matcher.nodes[current].output_patterns.push(pid);
        }

        // 2. Build Failure & Output Links via BFS
        let mut queue = VecDeque::new();
        let root_children: Vec<usize> = matcher.nodes[0].children.values().copied().collect();
        for child in root_children {
            matcher.nodes[child].failure_link = 0;
            queue.push_back(child);
        }

        while let Some(current) = queue.pop_front() {
            let current_fail = matcher.nodes[current].failure_link;

            // Collect output from failure link
            let fail_outputs = matcher.nodes[current_fail].output_patterns.clone();
            matcher.nodes[current].output_patterns.extend(fail_outputs);

            let children = matcher.nodes[current].children.clone();
            for (&b, &child) in &children {
                let mut fail = current_fail;
                while fail > 0 && !matcher.nodes[fail].children.contains_key(&b) {
                    fail = matcher.nodes[fail].failure_link;
                }
                if let Some(&target) = matcher.nodes[fail].children.get(&b) {
                    if target != child {
                        matcher.nodes[child].failure_link = target;
                    }
                } else {
                    matcher.nodes[child].failure_link = 0;
                }
                queue.push_back(child);
            }
        }

        matcher
    }

    /// Finds all matching pattern IDs and byte offsets in text
    pub fn find_all(&self, text: &str) -> Vec<(usize, usize, &str)> {
        let bytes = if self.case_insensitive {
            text.to_lowercase().into_bytes()
        } else {
            text.as_bytes().to_vec()
        };

        let mut matches = Vec::new();
        let mut current = 0;

        for (idx, &b) in bytes.iter().enumerate() {
            while current > 0 && !self.nodes[current].children.contains_key(&b) {
                current = self.nodes[current].failure_link;
            }
            if let Some(&next) = self.nodes[current].children.get(&b) {
                current = next;
            } else {
                current = 0;
            }

            for &pid in &self.nodes[current].output_patterns {
                let pat = &self.patterns[pid];
                let start_idx = idx + 1 - pat.len();
                matches.push((pid, start_idx, pat.as_str()));
            }
        }

        matches
    }

    pub fn matches_any(&self, text: &str) -> bool {
        !self.find_all(text).is_empty()
    }
}

/// 07-036: Predicate Short-Circuit Evaluator
pub struct PredicateEvaluator;

impl PredicateEvaluator {
    pub fn evaluate_string_filter(
        actual: Option<&str>,
        expected: &str,
        is_prefix: bool,
        is_suffix: bool,
    ) -> bool {
        let actual = match actual {
            Some(a) => a,
            None => return false,
        };

        if is_prefix {
            actual.starts_with(expected)
        } else if is_suffix {
            actual.ends_with(expected)
        } else {
            actual.contains(expected)
        }
    }
}
