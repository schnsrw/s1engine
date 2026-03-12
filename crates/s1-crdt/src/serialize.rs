//! Binary serialization for CRDT operations and state vectors.
//!
//! Custom binary format using varint encoding. No external dependencies (no serde).
//! This is used for network transport and storage of the operation log.

use crate::op_id::OpId;
use crate::state_vector::StateVector;

/// Encode a u64 as a variable-length integer (LEB128).
pub fn encode_varint(value: u64, buf: &mut Vec<u8>) {
    let mut v = value;
    loop {
        let mut byte = (v & 0x7F) as u8;
        v >>= 7;
        if v != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if v == 0 {
            break;
        }
    }
}

/// Decode a variable-length integer (LEB128) from a byte slice.
///
/// Returns `(value, bytes_consumed)` or `None` if the input is too short.
pub fn decode_varint(data: &[u8]) -> Option<(u64, usize)> {
    let mut result: u64 = 0;
    let mut shift = 0;

    for (i, &byte) in data.iter().enumerate() {
        if shift >= 64 {
            return None; // Overflow
        }
        result |= ((byte & 0x7F) as u64) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            return Some((result, i + 1));
        }
    }

    None // Incomplete
}

/// Encode an OpId to bytes.
pub fn encode_op_id(id: &OpId, buf: &mut Vec<u8>) {
    encode_varint(id.replica, buf);
    encode_varint(id.lamport, buf);
}

/// Decode an OpId from bytes. Returns `(OpId, bytes_consumed)`.
pub fn decode_op_id(data: &[u8]) -> Option<(OpId, usize)> {
    let (replica, n1) = decode_varint(data)?;
    let (lamport, n2) = decode_varint(&data[n1..])?;
    Some((OpId::new(replica, lamport), n1 + n2))
}

/// Encode a StateVector to bytes.
pub fn encode_state_vector(sv: &StateVector, buf: &mut Vec<u8>) {
    let entries = sv.entries();
    encode_varint(entries.len() as u64, buf);

    // Sort entries for deterministic encoding
    let mut sorted: Vec<_> = entries.iter().collect();
    sorted.sort_by_key(|(&replica, _)| replica);

    for (&replica, &lamport) in sorted {
        encode_varint(replica, buf);
        encode_varint(lamport, buf);
    }
}

/// Decode a StateVector from bytes. Returns `(StateVector, bytes_consumed)`.
pub fn decode_state_vector(data: &[u8]) -> Option<(StateVector, usize)> {
    let (count, mut offset) = decode_varint(data)?;
    let mut sv = StateVector::new();

    for _ in 0..count {
        let (replica, n1) = decode_varint(&data[offset..])?;
        offset += n1;
        let (lamport, n2) = decode_varint(&data[offset..])?;
        offset += n2;
        sv.set(replica, lamport);
    }

    Some((sv, offset))
}

/// Encode a string to bytes (length-prefixed).
pub fn encode_string(s: &str, buf: &mut Vec<u8>) {
    encode_varint(s.len() as u64, buf);
    buf.extend_from_slice(s.as_bytes());
}

/// Decode a string from bytes. Returns `(String, bytes_consumed)`.
pub fn decode_string(data: &[u8]) -> Option<(String, usize)> {
    let (len, n) = decode_varint(data)?;
    let len = len as usize;
    if data.len() < n + len {
        return None;
    }
    let s = std::str::from_utf8(&data[n..n + len]).ok()?;
    Some((s.to_string(), n + len))
}

/// Encode an optional OpId.
pub fn encode_option_op_id(opt: &Option<OpId>, buf: &mut Vec<u8>) {
    match opt {
        Some(id) => {
            buf.push(1);
            encode_op_id(id, buf);
        }
        None => {
            buf.push(0);
        }
    }
}

/// Decode an optional OpId.
pub fn decode_option_op_id(data: &[u8]) -> Option<(Option<OpId>, usize)> {
    if data.is_empty() {
        return None;
    }
    match data[0] {
        0 => Some((None, 1)),
        1 => {
            let (id, n) = decode_op_id(&data[1..])?;
            Some((Some(id), 1 + n))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varint_zero() {
        let mut buf = Vec::new();
        encode_varint(0, &mut buf);
        assert_eq!(buf, vec![0]);
        assert_eq!(decode_varint(&buf), Some((0, 1)));
    }

    #[test]
    fn varint_small() {
        let mut buf = Vec::new();
        encode_varint(42, &mut buf);
        assert_eq!(decode_varint(&buf), Some((42, buf.len())));
    }

    #[test]
    fn varint_127() {
        let mut buf = Vec::new();
        encode_varint(127, &mut buf);
        assert_eq!(buf.len(), 1); // Fits in one byte
        assert_eq!(decode_varint(&buf), Some((127, 1)));
    }

    #[test]
    fn varint_128() {
        let mut buf = Vec::new();
        encode_varint(128, &mut buf);
        assert_eq!(buf.len(), 2);
        assert_eq!(decode_varint(&buf), Some((128, 2)));
    }

    #[test]
    fn varint_large() {
        let mut buf = Vec::new();
        let val = u64::MAX;
        encode_varint(val, &mut buf);
        assert_eq!(decode_varint(&buf), Some((val, buf.len())));
    }

    #[test]
    fn varint_roundtrip_various() {
        for &val in &[0, 1, 127, 128, 255, 256, 16383, 16384, 1_000_000, u64::MAX] {
            let mut buf = Vec::new();
            encode_varint(val, &mut buf);
            let (decoded, _) = decode_varint(&buf).unwrap();
            assert_eq!(decoded, val, "Failed for value {val}");
        }
    }

    #[test]
    fn op_id_roundtrip() {
        let id = OpId::new(42, 100);
        let mut buf = Vec::new();
        encode_op_id(&id, &mut buf);

        let (decoded, n) = decode_op_id(&buf).unwrap();
        assert_eq!(decoded, id);
        assert_eq!(n, buf.len());
    }

    #[test]
    fn state_vector_roundtrip() {
        let mut sv = StateVector::new();
        sv.set(1, 5);
        sv.set(2, 10);
        sv.set(100, 99);

        let mut buf = Vec::new();
        encode_state_vector(&sv, &mut buf);

        let (decoded, n) = decode_state_vector(&buf).unwrap();
        assert_eq!(decoded, sv);
        assert_eq!(n, buf.len());
    }

    #[test]
    fn state_vector_empty_roundtrip() {
        let sv = StateVector::new();
        let mut buf = Vec::new();
        encode_state_vector(&sv, &mut buf);

        let (decoded, _) = decode_state_vector(&buf).unwrap();
        assert_eq!(decoded, sv);
    }

    #[test]
    fn string_roundtrip() {
        let s = "hello world";
        let mut buf = Vec::new();
        encode_string(s, &mut buf);

        let (decoded, n) = decode_string(&buf).unwrap();
        assert_eq!(decoded, s);
        assert_eq!(n, buf.len());
    }

    #[test]
    fn string_empty_roundtrip() {
        let s = "";
        let mut buf = Vec::new();
        encode_string(s, &mut buf);

        let (decoded, _) = decode_string(&buf).unwrap();
        assert_eq!(decoded, s);
    }

    #[test]
    fn string_unicode_roundtrip() {
        let s = "héllo wörld 你好";
        let mut buf = Vec::new();
        encode_string(s, &mut buf);

        let (decoded, _) = decode_string(&buf).unwrap();
        assert_eq!(decoded, s);
    }

    #[test]
    fn option_op_id_roundtrip_some() {
        let id = Some(OpId::new(5, 10));
        let mut buf = Vec::new();
        encode_option_op_id(&id, &mut buf);

        let (decoded, _) = decode_option_op_id(&buf).unwrap();
        assert_eq!(decoded, id);
    }

    #[test]
    fn option_op_id_roundtrip_none() {
        let id: Option<OpId> = None;
        let mut buf = Vec::new();
        encode_option_op_id(&id, &mut buf);

        let (decoded, n) = decode_option_op_id(&buf).unwrap();
        assert_eq!(decoded, id);
        assert_eq!(n, 1);
    }

    #[test]
    fn decode_varint_incomplete() {
        // 0x80 means "more bytes follow" but there are none
        assert!(decode_varint(&[0x80]).is_none());
    }

    #[test]
    fn decode_string_truncated() {
        let mut buf = Vec::new();
        encode_varint(10, &mut buf); // Claims 10 bytes
        buf.push(b'h'); // Only 1 byte
        assert!(decode_string(&buf).is_none());
    }

    #[test]
    fn multiple_values_in_buffer() {
        let mut buf = Vec::new();
        encode_varint(42, &mut buf);
        encode_varint(100, &mut buf);

        let (v1, n1) = decode_varint(&buf).unwrap();
        let (v2, _n2) = decode_varint(&buf[n1..]).unwrap();
        assert_eq!(v1, 42);
        assert_eq!(v2, 100);
    }
}
