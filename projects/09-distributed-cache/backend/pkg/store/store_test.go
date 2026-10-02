package store_test

import (
	"bytes"
	"testing"
	"time"

	"github.com/AVISHJAIN9/distributed-cache/pkg/store"
)

// [FEAT: 09-001, 09-002, 09-003, 09-004, 09-005, 09-006, 09-010, 09-026]
func TestCacheStoreBasicOperationsAndCAS(t *testing.T) {
	cs := store.NewCacheStore(8, 1024*1024, 0)
	defer cs.Close()

	// 09-002: SET
	err := cs.Set("session:123", []byte("user_data"), 0)
	if err != nil {
		t.Fatalf("unexpected set error: %v", err)
	}

	// 09-001: GET
	val, ok := cs.Get("session:123")
	if !ok || string(val) != "user_data" {
		t.Fatalf("expected user_data, got %s (ok=%v)", string(val), ok)
	}

	// 09-004: EXISTS
	if !cs.Exists("session:123") {
		t.Fatalf("expected session:123 to exist")
	}
	if cs.Exists("nonexistent") {
		t.Fatalf("expected nonexistent key to not exist")
	}

	// 09-010: CAS Success
	casOk, err := cs.CompareAndSwap("session:123", []byte("user_data"), []byte("user_data_v2"))
	if err != nil || !casOk {
		t.Fatalf("CAS failed: ok=%v, err=%v", casOk, err)
	}
	val2, _ := cs.Get("session:123")
	if string(val2) != "user_data_v2" {
		t.Fatalf("expected user_data_v2, got %s", string(val2))
	}

	// Negative: CAS Mismatch
	casFail, err := cs.CompareAndSwap("session:123", []byte("stale_data"), []byte("new_data"))
	if err != nil || casFail {
		t.Fatalf("expected CAS mismatch failure, got ok=%v, err=%v", casFail, err)
	}

	// 09-005, 09-006: MSET & MGET
	kvs := map[string][]byte{
		"k1": []byte("v1"),
		"k2": []byte("v2"),
	}
	if err := cs.MSet(kvs, 0); err != nil {
		t.Fatalf("MSet failed: %v", err)
	}
	mres := cs.MGet("k1", "k2", "k3")
	if len(mres) != 3 || !bytes.Equal(mres[0], []byte("v1")) || !bytes.Equal(mres[1], []byte("v2")) || mres[2] != nil {
		t.Fatalf("MGet unexpected result: %v", mres)
	}

	// 09-026: Memory usage
	if cs.MemoryUsage() <= 0 {
		t.Fatalf("expected non-zero memory usage, got %d", cs.MemoryUsage())
	}

	// 09-003: DELETE
	if !cs.Delete("session:123") {
		t.Fatalf("expected delete to succeed")
	}
	if cs.Delete("session:123") {
		t.Fatalf("expected second delete to return false")
	}
}

// [FEAT: 09-007, 09-008, 09-009, 09-031, 09-034]
func TestCacheStoreTTLExpirationAndPersist(t *testing.T) {
	cs := store.NewCacheStore(8, 1024*1024, 0)
	defer cs.Close()

	// Set with short TTL
	err := cs.Set("temp_key", []byte("temp_val"), 50*time.Millisecond)
	if err != nil {
		t.Fatalf("set error: %v", err)
	}

	// 09-007: TTL read
	ttlRemaining, hasTTL := cs.TTL("temp_key")
	if !hasTTL || ttlRemaining <= 0 {
		t.Fatalf("expected active TTL, got %v (hasTTL=%v)", ttlRemaining, hasTTL)
	}

	// 09-009: PERSIST key
	if !cs.Persist("temp_key") {
		t.Fatalf("expected persist to succeed")
	}
	ttlPersist, _ := cs.TTL("temp_key")
	if ttlPersist != -1 {
		t.Fatalf("expected -1 for persisted key, got %v", ttlPersist)
	}

	// 09-008: EXPIRE update
	if !cs.Expire("temp_key", 40*time.Millisecond) {
		t.Fatalf("expected expire update to succeed")
	}

	// Wait for expiration
	time.Sleep(70 * time.Millisecond)

	// 09-034: Lazy expiration on GET
	if _, ok := cs.Get("temp_key"); ok {
		t.Fatalf("expected temp_key to be expired and not found")
	}
}

// [FEAT: 09-021, 09-022, 09-028]
func TestCacheStoreMaxSizeEnforcementAndEviction(t *testing.T) {
	// Small cache max size: 300 bytes
	cs := store.NewCacheStore(1, 300, 0)
	defer cs.Close()

	// Fill entries
	for i := 0; i < 5; i++ {
		key := string(rune('a' + i))
		err := cs.Set(key, []byte("large_payload_bytes_for_eviction_testing"), 0)
		if err != nil {
			t.Fatalf("set %s failed: %v", key, err)
		}
	}

	// Cache must stay within 300 bytes by evicting older items
	if cs.MemoryUsage() > 300 {
		t.Fatalf("memory usage %d exceeded max size 300", cs.MemoryUsage())
	}
}
