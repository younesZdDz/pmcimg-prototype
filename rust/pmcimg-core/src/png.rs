use crate::error::{PmcImgError, PmcImgResult};

const PNG_SIG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

pub fn parse_png_dimensions(bytes: &[u8]) -> PmcImgResult<(u32, u32)> {
    if bytes.len() < 24 {
        return Err(PmcImgError::Png("too short".into()));
    }
    if bytes[0..8] != PNG_SIG {
        return Err(PmcImgError::Png("bad PNG signature".into()));
    }
    // First chunk should be IHDR
    // Layout: length(4) type(4) data(length) crc(4)
    // IHDR data: width(4) height(4) ...
    let len = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    let ctype = &bytes[12..16];
    if ctype != b"IHDR" {
        return Err(PmcImgError::Png("IHDR not found".into()));
    }
    if len < 8 || bytes.len() < 16 + len {
        return Err(PmcImgError::Png("IHDR too short".into()));
    }
    let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    Ok((w, h))
}




