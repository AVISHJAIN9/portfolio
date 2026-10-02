package cluster_test

import (
	"testing"

	"github.com/AVISHJAIN9/distributed-cache/pkg/cluster"
)

// [FEAT: 09-060, 09-061, 09-065]
func TestConsistentHashRingVNodesAndLocate(t *testing.T) {
	ring := cluster.NewHashRing(128)

	// Add nodes
	ring.AddNode("node-1:6379")
	ring.AddNode("node-2:6379")
	ring.AddNode("node-3:6379")

	if ring.NodeCount() != 3 {
		t.Fatalf("expected 3 nodes, got %d", ring.NodeCount())
	}
	if ring.VNodeCount() != 3*128 {
		t.Fatalf("expected %d vnodes, got %d", 3*128, ring.VNodeCount())
	}

	// Deterministic key location
	n1, err := ring.LocateKey("user:profile:1001")
	if err != nil {
		t.Fatalf("locate failed: %v", err)
	}
	n2, err := ring.LocateKey("user:profile:1001")
	if err != nil || n1 != n2 {
		t.Fatalf("expected deterministic locate %s == %s", n1, n2)
	}

	// Remove node
	ring.RemoveNode("node-2:6379")
	if ring.NodeCount() != 2 {
		t.Fatalf("expected 2 nodes, got %d", ring.NodeCount())
	}

	// Negative: Empty ring returns error
	emptyRing := cluster.NewHashRing(128)
	if _, err := emptyRing.LocateKey("key"); err == nil {
		t.Fatalf("expected error on empty ring")
	}
}
