package persist

import (
	"encoding/binary"
	"hash/crc32"
	"io"
	"os"
	"sync"
)

type OpType uint8

const (
	OpSet OpType = 1
	OpDel OpType = 2
)

type JournalRecord struct {
	Op       OpType
	Key      []byte
	Value    []byte
	TTLMs    int64
	Checksum uint32
}

// 09-041..09-052: Append-only durability journal
type Journal struct {
	mu   sync.Mutex
	file *os.File
	path string
}

func OpenJournal(path string) (*Journal, error) {
	f, err := os.OpenFile(path, os.O_CREATE|os.O_RDWR|os.O_APPEND, 0644)
	if err != nil {
		return nil, err
	}
	return &Journal{
		file: f,
		path: path,
	}, nil
}

func (j *Journal) Append(op OpType, key, value []byte, ttlMs int64) error {
	j.mu.Lock()
	defer j.mu.Unlock()

	keyLen := uint32(len(key))
	valLen := uint32(len(value))

	// Payload: Op(1) + TTL(8) + KeyLen(4) + Key + ValLen(4) + Val
	buf := make([]byte, 1+8+4+keyLen+4+valLen)
	buf[0] = byte(op)
	binary.BigEndian.PutUint64(buf[1:9], uint64(ttlMs))
	binary.BigEndian.PutUint32(buf[9:13], keyLen)
	copy(buf[13:13+keyLen], key)
	binary.BigEndian.PutUint32(buf[13+keyLen:17+keyLen], valLen)
	copy(buf[17+keyLen:], value)

	checksum := crc32.ChecksumIEEE(buf)
	recordBuf := make([]byte, 4+len(buf))
	binary.BigEndian.PutUint32(recordBuf[0:4], checksum)
	copy(recordBuf[4:], buf)

	if _, err := j.file.Write(recordBuf); err != nil {
		return err
	}
	return j.file.Sync()
}

func (j *Journal) Replay() ([]JournalRecord, error) {
	j.mu.Lock()
	defer j.mu.Unlock()

	if _, err := j.file.Seek(0, io.SeekStart); err != nil {
		return nil, err
	}

	var records []JournalRecord
	for {
		header := make([]byte, 4+1+8+4)
		if _, err := io.ReadFull(j.file, header); err != nil {
			if err == io.EOF || err == io.ErrUnexpectedEOF {
				break
			}
			return nil, err
		}

		expectedChecksum := binary.BigEndian.Uint32(header[0:4])
		op := OpType(header[4])
		ttlMs := int64(binary.BigEndian.Uint64(header[5:13]))
		keyLen := binary.BigEndian.Uint32(header[13:17])

		key := make([]byte, keyLen)
		if _, err := io.ReadFull(j.file, key); err != nil {
			break
		}

		valLenBuf := make([]byte, 4)
		if _, err := io.ReadFull(j.file, valLenBuf); err != nil {
			break
		}
		valLen := binary.BigEndian.Uint32(valLenBuf)

		val := make([]byte, valLen)
		if _, err := io.ReadFull(j.file, val); err != nil {
			break
		}

		// Verify CRC
		payload := make([]byte, 1+8+4+keyLen+4+valLen)
		payload[0] = byte(op)
		binary.BigEndian.PutUint64(payload[1:9], uint64(ttlMs))
		binary.BigEndian.PutUint32(payload[9:13], keyLen)
		copy(payload[13:13+keyLen], key)
		binary.BigEndian.PutUint32(payload[13+keyLen:17+keyLen], valLen)
		copy(payload[17+keyLen:], val)

		if crc32.ChecksumIEEE(payload) != expectedChecksum {
			// Corrupted record encountered: stop replay
			break
		}

		records = append(records, JournalRecord{
			Op:       op,
			Key:      key,
			Value:    val,
			TTLMs:    ttlMs,
			Checksum: expectedChecksum,
		})
	}

	return records, nil
}

func (j *Journal) Close() error {
	j.mu.Lock()
	defer j.mu.Unlock()
	return j.file.Close()
}
