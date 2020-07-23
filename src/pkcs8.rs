use rand::{thread_rng, Rng};
use sodalite::{ 
  SecretboxKey, SecretboxNonce, 
  SECRETBOX_KEY_LEN, SECRETBOX_NONCE_LEN, 
  secretbox_open, secretbox
};
pub const SECRETBOX_BOXZEROBYTES: usize = 16;
pub const SECRETBOX_ZEROBYTES: usize = 32;

pub const PKCS8_DIVIDER: [u8; 5] = [161, 35, 3, 33, 0];
pub const PKCS8_HEADER: [u8; 16] = [48, 83, 2, 1, 1, 48, 5, 6, 3, 43, 101, 112, 4, 34, 4, 32];

pub const SEC_LENGTH: usize = 64;
pub const SEED_LENGTH: usize = 32;

pub fn decode(encoded: &[u8], passphrase: Option<String>) -> Result<(Vec<u8>, Vec<u8>), ()> {
  let encoded_length = encoded.len();

  let mut nonce: SecretboxNonce = [0u8; SECRETBOX_NONCE_LEN];
  nonce.copy_from_slice(&encoded[0..SECRETBOX_NONCE_LEN]);

  let msg = match passphrase {
    Some(passphrase) if !passphrase.is_empty() => {
      let pass_bytes = passphrase.as_bytes();
      let mut key: SecretboxKey = [0u8; SECRETBOX_KEY_LEN];
      key[..pass_bytes.len()].copy_from_slice(pass_bytes);

      let mut encrypted = vec![0u8; SECRETBOX_BOXZEROBYTES + encoded_length - SECRETBOX_NONCE_LEN];
      encrypted[SECRETBOX_BOXZEROBYTES..].copy_from_slice(&encoded[SECRETBOX_NONCE_LEN..]);

      let mut raw = vec![0u8; encrypted.len()];
      secretbox_open(&mut raw, &encrypted, &nonce, &key).map_err(|_| () )?;

      let mut decrypted = vec![0u8; raw.len() - SECRETBOX_ZEROBYTES];
      decrypted.copy_from_slice(&raw[SECRETBOX_ZEROBYTES..]);
      decrypted
    },
    _ => encoded.to_vec(),
  };

  let mut header = [0u8; PKCS8_HEADER.len()];
  header.copy_from_slice(&msg[..PKCS8_HEADER.len()]);

  if header != PKCS8_HEADER {
    return Err(())
  }

  let mut secret_key = [0u8; SEC_LENGTH];
  let start: usize = PKCS8_HEADER.len();
  let end: usize = PKCS8_HEADER.len() + SEC_LENGTH;
  secret_key.copy_from_slice(&msg[start..end]);

  let divider_offset = PKCS8_HEADER.len() + SEC_LENGTH;
  let divider_end = divider_offset + PKCS8_DIVIDER.len();
  let mut divider = [0u8; PKCS8_DIVIDER.len()];
  divider.copy_from_slice(&msg[divider_offset..divider_end]);

  if divider != PKCS8_DIVIDER {
    let mut secret_key = [0u8; SEED_LENGTH];
    let start: usize = PKCS8_HEADER.len();
    let end: usize = PKCS8_HEADER.len() + SEED_LENGTH;
    secret_key.copy_from_slice(&msg[start..end]);

    let divider_offset = PKCS8_HEADER.len() + secret_key.len();
    let divider_end = divider_offset + PKCS8_DIVIDER.len();
    let mut divider = [0u8; PKCS8_DIVIDER.len()];
    divider.copy_from_slice(&msg[divider_offset..divider_end]);

    if divider != PKCS8_DIVIDER {
      return Err(())
    }

    let pub_offset = PKCS8_HEADER.len() + secret_key.len() + PKCS8_DIVIDER.len();
    let mut public_key: Vec<u8> = vec![0u8; msg.len() - pub_offset];
    public_key.copy_from_slice(&msg[pub_offset..]);

    Ok((public_key.to_vec(), secret_key.to_vec()))
  } else {
    let pub_offset = PKCS8_HEADER.len() + secret_key.len() + PKCS8_DIVIDER.len();
    let mut public_key = vec![0u8; msg.len() - pub_offset];
    public_key.copy_from_slice(&msg[pub_offset..]);

    Ok((public_key.to_vec(), secret_key.to_vec()))
  }
}

pub fn encode(secret_key: &[u8], public_key: &[u8], passphrase: Option<String>) -> Result<Vec<u8>, ()> {
  let sec_length: usize = secret_key.len();
  let pub_length: usize = public_key.len();

  let encoded_length: usize = PKCS8_HEADER.len() + sec_length + PKCS8_DIVIDER.len() + pub_length;
  let mut encoded = vec![0u8; encoded_length];

  let end = PKCS8_HEADER.len();
  encoded[..end].copy_from_slice(&PKCS8_HEADER[..]);

  let start = PKCS8_HEADER.len();
  let end = start + sec_length;
  encoded[start..end].copy_from_slice(&secret_key[..]);

  let start = PKCS8_HEADER.len() + sec_length;
  let end = start + PKCS8_DIVIDER.len();
  encoded[start..end].copy_from_slice(&PKCS8_DIVIDER[..]);

  let start = PKCS8_HEADER.len() + sec_length + PKCS8_DIVIDER.len();
  encoded[start..].copy_from_slice(&public_key[..]);


  let passphrase: String = match passphrase {
    Some(v) if !v.is_empty() => v,
    _ => {
      return Ok(encoded)
    },
  };

  let pass_bytes = passphrase.as_bytes();

  let mut key = [0u8; SECRETBOX_KEY_LEN];
  key[..pass_bytes.len()].copy_from_slice(pass_bytes);

  let mut rng = thread_rng();
  let mut nonce = [0u8; SECRETBOX_NONCE_LEN];
  rng.fill(&mut nonce);

  let mut msg = vec![0u8; SECRETBOX_ZEROBYTES + encoded_length];
  msg[SECRETBOX_ZEROBYTES..].copy_from_slice(&encoded[..]);

  let mut encrypted = vec![0u8; msg.len()];
  secretbox(&mut encrypted, &msg, &nonce, &key).map_err(|_| () )?;


  let result_length: usize = encoded_length + SECRETBOX_NONCE_LEN + SECRETBOX_BOXZEROBYTES;
  let mut result = vec![0u8; result_length];

  result[..SECRETBOX_NONCE_LEN].copy_from_slice(&nonce[..]);
  result[SECRETBOX_NONCE_LEN..].copy_from_slice(&encrypted[SECRETBOX_BOXZEROBYTES..]);

  Ok(result)
}