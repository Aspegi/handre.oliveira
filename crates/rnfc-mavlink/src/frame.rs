#![deny(unsafe_code)]

use std::vec::Vec;

pub const MAGIC: u8 = 0xFD;
pub const SYSID: u8 = 0x01;
pub const COMPID: u8 = 0x01;
pub const MSGID_HANDSHAKE: u32 = 130;
pub const MSGID_ENCAP: u32 = 131;
pub const CHUNK_SIZE: usize = 253;

/// Handshake MAVLink (msgid=130, len=14). Listener só verifica magic+len+msgid.
pub fn encode_handshake(seq: u8, width: u16, height: u16, num_chunks: u16) -> Vec<u8> {
    let mut p = Vec::with_capacity(26);
    p.push(MAGIC);
    p.push(14);
    p.push(0);
    p.push(0);
    p.push(seq);
    p.push(SYSID);
    p.push(COMPID);
    p.extend_from_slice(&MSGID_HANDSHAKE.to_le_bytes()[..3]);
    // payload 14 bytes: size(1) + width(2) + height(2) + packets(2) + chunk_size(1) + quality(1) + pad(5)
    p.push(0);
    p.extend_from_slice(&width.to_le_bytes());
    p.extend_from_slice(&height.to_le_bytes());
    p.extend_from_slice(&num_chunks.to_le_bytes());
    p.push(CHUNK_SIZE as u8);
    p.push(0);
    p.extend_from_slice(&[0, 0, 0, 0, 0]);
    // CRC dummy (listener nao verifica)
    p.push(0);
    p.push(0);
    p
}

/// Chunk (msgid=131). Sub-header de 2 bytes (chunk_idx LE) + data.
pub fn encode_chunk(seq: u8, chunk_idx: u16, data: &[u8]) -> Vec<u8> {
    let payload_len = 2 + data.len();
    let mut p = Vec::with_capacity(10 + payload_len + 2);
    p.push(MAGIC);
    p.push(payload_len as u8);
    p.push(0);
    p.push(0);
    p.push(seq);
    p.push(SYSID);
    p.push(COMPID);
    p.extend_from_slice(&MSGID_ENCAP.to_le_bytes()[..3]);
    p.extend_from_slice(&chunk_idx.to_le_bytes());
    p.extend_from_slice(data);
    p.push(0);
    p.push(0);
    p
}

/// Codifica uma imagem completa: 1 handshake + N chunks. Avança *seq.
pub fn encode_frame(seq: &mut u8, width: u16, height: u16, image: &[u8]) -> Vec<Vec<u8>> {
    let num_chunks = image.len().div_ceil(CHUNK_SIZE);
    let mut packets = Vec::with_capacity(1 + num_chunks);
    packets.push(encode_handshake(*seq, width, height, num_chunks as u16));
    *seq = seq.wrapping_add(1);
    for i in 0..num_chunks {
        let start = i * CHUNK_SIZE;
        let end = (start + CHUNK_SIZE).min(image.len());
        packets.push(encode_chunk(*seq, i as u16, &image[start..end]));
        *seq = seq.wrapping_add(1);
    }
    packets
}

/// Gera imagem RGB com gradient + tint (para distinguir waypoints visualmente).
pub fn generate_test_image(width: u16, height: u16, tint: [u8; 3]) -> Vec<u8> {
    let mut img = Vec::with_capacity(width as usize * height as usize * 3);
    for y in 0..height {
        for x in 0..width {
            let r = ((x as u32 * tint[0] as u32) / width as u32) as u8;
            let g = ((y as u32 * tint[1] as u32) / height as u32) as u8;
            let b = tint[2];
            img.push(r);
            img.push(g);
            img.push(b);
        }
    }
    img
}
