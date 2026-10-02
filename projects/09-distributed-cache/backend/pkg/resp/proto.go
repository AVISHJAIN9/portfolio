package resp

import (
	"bufio"
	"bytes"
	"fmt"
	"io"
	"strconv"
)

// 09-076..09-085: Redis RESP2 Wire Protocol Parser & Serializer

type Type byte

const (
	SimpleString Type = '+'
	Error        Type = '-'
	Integer      Type = ':'
	BulkString   Type = '$'
	Array        Type = '*'
)

type Value struct {
	Type   Type
	Str    string
	Num    int64
	Bulk   []byte
	Array  []Value
	IsNull bool
}

func (v Value) Encode() []byte {
	var buf bytes.Buffer
	switch v.Type {
	case SimpleString:
		buf.WriteString(fmt.Sprintf("+%s\r\n", v.Str))
	case Error:
		buf.WriteString(fmt.Sprintf("-%s\r\n", v.Str))
	case Integer:
		buf.WriteString(fmt.Sprintf(":%d\r\n", v.Num))
	case BulkString:
		if v.IsNull {
			buf.WriteString("$-1\r\n")
		} else {
			buf.WriteString(fmt.Sprintf("$%d\r\n%s\r\n", len(v.Bulk), string(v.Bulk)))
		}
	case Array:
		if v.IsNull {
			buf.WriteString("*-1\r\n")
		} else {
			buf.WriteString(fmt.Sprintf("*%d\r\n", len(v.Array)))
			for _, item := range v.Array {
				buf.Write(item.Encode())
			}
		}
	}
	return buf.Bytes()
}

func Parse(r *bufio.Reader) (Value, error) {
	prefix, err := r.ReadByte()
	if err != nil {
		return Value{}, err
	}

	line, err := readLine(r)
	if err != nil {
		return Value{}, err
	}

	switch Type(prefix) {
	case SimpleString:
		return Value{Type: SimpleString, Str: string(line)}, nil
	case Error:
		return Value{Type: Error, Str: string(line)}, nil
	case Integer:
		num, err := strconv.ParseInt(string(line), 10, 64)
		if err != nil {
			return Value{}, err
		}
		return Value{Type: Integer, Num: num}, nil
	case BulkString:
		length, err := strconv.Atoi(string(line))
		if err != nil {
			return Value{}, err
		}
		if length == -1 {
			return Value{Type: BulkString, IsNull: true}, nil
		}
		buf := make([]byte, length+2)
		if _, err := io.ReadFull(r, buf); err != nil {
			return Value{}, err
		}
		return Value{Type: BulkString, Bulk: buf[:length]}, nil
	case Array:
		count, err := strconv.Atoi(string(line))
		if err != nil {
			return Value{}, err
		}
		if count == -1 {
			return Value{Type: Array, IsNull: true}, nil
		}
		items := make([]Value, count)
		for i := 0; i < count; i++ {
			val, err := Parse(r)
			if err != nil {
				return Value{}, err
			}
			items[i] = val
		}
		return Value{Type: Array, Array: items}, nil
	default:
		return Value{}, fmt.Errorf("unknown RESP type: %c", prefix)
	}
}

func readLine(r *bufio.Reader) ([]byte, error) {
	line, err := r.ReadBytes('\n')
	if err != nil {
		return nil, err
	}
	if len(line) >= 2 && line[len(line)-2] == '\r' {
		return line[:len(line)-2], nil
	}
	return line[:len(line)-1], nil
}
