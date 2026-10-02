package store

import (
	"bytes"
	"fmt"
	"hash/fnv"
	"sync"
	"sync/atomic"
	"time"

	"github.com/AVISHJAIN9/distributed-cache/pkg/evict"
	"github.com/AVISHJAIN9/distributed-cache/pkg/ttl"
)

type Entry struct {
	Key       string
	Value     []byte
	Version   uint64
	ExpiresAt time.Time
	HasTTL    bool
	SizeBytes int64
}

type Shard struct {
	mu     sync.RWMutex
	data   map[string]*Entry
	policy *evict.CachePolicy
}

// 09-001..09-030: Sharded In-Memory Cache Store with TinyLFU Eviction & Exact Memory Accounting
type CacheStore struct {
	shards       []*Shard
	numShards    int
	maxSizeBytes int64
	currentBytes int64
	timingWheel  *ttl.TimingWheel
	defaultTTL   time.Duration
}

func NewCacheStore(numShards int, maxSizeBytes int64, defaultTTL time.Duration) *CacheStore {
	if numShards <= 0 {
		numShards = 64
	}
	cs := &CacheStore{
		shards:       make([]*Shard, numShards),
		numShards:    numShards,
		maxSizeBytes: maxSizeBytes,
		currentBytes: 0,
		defaultTTL:   defaultTTL,
	}

	for i := 0; i < numShards; i++ {
		cs.shards[i] = &Shard{
			data:   make(map[string]*Entry),
			policy: evict.NewCachePolicy(1000),
		}
	}

	cs.timingWheel = ttl.NewTimingWheel(100*time.Millisecond, 3600, func(expiredKey string) {
		cs.Delete(expiredKey)
	})
	cs.timingWheel.Start()

	return cs
}

func (cs *CacheStore) shardIndex(key string) int {
	h := fnv.New32a()
	h.Write([]byte(key))
	return int(h.Sum32()) % cs.numShards
}

func (cs *CacheStore) entrySize(key string, val []byte) int64 {
	// Key len + Value len + 48 bytes metadata struct overhead (09-027)
	return int64(len(key) + len(val) + 48)
}

// 09-002: SET cache entry with optional TTL
func (cs *CacheStore) Set(key string, val []byte, customTTL time.Duration) error {
	sz := cs.entrySize(key, val)
	if cs.maxSizeBytes > 0 && sz > cs.maxSizeBytes {
		return fmt.Errorf("value exceeds maximum cache size")
	}

	shard := cs.shards[cs.shardIndex(key)]
	shard.mu.Lock()
	defer shard.mu.Unlock()

	var expiresAt time.Time
	hasTTL := false
	effectiveTTL := customTTL
	if effectiveTTL == 0 && cs.defaultTTL > 0 {
		effectiveTTL = cs.defaultTTL
	}
	if effectiveTTL > 0 {
		expiresAt = time.Now().Add(effectiveTTL)
		hasTTL = true
	}

	// Evict if over capacity (09-028)
	if cs.maxSizeBytes > 0 {
		for atomic.LoadInt64(&cs.currentBytes)+sz > cs.maxSizeBytes {
			victimKey, victimSize, ok := shard.policy.EvictCandidate()
			if !ok {
				break
			}
			if _, exists := shard.data[victimKey]; exists {
				delete(shard.data, victimKey)
				atomic.AddInt64(&cs.currentBytes, -victimSize)
			}
		}
	}

	oldEntry, exists := shard.data[key]
	var newVersion uint64 = 1
	if exists {
		atomic.AddInt64(&cs.currentBytes, -oldEntry.SizeBytes)
		newVersion = oldEntry.Version + 1
	}

	newEntry := &Entry{
		Key:       key,
		Value:     val,
		Version:   newVersion,
		ExpiresAt: expiresAt,
		HasTTL:    hasTTL,
		SizeBytes: sz,
	}

	shard.data[key] = newEntry
	shard.policy.Add(key, sz)
	atomic.AddInt64(&cs.currentBytes, sz)

	if hasTTL {
		cs.timingWheel.Schedule(key, effectiveTTL)
	}

	return nil
}

// 09-001: GET cache entry (with lazy expiration 09-034)
func (cs *CacheStore) Get(key string) ([]byte, bool) {
	shard := cs.shards[cs.shardIndex(key)]
	shard.mu.RLock()
	entry, exists := shard.data[key]
	if !exists {
		shard.mu.RUnlock()
		return nil, false
	}

	if entry.HasTTL && time.Now().After(entry.ExpiresAt) {
		shard.mu.RUnlock()
		// Lazy expire
		cs.Delete(key)
		return nil, false
	}

	val := make([]byte, len(entry.Value))
	copy(val, entry.Value)
	shard.mu.RUnlock()

	shard.policy.RecordAccess(key)
	return val, true
}

// 09-003: DELETE cache entry
func (cs *CacheStore) Delete(key string) bool {
	shard := cs.shards[cs.shardIndex(key)]
	shard.mu.Lock()
	defer shard.mu.Unlock()

	entry, exists := shard.data[key]
	if !exists {
		return false
	}

	delete(shard.data, key)
	shard.policy.Remove(key)
	atomic.AddInt64(&cs.currentBytes, -entry.SizeBytes)
	return true
}

// 09-004: EXISTS lookup
func (cs *CacheStore) Exists(key string) bool {
	_, ok := cs.Get(key)
	return ok
}

// 09-005: MGET batch read
func (cs *CacheStore) MGet(keys ...string) [][]byte {
	results := make([][]byte, len(keys))
	for i, k := range keys {
		if val, ok := cs.Get(k); ok {
			results[i] = val
		} else {
			results[i] = nil
		}
	}
	return results
}

// 09-006: MSET batch write
func (cs *CacheStore) MSet(kvs map[string][]byte, ttl time.Duration) error {
	for k, v := range kvs {
		if err := cs.Set(k, v, ttl); err != nil {
			return err
		}
	}
	return nil
}

// 09-007: TTL read
func (cs *CacheStore) TTL(key string) (time.Duration, bool) {
	shard := cs.shards[cs.shardIndex(key)]
	shard.mu.RLock()
	defer shard.mu.RUnlock()

	entry, exists := shard.data[key]
	if !exists {
		return 0, false
	}
	if !entry.HasTTL {
		return -1, true // persistent key
	}
	remaining := time.Until(entry.ExpiresAt)
	if remaining < 0 {
		return 0, false
	}
	return remaining, true
}

// 09-008: EXPIRE update
func (cs *CacheStore) Expire(key string, ttlDur time.Duration) bool {
	shard := cs.shards[cs.shardIndex(key)]
	shard.mu.Lock()
	defer shard.mu.Unlock()

	entry, exists := shard.data[key]
	if !exists {
		return false
	}
	entry.HasTTL = true
	entry.ExpiresAt = time.Now().Add(ttlDur)
	cs.timingWheel.Schedule(key, ttlDur)
	return true
}

// 09-009: PERSIST key
func (cs *CacheStore) Persist(key string) bool {
	shard := cs.shards[cs.shardIndex(key)]
	shard.mu.Lock()
	defer shard.mu.Unlock()

	entry, exists := shard.data[key]
	if !exists || !entry.HasTTL {
		return false
	}
	entry.HasTTL = false
	return true
}

// 09-010: Atomic Compare-And-Swap
func (cs *CacheStore) CompareAndSwap(key string, expectedVal, newVal []byte) (bool, error) {
	shard := cs.shards[cs.shardIndex(key)]
	shard.mu.Lock()
	defer shard.mu.Unlock()

	entry, exists := shard.data[key]
	if !exists {
		return false, fmt.Errorf("key not found")
	}

	if !bytes.Equal(entry.Value, expectedVal) {
		return false, nil // Value mismatch, CAS failed
	}

	oldSize := entry.SizeBytes
	newSize := cs.entrySize(key, newVal)
	entry.Value = newVal
	entry.Version++
	entry.SizeBytes = newSize

	shard.policy.Add(key, newSize)
	atomic.AddInt64(&cs.currentBytes, newSize-oldSize)
	return true, nil
}

func (cs *CacheStore) MemoryUsage() int64 {
	return atomic.LoadInt64(&cs.currentBytes)
}

func (cs *CacheStore) Close() {
	cs.timingWheel.Stop()
}
