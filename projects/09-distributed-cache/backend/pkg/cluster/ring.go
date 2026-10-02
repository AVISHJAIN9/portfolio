package cluster

import (
	"fmt"
	"hash/fnv"
	"sort"
	"strconv"
	"sync"
)

// 09-060..09-075: Consistent Hash Ring with >= 128 VNodes per node
type HashRing struct {
	mu       sync.RWMutex
	vnodes   int
	ring     []uint32
	ringMap  map[uint32]string
	nodeList []string
}

func NewHashRing(vnodes int) *HashRing {
	if vnodes < 128 {
		vnodes = 128
	}
	return &HashRing{
		vnodes:  vnodes,
		ringMap: make(map[uint32]string),
	}
}

func (h *HashRing) hash(key string) uint32 {
	hasher := fnv.New32a()
	hasher.Write([]byte(key))
	return hasher.Sum32()
}

func (h *HashRing) AddNode(node string) {
	h.mu.Lock()
	defer h.mu.Unlock()

	for _, n := range h.nodeList {
		if n == node {
			return
		}
	}

	h.nodeList = append(h.nodeList, node)
	for i := 0; i < h.vnodes; i++ {
		vkey := node + "#" + strconv.Itoa(i)
		hashVal := h.hash(vkey)
		h.ring = append(h.ring, hashVal)
		h.ringMap[hashVal] = node
	}
	sort.Slice(h.ring, func(i, j int) bool { return h.ring[i] < h.ring[j] })
}

func (h *HashRing) RemoveNode(node string) {
	h.mu.Lock()
	defer h.mu.Unlock()

	var newNodes []string
	for _, n := range h.nodeList {
		if n != node {
			newNodes = append(newNodes, n)
		}
	}
	h.nodeList = newNodes

	var newRing []uint32
	for i := 0; i < h.vnodes; i++ {
		vkey := node + "#" + strconv.Itoa(i)
		hashVal := h.hash(vkey)
		delete(h.ringMap, hashVal)
	}

	for _, v := range h.ring {
		if _, exists := h.ringMap[v]; exists {
			newRing = append(newRing, v)
		}
	}
	h.ring = newRing
}

func (h *HashRing) LocateKey(key string) (string, error) {
	h.mu.RLock()
	defer h.mu.RUnlock()

	if len(h.ring) == 0 {
		return "", fmt.Errorf("empty hash ring")
	}

	hashVal := h.hash(key)
	idx := sort.Search(len(h.ring), func(i int) bool {
		return h.ring[i] >= hashVal
	})

	if idx == len(h.ring) {
		idx = 0
	}

	return h.ringMap[h.ring[idx]], nil
}

func (h *HashRing) NodeCount() int {
	h.mu.RLock()
	defer h.mu.RUnlock()
	return len(h.nodeList)
}

func (h *HashRing) VNodeCount() int {
	h.mu.RLock()
	defer h.mu.RUnlock()
	return len(h.ring)
}
