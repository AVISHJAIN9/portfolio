use columnar_query_engine::storage::chunk::ColumnChunk;
use columnar_query_engine::storage::table::{ColumnarTable, RowGroup};
use columnar_query_engine::execution::aggregate::{AggOp, AggValue};
use columnar_query_engine::execution::vector::Predicate;
use columnar_query_engine::optimizer::plan::{PhysicalEngine, QueryPlan, QueryResult};

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Initializing Vectorized Columnar OLAP Engine v0.1.0...");

    let ids = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let amounts = vec![100, 250, 40, 500, 300, 120, 800, 50, 950, 200];
    let regions = vec![
        "US-East".into(), "US-West".into(), "EU-Central".into(), "US-East".into(), "APAC".into(),
        "US-East".into(), "EU-Central".into(), "APAC".into(), "US-East".into(), "US-West".into()
    ];

    let c_id = ColumnChunk::new_int64("id", ids);
    let c_amount = ColumnChunk::new_int64("amount", amounts);
    let c_region = ColumnChunk::new_utf8("region", regions);

    let rg = RowGroup::new(vec![c_id, c_amount, c_region]);
    let mut table = ColumnarTable::new("orders");
    table.add_row_group(rg);

    tracing::info!("Loaded table 'orders' with {} rows", table.total_rows());

    // Query: SELECT region, SUM(amount) FROM orders WHERE amount > 100 GROUP BY region
    let plan = QueryPlan {
        table_name: "orders".into(),
        projections: vec!["region".into(), "amount".into()],
        predicates: vec![Predicate::Gt("amount".into(), 100)],
        aggregate: Some(AggOp::Sum("amount".into())),
        group_by_col: Some("region".into()),
    };

    let result = PhysicalEngine::execute(&plan, &table).expect("Execution failed");
    if let QueryResult::Grouped(groups) = result {
        for (region, val) in groups {
            if let AggValue::Int(sum) = val {
                tracing::info!("  Region: {:<12} | SUM(amount): {}", region, sum);
            }
        }
    }
}
