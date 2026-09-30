//! Arrow-Compatible Columnar Chunk Representation & Zone Maps

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataType {
    Int64,
    Float64,
    Utf8,
    Boolean,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZoneMap {
    pub min_int64: Option<i64>,
    pub max_int64: Option<i64>,
    pub min_float64: Option<f64>,
    pub max_float64: Option<f64>,
    pub null_count: usize,
    pub row_count: usize,
}

impl ZoneMap {
    pub fn new() -> Self {
        Self {
            min_int64: None,
            max_int64: None,
            min_float64: None,
            max_float64: None,
            null_count: 0,
            row_count: 0,
        }
    }

    pub fn can_prune_gt(&self, val: i64) -> bool {
        if let Some(max) = self.max_int64 {
            max <= val
        } else {
            false
        }
    }

    pub fn can_prune_lt(&self, val: i64) -> bool {
        if let Some(min) = self.min_int64 {
            min >= val
        } else {
            false
        }
    }
}

impl Default for ZoneMap {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum ColumnVector {
    Int64(Vec<i64>),
    Float64(Vec<f64>),
    Utf8(Vec<String>),
    Boolean(Vec<bool>),
}

#[derive(Debug, Clone)]
pub struct ColumnChunk {
    pub name: String,
    pub data_type: DataType,
    pub data: ColumnVector,
    pub null_bitmap: Vec<bool>,
    pub zone_map: ZoneMap,
}

impl ColumnChunk {
    pub fn new_int64(name: &str, values: Vec<i64>) -> Self {
        let mut min_val = i64::MAX;
        let mut max_val = i64::MIN;
        for &v in &values {
            if v < min_val { min_val = v; }
            if v > max_val { max_val = v; }
        }

        let zone_map = ZoneMap {
            min_int64: if values.is_empty() { None } else { Some(min_val) },
            max_int64: if values.is_empty() { None } else { Some(max_val) },
            min_float64: None,
            max_float64: None,
            null_count: 0,
            row_count: values.len(),
        };

        Self {
            name: name.to_string(),
            data_type: DataType::Int64,
            null_bitmap: vec![false; values.len()],
            data: ColumnVector::Int64(values),
            zone_map,
        }
    }

    pub fn new_float64(name: &str, values: Vec<f64>) -> Self {
        let mut min_val = f64::INFINITY;
        let mut max_val = f64::NEG_INFINITY;
        for &v in &values {
            if v < min_val { min_val = v; }
            if v > max_val { max_val = v; }
        }

        let zone_map = ZoneMap {
            min_int64: None,
            max_int64: None,
            min_float64: if values.is_empty() { None } else { Some(min_val) },
            max_float64: if values.is_empty() { None } else { Some(max_val) },
            null_count: 0,
            row_count: values.len(),
        };

        Self {
            name: name.to_string(),
            data_type: DataType::Float64,
            null_bitmap: vec![false; values.len()],
            data: ColumnVector::Float64(values),
            zone_map,
        }
    }

    pub fn new_utf8(name: &str, values: Vec<String>) -> Self {
        let len = values.len();
        Self {
            name: name.to_string(),
            data_type: DataType::Utf8,
            null_bitmap: vec![false; len],
            data: ColumnVector::Utf8(values),
            zone_map: ZoneMap {
                min_int64: None,
                max_int64: None,
                min_float64: None,
                max_float64: None,
                null_count: 0,
                row_count: len,
            },
        }
    }

    pub fn len(&self) -> usize {
        self.zone_map.row_count
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
