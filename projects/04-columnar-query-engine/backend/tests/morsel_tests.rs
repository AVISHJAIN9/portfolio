use columnar_query_engine::execution::morsel::{
    Morsel, MorselScheduler, MorselThreadPool, ParallelAggregation, ParallelHashJoin, ParallelScan,
    DEFAULT_MORSEL_SIZE,
};
use std::sync::Arc;

const NUM_ROWS: usize = 10_000;

// FEAT: 04-079 "Selection vectors"
// ACCEPT: Selection vectors represent row index filters without copying intermediate columnar data
#[test]
fn test_morsel_selection_vectors() {
    let scheduler = Arc::new(MorselScheduler::new(NUM_ROWS, DEFAULT_MORSEL_SIZE));
    let pool = MorselThreadPool::new(4);

    let sel = ParallelScan::scan_filtered(&scheduler, &pool, |row| row % 2 == 0);
    assert_eq!(sel.len(), NUM_ROWS / 2);
    assert!(!sel.is_empty());
    assert_eq!(sel.indices[0], 0);
    assert_eq!(sel.indices[1], 2);

    let empty_morsel = Morsel { start: 5, end: 5 };
    assert!(empty_morsel.is_empty());
    assert_eq!(empty_morsel.len(), 0);

    let non_empty_morsel = Morsel { start: 0, end: 10 };
    assert!(!non_empty_morsel.is_empty());
    assert_eq!(non_empty_morsel.len(), 10);
}

// FEAT: 04-080 "Morsel-driven scheduling"
// ACCEPT: Morsel-driven scheduling dynamically assigns fixed-size chunk morsels to workers
#[test]
fn test_morsel_driven_scheduling() {
    let scheduler = MorselScheduler::new(25_000, 10_000);
    let m1 = scheduler.next_morsel().unwrap();
    assert_eq!(
        m1,
        Morsel {
            start: 0,
            end: 10_000
        }
    );

    let m2 = scheduler.next_morsel().unwrap();
    assert_eq!(
        m2,
        Morsel {
            start: 10_000,
            end: 20_000
        }
    );

    let m3 = scheduler.next_morsel().unwrap();
    assert_eq!(
        m3,
        Morsel {
            start: 20_000,
            end: 25_000
        }
    );

    assert_eq!(scheduler.next_morsel(), None);

    // Test reset
    scheduler.reset();
    let m1_again = scheduler.next_morsel().unwrap();
    assert_eq!(
        m1_again,
        Morsel {
            start: 0,
            end: 10_000
        }
    );
}

// FEAT: 04-081 "Parallel scans"
// ACCEPT: Parallel scans execute across 1, 2, and 8 threads yielding identical filtered row sets
#[test]
fn test_parallel_scans_across_threads() {
    let pool1 = MorselThreadPool::new(1);
    let pool2 = MorselThreadPool::new(2);
    let pool8 = MorselThreadPool::new(8);

    let sched1 = Arc::new(MorselScheduler::new(NUM_ROWS, DEFAULT_MORSEL_SIZE));
    let sched2 = Arc::new(MorselScheduler::new(NUM_ROWS, DEFAULT_MORSEL_SIZE));
    let sched8 = Arc::new(MorselScheduler::new(NUM_ROWS, DEFAULT_MORSEL_SIZE));

    let res1 = ParallelScan::scan_filtered(&sched1, &pool1, |row| row % 5 == 0);
    let res2 = ParallelScan::scan_filtered(&sched2, &pool2, |row| row % 5 == 0);
    let res8 = ParallelScan::scan_filtered(&sched8, &pool8, |row| row % 5 == 0);

    assert_eq!(res1.indices.len(), NUM_ROWS / 5);
    assert_eq!(res1.indices, res2.indices);
    assert_eq!(res1.indices, res8.indices);
}

// FEAT: 04-082 "Parallel hash join"
// ACCEPT: Parallel hash join builds shared hash table and probes in parallel yielding identical row sets
#[test]
fn test_parallel_hash_join_execution() {
    let pool1 = MorselThreadPool::new(1);
    let pool4 = MorselThreadPool::new(4);
    let pool8 = MorselThreadPool::new(8);

    let build_rows = 1_000;
    let probe_rows = NUM_ROWS;

    let res1 = ParallelHashJoin::inner_join_int(
        build_rows,
        probe_rows,
        &pool1,
        |r| (r % 100) as i64,
        |r| r as i64,
        |r| (r % 100) as i64,
        |r| (r * 10) as i64,
    );

    let res4 = ParallelHashJoin::inner_join_int(
        build_rows,
        probe_rows,
        &pool4,
        |r| (r % 100) as i64,
        |r| r as i64,
        |r| (r % 100) as i64,
        |r| (r * 10) as i64,
    );

    let res8 = ParallelHashJoin::inner_join_int(
        build_rows,
        probe_rows,
        &pool8,
        |r| (r % 100) as i64,
        |r| r as i64,
        |r| (r % 100) as i64,
        |r| (r * 10) as i64,
    );

    assert_eq!(res1.len(), res4.len());
    assert_eq!(res1.len(), res8.len());
    assert_eq!(res1.len(), 100_000); // 1000 build matches 10000 probe (each build matches 100 probe rows)
    assert!(res1.contains(&(0, 0)));
    assert!(res1.contains(&(1, 10)));
}

// FEAT: 04-083 "Parallel aggregation"
// ACCEPT: Parallel grouped aggregation combines thread-local accumulators yielding identical totals
#[test]
fn test_parallel_aggregation_across_threads() {
    let pool1 = MorselThreadPool::new(1);
    let pool2 = MorselThreadPool::new(2);
    let pool8 = MorselThreadPool::new(8);

    let sched1 = Arc::new(MorselScheduler::new(NUM_ROWS, DEFAULT_MORSEL_SIZE));
    let sched2 = Arc::new(MorselScheduler::new(NUM_ROWS, DEFAULT_MORSEL_SIZE));
    let sched8 = Arc::new(MorselScheduler::new(NUM_ROWS, DEFAULT_MORSEL_SIZE));

    let agg1 =
        ParallelAggregation::sum_grouped_i64(&sched1, &pool1, |r| r % 4, |r| (r % 100) as i64);
    let agg2 =
        ParallelAggregation::sum_grouped_i64(&sched2, &pool2, |r| r % 4, |r| (r % 100) as i64);
    let agg8 =
        ParallelAggregation::sum_grouped_i64(&sched8, &pool8, |r| r % 4, |r| (r % 100) as i64);

    assert_eq!(agg1.len(), 4);
    assert_eq!(agg1, agg2);
    assert_eq!(agg1, agg8);
    assert_eq!(agg1.get(&0), Some(&120_000));
    assert_eq!(agg1.get(&1), Some(&122_500));
    assert_eq!(agg1.get(&2), Some(&125_000));
    assert_eq!(agg1.get(&3), Some(&127_500));
}

// FEAT: 04-084 "Task scheduler"
// ACCEPT: Dynamic task scheduler assigns morsels and tracks work exhaustion across concurrent threads
#[test]
fn test_task_scheduler_morsel_exhaustion() {
    let scheduler = Arc::new(MorselScheduler::new(NUM_ROWS, 2_000));
    let pool = MorselThreadPool::new(4);

    let count: usize = pool.execute(|| {
        let mut local_count = 0;
        while let Some(morsel) = scheduler.next_morsel() {
            local_count += morsel.len();
        }
        local_count
    });

    assert_eq!(count, NUM_ROWS);
    assert_eq!(scheduler.next_morsel(), None);
}

// FEAT: 04-085 "Thread pool"
// ACCEPT: Thread pool configures worker thread count and executes closures within pool context
#[test]
fn test_thread_pool_sizing() {
    for threads in [1, 2, 4, 8] {
        let pool = MorselThreadPool::new(threads);
        assert_eq!(pool.thread_count, threads);
        let val = pool.execute(rayon::current_num_threads);
        assert_eq!(val, threads);
    }
}
