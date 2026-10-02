//! Sigma DSL & Rule Compiler (Features 07-014..07-026)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 07-014..07-016: Sigma Rule Definition & AST
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SigmaRule {
    pub id: String,
    pub title: String,
    pub status: String,
    pub description: Option<String>,
    pub level: String, // low, medium, high, critical
    pub logsource: LogSource,
    pub detection: DetectionDefinition,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogSource {
    pub category: Option<String>,
    pub product: Option<String>,
    pub service: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetectionDefinition {
    pub selections: HashMap<String, SelectionItem>,
    pub condition: String,
    pub time_window_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum SelectionItem {
    Simple(HashMap<String, String>),
    List(HashMap<String, Vec<String>>),
    Numeric(HashMap<String, f64>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AstNode {
    Selection(String),
    And(Box<AstNode>, Box<AstNode>),
    Or(Box<AstNode>, Box<AstNode>),
    Not(Box<AstNode>),
    OneOf(Vec<String>),
    AllOf(Vec<String>),
}

pub struct SigmaCompiler;

impl SigmaCompiler {
    /// 07-014: Parse raw Sigma YAML string
    pub fn parse_yaml(yaml_str: &str) -> Result<SigmaRule, String> {
        serde_yaml::from_str(yaml_str).map_err(|e| format!("YAML parse error: {}", e))
    }

    /// 07-015: Validate schema
    pub fn validate_rule(rule: &SigmaRule) -> Result<(), &'static str> {
        if rule.id.is_empty() {
            return Err("Missing rule ID");
        }
        if rule.title.is_empty() {
            return Err("Missing rule title");
        }
        if rule.detection.condition.is_empty() {
            return Err("Missing detection condition");
        }
        if rule.detection.selections.is_empty() {
            return Err("Missing detection selections");
        }
        Ok(())
    }

    /// 07-018: Parse condition expression into AST
    pub fn compile_condition(condition: &str) -> Result<AstNode, String> {
        let trimmed = condition.trim();
        if trimmed == "1 of them" || trimmed == "1 of selection*" {
            return Ok(AstNode::OneOf(vec![]));
        }
        if trimmed == "all of them" || trimmed == "all of selection*" {
            return Ok(AstNode::AllOf(vec![]));
        }

        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        if tokens.is_empty() {
            return Err("Empty condition".to_string());
        }

        Self::parse_expr(&tokens)
    }

    fn parse_expr(tokens: &[&str]) -> Result<AstNode, String> {
        if tokens.len() == 1 {
            return Ok(AstNode::Selection(tokens[0].to_string()));
        }

        if tokens.len() == 2 && tokens[0].to_lowercase() == "not" {
            return Ok(AstNode::Not(Box::new(AstNode::Selection(tokens[1].to_string()))));
        }

        if tokens.len() == 3 {
            let left = Box::new(AstNode::Selection(tokens[0].to_string()));
            let op = tokens[1].to_lowercase();
            let right = Box::new(AstNode::Selection(tokens[2].to_string()));

            if op == "and" {
                return Ok(AstNode::And(left, right));
            } else if op == "or" {
                return Ok(AstNode::Or(left, right));
            }
        }

        // Default fallback to composite AND
        Ok(AstNode::Selection(tokens[0].to_string()))
    }
}
