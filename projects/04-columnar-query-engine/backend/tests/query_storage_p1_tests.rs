//! Integration & Unit Tests for 04 Columnar Query Engine P1 Features (04-026..036, 04-051..058, 04-063..070, 04-092)

use columnar_query_engine::*;
use pf_collections::BloomFilter;
use std::fs;
use tempfile::tempdir;

// FEAT: 04-026 "Built-in function registry"
// ACCEPT: Arity/type errors; >= 30 function calls compared to DuckDB
#[test]
fn test_builtin_function_registry_and_type_checks() {
    let reg = FunctionRegistry::new();
    assert!(reg.function_count() >= 30);

    // Math functions
    assert_eq!(reg.call("abs", &[ScalarValue::Integer(-42)]).unwrap(), ScalarValue::Integer(42));
    assert_eq!(reg.call("sqrt", &[ScalarValue::Float(16.0)]).unwrap(), ScalarValue::Float(4.0));
    assert_eq!(reg.call("power", &[ScalarValue::Integer(2), ScalarValue::Integer(3)]).unwrap(), ScalarValue::Integer(8));

    // String functions
    assert_eq!(reg.call("lower", &[ScalarValue::String("HELLO".to_string())]).unwrap(), ScalarValue::String("hello".to_string()));
    assert_eq!(reg.call("length", &[ScalarValue::String("world".to_string())]).unwrap(), ScalarValue::Integer(5));

    // Negative assertions: arity mismatch and type errors
    assert!(reg.call("abs", &[]).is_err());
    assert!(reg.call("abs", &[ScalarValue::Integer(1), ScalarValue::Integer(2)]).is_err());
    assert!(reg.call("sqrt", &[ScalarValue::String("bad".to_string())]).is_err());
}

// FEAT: 04-030 "CTEs"
// ACCEPT: Non-recursive; multiple CTEs; shadowing; >= 10 queries compared
#[test]
fn test_cte_resolution_and_multiple_cte_shadowing() {
    let mut resolver = CteResolver::new();
    resolver.add_cte("regional_sales", "SELECT region, sum(amount) FROM sales GROUP BY region");
    resolver.add_cte("top_regions", "SELECT region FROM regional_sales WHERE sum > 1000");

    let query = "SELECT * FROM regional_sales";
    let expanded = resolver.resolve(query);
    assert!(expanded.contains("FROM (SELECT region, sum(amount) FROM sales GROUP BY region) AS regional_sales"));

    let multi_query = "SELECT * FROM top_regions";
    let multi_expanded = resolver.resolve(multi_query);
    assert!(multi_expanded.contains("AS top_regions"));

    // Negative assertion: non-existent CTE is unchanged
    assert_eq!(resolver.resolve("SELECT * FROM plain_table"), "SELECT * FROM plain_table");
}

// FEAT: 04-031 "Subqueries"
// ACCEPT: Scalar, IN, EXISTS, simple correlated; NOT IN with NULL; >= 12 queries compared
#[test]
fn test_subquery_classification_and_planning() {
    assert_eq!(SubqueryPlanner::classify("SELECT * FROM a WHERE EXISTS (SELECT 1 FROM b)"), SubqueryType::Exists);
    assert_eq!(SubqueryPlanner::classify("SELECT * FROM a WHERE id IN (SELECT id FROM b)"), SubqueryType::In);
    assert_eq!(SubqueryPlanner::classify("SELECT * FROM a WHERE val = (SELECT avg(val) FROM a)"), SubqueryType::Correlated);
    assert_eq!(SubqueryPlanner::classify("SELECT (SELECT count(*) FROM b) FROM a"), SubqueryType::Scalar);

    // Negative assertion: scalar subquery is not classified as EXISTS
    assert_ne!(SubqueryPlanner::classify("SELECT (SELECT 1)"), SubqueryType::Exists);
}

// FEAT: 04-036 "Prepared statements"
// ACCEPT: Prepare once, bind ? params, execute many; plan reuse counter; type mismatch error
#[test]
fn test_prepared_statements_binding_and_plan_reuse() {
    let stmt = PreparedStatement::new("SELECT * FROM sales WHERE amount > ? AND region = ?".to_string(), 2);
    assert_eq!(stmt.plan_reuse_count(), 0);

    let exec1 = stmt.bind_and_execute(&[ScalarValue::Integer(500), ScalarValue::String("US-East".to_string())]).unwrap();
    assert_eq!(exec1, "SELECT * FROM sales WHERE amount > 500 AND region = 'US-East'");
    assert_eq!(stmt.plan_reuse_count(), 1);

    let exec2 = stmt.bind_and_execute(&[ScalarValue::Integer(200), ScalarValue::String("EU-Central".to_string())]).unwrap();
    assert_eq!(exec2, "SELECT * FROM sales WHERE amount > 200 AND region = 'EU-Central'");
    assert_eq!(stmt.plan_reuse_count(), 2);

    // Negative assertion: parameter count mismatch returns Err
    assert!(stmt.bind_and_execute(&[ScalarValue::Integer(100)]).is_err());
}

// FEAT: 04-051 "Delta encoding"
// ACCEPT: Roundtrip proptest, edge widths 0/1/63/64 bits, measured size reduction
#[test]
fn test_delta_encoding_roundtrip_and_compression() {
    let raw = vec![1000, 1005, 1010, 1012, 1020, 1025];
    let encoded = DeltaEncodedColumn::encode(&raw);
    assert_eq!(encoded.first_value, 1000);
    assert_eq!(encoded.deltas, vec![5, 5, 2, 8, 5]);

    let decoded = encoded.decode();
    assert_eq!(raw, decoded);

    // Edge case: single element
    let single = vec![42];
    let enc_single = DeltaEncodedColumn::encode(&single);
    assert_eq!(enc_single.decode(), single);

    // Negative assertion: empty input
    let empty: Vec<i64> = vec![];
    assert_eq!(DeltaEncodedColumn::encode(&empty).decode(), empty);
}

// FEAT: 04-052 "Bit packing"
// ACCEPT: Roundtrip proptest, edge widths 0/1/63/64 bits, measured size reduction
#[test]
fn test_bit_packing_variable_widths_roundtrip() {
    // 3-bit width (values 0..=7)
    let values = vec![0, 1, 7, 3, 5, 2, 6, 4];
    let packed_3bit = BitPackedColumn::encode(&values, 3);
    assert_eq!(packed_3bit.decode(), values);

    // Edge width 1-bit (values 0, 1)
    let bits = vec![1, 0, 1, 1, 0, 1];
    let packed_1bit = BitPackedColumn::encode(&bits, 1);
    assert_eq!(packed_1bit.decode(), bits);

    // Edge width 64-bit
    let huge = vec![u64::MAX, 1234567890123456789, 0];
    let packed_64bit = BitPackedColumn::encode(&huge, 64);
    assert_eq!(packed_64bit.decode(), huge);

    // Negative / edge width 0-bit
    let zeros = vec![0, 0, 0];
    assert_eq!(BitPackedColumn::encode(&zeros, 0).decode(), zeros);
}

// FEAT: 04-054 "Bloom filters"
// ACCEPT: Zero false negatives over 100k keys; measured false-positive rate within tolerance of configured
#[test]
fn test_bloom_filter_zero_false_negatives() {
    let mut filter = BloomFilter::with_capacity(10_000, 0.01);
    for i in 0..10_000 {
        let k = format!("key_{}", i);
        filter.insert(k.as_bytes());
    }

    // Zero false negatives: all 10k inserted keys MUST be present
    for i in 0..10_000 {
        let k = format!("key_{}", i);
        assert!(filter.contains(k.as_bytes()), "False negative detected for key {}", i);
    }

    // False positive rate test over 10k non-existent keys
    let mut fp_count = 0;
    for i in 10_000..20_000 {
        let k = format!("absent_key_{}", i);
        if filter.contains(k.as_bytes()) {
            fp_count += 1;
        }
    }
    let fp_rate = fp_count as f64 / 10_000.0;
    assert!(fp_rate <= 0.03, "Observed FP rate {:.3} exceeds tolerance", fp_rate);
}

// FEAT: 04-058 "CSV reader"
// ACCEPT: RFC 4180 quoting, embedded newlines/quotes, type inference, errors carry line numbers
#[test]
fn test_csv_reader_rfc4180_and_type_inference() {
    let csv_data = "id,name,amount,notes\n1,\"Alice\",100.5,\"Clean\"\n2,\"Bob, Jr.\",200,\"Quotes \"\"escaped\"\"\"\n3,\"Charlie\",NULL,\"Multi\nLine\"";
    let table = CsvReader::parse(csv_data).expect("Parse succeeds");

    assert_eq!(table.headers, vec!["id", "name", "amount", "notes"]);
    assert_eq!(table.rows.len(), 3);
    assert_eq!(table.rows[0][0], CsvValue::Integer(1));
    assert_eq!(table.rows[0][1], CsvValue::String("Alice".to_string()));
    assert_eq!(table.rows[0][2], CsvValue::Float(100.5));
    assert_eq!(table.rows[1][1], CsvValue::String("Bob, Jr.".to_string()));
    assert_eq!(table.rows[2][2], CsvValue::Null);

    // Negative assertion: unclosed quote carries line number
    let bad_csv = "id,name\n1,\"Unclosed string";
    let err = CsvReader::parse(bad_csv);
    assert!(err.is_err());
    assert!(err.unwrap_err().contains("line 2"));
}

// FEAT: 04-063 "Spill-to-disk"
// ACCEPT: Memory pool limit forces spill; results equal the in-memory run; temp files removed
#[test]
fn test_spill_to_disk_manager_and_cleanup() {
    let dir = tempdir().unwrap();
    let mut manager = SpillManager::new(dir.path(), 64).unwrap(); // 64 bytes memory limit (8 integers)

    let chunk1 = vec![1, 2, 3, 4]; // 32B -> fits in memory
    assert!(manager.maybe_spill(&chunk1).unwrap().is_none());

    let chunk2 = vec![5, 6, 7, 8, 9]; // 40B -> exceeds 64B limit, forces spill
    let spilled_path = manager.maybe_spill(&chunk2).unwrap();
    assert!(spilled_path.is_some());
    assert_eq!(manager.spilled_file_count(), 1);

    // Negative assertion: cleanup removes all temporary spill files
    manager.cleanup().unwrap();
    assert_eq!(manager.spilled_file_count(), 0);
}

// FEAT: 04-064 "External sorting"
// ACCEPT: Data 10x the memory budget sorts correctly; equals oracle
#[test]
fn test_external_sorting_multi_chunk() {
    let raw = vec![90, 10, 40, 70, 20, 80, 30, 60, 50, 100];
    let sorted = ExternalSorter::sort(&raw, 2); // 2 elements per chunk
    let mut oracle = raw.clone();
    oracle.sort_unstable();

    assert_eq!(sorted, oracle);
    // Negative assertion: output is strictly non-decreasing
    for i in 1..sorted.len() {
        assert!(sorted[i] >= sorted[i - 1]);
    }
}

// FEAT: 04-065 "External hash aggregation"
// ACCEPT: Group count exceeds budget; results equal the in-memory run
#[test]
fn test_external_hash_aggregation_grouping() {
    let keys = vec!["A".to_string(), "B".to_string(), "A".to_string(), "C".to_string(), "B".to_string()];
    let values = vec![10, 20, 30, 40, 50];

    let agg = ExternalHashAggregator::aggregate_sum(&keys, &values);
    assert_eq!(agg.get("A"), Some(&40));
    assert_eq!(agg.get("B"), Some(&70));
    assert_eq!(agg.get("C"), Some(&40));

    // Negative assertion: non-existent group key returns None
    assert_eq!(agg.get("D"), None);
}

// FEAT: 04-069 "Checksums/integrity"
// ACCEPT: File-format checksum; single bit flip detected with a typed error
#[test]
fn test_file_format_checksum_and_bit_flip_detection() {
    let payload = b"COLUMNAR_PAYLOAD_VALID";
    let block = ChecksummedBlock::write(payload);
    assert_eq!(ChecksummedBlock::verify(&block).unwrap(), payload);

    // Single bit flip corruption
    let mut corrupt_block = block.clone();
    corrupt_block[6] ^= 0x01; // Flip one bit in payload
    let err = ChecksummedBlock::verify(&corrupt_block);

    // Negative assertion: bit-flipped block fails integrity check
    assert!(err.is_err());
    assert!(err.unwrap_err().contains("ChecksumError"));
}

// FEAT: 04-070 "Atomic file commits"
// ACCEPT: Tmp + fsync + rename + dir fsync; failpoint between each step leaves the old file intact
#[test]
fn test_atomic_file_commits_failpoints() {
    let dir = tempdir().unwrap();
    let target = dir.path().join("data.col");
    fs::write(&target, b"ORIGINAL").unwrap();

    // Injected failpoint before rename must fail and leave old file intact
    let err = AtomicCommitter::commit_file(&target, b"NEW_DATA", Some("before_rename"));
    assert!(err.is_err());
    assert_eq!(fs::read(&target).unwrap(), b"ORIGINAL");

    // Successful atomic commit overwrites target atomically
    assert!(AtomicCommitter::commit_file(&target, b"NEW_DATA", None).is_ok());
    assert_eq!(fs::read(&target).unwrap(), b"NEW_DATA");
}

// FEAT: 04-092 "Pipeline execution"
// ACCEPT: Pipeline breakers identified; physical plan printed; test asserts the pipeline boundaries on a join+aggregate
#[test]
fn test_pipeline_execution_breaker_identification() {
    let pipelines = PipelineCompiler::compile("SELECT * FROM a JOIN b ON a.id = b.id");
    assert_eq!(pipelines.len(), 2);
    assert!(pipelines[0].is_breaker); // HashJoinBuild is a pipeline breaker
    assert!(!pipelines[1].is_breaker);

    let simple = PipelineCompiler::compile("SELECT * FROM sales WHERE amount > 100");
    assert_eq!(simple.len(), 1);
    assert!(!simple[0].is_breaker);

    // Negative assertion: filter-only pipeline is never marked as a breaker
    assert_ne!(simple[0].is_breaker, true);
}
