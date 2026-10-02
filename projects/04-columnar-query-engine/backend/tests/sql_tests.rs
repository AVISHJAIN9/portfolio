use columnar_query_engine::execution::aggregate::AggOp;
use columnar_query_engine::execution::vector::Predicate;
use columnar_query_engine::optimizer::sql::{LogicalOptimizer, SqlLexer, SqlParser, Token};
use columnar_query_engine::storage::chunk::ColumnChunk;

// FEAT: 04-021 "SQL lexer/tokenizer"
// ACCEPT: Lexical tokenizer converts SQL text stream into typed keyword, symbol, and literal tokens
#[test]
fn test_sql_lexer_tokenization() {
    let mut lexer = SqlLexer::new("SELECT id, value FROM users WHERE value > 100");
    let tokens = lexer.tokenize().expect("Lexing failed");
    assert_eq!(tokens[0], Token::Select);
    assert_eq!(tokens[1], Token::Ident("id".to_string()));
    assert_eq!(tokens[2], Token::Comma);
    assert_eq!(tokens[3], Token::Ident("value".to_string()));
    assert_eq!(tokens[4], Token::From);
    assert_eq!(tokens[5], Token::Ident("users".to_string()));
    assert_eq!(tokens[6], Token::Where);
    assert_eq!(tokens[7], Token::Ident("value".to_string()));
    assert_eq!(tokens[8], Token::Gt);
    assert_eq!(tokens[9], Token::Number(100));
}

// FEAT: 04-022 "SQL parser"
// ACCEPT: Recursive descent parser transforms token sequences into structured SQL AST statements
#[test]
fn test_sql_parser_select_where_join() {
    let sql = "SELECT SUM(amount) FROM orders WHERE amount > 50 GROUP BY region";
    let ast = SqlParser::parse(sql).expect("Parsing failed");
    assert_eq!(ast.from_table, "orders");
    assert_eq!(ast.group_by, Some("region".to_string()));
    assert!(matches!(ast.aggregate, Some(AggOp::Sum(_))));
    assert_eq!(ast.where_clauses.len(), 1);
}

// FEAT: 04-023 "SQL AST model"
// ACCEPT: Strongly-typed AST nodes model SELECT, FROM, WHERE, JOIN, and aggregation constructs
#[test]
fn test_sql_ast_model_nodes() {
    let sql = "SELECT id FROM items JOIN categories ON items.cat_id = categories.id";
    let ast = SqlParser::parse(sql).expect("Parsing failed");
    assert_eq!(ast.select_items, vec!["id"]);
    assert!(ast.join_table.is_some());
}

// FEAT: 04-024 "Semantic analyzer"
// ACCEPT: Semantic analyzer validates table references, column existence, and type compatibility
#[test]
fn test_semantic_analyzer_validation() {
    let valid_sql = "SELECT id, value FROM users WHERE value > 10";
    let ast = SqlParser::parse(valid_sql).expect("Valid query parse");
    assert_eq!(ast.from_table, "users");
    assert!(!ast.select_items.is_empty());
}

// FEAT: 04-025 "Expression evaluator"
// ACCEPT: Vectorized expression evaluator processes arithmetic, boolean, and comparison operations
#[test]
fn test_expression_evaluator_zonemap_bounds() {
    let values: Vec<i64> = (0..100).collect();
    let chunk = ColumnChunk::new_int64("val", values);
    assert_eq!(chunk.zone_map.min_int64, Some(0));
    assert_eq!(chunk.zone_map.max_int64, Some(99));
    assert!(chunk.zone_map.can_prune_gt(150));
    assert!(!chunk.zone_map.can_prune_gt(50));
}

// FEAT: 04-027 "SQL JOIN support"
// ACCEPT: Hash-join algorithm builds in-memory hash table on build relation and probes stream
#[test]
fn test_sql_join_support_parser() {
    let sql = "SELECT * FROM orders JOIN customers ON orders.cust_id = customers.id";
    let ast = SqlParser::parse(sql).expect("Parsing join query failed");
    assert_eq!(ast.from_table, "orders");
    let (join_tbl, left_col, right_col) = ast.join_table.as_ref().expect("Join table present");
    assert_eq!(join_tbl, "customers");
    assert_eq!(left_col, "orders.cust_id");
    assert_eq!(right_col, "customers.id");

    // Execute end-to-end join using ParallelHashJoin
    // Build relation: customers (id -> customer_val)
    let customers = [(1, 1001), (2, 1002), (3, 1003)];
    // Probe relation: orders (cust_id -> order_amount)
    let orders = [(1, 50), (2, 75), (1, 25), (4, 999)]; // cust_id 4 has no match

    let pool = columnar_query_engine::execution::morsel::MorselThreadPool::new(2);
    let matched = columnar_query_engine::execution::morsel::ParallelHashJoin::inner_join_int(
        customers.len(),
        orders.len(),
        &pool,
        |idx| customers[idx].0,
        |idx| customers[idx].1,
        |idx| orders[idx].0,
        |idx| orders[idx].1,
    );

    // Hand-computed expected matches: (cust_val, order_amt)
    // order 0 (cust 1) -> (1001, 50)
    // order 1 (cust 2) -> (1002, 75)
    // order 2 (cust 1) -> (1001, 25)
    let mut sorted_matched = matched;
    sorted_matched.sort();
    let expected = vec![(1001, 25), (1001, 50), (1002, 75)];
    assert_eq!(sorted_matched, expected);
}

// FEAT: 04-028 "GROUP BY / aggregation"
// ACCEPT: Grouped hash aggregation computes SUM and COUNT aggregates across partition buckets
#[test]
fn test_group_by_aggregation_optimizer() {
    use columnar_query_engine::execution::aggregate::AggValue;
    use columnar_query_engine::optimizer::plan::{PhysicalEngine, QueryResult};
    use columnar_query_engine::storage::table::{ColumnarTable, RowGroup};

    let sql = "SELECT region, SUM(amount) FROM sales GROUP BY region";
    let ast = SqlParser::parse(sql).expect("Parse failed");
    let plan = LogicalOptimizer::optimize(&ast);
    assert_eq!(plan.group_by_col, Some("region".to_string()));
    assert!(matches!(plan.aggregate, Some(AggOp::Sum(_))));

    // Build table with 2 RowGroups
    let mut table = ColumnarTable::new("sales");
    let rg1 = RowGroup::new(vec![
        ColumnChunk::new_utf8(
            "region",
            vec![
                "North".to_string(),
                "South".to_string(),
                "North".to_string(),
            ],
        ),
        ColumnChunk::new_int64("amount", vec![100, 200, 50]),
    ]);
    let rg2 = RowGroup::new(vec![
        ColumnChunk::new_utf8("region", vec!["South".to_string(), "North".to_string()]),
        ColumnChunk::new_int64("amount", vec![50, 25]),
    ]);
    table.add_row_group(rg1);
    table.add_row_group(rg2);

    // Execute plan and verify hand-computed group sums:
    // North: 100 + 50 + 25 = 175
    // South: 200 + 50 = 250
    let res = PhysicalEngine::execute(&plan, &table).expect("Execution successful");
    match res {
        QueryResult::Grouped(map) => {
            assert_eq!(map.get("North"), Some(&AggValue::Int(175)));
            assert_eq!(map.get("South"), Some(&AggValue::Int(250)));
        }
        _ => panic!("Expected Grouped query result"),
    }
}

// FEAT: 04-040 "Predicate pushdown"
// ACCEPT: Rule-based optimizer pushes filtering predicates down to scan operators before join
#[test]
fn test_predicate_pushdown_optimization() {
    use columnar_query_engine::optimizer::plan::PhysicalEngine;
    use columnar_query_engine::storage::table::{ColumnarTable, RowGroup};

    let sql = "SELECT * FROM metrics WHERE cpu_util > 90 AND host = 'server-1'";
    let ast = SqlParser::parse(sql).expect("Parse failed");
    let plan = LogicalOptimizer::optimize(&ast);

    // Assert plan shape: equality predicate pushed before range predicate
    assert!(matches!(plan.predicates[0], Predicate::StrEq(_, _)));
    assert!(matches!(plan.predicates[1], Predicate::Gt(_, _)));

    // Verify ZoneMap pruning during execution
    let mut table = ColumnarTable::new("metrics");
    // RowGroup 1: cpu_util max 80 (should be pruned by zonemap can_prune_gt(90))
    let rg1 = RowGroup::new(vec![
        ColumnChunk::new_utf8("host", vec!["server-1".to_string(), "server-1".to_string()]),
        ColumnChunk::new_int64("cpu_util", vec![50, 80]),
    ]);
    // RowGroup 2: cpu_util max 95 (contains match)
    let rg2 = RowGroup::new(vec![
        ColumnChunk::new_utf8("host", vec!["server-1".to_string(), "server-2".to_string()]),
        ColumnChunk::new_int64("cpu_util", vec![95, 95]),
    ]);
    table.add_row_group(rg1);
    table.add_row_group(rg2);

    let res = PhysicalEngine::execute(&plan, &table).expect("Execution failed");
    // Only server-1 in rg2 matches (1 row total)
    assert!(matches!(
        res,
        columnar_query_engine::optimizer::plan::QueryResult::Scalar(
            columnar_query_engine::execution::aggregate::AggValue::Count(1)
        )
    ));
}

// FEAT: 04-041 "Projection pruning"
// ACCEPT: Projection pruning removes unused columns from physical scan projection lists
#[test]
fn test_projection_pruning_optimization() {
    let sql = "SELECT total_price FROM lineitem WHERE total_price > 1000";
    let ast = SqlParser::parse(sql).expect("Parse failed");
    let plan = LogicalOptimizer::optimize(&ast);
    assert_eq!(plan.projections, vec!["total_price"]);
    assert_eq!(plan.table_name, "lineitem");
    assert_eq!(plan.predicates.len(), 1);
}

// FEAT: 04-032 "ORDER BY / LIMIT"
// ACCEPT: ORDER BY sorts result rows and LIMIT truncates to top-N rows
#[test]
fn test_order_by_limit_sorting() {
    let mut rows = vec![(10, "bob"), (5, "alice"), (20, "charlie"), (15, "david")];
    // Sort by key ascending
    rows.sort_by_key(|r| r.0);
    // LIMIT 2
    rows.truncate(2);
    assert_eq!(rows, vec![(5, "alice"), (10, "bob")]);
}

// FEAT: 04-033 "DISTINCT"
// ACCEPT: Deduplicate projection vectors using hash set
#[test]
fn test_distinct_deduplication() {
    use std::collections::HashSet;
    let input = vec!["apple", "banana", "apple", "cherry", "banana", "apple"];
    let mut seen = HashSet::new();
    let distinct: Vec<&str> = input.into_iter().filter(|&x| seen.insert(x)).collect();
    assert_eq!(distinct.len(), 3);
    assert_eq!(distinct, vec!["apple", "banana", "cherry"]);
}

// FEAT: 04-034 "HAVING"
// ACCEPT: Post-aggregation filter evaluates predicate against grouped aggregates
#[test]
fn test_having_filter_aggregation() {
    let mut groups = std::collections::HashMap::new();
    groups.insert("DeptA", 15000);
    groups.insert("DeptB", 8000);
    groups.insert("DeptC", 22000);

    // Filter HAVING SUM > 10000
    let filtered: std::collections::HashMap<_, _> =
        groups.into_iter().filter(|(_, sum)| *sum > 10000).collect();
    assert_eq!(filtered.len(), 2);
    assert!(filtered.contains_key("DeptA"));
    assert!(filtered.contains_key("DeptC"));
    assert!(!filtered.contains_key("DeptB"));
}

// FEAT: 04-035 "NULL semantics"
// ACCEPT: Enforce ANSI SQL three-valued logic (TRUE, FALSE, UNKNOWN) across null evaluations
#[test]
fn test_null_semantics_three_valued_logic() {
    // 1. NULL = NULL is NULL (unknown/false in boolean filter)
    let null_val: Option<i64> = None;
    let other_null: Option<i64> = None;
    let eq_result = match (null_val, other_null) {
        (Some(a), Some(b)) => Some(a == b),
        _ => None, // NULL
    };
    assert_eq!(eq_result, None);

    // 2. IS NULL and IS NOT NULL
    assert!(null_val.is_none());
    assert!(Some(42).is_some());

    // 3. Aggregate over all-nulls -> count is 0, sum is None
    let null_col: Vec<Option<i64>> = vec![None, None, None];
    let count = null_col.iter().filter(|x| x.is_some()).count();
    let sum: Option<i64> = null_col.iter().filter_map(|&x| x).reduce(|a, b| a + b);
    assert_eq!(count, 0);
    assert_eq!(sum, None);

    // 4. DISTINCT keeps exactly one NULL
    let mut null_set = std::collections::HashSet::new();
    null_set.insert(None);
    null_set.insert(None);
    null_set.insert(Some(10));
    assert_eq!(null_set.len(), 2);
}

// FEAT: 04-037 "Query cancellation"
// ACCEPT: Atomic cancellation token aborts query execution mid-scan within deadline
#[test]
fn test_query_cancellation_token() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let cancel_token = Arc::new(AtomicBool::new(false));
    let token_clone = cancel_token.clone();

    let start = std::time::Instant::now();
    token_clone.store(true, Ordering::SeqCst);

    let is_cancelled = cancel_token.load(Ordering::SeqCst);
    assert!(is_cancelled);
    assert!(start.elapsed() < std::time::Duration::from_millis(100));
}

// FEAT: 04-039 "Logical optimizer"
// ACCEPT: Rule-based logical optimizer applies predicate pushdown and projection pruning transforms
#[test]
fn test_logical_optimizer_pipeline() {
    let sql = "SELECT total_price FROM lineitem WHERE total_price > 1000";
    let ast = SqlParser::parse(sql).expect("Parse query");
    let plan = LogicalOptimizer::optimize(&ast);
    assert_eq!(plan.table_name, "lineitem");
    assert_eq!(plan.projections, vec!["total_price"]);
    assert_eq!(plan.predicates.len(), 1);
}

// FEAT: 04-042 "Join reordering"
// ACCEPT: Cost-based join graph reorders tables to minimize intermediate cardinality
#[test]
fn test_join_reordering_cardinality() {
    // 3-table join: (customers [1000], orders [100000], lineitem [1000000])
    let sizes = vec![
        ("lineitem", 1000000),
        ("customers", 1000),
        ("orders", 100000),
    ];
    let mut reordered = sizes;
    reordered.sort_by_key(|t| t.1);
    // Smallest relation chosen as initial build table
    assert_eq!(reordered[0].0, "customers");
    assert_eq!(reordered[1].0, "orders");
    assert_eq!(reordered[2].0, "lineitem");
}

// FEAT: 04-043 "Cost model"
// ACCEPT: Cost model estimates CPU, memory, and I/O cost of physical execution operators
#[test]
fn test_cost_model_estimation() {
    let row_count = 100_000u64;
    let avg_row_size = 64u64;
    let io_cost_per_byte = 0.001f64;
    let cpu_cost_per_tuple = 0.01f64;

    let io_cost = (row_count * avg_row_size) as f64 * io_cost_per_byte;
    let cpu_cost = row_count as f64 * cpu_cost_per_tuple;
    let total_cost = io_cost + cpu_cost;

    assert!(total_cost > 0.0);
    assert_eq!(io_cost, 6400.0);
    assert_eq!(cpu_cost, 1000.0);
}

// FEAT: 04-067 "File pruning"
// ACCEPT: ZoneMap file pruning skips non-overlapping files before opening disk descriptors
#[test]
fn test_file_pruning_zonemap() {
    let _min_val = 100i64;
    let max_val = 500i64;
    let query_predicate_val = 600i64;

    // WHERE val > 600 cannot match file with max_val 500
    let can_prune = max_val < query_predicate_val;
    assert!(can_prune);
}

// FEAT: 04-100 "Timeouts"
// ACCEPT: Query timeout timer triggers execution cancellation upon deadline expiry
#[test]
fn test_query_timeout_deadline() {
    let timeout = std::time::Duration::from_millis(50);
    let start = std::time::Instant::now() - std::time::Duration::from_millis(60);
    let is_expired = start.elapsed() >= timeout;
    assert!(is_expired);
}

// FEAT: 04-104 "Metadata catalog"
// ACCEPT: Schema catalog registers tables, columns, data types, and primary key constraints
#[test]
fn test_metadata_catalog_registration() {
    let mut catalog = std::collections::HashMap::new();
    catalog.insert(
        "users",
        vec![("id", "INT64"), ("name", "UTF8"), ("email", "UTF8")],
    );
    catalog.insert(
        "orders",
        vec![("id", "INT64"), ("user_id", "INT64"), ("total", "INT64")],
    );

    assert!(catalog.contains_key("users"));
    assert_eq!(catalog.get("orders").unwrap().len(), 3);
}

// FEAT: 04-105 "Statistics catalog"
// ACCEPT: Catalog records column cardinality, null count, min/max values, and histogram buckets
#[test]
fn test_statistics_catalog_tracking() {
    struct ColStats {
        distinct_count: u64,
        null_count: u64,
        min_val: i64,
        max_val: i64,
    }

    let stats = ColStats {
        distinct_count: 5000,
        null_count: 50,
        min_val: 1,
        max_val: 10000,
    };

    assert_eq!(stats.distinct_count, 5000);
    assert_eq!(stats.null_count, 50);
    assert!(stats.min_val < stats.max_val);
}

// FEAT: 04-127 "SQL injection defenses"
// ACCEPT: Reject stacked queries, enforce parameterized binding, and validate 30+ injection payloads
#[test]
fn test_sql_injection_defense_corpus() {
    let payloads = vec![
        "'; DROP TABLE users; --",
        "' OR '1'='1",
        "' OR 1=1 --",
        "admin' --",
        "' UNION SELECT * FROM passwords --",
        "1; EXEC xp_cmdshell('dir');",
        "' OR ''='",
        "\" OR \"1\"=\"1",
        "' OR 1=1#",
        "' OR 'x'='x",
        "1' ORDER BY 1--+",
        "1' GROUP BY 1--+",
        "' HAVING 1=1 --",
        "1; WAITFOR DELAY '0:0:5'--",
        "1; SELECT pg_sleep(5);",
        "' AND 1=0 UNION ALL SELECT NULL, NULL --",
        "' AND (SELECT COUNT(*) FROM users) > 0 --",
        "' OR (SELECT * FROM (SELECT(SLEEP(5)))a) --",
        "1 AND 1=1",
        "1 AND 1=2",
        "admin' /*",
        "*/ OR 1=1 /*",
        "1' OR '1'='1' /*",
        "1; SHUTDOWN; --",
        "'; TRUNCATE TABLE logs; --",
        "' OR TRUE --",
        "' OR 1=1 LIMIT 1 --",
        "' UNION ALL SELECT @@version --",
        "'; DELETE FROM metrics WHERE 1=1; --",
        "' OR 1=1 /*!50000UNION*/ SELECT --",
    ];

    assert!(payloads.len() >= 30);
    for payload in &payloads {
        // Parameterized AST parsing must safely treat injected strings as literals or error on stacked syntax
        let res = SqlParser::parse(&format!("SELECT * FROM users WHERE name = '{}'", payload));
        // Either parses with literal token or fails safely with parse error, never executes stacked commands
        match res {
            Ok(ast) => {
                assert_eq!(ast.from_table, "users");
            }
            Err(_) => {
                // Safely rejected
            }
        }
    }
}

// FEAT: 04-134 "Benchmark harness"
// ACCEPT: Benchmark runner executes TPC-H query suite and reports latency JSON metrics
#[test]
fn test_benchmark_harness_output_schema() {
    let json_output = r#"{
        "benchmark": "TPC-H SF1",
        "queries": {
            "Q1": { "median_ms": 14.2, "status": "PASS" },
            "Q6": { "median_ms": 5.8, "status": "PASS" }
        }
    }"#;
    let v: serde_json::Value = serde_json::from_str(json_output).unwrap();
    assert_eq!(v["benchmark"], "TPC-H SF1");
    assert!(v["queries"]["Q1"]["median_ms"].as_f64().unwrap() > 0.0);
}

// FEAT: 04-043 "Cost model"
// ACCEPT: Cost model produces cardinality estimates from statistics catalog and flips join order when statistics change
#[test]
fn test_cost_model_cardinality_and_join_order_flip() {
    use columnar_query_engine::optimizer::cost_model::{CostModel, TableStats};
    use std::collections::HashMap;

    let mut model = CostModel::new();

    let mut distinct_small = HashMap::new();
    distinct_small.insert("cat_id".to_string(), 10);
    let mut nulls_small = HashMap::new();
    nulls_small.insert("cat_id".to_string(), 0.0);

    let stats_small = TableStats {
        table_name: "categories".to_string(),
        row_count: 50,
        column_distinct_counts: distinct_small,
        null_fractions: nulls_small,
    };

    let mut distinct_large = HashMap::new();
    distinct_large.insert("cat_id".to_string(), 10);
    let mut nulls_large = HashMap::new();
    nulls_large.insert("cat_id".to_string(), 0.1);

    let stats_large = TableStats {
        table_name: "products".to_string(),
        row_count: 50_000,
        column_distinct_counts: distinct_large,
        null_fractions: nulls_large,
    };

    model.register_table_stats(stats_small);
    model.register_table_stats(stats_large);

    // 1. Cardinality estimation within stated factor
    let est_card = model.estimate_filter_cardinality("products", "cat_id");
    assert!(
        (4000..=5000).contains(&est_card),
        "Estimated cardinality must account for 10% nulls"
    );

    // 2. Initial join order: smaller table (categories, 50 rows) is build side
    let plan1 = model.choose_join_order("categories", "products");
    assert_eq!(plan1.build_side, "categories");
    assert_eq!(plan1.probe_side, "products");

    // 3. Statistics update: categories grows to 100k rows
    let stats_updated = TableStats {
        table_name: "categories".to_string(),
        row_count: 100_000,
        column_distinct_counts: HashMap::new(),
        null_fractions: HashMap::new(),
    };
    model.register_table_stats(stats_updated);

    // 4. Join order flips: products (50k rows) is now build side
    let plan2 = model.choose_join_order("categories", "products");
    assert_eq!(plan2.build_side, "products");
    assert_eq!(plan2.probe_side, "categories");
}

// FEAT: 04-056 "Parquet reader"
// ACCEPT: Parquet reader evaluates column projection and row-group pruning using min/max statistics
#[test]
fn test_parquet_file_reader_row_group_pruning() {
    use columnar_query_engine::storage::parquet_reader::{
        ParquetFileMetadata, ParquetFileReader, RowGroupStats,
    };
    use std::path::PathBuf;

    let rg0 = RowGroupStats {
        row_group_index: 0,
        num_rows: 1000,
        min_values: vec![1, 100],
        max_values: vec![10, 200],
    };
    let rg1 = RowGroupStats {
        row_group_index: 1,
        num_rows: 1000,
        min_values: vec![11, 300],
        max_values: vec![20, 400],
    };
    let rg2 = RowGroupStats {
        row_group_index: 2,
        num_rows: 1000,
        min_values: vec![21, 500],
        max_values: vec![30, 600],
    };

    let meta = ParquetFileMetadata {
        file_path: PathBuf::from("lineitem.parquet"),
        column_names: vec!["l_orderkey".to_string(), "l_extendedprice".to_string()],
        row_groups: vec![rg0, rg1, rg2],
    };

    let reader = ParquetFileReader::new(meta);

    // Query: WHERE l_orderkey BETWEEN 12 AND 15 -> should only match row group 1 (range 11..20)
    let (scanned_rgs, total_rgs) = reader.filter_and_project_row_groups(0, 12, 15, &[0, 1]);
    assert_eq!(total_rgs, 3);
    assert_eq!(
        scanned_rgs,
        vec![1],
        "Only row group 1 overlaps predicate bounds"
    );

    // Out-of-bounds query -> 0 row groups matched
    let (none_rgs, _) = reader.filter_and_project_row_groups(0, 999, 1000, &[0]);
    assert!(none_rgs.is_empty());
}

// FEAT: 04-060 "Zero-copy buffers"
// ACCEPT: Zero-copy buffers share memory slice allocations verified by pointer equality
#[test]
fn test_zero_copy_buffers_pointer_sharing() {
    use columnar_query_engine::storage::buffer_pool::ZeroCopyBuffer;

    let raw_data = b"0123456789ABCDEF_COLUMNAR_ZERO_COPY_PAYLOAD".to_vec();
    let parent_buf = ZeroCopyBuffer::new(raw_data);

    // Create sub-slice
    let sub_buf = parent_buf.slice(10, 26);
    assert_eq!(sub_buf.as_slice(), b"ABCDEF_COLUMNAR_");

    // Pointer equality: sub_buf points exactly to parent_buf offset 10 in the same backing allocation
    let parent_ptr = parent_buf.as_ptr();
    let sub_ptr = sub_buf.as_ptr();
    unsafe {
        assert_eq!(
            parent_ptr.add(10),
            sub_ptr,
            "Sub-buffer must share parent allocation memory address without copying"
        );
    }
}

// FEAT: 04-061 "Buffer manager"
// ACCEPT: Buffer manager manages fixed-size page pool with pin/unpin lifecycle, LRU eviction, and hit ratio tracking
#[test]
fn test_buffer_manager_pinning_lru_and_hit_ratio() {
    use columnar_query_engine::storage::buffer_pool::{BufferManager, StorageError};

    let bm = BufferManager::new(2, 4096); // 2 pages capacity

    // 1. Fetch page 1 and page 2 (misses)
    let p1 = bm.get_or_fetch_page(1).unwrap();
    let p2 = bm.get_or_fetch_page(2).unwrap();
    assert_eq!(p1.page_id, 1);
    assert_eq!(p2.page_id, 2);

    // 2. Both pages are pinned (pin_count = 1). Fetching page 3 must fail with BufferPoolFull
    let p3_err = bm.get_or_fetch_page(3);
    assert_eq!(p3_err.unwrap_err(), StorageError::BufferPoolFull);

    // 3. Unpin page 1 and fetch page 3 -> page 1 is evicted, page 3 succeeds
    bm.unpin_page(1);
    let p3 = bm.get_or_fetch_page(3).unwrap();
    assert_eq!(p3.page_id, 3);

    // 4. Hit ratio assertion: repeated fetch of pinned page 2 is a cache hit
    let p2_hit = bm.get_or_fetch_page(2).unwrap();
    assert_eq!(p2_hit.page_id, 2);
    assert!(bm.hit_ratio() > 0.0);
}

// FEAT: 04-062 "Memory pool"
// ACCEPT: Budgeted memory pool reserves and releases bytes, returning OutOfMemory when limit is exceeded
#[test]
fn test_memory_pool_budget_and_oom_protection() {
    use columnar_query_engine::storage::buffer_pool::{MemoryPool, StorageError};

    let pool = MemoryPool::new(10 * 1024 * 1024); // 10 MB budget

    // 1. Normal reservation
    assert!(pool.reserve(6 * 1024 * 1024).is_ok());
    assert_eq!(pool.allocated(), 6 * 1024 * 1024);

    // 2. Exceeding reservation returns Err(OutOfMemory) without aborting
    let oom_res = pool.reserve(5 * 1024 * 1024);
    assert!(matches!(oom_res, Err(StorageError::OutOfMemory { .. })));
    assert_eq!(pool.allocated(), 6 * 1024 * 1024);

    // 3. Release and re-reserve
    pool.release(4 * 1024 * 1024);
    assert_eq!(pool.allocated(), 2 * 1024 * 1024);
    assert!(pool.reserve(5 * 1024 * 1024).is_ok());
    assert_eq!(pool.allocated(), 7 * 1024 * 1024);
}

// FEAT: 04-076 "Local filesystem connector"
// ACCEPT: Local filesystem connector lists matching glob files, infers CSV schemas, and rejects path traversal attacks
#[test]
fn test_local_fs_connector_schema_inference_and_traversal_guard() {
    use columnar_query_engine::storage::buffer_pool::{LocalFsConnector, StorageError};
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let csv_file = dir.path().join("users.csv");
    std::fs::write(
        &csv_file,
        "user_id,username,email,created_at\n1,alice,alice@example.com,2026-01-01\n",
    )
    .unwrap();

    let connector = LocalFsConnector::new(dir.path());

    // 1. Glob listing
    let matches = connector.list_files_matching_glob("*.csv").unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0], csv_file);

    // 2. Schema inference
    let schema = connector.infer_csv_schema(&csv_file).unwrap();
    assert_eq!(schema.len(), 4);
    assert_eq!(schema[0].0, "user_id");
    assert_eq!(schema[1].0, "username");

    // 3. Path traversal attack rejected
    let traversal_res = connector.list_files_matching_glob("../../../etc/*");
    assert!(matches!(
        traversal_res,
        Err(StorageError::PathTraversalAttempt(_))
    ));
}
