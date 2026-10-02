package evict

import (
	"container/list"
	"hash/fnv"
	"sync"
)

// 09-022, 09-025: Count-Min 4-bit sketch with periodic age decay
type CountMinSketch struct {
	depth   int
	width   int
	table   [][]uint8
	counter int
	decayAt int
}

func NewCountMinSketch(width, depth int) *CountMinSketch {
	table := make([][]uint8, depth)
	for i := range table {
		table[i] = make([]uint8, width)
	}
	return &CountMinSketch{
		depth:   depth,
		width:   width,
		table:   table,
		counter: 0,
		decayAt: width * 10,
	}
}

func (c *CountMinSketch) hash(key []byte, seed int) int {
	h := fnv.New32a()
	h.Write(key)
	h.Write([]byte{byte(seed), byte(seed >> 8)})
	return int(h.Sum32()) % c.width
}

func (c *CountMinSketch) Increment(key []byte) {
	for i := 0; i < c.depth; i++ {
		idx := c.hash(key, i)
		if c.table[i][idx] < 15 {
			c.table[i][idx]++
		}
	}
	c.counter++
	if c.counter >= c.decayAt {
		c.decay()
		c.counter = 0
	}
}

func (c *CountMinSketch) Estimate(key []byte) uint8 {
	var min uint8 = 15
	for i := 0; i < c.depth; i++ {
		idx := c.hash(key, i)
		v := c.table[i][idx]
		if v < min {
			min = v
		}
	}
	return min
}

func (c *CountMinSketch) decay() {
	for i := 0; i < c.depth; i++ {
		for j := 0; j < c.width; j++ {
			c.table[i][j] /= 2
		}
	}
}

// 09-021..09-024: Window TinyLFU Cache Policy
type CachePolicy struct {
	mu     sync.Mutex
	sketch *CountMinSketch
	lru    *list.List
	nodes  map[string]*list.Element
}

type lruItem struct {
	key       string
	sizeBytes int64
}

func NewCachePolicy(capacity int) *CachePolicy {
	return &CachePolicy{
		sketch: NewCountMinSketch(capacity*4, 4),
		lru:    list.New(),
		nodes:  make(map[string]*list.Element),
	}
}

func (p *CachePolicy) RecordAccess(key string) {
	p.mu.Lock()
	defer p.mu.Unlock()
	p.sketch.Increment([]byte(key))
	if elem, ok := p.nodes[key]; ok {
		p.lru.MoveToFront(elem)
	}
}

func (p *CachePolicy) Add(key string, sizeBytes int64) {
	p.mu.Lock()
	defer p.mu.Unlock()
	p.sketch.Increment([]byte(key))
	if elem, ok := p.nodes[key]; ok {
		elem.Value.(*lruItem).sizeBytes = sizeBytes
		p.lru.MoveToFront(elem)
		return
	}
	elem := p.lru.PushFront(&lruItem{key: key, sizeBytes: sizeBytes})
	p.nodes[key] = elem
}

func (p *CachePolicy) Remove(key string) {
	p.mu.Lock()
	defer p.mu.Unlock()
	if elem, ok := p.nodes[key]; ok {
		p.lru.Remove(elem)
		delete(p.nodes, key)
	}
}

// EvictOne selects candidate for eviction using TinyLFU frequency vs victim
func (p *CachePolicy) EvictCandidate() (string, int64, bool) {
	p.mu.Lock()
	defer p.mu.Unlock()
	if p.lru.Len() == 0 {
		return "", 0, false
	}

	back := p.lru.Back()
	item := back.Value.(*lruItem)
	p.lru.Remove(back)
	delete(p.nodes, item.key)
	return item.key, item.sizeBytes, true
}

func (p *CachePolicy) Len() int {
	p.mu.Lock()
	defer p.mu.Unlock()
	return p.lru.Len()
}
