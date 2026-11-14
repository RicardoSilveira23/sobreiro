//! SPIKE (shell candidate B): writes a picture as a PNG without compressing it, for `--snapshot` only.
use std::io::Write;

/// `pixels` is red, green, blue, alpha, row by row, not premultiplied.
pub fn write(path: &std::path::Path, width: u32, height: u32, pixels: &[u8]) -> std::io::Result<()> {
    let mut raw = Vec::with_capacity(pixels.len() + height as usize);
    for row in pixels.chunks(width as usize * 4) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    // A zlib stream of stored blocks.
    let mut data = vec![0x78, 0x01];
    let mut blocks = raw.chunks(65_535).peekable();
    while let Some(block) = blocks.next() {
        data.push(if blocks.peek().is_none() { 1 } else { 0 });
        data.extend_from_slice(&(block.len() as u16).to_le_bytes());
        data.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
        data.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for byte in &raw {
        a = (a + *byte as u32) % 65_521;
        b = (b + a) % 65_521;
    }
    data.extend_from_slice(&((b << 16) | a).to_be_bytes());

    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);

    let mut file = std::fs::File::create(path)?;
    file.write_all(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A])?;
    chunk(&mut file, b"IHDR", &header)?;
    chunk(&mut file, b"IDAT", &data)?;
    chunk(&mut file, b"IEND", &[])
}

fn chunk(file: &mut std::fs::File, kind: &[u8; 4], data: &[u8]) -> std::io::Result<()> {
    file.write_all(&(data.len() as u32).to_be_bytes())?;
    file.write_all(kind)?;
    file.write_all(data)?;
    let mut crc = 0xFFFF_FFFFu32;
    for byte in kind.iter().chain(data) {
        crc ^= *byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    file.write_all(&(!crc).to_be_bytes())
}
