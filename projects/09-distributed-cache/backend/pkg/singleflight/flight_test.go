package singleflight_test

import (
	"sync"
	"sync/atomic"
	"testing"
	"time"

	"github.com/AVISHJAIN9/distributed-cache/pkg/singleflight"
)

// [FEAT: 09-056, 09-057]
func TestSingleFlightCoalescing(t *testing.T) {
	g := singleflight.NewGroup()

	var execCount int32
	var wg sync.WaitGroup

	for i := 0; i < 10; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			val, err, _ := g.Do("heavy_query_key", func() ([]byte, error) {
				atomic.AddInt32(&execCount, 1)
				time.Sleep(20 * time.Millisecond)
				return []byte("heavy_result"), nil
			})
			if err != nil || string(val) != "heavy_result" {
				t.Errorf("unexpected singleflight result: %s, %v", string(val), err)
			}
		}()
	}

	wg.Wait()

	if atomic.LoadInt32(&execCount) != 1 {
		t.Fatalf("expected exactly 1 execution across 10 concurrent requests, got %d", execCount)
	}
}
