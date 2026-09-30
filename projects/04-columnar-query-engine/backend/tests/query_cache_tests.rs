// FEAT: FEAT-04-044
use columnar_query_engine::cache::query_cache::QueryResultCache;

#[test]
fn test_query_result_cache_hit_and_eviction() {
    let cache = QueryResultCache::new(2);

    cache.put("SELECT SUM(amount) FROM sales", "150000".into());
    cache.put("SELECT COUNT(*) FROM users", "420".into());

    assert_eq!(cache.get("SELECT SUM(amount) FROM sales"), Some("150000".into()));
    assert_eq!(cache.get("SELECT COUNT(*) FROM users"), Some("420".into()));

    // Insert 3rd item to trigger S3-FIFO eviction
    cache.put("SELECT MAX(latency) FROM telemetry", "12.4".into());
    assert_eq!(cache.get("SELECT MAX(latency) FROM telemetry"), Some("12.4".into()));
}
