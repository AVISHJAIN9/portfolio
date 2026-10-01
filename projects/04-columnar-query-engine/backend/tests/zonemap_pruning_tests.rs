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
    let rg1 = RowGroup::new(vec![
        ColumnChunk::new_int64("val", (1..=1000).collect()),
    ]);

    // RowGroup 2: values 5000..6000
    let rg2 = RowGroup::new(vec![
        ColumnChunk::new_int64("val", (5000..=6000).collect()),
    ]);

    table.add_row_group(rg1);
    table.add_row_group(rg2);

    // Query: SELECT COUNT(*) WHERE val > 4000 (Should prune RG1 completely)
    let plan = QueryPlan {
        table_name: "metrics".into(),
        projections: vec!["val".into()],
        predicates: vec![Predicate::Gt("val".into(), 4000)],
        aggregate: Some(AggOp::Count),
        group_by_col: None,
    };

    let result = PhysicalEngine::execute(&plan, &table).expect("Execution failed");
    if let QueryResult::Scalar(AggValue::Count(c)) = result {
        assert_eq!(c, 1001);
    } else {
        panic!("Expected Scalar Count");
    }
}
