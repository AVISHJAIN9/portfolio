// FEAT: 04-078 "Vectorized execution"
// ACCEPT: Vectorized execution engine processes column vectors in batch slices with SIMD acceleration
use columnar_query_engine::execution::aggregate::{AggOp, AggValue};
use columnar_query_engine::execution::vector::Predicate;
use columnar_query_engine::optimizer::plan::{PhysicalEngine, QueryPlan, QueryResult};
use columnar_query_engine::storage::chunk::ColumnChunk;
use columnar_query_engine::storage::table::{ColumnarTable, RowGroup};

#[test]
fn test_vectorized_sum_filter_aggregation() {
    let ids: Vec<i64> = (1..=10_000).collect();
    let amounts: Vec<i64> = (1..=10_000).map(|i| i * 10).collect();
    let categories: Vec<String> = (1..=10_000)
        .map(|i| {
            if i % 2 == 0 {
                "Electronics".into()
            } else {
                "Books".into()
            }
        })
        .collect();

    let c_id = ColumnChunk::new_int64("id", ids);
    let c_amount = ColumnChunk::new_int64("amount", amounts);
    let c_cat = ColumnChunk::new_utf8("category", categories);

    let rg = RowGroup::new(vec![c_id, c_amount, c_cat]);
    let mut table = ColumnarTable::new("sales");
    table.add_row_group(rg);

    // Query: SELECT category, SUM(amount) FROM sales WHERE amount > 50000 GROUP BY category
    let plan = QueryPlan {
        table_name: "sales".into(),
        projections: vec!["category".into(), "amount".into()],
        predicates: vec![Predicate::Gt("amount".into(), 50000)],
        aggregate: Some(AggOp::Sum("amount".into())),
        group_by_col: Some("category".into()),
    };

    let result = PhysicalEngine::execute(&plan, &table).expect("Execution failed");
    if let QueryResult::Grouped(groups) = result {
        assert_eq!(groups.len(), 2, "Should return exactly 2 distinct groups");
        assert!(matches!(groups.get("Electronics"), Some(AggValue::Int(_))), "Electronics group present");
        assert!(matches!(groups.get("Books"), Some(AggValue::Int(_))), "Books group present");
        assert_eq!(groups.get("Clothing"), None, "Non-existent category should return None");
    } else {
        panic!("Expected Grouped QueryResult");
    }

    // Edge case: Empty vector / impossible filter
    let empty_plan = QueryPlan {
        table_name: "sales".into(),
        projections: vec!["category".into(), "amount".into()],
        predicates: vec![Predicate::Gt("amount".into(), 1_000_000)],
        aggregate: Some(AggOp::Sum("amount".into())),
        group_by_col: Some("category".into()),
    };
    let empty_res = PhysicalEngine::execute(&empty_plan, &table).expect("Execution on empty filter failed");
    if let QueryResult::Grouped(groups) = empty_res {
        assert_eq!(groups.len(), 0, "No groups should match impossible predicate");
    } else {
        panic!("Expected Grouped QueryResult");
    }
}
