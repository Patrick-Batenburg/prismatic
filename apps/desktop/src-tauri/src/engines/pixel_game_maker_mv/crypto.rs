use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use twofish::Twofish;
use twofish::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
use twofish::cipher::generic_array::GenericArray;

/// Fixed IV for all PGMMV encryption
const PGMM_IV: [u8; 16] = [
    0xA0, 0x47, 0xE9, 0x3D, 0x23, 0x0A, 0x4C, 0x62,
    0xA7, 0x44, 0xB1, 0xA4, 0xEE, 0x85, 0x7F, 0xBA,
];

/// Rotate bytes left by n positions
fn rol_bytes(bs: &[u8], n: usize) -> Vec<u8> {
    let n = n % bs.len();
    [&bs[n..], &bs[..n]].concat()
}

/// Rotate bytes right by n positions
fn ror_bytes(bs: &[u8], n: usize) -> Vec<u8> {
    let n = n % bs.len();
    [&bs[bs.len() - n..], &bs[..bs.len() - n]].concat()
}

/// XOR two byte slices
fn xor_block(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter().zip(b.iter()).map(|(x, y)| x ^ y).collect()
}

/// Weakfish encrypt: simplified cipher for keys <= 8 bytes
fn weakfish_encrypt(block: &[u8; 16]) -> [u8; 16] {
    let mut result = Vec::with_capacity(16);
    result.extend_from_slice(&rol_bytes(&block[0..4], 1));
    result.extend_from_slice(&ror_bytes(&block[4..8], 1));
    result.extend_from_slice(&rol_bytes(&block[8..12], 1));
    result.extend_from_slice(&ror_bytes(&block[12..16], 1));
    let rotated = rol_bytes(&result, 8);
    let mut out = [0u8; 16];
    out.copy_from_slice(&rotated);
    out
}

/// Weakfish decrypt: reverse of encrypt
fn weakfish_decrypt(block: &[u8; 16]) -> [u8; 16] {
    let unrotated = ror_bytes(block, 8);
    let mut result = Vec::with_capacity(16);
    result.extend_from_slice(&ror_bytes(&unrotated[0..4], 1));
    result.extend_from_slice(&rol_bytes(&unrotated[4..8], 1));
    result.extend_from_slice(&ror_bytes(&unrotated[8..12], 1));
    result.extend_from_slice(&rol_bytes(&unrotated[12..16], 1));
    let mut out = [0u8; 16];
    out.copy_from_slice(&result);
    out
}

/// Derive subkey for Twofish from master key and plaintext length
fn derive_subkey(key: &[u8], plaintext_len: usize) -> Vec<u8> {
    let mut padded_key = key.to_vec();
    while padded_key.len() < 8 {
        padded_key.push(0);
    }

    // Encode plaintext length as little-endian 8 bytes, strip trailing nulls
    let ptl_bytes_full = (plaintext_len as u64).to_le_bytes();
    let ptl_bytes: Vec<u8> = {
        let last_nonzero = ptl_bytes_full.iter().rposition(|&b| b != 0).map(|p| p + 1).unwrap_or(1);
        ptl_bytes_full[..last_nonzero].to_vec()
    };

    // XOR with key, replace null bytes with 0x01
    let xor_result: Vec<u8> = xor_block(&ptl_bytes, &padded_key[..ptl_bytes.len()])
        .iter()
        .map(|&b| if b == 0 { 1 } else { b })
        .collect();

    // Final key = xor_result + remainder of original key
    let mut final_key = xor_result;
    if final_key.len() < padded_key.len() {
        final_key.extend_from_slice(&padded_key[final_key.len()..]);
    }
    final_key
}

enum CipherMode {
    Weakfish,
    TwofishCipher(Twofish),
}

impl CipherMode {
    fn decrypt_block(&self, block: &[u8; 16]) -> [u8; 16] {
        match self {
            CipherMode::Weakfish => weakfish_decrypt(block),
            CipherMode::TwofishCipher(cipher) => {
                let mut buf = GenericArray::clone_from_slice(block);
                cipher.decrypt_block(&mut buf);
                let mut out = [0u8; 16];
                out.copy_from_slice(&buf);
                out
            }
        }
    }

    fn encrypt_block(&self, block: &[u8; 16]) -> [u8; 16] {
        match self {
            CipherMode::Weakfish => weakfish_encrypt(block),
            CipherMode::TwofishCipher(cipher) => {
                let mut buf = GenericArray::clone_from_slice(block);
                cipher.encrypt_block(&mut buf);
                let mut out = [0u8; 16];
                out.copy_from_slice(&buf);
                out
            }
        }
    }
}

/// CBC decrypt
fn cbc_decrypt(cipher: &CipherMode, iv: &[u8; 16], ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    if ciphertext.len() % 16 != 0 {
        return Err("Ciphertext length must be multiple of 16".into());
    }

    let mut result = Vec::with_capacity(ciphertext.len());
    let mut prev = iv.to_vec();

    for chunk in ciphertext.chunks(16) {
        let mut block = [0u8; 16];
        block.copy_from_slice(chunk);

        let decrypted = cipher.decrypt_block(&block);
        let pt_block = xor_block(&decrypted, &prev);
        result.extend_from_slice(&pt_block);
        prev = chunk.to_vec();
    }

    Ok(result)
}

/// CBC encrypt
fn cbc_encrypt(cipher: &CipherMode, iv: &[u8; 16], plaintext: &[u8]) -> Vec<u8> {
    let mut padded = plaintext.to_vec();
    let padding_len = (16 - (padded.len() % 16)) % 16;
    padded.extend(vec![0u8; padding_len]);

    let mut result = Vec::with_capacity(padded.len());
    let mut prev = iv.to_vec();

    for chunk in padded.chunks(16) {
        let mixed = xor_block(chunk, &prev);
        let mut block = [0u8; 16];
        block.copy_from_slice(&mixed);

        let encrypted = cipher.encrypt_block(&block);
        result.extend_from_slice(&encrypted);
        prev = encrypted.to_vec();
    }

    result
}

/// Load and decrypt the encryption key from info.json
pub fn load_key(info_json_path: &std::path::Path) -> Result<Vec<u8>, String> {
    let content = std::fs::read_to_string(info_json_path)
        .map_err(|e| format!("Failed to read info.json: {e}"))?;
    let info: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse info.json: {e}"))?;

    let key_b64 = info["key"]
        .as_str()
        .ok_or("No 'key' field in info.json")?;

    let encrypted_key = BASE64.decode(key_b64)
        .map_err(|e| format!("Failed to base64 decode key: {e}"))?;

    // Always use Weakfish to decrypt the key itself
    let cipher = CipherMode::Weakfish;
    let decrypted = cbc_decrypt(&cipher, &PGMM_IV, &encrypted_key)?;

    // Strip trailing nulls
    let key = decrypted.into_iter()
        .rev()
        .skip_while(|&b| b == 0)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();

    Ok(key)
}

/// Create the appropriate cipher mode based on key length
fn make_cipher(key: &[u8], plaintext_len: usize) -> Result<CipherMode, String> {
    if key.len() <= 8 {
        Ok(CipherMode::Weakfish)
    } else {
        let subkey = derive_subkey(key, plaintext_len);
        // Twofish needs 16, 24, or 32 byte key
        let padded_key = match subkey.len() {
            0..=16 => {
                let mut k = subkey.clone();
                k.resize(16, 0);
                k
            }
            17..=24 => {
                let mut k = subkey.clone();
                k.resize(24, 0);
                k
            }
            _ => {
                let mut k = subkey.clone();
                k.resize(32, 0);
                k
            }
        };
        let cipher = Twofish::new_from_slice(&padded_key)
            .map_err(|e| format!("Failed to create Twofish cipher: {e}"))?;
        Ok(CipherMode::TwofishCipher(cipher))
    }
}

/// Decrypt a PGMMV resource file (save or project.json)
pub fn decrypt_resource(data: &[u8], key: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 4 || &data[0..3] != b"enc" {
        // Not encrypted, return as-is
        return Ok(data.to_vec());
    }

    let padding_len = data[3] as usize;
    let encrypted_data = &data[4..];
    let pt_len = encrypted_data.len().saturating_sub(padding_len);

    // For Twofish, we need the plaintext length for subkey derivation
    let cipher = make_cipher(key, pt_len)?;
    let decrypted = cbc_decrypt(&cipher, &PGMM_IV, encrypted_data)?;

    Ok(decrypted[..pt_len].to_vec())
}

/// Encrypt data for writing back to a PGMMV resource file
pub fn encrypt_resource(plaintext: &[u8], key: &[u8]) -> Result<Vec<u8>, String> {
    let pt_len = plaintext.len();
    let padding_len = (16 - (pt_len % 16)) % 16;

    let mut padded = plaintext.to_vec();
    padded.extend(vec![0u8; padding_len]);

    let cipher = make_cipher(key, pt_len)?;
    let encrypted = cbc_encrypt(&cipher, &PGMM_IV, &padded);

    let mut result = Vec::with_capacity(4 + encrypted.len());
    result.extend_from_slice(b"enc");
    result.push(padding_len as u8);
    result.extend_from_slice(&encrypted);

    Ok(result)
}
