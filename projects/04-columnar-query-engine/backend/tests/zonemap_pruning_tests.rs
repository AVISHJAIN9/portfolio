// FEAT: 04-053 "Zone maps"
// ACCEPT: Zone maps record per-chunk min/max boundaries and prune row groups before deserialization
use columnar_query_engine::execution::aggregate::{AggOp, AggValue};
use columnar_query_engine::execution::vector::Predicate;
use columnar_query_engine::optimizer::plan::{PhysicalEngine, QueryPlan, QueryResult};
use columnar_query_engine::storage::chunk::ColumnChunk;
use columnar_query_engine::storage::table::{ColumnarTable, RowGroup};

#[test]
fn test_zonemap_rowgroup_skipping() {
    let mut table = ColumnarTable::new("metrics");

    // RowGroup 1: values 1..1000
    let rg1 = RowGroup::new(vec![ColumnChunk::new_int64("val", (1..=1000).collect())]);

    // RowGroup 2: values 5000..6000
    let rg2 = RowGroup::new(vec![ColumnChunk::new_int64("val", (5000..=6000).collect())]);

    table.add_row_group(rg1);
    table.add_row_group(rg2);

    // Query 1: SELECT COUNT(*) WHERE val > 4000 (Should prune RG1 completely)
    let plan1 = QueryPlan {
        table_name: "metrics".into(),
        projections: vec!["val".into()],
        predicates: vec![Predicate::Gt("val".into(), 4000)],
        aggregate: Some(AggOp::Count),
        group_by_col: None,
    };

    let result1 = PhysicalEngine::execute(&plan1, &table).expect("Execution failed");
    if let QueryResult::Scalar(AggValue::Count(c)) = result1 {
        assert_eq!(c, 1001, "Should count only RG2 rows matching predicate");
    } else {
        panic!("Expected Scalar Count");
    }

    // Query 2: Negative/Edge - Query for values outside both row groups (val > 10000 -> prune both)
    let plan2 = QueryPlan {
        table_name: "metrics".into(),
        projections: vec!["val".into()],
        predicates: vec![Predicate::Gt("val".into(), 10000)],
        aggregate: Some(AggOp::Count),
        group_by_col: None,
    };
    let result2 = PhysicalEngine::execute(&plan2, &table).expect("Execution failed");
    if let QueryResult::Scalar(AggValue::Count(c)) = result2 {
        assert_eq!(c, 0, "Should prune all row groups when predicate out of bounds");
    } else {
        panic!("Expected Scalar Count");
    }

    // Query 3: Negative/Edge - Query matching only RG1 (val < 2000 -> prune RG2 completely)
    let plan3 = QueryPlan {
        table_name: "metrics".into(),
        projections: vec!["val".into()],
        predicates: vec![Predicate::Lt("val".into(), 2000)],
        aggregate: Some(AggOp::Count),
        group_by_col: None,
    };
    let result3 = PhysicalEngine::execute(&plan3, &table).expect("Execution failed");
    if let QueryResult::Scalar(AggValue::Count(c)) = result3 {
        assert_eq!(c, 1000, "Should match exactly RG1 rows");
        assert_ne!(c, 2001, "Should not include RG2 rows");
    } else {
        panic!("Expected Scalar Count");
    }
}
