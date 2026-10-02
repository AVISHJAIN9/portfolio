use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TableStats {
    pub table_name: String,
    pub row_count: usize,
    pub column_distinct_counts: HashMap<String, usize>,
    pub null_fractions: HashMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinPlan {
    pub build_side: String,
    pub probe_side: String,
    pub estimated_cost: usize,
}

/// Cost Model & Cardinality Estimator (04-043)
pub struct CostModel {
    catalog_stats: HashMap<String, TableStats>,
}

impl Default for CostModel {
    fn default() -> Self {
        Self::new()
    }
}

impl CostModel {
    pub fn new() -> Self {
        Self {
            catalog_stats: HashMap::new(),
        }
    }

    pub fn register_table_stats(&mut self, stats: TableStats) {
        self.catalog_stats.insert(stats.table_name.clone(), stats);
    }

    /// Estimates cardinality after equality filter
    pub fn estimate_filter_cardinality(&self, table: &str, column: &str) -> usize {
        if let Some(stats) = self.catalog_stats.get(table) {
            let distinct = stats
                .column_distinct_counts
                .get(column)
                .copied()
                .unwrap_or(1)
                .max(1);
            let null_frac = stats.null_fractions.get(column).copied().unwrap_or(0.0);
            let valid_rows = (stats.row_count as f64 * (1.0 - null_frac)) as usize;
            (valid_rows / distinct).max(1)
        } else {
            1
        }
    }

    /// Selects optimal 2-way join order: smaller table on build side, larger table on probe side
    pub fn choose_join_order(&self, table_a: &str, table_b: &str) -> JoinPlan {
        let rows_a = self
            .catalog_stats
            .get(table_a)
            .map(|s| s.row_count)
            .unwrap_or(1000);
        let rows_b = self
            .catalog_stats
            .get(table_b)
            .map(|s| s.row_count)
            .unwrap_or(1000);

        if rows_a <= rows_b {
            JoinPlan {
                build_side: table_a.to_string(),
                probe_side: table_b.to_string(),
                estimated_cost: rows_a * 2 + rows_b, // Hash build + probe cost
            }
        } else {
            JoinPlan {
                build_side: table_b.to_string(),
                probe_side: table_a.to_string(),
                estimated_cost: rows_b * 2 + rows_a,
            }
        }
    }
}
