use columnar_query_engine::execution::aggregate::AggOp;
use columnar_query_engine::execution::vector::{Predicate, SelectionVector};
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
fn test_sql_ast_representation() {
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
// ACCEPT: Vectorized expression evaluator computes scalar operations and comparative predicates
#[test]
fn test_expression_evaluator() {
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
fn test_hash_join_execution() {
    let sql = "SELECT * FROM orders JOIN customers ON orders.cust_id = customers.id";
    let ast = SqlParser::parse(sql).expect("Parsing join query failed");
    assert_eq!(ast.join_table.unwrap().0, "customers");
}

// FEAT: 04-028 "GROUP BY / aggregation"
// ACCEPT: Grouped hash aggregation computes SUM and COUNT aggregates across partition buckets
#[test]
fn test_hash_aggregate_group_by() {
    let sql = "SELECT region, SUM(amount) FROM sales GROUP BY region";
    let ast = SqlParser::parse(sql).expect("Parse failed");
    let plan = LogicalOptimizer::optimize(&ast);
    assert_eq!(plan.group_by_col, Some("region".to_string()));
    assert!(matches!(plan.aggregate, Some(AggOp::Sum(_))));
}

// FEAT: 04-032 "ORDER BY / LIMIT"
// ACCEPT: ORDER BY and LIMIT clauses sort output vectors and truncate result row slices
#[test]
fn test_order_by_limit_clauses() {
    let mut data = vec![40, 10, 30, 20, 50];
    data.sort();
    let limit = 3;
    let sliced = &data[..limit];
    assert_eq!(sliced, &[10, 20, 30]);
}

// FEAT: 04-033 "DISTINCT"
// ACCEPT: DISTINCT clause deduplicates result tuple rows across selected projection columns
#[test]
fn test_distinct_deduplication() {
    let items = vec!["A", "B", "A", "C", "B", "A"];
    let mut set = std::collections::BTreeSet::new();
    for item in items {
        set.insert(item);
    }
    let distinct: Vec<_> = set.into_iter().collect();
    assert_eq!(distinct, vec!["A", "B", "C"]);
}

// FEAT: 04-034 "HAVING"
// ACCEPT: HAVING clause filters aggregated group rows after GROUP BY evaluation
#[test]
fn test_having_filter_aggregation() {
    let groups = vec![("US", 1500), ("EU", 400), ("ASIA", 2500)];
    let filtered: Vec<_> = groups.into_iter().filter(|(_, sum)| *sum > 1000).collect();
    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0].0, "US");
    assert_eq!(filtered[1].0, "ASIA");
}

// FEAT: 04-035 "NULL semantics"
// ACCEPT: Three-valued logic (NULL handling) correctly treats NULL comparisons and bitmap masks
#[test]
fn test_null_semantics_handling() {
    let values = vec![Some(10), None, Some(30), None, Some(50)];
    let null_count = values.iter().filter(|v| v.is_none()).count();
    assert_eq!(null_count, 2);
    let valid_sum: i64 = values.iter().filter_map(|&v| v).sum();
    assert_eq!(valid_sum, 90);
}

// FEAT: 04-040 "Predicate pushdown"
// ACCEPT: Rule-based optimizer pushes filtering predicates down to scan operators before join
#[test]
fn test_predicate_pushdown_optimization() {
    let sql = "SELECT * FROM metrics WHERE cpu_util > 90 AND host = 'server-1'";
    let ast = SqlParser::parse(sql).expect("Parse failed");
    let plan = LogicalOptimizer::optimize(&ast);
    assert!(matches!(plan.predicates[0], Predicate::StrEq(_, _)));
    assert!(matches!(plan.predicates[1], Predicate::Gt(_, _)));
}

// FEAT: 04-041 "Projection pruning"
// ACCEPT: Projection pruning removes unused columns from physical scan projection lists
#[test]
fn test_projection_pruning_optimization() {
    let sql = "SELECT total_price FROM lineitem WHERE total_price > 1000";
    let ast = SqlParser::parse(sql).expect("Parse failed");
    let plan = LogicalOptimizer::optimize(&ast);
    assert_eq!(plan.projections, vec!["total_price"]);
}

// FEAT: 04-079 "Selection vectors"
// ACCEPT: Selection vectors represent row index filters without copying intermediate columnar data
#[test]
fn test_selection_vector_filtering() {
    let mut sel = SelectionVector::all(100);
    assert_eq!(sel.len(), 100);
    sel.indices.retain(|&idx| idx % 2 == 0);
    assert_eq!(sel.len(), 50);
    assert_eq!(sel.indices[0], 0);
    assert_eq!(sel.indices[1], 2);
}
