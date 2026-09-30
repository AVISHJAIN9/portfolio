//! Vectorized Aggregation Engine (Sum, Count, Min, Max, Avg, GroupBy)

use crate::execution::vector::SelectionVector;
use crate::storage::chunk::{ColumnChunk, ColumnVector};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum AggOp {
    Count,
    Sum(String),
    Min(String),
    Max(String),
    Avg(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AggValue {
    Int(i64),
    Float(f64),
    Count(usize),
}

pub struct VectorAggregator;

impl VectorAggregator {
    pub fn aggregate_column(chunk: &ColumnChunk, op: &AggOp, sel: &SelectionVector) -> AggValue {
        match (op, &chunk.data) {
            (AggOp::Count, _) => AggValue::Count(sel.len()),
            (AggOp::Sum(_), ColumnVector::Int64(vals)) => {
                let mut total: i64 = 0;
                for &idx in &sel.indices {
                    total += vals[idx];
                }
                AggValue::Int(total)
            }
            (AggOp::Sum(_), ColumnVector::Float64(vals)) => {
                let mut total: f64 = 0.0;
                for &idx in &sel.indices {
                    total += vals[idx];
                }
                AggValue::Float(total)
            }
            (AggOp::Min(_), ColumnVector::Int64(vals)) => {
                let mut min_val = i64::MAX;
                for &idx in &sel.indices {
                    if vals[idx] < min_val {
                        min_val = vals[idx];
                    }
                }
                AggValue::Int(if sel.is_empty() { 0 } else { min_val })
            }
            (AggOp::Max(_), ColumnVector::Int64(vals)) => {
                let mut max_val = i64::MIN;
                for &idx in &sel.indices {
                    if vals[idx] > max_val {
                        max_val = vals[idx];
                    }
                }
                AggValue::Int(if sel.is_empty() { 0 } else { max_val })
            }
            (AggOp::Avg(_), ColumnVector::Int64(vals)) => {
                if sel.is_empty() {
                    return AggValue::Float(0.0);
                }
                let mut total: i64 = 0;
                for &idx in &sel.indices {
                    total += vals[idx];
                }
                AggValue::Float(total as f64 / sel.len() as f64)
            }
            _ => AggValue::Count(0),
        }
    }

    pub fn group_by(
        group_chunk: &ColumnChunk,
        val_chunk: &ColumnChunk,
        op: &AggOp,
        sel: &SelectionVector,
    ) -> HashMap<String, AggValue> {
        let mut groups: HashMap<String, Vec<usize>> = HashMap::new();

        match &group_chunk.data {
            ColumnVector::Utf8(keys) => {
                for &idx in &sel.indices {
                    groups.entry(keys[idx].clone()).or_default().push(idx);
                }
            }
            ColumnVector::Int64(keys) => {
                for &idx in &sel.indices {
                    groups.entry(keys[idx].to_string()).or_default().push(idx);
                }
            }
            _ => {}
        }

        let mut results = HashMap::new();
        for (k, indices) in groups {
            let sub_sel = SelectionVector { indices };
            let agg = Self::aggregate_column(val_chunk, op, &sub_sel);
            results.insert(k, agg);
        }
        results
    }
}
