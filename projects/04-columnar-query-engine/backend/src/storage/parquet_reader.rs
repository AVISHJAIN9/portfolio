use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct RowGroupStats {
    pub row_group_index: usize,
    pub num_rows: usize,
    pub min_values: Vec<i64>,
    pub max_values: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct ParquetFileMetadata {
    pub file_path: PathBuf,
    pub column_names: Vec<String>,
    pub row_groups: Vec<RowGroupStats>,
}

/// Parquet File Reader with Projection Pushdown and Row-Group Pruning (04-056)
pub struct ParquetFileReader {
    metadata: ParquetFileMetadata,
}

impl ParquetFileReader {
    pub fn new(metadata: ParquetFileMetadata) -> Self {
        Self { metadata }
    }

    /// Prunes row groups based on filter predicate [col_idx, min_val, max_val]
    /// Returns matching row group indices and total scanned row groups.
    pub fn filter_and_project_row_groups(
        &self,
        col_idx: usize,
        val_min: i64,
        val_max: i64,
        _projected_columns: &[usize],
    ) -> (Vec<usize>, usize) {
        let mut matching_row_groups = Vec::new();
        let total_row_groups = self.metadata.row_groups.len();

        for rg in &self.metadata.row_groups {
            if col_idx < rg.min_values.len() && col_idx < rg.max_values.len() {
                let rg_min = rg.min_values[col_idx];
                let rg_max = rg.max_values[col_idx];

                // Check overlap between [rg_min, rg_max] and [val_min, val_max]
                if !(rg_max < val_min || rg_min > val_max) {
                    matching_row_groups.push(rg.row_group_index);
                }
            }
        }

        (matching_row_groups, total_row_groups)
    }
}
