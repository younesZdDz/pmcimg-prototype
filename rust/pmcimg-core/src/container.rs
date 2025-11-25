use crate::error::{PmcImgError, PmcImgResult};

const MAGIC: &[u8; 4] = b"PMCI";
const VERSION_V0: u8 = 0x01;

pub struct ContainerParts {
    pub manifest_bytes: Vec<u8>,
    pub ciphertext_full: Vec<u8>,
}

pub fn build_container(manifest_bytes: &[u8], ciphertext_full: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + 1 + 4 + 8 + manifest_bytes.len() + ciphertext_full.len());
    out.extend_from_slice(MAGIC);
    out.push(VERSION_V0);
    out.extend_from_slice(&(manifest_bytes.len() as u32).to_be_bytes());
    out.extend_from_slice(&(ciphertext_full.len() as u64).to_be_bytes());
    out.extend_from_slice(manifest_bytes);
    out.extend_from_slice(ciphertext_full);
    out
}

pub fn parse_container(bytes: &[u8]) -> PmcImgResult<ContainerParts> {
    if bytes.len() < 4 + 1 + 4 + 8 {
        return Err(PmcImgError::InvalidLengths);
    }
    if &bytes[0..4] != MAGIC {
        return Err(PmcImgError::InvalidMagic);
    }
    let ver = bytes[4];
    if ver != VERSION_V0 {
        return Err(PmcImgError::UnsupportedVersion(ver));
    }
    let manifest_len = u32::from_be_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]) as usize;
    let ct_len = u64::from_be_bytes([
        bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15], bytes[16],
    ]) as usize;

    let header_len = 4 + 1 + 4 + 8;
    let needed = header_len + manifest_len + ct_len;
    if bytes.len() < needed {
        return Err(PmcImgError::InvalidLengths);
    }
    let manifest_bytes = bytes[header_len..header_len + manifest_len].to_vec();
    let ciphertext_full = bytes[header_len + manifest_len..needed].to_vec();
    Ok(ContainerParts {
        manifest_bytes,
        ciphertext_full,
    })
}




