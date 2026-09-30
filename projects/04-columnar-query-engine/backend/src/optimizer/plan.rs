//! Basic Query Planner and Physical Optimizer with ZoneMap Pruning

use crate::execution::aggregate::{AggOp, AggValue, VectorAggregator};
use crate::execution::vector::{Predicate, SelectionVector, VectorizedExecutor};
use crate::storage::table::ColumnarTable;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct QueryPlan {
    pub table_name: String,
    pub projections: Vec<String>,
    pub predicates: Vec<Predicate>,
    pub aggregate: Option<AggOp>,
    pub group_by_col: Option<String>,
}

#[derive(Debug, Clone)]
pub enum QueryResult {
    Scalar(AggValue),
    Grouped(HashMap<String, AggValue>),
    Table(Vec<HashMap<String, String>>),
}

pub struct PhysicalEngine;

impl PhysicalEngine {
    pub fn execute(plan: &QueryPlan, table: &ColumnarTable) -> Result<QueryResult, &'static str> {
        let mut global_matched_rows: usize = 0;
        let mut grouped_acc: HashMap<String, AggValue> = HashMap::new();
        let mut scalar_total_sum: i64 = 0;
        let mut scalar_total_count: usize = 0;

        for rg in &table.row_groups {
            // 1. Predicate pushdown via ZoneMap pruning
            let mut can_skip_rg = false;
            for pred in &plan.predicates {
                match pred {
                    Predicate::Gt(col_name, threshold) => {
                        if let Some(col) = rg.get_column(col_name) {
                            if col.zone_map.can_prune_gt(*threshold) {
                                can_skip_rg = true;
                                break;
                            }
                        }
                    }
                    Predicate::Lt(col_name, threshold) => {
                        if let Some(col) = rg.get_column(col_name) {
                            if col.zone_map.can_prune_lt(*threshold) {
                                can_skip_rg = true;
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }

            if can_skip_rg {
                continue;
            }

            // 2. Evaluate selection vectors
            let mut sel = SelectionVector::all(rg.num_rows);
            for pred in &plan.predicates {
                let target_col = match pred {
                    Predicate::Gt(c, _) | Predicate::Lt(c, _) | Predicate::Eq(c, _) | Predicate::Between(c, _, _) | Predicate::StrEq(c, _) => c,
                };
                if let Some(col) = rg.get_column(target_col) {
                    sel = VectorizedExecutor::evaluate_filter(col, pred, &sel);
                }
            }

            if sel.is_empty() {
                continue;
            }

            global_matched_rows += sel.len();

            // 3. Evaluate Aggregations
            if let Some(agg_op) = &plan.aggregate {
                if let Some(grp_col) = &plan.group_by_col {
                    let g_chunk = rg.get_column(grp_col).ok_or("Group by column not found")?;
                    let val_col_name = match agg_op {
                        AggOp::Sum(c) | AggOp::Min(c) | AggOp::Max(c) | AggOp::Avg(c) => c,
                        AggOp::Count => grp_col,
                    };
                    let v_chunk = rg.get_column(val_col_name).ok_or("Aggregate column not found")?;
                    let batch_res = VectorAggregator::group_by(g_chunk, v_chunk, agg_op, &sel);

                    for (k, v) in batch_res {
                        match v {
                            AggValue::Int(n) => {
                                let cur = grouped_acc.entry(k).or_insert(AggValue::Int(0));
                                if let AggValue::Int(c) = cur {
                                    *c += n;
                                }
                            }
                            AggValue::Count(c) => {
                                let cur = grouped_acc.entry(k).or_insert(AggValue::Count(0));
                                if let AggValue::Count(existing) = cur {
                                    *existing += c;
                                }
                            }
                            _ => {}
                        }
                    }
                } else {
                    let val_col_name = match agg_op {
                        AggOp::Sum(c) | AggOp::Min(c) | AggOp::Max(c) | AggOp::Avg(c) => c,
                        AggOp::Count => "",
                    };
                    if !val_col_name.is_empty() {
                        let v_chunk = rg.get_column(val_col_name).ok_or("Aggregate column not found")?;
                        let agg = VectorAggregator::aggregate_column(v_chunk, agg_op, &sel);
                        if let AggValue::Int(n) = agg {
                            scalar_total_sum += n;
                        }
                    } else {
                        scalar_total_count += sel.len();
                    }
                }
            }
        }

        if let Some(agg_op) = &plan.aggregate {
            if plan.group_by_col.is_some() {
                Ok(QueryResult::Grouped(grouped_acc))
            } else if matches!(agg_op, AggOp::Count) {
                Ok(QueryResult::Scalar(AggValue::Count(scalar_total_count)))
            } else {
                Ok(QueryResult::Scalar(AggValue::Int(scalar_total_sum)))
            }
        } else {
            Ok(QueryResult::Scalar(AggValue::Count(global_matched_rows)))
        }
    }
}
