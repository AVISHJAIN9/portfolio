package persist_test

import (
	"bytes"
	"os"
	"path/filepath"
	"testing"

	"github.com/AVISHJAIN9/distributed-cache/pkg/persist"
)

// [FEAT: 09-041, 09-049, 09-050]
func TestJournalAppendReplayAndRecovery(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "journal_test_*")
	if err != nil {
		t.Fatalf("temp dir creation failed: %v", err)
	}
	defer os.RemoveAll(tmpDir)

	jPath := filepath.Join(tmpDir, "cache.journal")
	j, err := persist.OpenJournal(jPath)
	if err != nil {
		t.Fatalf("open journal failed: %v", err)
	}

	if err := j.Append(persist.OpSet, []byte("key1"), []byte("val1"), 1000); err != nil {
		t.Fatalf("append 1 failed: %v", err)
	}
	if err := j.Append(persist.OpSet, []byte("key2"), []byte("val2"), 0); err != nil {
		t.Fatalf("append 2 failed: %v", err)
	}
	if err := j.Append(persist.OpDel, []byte("key1"), nil, 0); err != nil {
		t.Fatalf("append 3 failed: %v", err)
	}

	replayed, err := j.Replay()
	if err != nil {
		t.Fatalf("replay failed: %v", err)
	}
	if len(replayed) != 3 {
		t.Fatalf("expected 3 replayed records, got %d", len(replayed))
	}
	if replayed[0].Op != persist.OpSet || !bytes.Equal(replayed[0].Key, []byte("key1")) {
		t.Fatalf("record 0 mismatch: %+v", replayed[0])
	}
	if replayed[2].Op != persist.OpDel {
		t.Fatalf("record 2 mismatch: %+v", replayed[2])
	}

	// Fault injection: Corrupted trailing bytes
	f, err := os.OpenFile(jPath, os.O_APPEND|os.O_WRONLY, 0644)
	if err != nil {
		t.Fatalf("open append failed: %v", err)
	}
	f.Write([]byte{0xBA, 0xAD, 0xCA, 0xFE, 0x00, 0x01})
	f.Close()

	// Replay should gracefully handle corrupted tail and still return valid prefix
	replayedAfterCorruption, err := j.Replay()
	if err != nil {
		t.Fatalf("replay after corruption failed: %v", err)
	}
	if len(replayedAfterCorruption) != 3 {
		t.Fatalf("expected 3 valid prefix records, got %d", len(replayedAfterCorruption))
	}

	j.Close()
}
