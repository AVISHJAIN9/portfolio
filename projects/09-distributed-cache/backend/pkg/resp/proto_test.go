package resp_test

import (
	"bufio"
	"bytes"
	"testing"

	"github.com/AVISHJAIN9/distributed-cache/pkg/resp"
)

// [FEAT: 09-076, 09-077, 09-078]
func TestRESP2EncodingAndParsing(t *testing.T) {
	// Simple String
	ss := resp.Value{Type: resp.SimpleString, Str: "OK"}
	encSS := ss.Encode()
	if string(encSS) != "+OK\r\n" {
		t.Fatalf("unexpected SimpleString encoding: %q", string(encSS))
	}
	decSS, err := resp.Parse(bufio.NewReader(bytes.NewReader(encSS)))
	if err != nil || decSS.Str != "OK" {
		t.Fatalf("unexpected SimpleString decoding: %+v, %v", decSS, err)
	}

	// Bulk String
	bs := resp.Value{Type: resp.BulkString, Bulk: []byte("helloworld")}
	encBS := bs.Encode()
	if string(encBS) != "$10\r\nhelloworld\r\n" {
		t.Fatalf("unexpected BulkString encoding: %q", string(encBS))
	}
	decBS, err := resp.Parse(bufio.NewReader(bytes.NewReader(encBS)))
	if err != nil || string(decBS.Bulk) != "helloworld" {
		t.Fatalf("unexpected BulkString decoding: %+v, %v", decBS, err)
	}

	// Null Bulk String
	nullBS := resp.Value{Type: resp.BulkString, IsNull: true}
	encNull := nullBS.Encode()
	if string(encNull) != "$-1\r\n" {
		t.Fatalf("unexpected NullBulkString encoding: %q", string(encNull))
	}
	decNull, err := resp.Parse(bufio.NewReader(bytes.NewReader(encNull)))
	if err != nil || !decNull.IsNull {
		t.Fatalf("unexpected NullBulkString decoding: %+v, %v", decNull, err)
	}

	// Array (MGET response style)
	arr := resp.Value{
		Type: resp.Array,
		Array: []resp.Value{
			{Type: resp.BulkString, Bulk: []byte("GET")},
			{Type: resp.BulkString, Bulk: []byte("user:1")},
		},
	}
	encArr := arr.Encode()
	decArr, err := resp.Parse(bufio.NewReader(bytes.NewReader(encArr)))
	if err != nil || len(decArr.Array) != 2 || string(decArr.Array[0].Bulk) != "GET" {
		t.Fatalf("unexpected Array decoding: %+v, %v", decArr, err)
	}

	// Negative: Invalid prefix
	_, err = resp.Parse(bufio.NewReader(bytes.NewReader([]byte("?invalid\r\n"))))
	if err == nil {
		t.Fatalf("expected error on invalid prefix")
	}
}
