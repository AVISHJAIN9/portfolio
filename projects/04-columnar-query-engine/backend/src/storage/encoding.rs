//! Columnar Table Format, Row Groups, Null Bitmaps, Dictionary and RLE Encodings (04-046..050, 04-055)

use std::collections::HashMap;

/// Validity / Null Bitmap representation (04-048)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NullBitmap {
    pub bits: Vec<u8>,
    pub null_count: usize,
    pub total_count: usize,
}

impl NullBitmap {
    pub fn new(total_count: usize) -> Self {
        let byte_len = total_count.div_ceil(8);
        Self {
            bits: vec![0u8; byte_len],
            null_count: 0,
            total_count,
        }
    }

    pub fn set_null(&mut self, idx: usize) {
        if idx < self.total_count {
            let byte_idx = idx / 8;
            let bit_idx = idx % 8;
            if self.bits[byte_idx] & (1 << bit_idx) == 0 {
                self.bits[byte_idx] |= 1 << bit_idx;
                self.null_count += 1;
            }
        }
    }

    pub fn is_null(&self, idx: usize) -> bool {
        if idx >= self.total_count {
            return false;
        }
        let byte_idx = idx / 8;
        let bit_idx = idx % 8;
        (self.bits[byte_idx] & (1 << bit_idx)) != 0
    }
}

/// Dictionary Encoding for Strings (04-049)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictionaryEncodedColumn {
    pub dictionary: Vec<String>,
    pub indices: Vec<u32>,
    pub nulls: NullBitmap,
}

impl DictionaryEncodedColumn {
    pub fn encode(values: &[Option<String>]) -> Self {
        let mut dict = Vec::new();
        let mut map: HashMap<String, u32> = HashMap::new();
        let mut indices = Vec::with_capacity(values.len());
        let mut nulls = NullBitmap::new(values.len());

        for (i, val) in values.iter().enumerate() {
            match val {
                Some(s) => {
                    let idx = match map.get(s) {
                        Some(&id) => id,
                        None => {
                            let id = dict.len() as u32;
                            dict.push(s.clone());
                            map.insert(s.clone(), id);
                            id
                        }
                    };
                    indices.push(idx);
                }
                None => {
                    nulls.set_null(i);
                    indices.push(0);
                }
            }
        }

        Self {
            dictionary: dict,
            indices,
            nulls,
        }
    }

    pub fn decode(&self) -> Vec<Option<String>> {
        let mut result = Vec::with_capacity(self.indices.len());
        for (i, &idx) in self.indices.iter().enumerate() {
            if self.nulls.is_null(i) {
                result.push(None);
            } else {
                result.push(Some(self.dictionary[idx as usize].clone()));
            }
        }
        result
    }
}

/// Run-Length Encoding (RLE) for repeated integer values (04-050)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RleEncodedColumn {
    pub values: Vec<i64>,
    pub run_lengths: Vec<u32>,
    pub total_rows: usize,
}

impl RleEncodedColumn {
    pub fn encode(data: &[i64]) -> Self {
        if data.is_empty() {
            return Self {
                values: Vec::new(),
                run_lengths: Vec::new(),
                total_rows: 0,
            };
        }

        let mut values = Vec::new();
        let mut run_lengths = Vec::new();
        let mut current_val = data[0];
        let mut current_run = 1u32;

        for &val in &data[1..] {
            if val == current_val {
                current_run += 1;
            } else {
                values.push(current_val);
                run_lengths.push(current_run);
                current_val = val;
                current_run = 1;
            }
        }
        values.push(current_val);
        run_lengths.push(current_run);

        Self {
            values,
            run_lengths,
            total_rows: data.len(),
        }
    }

    pub fn decode(&self) -> Vec<i64> {
        let mut result = Vec::with_capacity(self.total_rows);
        for (&val, &run) in self.values.iter().zip(self.run_lengths.iter()) {
            for _ in 0..run {
                result.push(val);
            }
        }
        result
    }
}

/// Row Group with File Statistics (04-046, 04-047, 04-055)
#[derive(Debug, Clone)]
pub struct RowGroup {
    pub row_count: usize,
    pub dict_columns: HashMap<String, DictionaryEncodedColumn>,
    pub rle_columns: HashMap<String, RleEncodedColumn>,
    pub int_columns: HashMap<String, Vec<i64>>,
    pub min_values: HashMap<String, i64>,
    pub max_values: HashMap<String, i64>,
}

impl RowGroup {
    pub fn new(row_count: usize) -> Self {
        Self {
            row_count,
            dict_columns: HashMap::new(),
            rle_columns: HashMap::new(),
            int_columns: HashMap::new(),
            min_values: HashMap::new(),
            max_values: HashMap::new(),
        }
    }

    pub fn add_int_column(&mut self, name: &str, data: Vec<i64>) {
        if !data.is_empty() {
            let min_v = *data.iter().min().unwrap();
            let max_v = *data.iter().max().unwrap();
            self.min_values.insert(name.to_string(), min_v);
            self.max_values.insert(name.to_string(), max_v);
        }
        self.int_columns.insert(name.to_string(), data);
    }
}
