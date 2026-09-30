//! Vectorized Filter and Selection Evaluation

use crate::storage::chunk::{ColumnChunk, ColumnVector};

#[derive(Debug, Clone, PartialEq)]
pub enum Predicate {
    Gt(String, i64),
    Lt(String, i64),
    Eq(String, i64),
    Between(String, i64, i64),
    StrEq(String, String),
}

pub struct SelectionVector {
    pub indices: Vec<usize>,
}

impl SelectionVector {
    pub fn all(len: usize) -> Self {
        Self {
            indices: (0..len).collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.indices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

pub struct VectorizedExecutor;

impl VectorizedExecutor {
    pub fn evaluate_filter(chunk: &ColumnChunk, predicate: &Predicate, sel: &SelectionVector) -> SelectionVector {
        let mut matched = Vec::with_capacity(sel.len());

        match (&chunk.data, predicate) {
            (ColumnVector::Int64(values), Predicate::Gt(_, threshold)) => {
                for &idx in &sel.indices {
                    if values[idx] > *threshold {
                        matched.push(idx);
                    }
                }
            }
            (ColumnVector::Int64(values), Predicate::Lt(_, threshold)) => {
                for &idx in &sel.indices {
                    if values[idx] < *threshold {
                        matched.push(idx);
                    }
                }
            }
            (ColumnVector::Int64(values), Predicate::Eq(_, target)) => {
                for &idx in &sel.indices {
                    if values[idx] == *target {
                        matched.push(idx);
                    }
                }
            }
            (ColumnVector::Int64(values), Predicate::Between(_, min_val, max_val)) => {
                for &idx in &sel.indices {
                    let v = values[idx];
                    if v >= *min_val && v <= *max_val {
                        matched.push(idx);
                    }
                }
            }
            (ColumnVector::Utf8(values), Predicate::StrEq(_, target)) => {
                for &idx in &sel.indices {
                    if values[idx] == *target {
                        matched.push(idx);
                    }
                }
            }
            _ => {
                matched.extend_from_slice(&sel.indices);
            }
        }

        SelectionVector { indices: matched }
    }
}
