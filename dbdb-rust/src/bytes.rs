// Everything this database puts on disk is either a little-endian u64 or a
// blob with a u64 length in front of it, so these four functions describe the
// whole on-disk format.
//
// The length checks look fussier than they need to be, but a corrupt file --
// which is exactly what a crash part-way through a write leaves behind -- gets
// here with a nonsense length, and an error is a much better outcome than a
// panic or a gigantic allocation.

// Appends n to buffer, lowest byte first.
pub fn put_u64(n: u64, buffer: &mut Vec<u8>) {
    for i in 0..8 {
        buffer.push(((n >> (i * 8)) & 0xFF) as u8);
    }
}

// Reads a little-endian u64 at idx and moves idx past it.
pub fn get_u64(buffer: &[u8], idx: &mut usize) -> Result<u64, String> {
    if *idx > buffer.len() || buffer.len() - *idx < 8 {
        return Err("truncated buffer".to_string());
    }
    let mut n: u64 = 0;
    for i in 0..8 {
        n |= (buffer[*idx] as u64) << (i * 8);
        *idx += 1;
    }
    Ok(n)
}

// Appends b to buffer as [u64 length][bytes].
pub fn put_bytes(b: &[u8], buffer: &mut Vec<u8>) {
    put_u64(b.len() as u64, buffer);
    buffer.extend_from_slice(b);
}

// Reads a [u64 length][bytes] record at idx and moves idx past it.
pub fn get_bytes(buffer: &[u8], idx: &mut usize) -> Result<Vec<u8>, String> {
    let length = get_u64(buffer, idx)?;
    // Compared before the cast to usize, so a length that cannot fit in memory
    // is rejected here rather than wrapping into something that looks sensible.
    if length > buffer.len() as u64 || buffer.len() - *idx < length as usize {
        return Err("truncated buffer".to_string());
    }
    let out = buffer[*idx..*idx + length as usize].to_vec();
    *idx += length as usize;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u64_survives_a_round_trip() {
        let mut buffer = Vec::new();
        put_u64(0, &mut buffer);
        put_u64(1, &mut buffer);
        put_u64(u64::MAX, &mut buffer);

        let mut idx = 0;
        assert_eq!(get_u64(&buffer, &mut idx).unwrap(), 0);
        assert_eq!(get_u64(&buffer, &mut idx).unwrap(), 1);
        assert_eq!(get_u64(&buffer, &mut idx).unwrap(), u64::MAX);
        assert_eq!(idx, 24);
    }

    #[test]
    fn bytes_survive_a_round_trip() {
        let mut buffer = Vec::new();
        put_bytes(b"", &mut buffer);
        put_bytes(b"accountA", &mut buffer);

        let mut idx = 0;
        assert_eq!(get_bytes(&buffer, &mut idx).unwrap(), b"");
        assert_eq!(get_bytes(&buffer, &mut idx).unwrap(), b"accountA");
    }

    #[test]
    fn a_short_buffer_is_an_error_and_not_a_panic() {
        let mut idx = 0;
        assert!(get_u64(&[1, 2, 3], &mut idx).is_err());

        // A length that promises far more than the buffer holds.
        let mut buffer = Vec::new();
        put_u64(9999, &mut buffer);
        buffer.push(b'x');
        let mut idx = 0;
        assert!(get_bytes(&buffer, &mut idx).is_err());
    }
}
