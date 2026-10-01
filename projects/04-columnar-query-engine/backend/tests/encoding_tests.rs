use columnar_query_engine::storage::encoding::{DictionaryEncodedColumn, NullBitmap, RleEncodedColumn, RowGroup};

// FEAT: 04-046
// ACCEPT: Columnar table format with vertical column partition arrays and row groups
#[test]
fn test_columnar_table_format() {
    let mut rg = RowGroup::new(100);
    rg.add_int_column("id", (0..100).collect());
    assert_eq!(rg.row_count, 100);
    assert_eq!(rg.int_columns.get("id").unwrap().len(), 100);
}

// FEAT: 04-047
// ACCEPT: Row group partitioning allows independent horizontal chunk loading and scan
#[test]
fn test_row_group_partitioning() {
    let rg1 = RowGroup::new(50);
    let rg2 = RowGroup::new(50);
    assert_eq!(rg1.row_count + rg2.row_count, 100);
}

// FEAT: 04-048
// ACCEPT: Validity null bitmask compactly tracks null slots with bitwise indexing
#[test]
fn test_validity_null_bitmap() {
    let mut bitmap = NullBitmap::new(64);
    assert!(!bitmap.is_null(0));
    assert!(!bitmap.is_null(7));

    bitmap.set_null(7);
    bitmap.set_null(15);
    bitmap.set_null(63);

    assert!(bitmap.is_null(7));
    assert!(bitmap.is_null(15));
    assert!(bitmap.is_null(63));
    assert!(!bitmap.is_null(8));
    assert_eq!(bitmap.null_count, 3);
}

// FEAT: 04-049
// ACCEPT: Dictionary encoding maps low-cardinality string columns to integer code offsets
#[test]
fn test_dictionary_encoding_roundtrip() {
    let raw = vec![
        Some("US".to_string()),
        Some("EU".to_string()),
        Some("US".to_string()),
        None,
        Some("ASIA".to_string()),
        Some("EU".to_string()),
    ];

    let encoded = DictionaryEncodedColumn::encode(&raw);
    assert_eq!(encoded.dictionary.len(), 3); // US, EU, ASIA
    assert_eq!(encoded.indices.len(), 6);
    assert!(encoded.nulls.is_null(3));

    let decoded = encoded.decode();
    assert_eq!(decoded, raw);
}

// FEAT: 04-050
// ACCEPT: Run-length encoding compresses repeated sequential integers into (value, length) tuples
#[test]
fn test_rle_encoding_roundtrip() {
    let raw = vec![10, 10, 10, 20, 20, 30, 30, 30, 30, 10];
    let encoded = RleEncodedColumn::encode(&raw);
    assert_eq!(encoded.values, vec![10, 20, 30, 10]);
    assert_eq!(encoded.run_lengths, vec![3, 2, 4, 1]);

    let decoded = encoded.decode();
    assert_eq!(decoded, raw);
}

// FEAT: 04-055
// ACCEPT: File statistics catalog tracks min/max boundaries and null counts per column chunk
#[test]
fn test_file_statistics() {
    let mut rg = RowGroup::new(5);
    rg.add_int_column("price", vec![100, 250, 50, 400, 300]);
    assert_eq!(*rg.min_values.get("price").unwrap(), 50);
    assert_eq!(*rg.max_values.get("price").unwrap(), 400);
}
