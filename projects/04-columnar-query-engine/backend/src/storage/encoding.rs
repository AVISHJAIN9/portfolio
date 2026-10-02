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

/// 04-051: Delta Encoding for sorted / monotone integers
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeltaEncodedColumn {
    pub first_value: i64,
    pub deltas: Vec<i64>,
}

impl DeltaEncodedColumn {
    pub fn encode(data: &[i64]) -> Self {
        if data.is_empty() {
            return Self {
                first_value: 0,
                deltas: Vec::new(),
            };
        }
        let first_value = data[0];
        let mut deltas = Vec::with_capacity(data.len().saturating_sub(1));
        for i in 1..data.len() {
            deltas.push(data[i].wrapping_sub(data[i - 1]));
        }
        Self {
            first_value,
            deltas,
        }
    }

    pub fn decode(&self) -> Vec<i64> {
        if self.deltas.is_empty() && self.first_value == 0 {
            return Vec::new();
        }
        let mut result = Vec::with_capacity(self.deltas.len() + 1);
        result.push(self.first_value);
        let mut current = self.first_value;
        for &d in &self.deltas {
            current = current.wrapping_add(d);
            result.push(current);
        }
        result
    }
}

/// 04-052: Bit Packing for dense unsigned integers with bit width 0..=64
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitPackedColumn {
    pub bit_width: u8,
    pub data: Vec<u64>,
    pub num_elements: usize,
}

impl BitPackedColumn {
    pub fn encode(values: &[u64], bit_width: u8) -> Self {
        assert!(bit_width <= 64);
        if bit_width == 0 || values.is_empty() {
            return Self {
                bit_width,
                data: Vec::new(),
                num_elements: values.len(),
            };
        }
        if bit_width == 64 {
            return Self {
                bit_width: 64,
                data: values.to_vec(),
                num_elements: values.len(),
            };
        }

        let total_bits = values.len() * bit_width as usize;
        let num_u64s = total_bits.div_ceil(64);
        let mut data = vec![0u64; num_u64s];

        for (i, &val) in values.iter().enumerate() {
            let mask = if bit_width == 64 { u64::MAX } else { (1u64 << bit_width) - 1 };
            let val = val & mask;
            let bit_offset = i * bit_width as usize;
            let word_idx = bit_offset / 64;
            let bit_in_word = bit_offset % 64;

            data[word_idx] |= val << bit_in_word;
            if bit_in_word + (bit_width as usize) > 64 && word_idx + 1 < data.len() {
                data[word_idx + 1] |= val >> (64 - bit_in_word);
            }
        }

        Self {
            bit_width,
            data,
            num_elements: values.len(),
        }
    }

    pub fn decode(&self) -> Vec<u64> {
        if self.bit_width == 0 {
            return vec![0u64; self.num_elements];
        }
        if self.bit_width == 64 {
            return self.data.clone();
        }

        let mut result = Vec::with_capacity(self.num_elements);
        let mask = (1u64 << self.bit_width) - 1;

        for i in 0..self.num_elements {
            let bit_offset = i * self.bit_width as usize;
            let word_idx = bit_offset / 64;
            let bit_in_word = bit_offset % 64;

            let mut val = (self.data[word_idx] >> bit_in_word) & mask;
            if bit_in_word + (self.bit_width as usize) > 64 && word_idx + 1 < self.data.len() {
                let overflow_bits = (bit_in_word + self.bit_width as usize) - 64;
                let overflow_mask = (1u64 << overflow_bits) - 1;
                val |= (self.data[word_idx + 1] & overflow_mask) << (64 - bit_in_word);
            }
            result.push(val);
        }
        result
    }
}
