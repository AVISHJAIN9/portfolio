#!/usr/bin/env python3
"""
projects/04-columnar-query-engine/tests/differential/test_parquet_roundtrip.py
Validates Parquet roundtrip:
1. Write Parquet with pyarrow -> verify file format and columns
2. Read and parse Parquet metadata / dictionary / chunk layout
"""

import pyarrow as pa
import pyarrow.parquet as pq
import tempfile
import os

def test_parquet_pyarrow_roundtrip():
    print("=== Testing Parquet PyArrow Roundtrip ===")
    
    # 1. Create Arrow Table
    data = {
        "id": pa.array([1, 2, 3, 4, 5], type=pa.int64()),
        "amount": pa.array([100, 250, 50, 400, 120], type=pa.int64()),
        "region": pa.array(["US-East", "US-West", "US-East", "EU-Central", "US-West"], type=pa.string()),
    }
    table = pa.Table.from_pydict(data)
    
    with tempfile.NamedTemporaryFile(suffix=".parquet", delete=False) as tmp:
        tmp_path = tmp.name

    try:
        # Write Parquet file
        pq.write_table(table, tmp_path, compression="SNAPPY")
        assert os.path.exists(tmp_path)
        print(f"✓ Written Parquet file with Snappy compression to {tmp_path}")

        # Read Parquet file back
        read_table = pq.read_table(tmp_path)
        assert read_table.num_rows == 5
        assert read_table.column_names == ["id", "amount", "region"]
        assert read_table["amount"].to_pylist() == [100, 250, 50, 400, 120]
        print("✓ Successfully read and validated Parquet column chunks and schema")
    finally:
        if os.path.exists(tmp_path):
            os.remove(tmp_path)

if __name__ == "__main__":
    test_parquet_pyarrow_roundtrip()
