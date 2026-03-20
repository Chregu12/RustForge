//! Deployment tests for rf-encryption

#[cfg(test)]
mod tests {
    use rf_encryption::{Encryptor, Encrypted};

    // ── Key Generation ───────────────────────────────────────────

    #[test]
    fn generate_key() {
        let key = Encryptor::generate_key();
        assert!(!key.is_empty());
        // Keys should be unique
        let key2 = Encryptor::generate_key();
        assert_ne!(key, key2);
    }

    // ── Encrypt & Decrypt ────────────────────────────────────────

    #[test]
    fn encrypt_decrypt_string() {
        let key = Encryptor::generate_key();
        let encryptor = Encryptor::new().key(&key).build().expect("build");

        let plaintext = "Hello, RustForge!";
        let encrypted = encryptor.encrypt(plaintext).expect("encrypt");
        assert_ne!(encrypted, plaintext);

        let decrypted = encryptor.decrypt(&encrypted).expect("decrypt");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn encrypt_decrypt_bytes() {
        let key = Encryptor::generate_key();
        let encryptor = Encryptor::new().key(&key).build().expect("build");

        let data = b"binary data \x00\x01\x02";
        let encrypted = encryptor.encrypt_bytes(data).expect("encrypt");
        let decrypted = encryptor.decrypt_bytes(&encrypted).expect("decrypt");
        assert_eq!(decrypted, data);
    }

    #[test]
    fn different_keys_cannot_decrypt() {
        let key1 = Encryptor::generate_key();
        let key2 = Encryptor::generate_key();
        let enc1 = Encryptor::new().key(&key1).build().expect("build");
        let enc2 = Encryptor::new().key(&key2).build().expect("build");

        let encrypted = enc1.encrypt("secret").expect("encrypt");
        assert!(enc2.decrypt(&encrypted).is_err());
    }

    #[test]
    fn encrypt_produces_different_ciphertext() {
        let key = Encryptor::generate_key();
        let encryptor = Encryptor::new().key(&key).build().expect("build");

        let ct1 = encryptor.encrypt("same text").expect("encrypt");
        let ct2 = encryptor.encrypt("same text").expect("encrypt");
        // Due to random nonces, ciphertext should differ
        assert_ne!(ct1, ct2);
        // But both should decrypt to the same value
        assert_eq!(encryptor.decrypt(&ct1).expect("d1"), "same text");
        assert_eq!(encryptor.decrypt(&ct2).expect("d2"), "same text");
    }

    #[test]
    fn invalid_ciphertext_fails() {
        let key = Encryptor::generate_key();
        let encryptor = Encryptor::new().key(&key).build().expect("build");
        assert!(encryptor.decrypt("not-valid-ciphertext").is_err());
    }

    // ── Encrypted<T> Wrapper ─────────────────────────────────────

    #[test]
    fn encrypted_wrapper() {
        let key = Encryptor::generate_key();
        let encryptor = Encryptor::new().key(&key).build().expect("build");

        let mut enc = Encrypted::new("sensitive data".to_string());
        assert_eq!(enc.value(), "sensitive data");

        let ciphertext = enc.encrypt(&encryptor).expect("encrypt");
        assert!(!ciphertext.is_empty());

        let decrypted = Encrypted::<String>::decrypt(&ciphertext, &encryptor).expect("decrypt");
        assert_eq!(decrypted.value(), "sensitive data");
    }

    // ── Empty & Edge Cases ───────────────────────────────────────

    #[test]
    fn encrypt_empty_string() {
        let key = Encryptor::generate_key();
        let encryptor = Encryptor::new().key(&key).build().expect("build");

        let encrypted = encryptor.encrypt("").expect("encrypt");
        let decrypted = encryptor.decrypt(&encrypted).expect("decrypt");
        assert_eq!(decrypted, "");
    }

    #[test]
    fn encrypt_large_data() {
        let key = Encryptor::generate_key();
        let encryptor = Encryptor::new().key(&key).build().expect("build");

        let large = "A".repeat(100_000);
        let encrypted = encryptor.encrypt(&large).expect("encrypt");
        let decrypted = encryptor.decrypt(&encrypted).expect("decrypt");
        assert_eq!(decrypted, large);
    }
}
