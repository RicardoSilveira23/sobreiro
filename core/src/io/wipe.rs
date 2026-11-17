//! Overwriting a secret's bytes before they are given back.

/// Overwrites bytes with zeros, so that what the file held does not stay behind in memory
/// that is about to be given back.
pub(crate) fn wipe(bytes: &mut [u8]) {
    bytes.fill(0);
    // Nobody reads these bytes again, so a compiler could decide that the zeros need not be
    // written at all. `black_box` tells it to assume that someone does read them. The
    // standard library promises no more than its best effort here.
    std::hint::black_box(bytes);
}

#[cfg(test)]
mod tests {
    use super::wipe;

    #[test]
    fn wipe_zeroes_every_byte() {
        let mut bytes = b"Bearer secret".to_vec();
        wipe(&mut bytes);
        assert_eq!(bytes, vec![0; 13]);
    }
}
