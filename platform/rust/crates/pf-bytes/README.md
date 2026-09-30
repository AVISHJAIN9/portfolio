# `pf-bytes`

## Purpose
Provides zero-copy byte manipulation utilities, bit readers/writers, standard unsigned LEB128 encoders, and RFC 9000 IETF QUIC 62-bit variable-length integer codecs.

## API Summary
- `QuicVarInt::encode(val, &mut buf) -> PlatformResult<usize>`
- `QuicVarInt::decode(&mut buf) -> PlatformResult<u64>`
- `ULEB128::encode(val, &mut buf) -> usize`
- `ULEB128::decode(&mut buf) -> PlatformResult<u64>`
- `BitReader::new(slice) -> BitReader`
