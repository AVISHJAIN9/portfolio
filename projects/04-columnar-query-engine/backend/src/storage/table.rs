//! Columnar Table and RowGroup Partitioning

use crate::storage::chunk::ColumnChunk;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct RowGroup {
    pub columns: HashMap<String, ColumnChunk>,
    pub num_rows: usize,
}

impl RowGroup {
    pub fn new(columns: Vec<ColumnChunk>) -> Self {
        let num_rows = columns.first().map(|c| c.len()).unwrap_or(0);
        let mut map = HashMap::new();
        for col in columns {
            map.insert(col.name.clone(), col);
        }
        Self {
            columns: map,
            num_rows,
        }
    }

    pub fn get_column(&self, name: &str) -> Option<&ColumnChunk> {
        self.columns.get(name)
    }
}

#[derive(Debug, Clone)]
pub struct ColumnarTable {
    pub name: String,
    pub row_groups: Vec<RowGroup>,
}

impl ColumnarTable {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            row_groups: Vec::new(),
        }
    }

    pub fn add_row_group(&mut self, rg: RowGroup) {
        self.row_groups.push(rg);
    }

    pub fn total_rows(&self) -> usize {
        self.row_groups.iter().map(|rg| rg.num_rows).sum()
    }
}
