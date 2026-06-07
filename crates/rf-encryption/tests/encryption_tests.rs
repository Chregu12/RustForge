//! Integration tests for rf-encryption

use rf_encryption::{Cipher, EncryptionError, Encryptor};

// ── Key generation ────────────────────────────────────────────────────────────

#[test]
fn generate_key_returns_base64_prefixed_string() {
    let key = Encryptor::generate_key();
    assert!(key.starts_with("base64:"), "key must start with 'base64:'");
}

#[test]
fn generate_key_produces_unique_values() {
    let k1 = Encryptor::generate_key();
    let k2 = Encryptor::generate_key();
    assert_ne!(k1, k2);
}

#[test]
fn generate_key_can_be_used_to_build_encryptor() {
    let key = Encryptor::generate_key();
    assert!(Encryptor::new().key(&key).build().is_ok());
}

// ── Build errors ──────────────────────────────────────────────────────────────

#[test]
fn build_without_key_returns_error() {
    match Encryptor::new().build() {
        Err(EncryptionError::InvalidKey(_)) => {} // expected
        Err(other) => panic!("expected InvalidKey, got: {:?}", other),
        Ok(_) => panic!("expected error, got Ok"),
    }
}

#[test]
fn build_with_invalid_base64_key_returns_error() {
    let result = Encryptor::new().key("not-valid-base64!!!!!").build();
    // Might produce InvalidKey or Base64Error — either is correct
    assert!(result.is_err());
}

// ── Encrypt / Decrypt strings ─────────────────────────────────────────────────

#[test]
fn encrypt_and_decrypt_round_trips_plain_string() {
    let key = Encryptor::generate_key();
    let enc = Encryptor::new().key(&key).build().unwrap();

    let plain = "Hello, RustForge!";
    let cipher = enc.encrypt(plain).unwrap();
    let recovered = enc.decrypt(&cipher).unwrap();

    assert_eq!(plain, recovered);
}

#[test]
fn ciphertext_differs_from_plaintext() {
    let key = Encryptor::generate_key();
    let enc = Encryptor::new().key(&key).build().unwrap();

    let plain = "secret-data";
    let cipher = enc.encrypt(plain).unwrap();
    assert_ne!(plain, cipher);
}

#[test]
fn same_plaintext_produces_different_ciphertext_each_call() {
    let key = Encryptor::generate_key();
    let enc = Encryptor::new().key(&key).build().unwrap();

    let c1 = enc.encrypt("same-input").unwrap();
    let c2 = enc.encrypt("same-input").unwrap();
    // Different nonces → different ciphertexts
    assert_ne!(c1, c2);
    // But both decrypt to the same plaintext
    assert_eq!(enc.decrypt(&c1).unwrap(), "same-input");
    assert_eq!(enc.decrypt(&c2).unwrap(), "same-input");
}

#[test]
fn decrypt_with_wrong_key_returns_error() {
    let key1 = Encryptor::generate_key();
    let key2 = Encryptor::generate_key();
    let enc1 = Encryptor::new().key(&key1).build().unwrap();
    let enc2 = Encryptor::new().key(&key2).build().unwrap();

    let cipher = enc1.encrypt("secret").unwrap();
    let result = enc2.decrypt(&cipher);
    assert!(result.is_err());
}

#[test]
fn decrypt_garbage_returns_error() {
    let key = Encryptor::generate_key();
    let enc = Encryptor::new().key(&key).build().unwrap();
    assert!(enc.decrypt("not-valid-ciphertext").is_err());
}

#[test]
fn decrypt_too_short_payload_returns_error() {
    let key = Encryptor::generate_key();
    let enc = Encryptor::new().key(&key).build().unwrap();
    // "dGVzdA==" is base64 for "test" (4 bytes) — shorter than the 12-byte nonce
    assert!(enc.decrypt("dGVzdA==").is_err());
}

// ── Encrypt / Decrypt bytes ───────────────────────────────────────────────────

#[test]
fn encrypt_bytes_and_decrypt_bytes_round_trip() {
    let key = Encryptor::generate_key();
    let enc = Encryptor::new().key(&key).build().unwrap();

    let data = b"binary\x00data\xFF";
    let cipher = enc.encrypt_bytes(data).unwrap();
    let recovered = enc.decrypt_bytes(&cipher).unwrap();

    assert_eq!(data.as_slice(), recovered.as_slice());
}

// ── JSON encryption ───────────────────────────────────────────────────────────

#[test]
fn encrypt_json_string_and_decrypt_round_trips() {
    let key = Encryptor::generate_key();
    let enc = Encryptor::new().key(&key).build().unwrap();

    let json = r#"{"user_id":42,"role":"admin","data":[1,2,3]}"#;
    let cipher = enc.encrypt(json).unwrap();
    let recovered = enc.decrypt(&cipher).unwrap();

    assert_eq!(json, recovered);
}

// ── Unicode ───────────────────────────────────────────────────────────────────

#[test]
fn encrypt_unicode_string_round_trips() {
    let key = Encryptor::generate_key();
    let enc = Encryptor::new().key(&key).build().unwrap();

    let unicode = "Héllo Wörld 🔐 日本語";
    let cipher = enc.encrypt(unicode).unwrap();
    let recovered = enc.decrypt(&cipher).unwrap();

    assert_eq!(unicode, recovered);
}

// ── Empty string ──────────────────────────────────────────────────────────────

#[test]
fn encrypt_empty_string_round_trips() {
    let key = Encryptor::generate_key();
    let enc = Encryptor::new().key(&key).build().unwrap();

    let cipher = enc.encrypt("").unwrap();
    let recovered = enc.decrypt(&cipher).unwrap();
    assert_eq!("", recovered);
}

// ── Cipher explicit selection ─────────────────────────────────────────────────

#[test]
fn explicit_aes256gcm_cipher_works() {
    let key = Encryptor::generate_key();
    let enc = Encryptor::new()
        .key(&key)
        .cipher(Cipher::Aes256Gcm)
        .build()
        .unwrap();

    let plain = "aes256gcm-test";
    assert_eq!(enc.decrypt(&enc.encrypt(plain).unwrap()).unwrap(), plain);
}

// ── Builder encrypt/decrypt convenience methods ───────────────────────────────

#[test]
fn builder_encrypt_and_decrypt_convenience_methods_work() {
    let key = Encryptor::generate_key();
    let builder = Encryptor::new().key(&key);

    let plain = "via-builder";
    let cipher = builder.encrypt(plain).unwrap();
    let recovered = builder.decrypt(&cipher).unwrap();
    assert_eq!(plain, recovered);
}
