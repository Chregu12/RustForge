//! Cycle-8 tests for `rf_eloquent::casting`.
//!
//! Covers all cast types, round-trips, every error path, edge-case inputs
//! (empty string, negative numbers, boundary values) and custom casters.
//! Every test must be capable of failing; trivially-passing stubs are rejected.

use rf_eloquent::casting::{
    cast_value, register_caster, set_encryption_key, uncast_value, Castable, CastedValue,
    CastError, CastRegistry, CastType,
};

// ── helper ────────────────────────────────────────────────────────────────────

fn is_cast_err(e: &CastError) -> bool {
    matches!(e, CastError::CastFailed(_))
}

// ── CastType::String ─────────────────────────────────────────────────────────

#[test]
fn test_cast_string_round_trip() {
    let v = cast_value("hello world", CastType::String).unwrap();
    assert_eq!(v.as_string().unwrap(), "hello world");
    let back = uncast_value(v, CastType::String).unwrap();
    assert_eq!(back, "hello world");
}

#[test]
fn test_cast_string_empty() {
    let v = cast_value("", CastType::String).unwrap();
    assert_eq!(v.as_string().unwrap(), "");
}

// ── CastType::Integer ─────────────────────────────────────────────────────────

#[test]
fn test_cast_integer_positive() {
    let v = cast_value("42", CastType::Integer).unwrap();
    assert_eq!(v.as_i64().unwrap(), 42);
}

#[test]
fn test_cast_integer_zero() {
    let v = cast_value("0", CastType::Integer).unwrap();
    assert_eq!(v.as_i64().unwrap(), 0);
}

#[test]
fn test_cast_integer_negative() {
    let v = cast_value("-99", CastType::Integer).unwrap();
    assert_eq!(v.as_i64().unwrap(), -99);
}

#[test]
fn test_cast_integer_max() {
    let v = cast_value(&i64::MAX.to_string(), CastType::Integer).unwrap();
    assert_eq!(v.as_i64().unwrap(), i64::MAX);
}

#[test]
fn test_cast_integer_round_trip() {
    let v = cast_value("12345", CastType::Integer).unwrap();
    let back = uncast_value(v, CastType::Integer).unwrap();
    assert_eq!(back, "12345");
}

#[test]
fn test_cast_integer_invalid_returns_err() {
    let e = cast_value("not_a_number", CastType::Integer).unwrap_err();
    assert!(
        is_cast_err(&e),
        "non-numeric string should give CastFailed, got {e:?}"
    );
}

#[test]
fn test_cast_integer_float_string_returns_err() {
    // "3.14" is not a valid integer even though it is a valid float
    let e = cast_value("3.14", CastType::Integer).unwrap_err();
    assert!(is_cast_err(&e), "float string should fail integer cast: {e:?}");
}

#[test]
fn test_cast_integer_empty_returns_err() {
    assert!(cast_value("", CastType::Integer).is_err());
}

// ── CastType::Float ───────────────────────────────────────────────────────────

#[test]
fn test_cast_float_positive() {
    let v = cast_value("3.14", CastType::Float).unwrap();
    assert!((v.as_f64().unwrap() - 3.14).abs() < 1e-9);
}

#[test]
fn test_cast_float_zero() {
    let v = cast_value("0.0", CastType::Float).unwrap();
    assert_eq!(v.as_f64().unwrap(), 0.0);
}

#[test]
fn test_cast_float_negative() {
    let v = cast_value("-2.71828", CastType::Float).unwrap();
    assert!((v.as_f64().unwrap() - (-2.71828)).abs() < 1e-9);
}

#[test]
fn test_cast_float_whole_number_string() {
    // "42" should parse as f64
    let v = cast_value("42", CastType::Float).unwrap();
    assert!((v.as_f64().unwrap() - 42.0).abs() < 1e-9);
}

#[test]
fn test_cast_float_round_trip() {
    let v = cast_value("1.5", CastType::Float).unwrap();
    let back = uncast_value(v, CastType::Float).unwrap();
    // uncast serialises with f64::to_string which may differ from "1.5",
    // but the parsed value must be equivalent.
    let reparsed: f64 = back.parse().unwrap();
    assert!((reparsed - 1.5).abs() < 1e-9);
}

#[test]
fn test_cast_float_invalid_returns_err() {
    assert!(cast_value("nan_text", CastType::Float).is_err());
}

#[test]
fn test_cast_float_empty_returns_err() {
    assert!(cast_value("", CastType::Float).is_err());
}

// ── CastType::Boolean ─────────────────────────────────────────────────────────

#[test]
fn test_cast_bool_true_variants() {
    for s in &["true", "1", "yes", "on"] {
        let v = cast_value(s, CastType::Boolean)
            .unwrap_or_else(|_| panic!("'{s}' should cast to bool"));
        assert!(
            v.as_bool().unwrap(),
            "'{s}' should map to true"
        );
    }
}

#[test]
fn test_cast_bool_false_variants() {
    for s in &["false", "0", "no", "off"] {
        let v = cast_value(s, CastType::Boolean)
            .unwrap_or_else(|_| panic!("'{s}' should cast to bool"));
        assert!(
            !v.as_bool().unwrap(),
            "'{s}' should map to false"
        );
    }
}

#[test]
fn test_cast_bool_case_insensitive() {
    // The implementation lower-cases before matching.
    let v = cast_value("TRUE", CastType::Boolean).unwrap();
    assert!(v.as_bool().unwrap());
    let v = cast_value("FALSE", CastType::Boolean).unwrap();
    assert!(!v.as_bool().unwrap());
}

#[test]
fn test_cast_bool_invalid_returns_err() {
    let e = cast_value("maybe", CastType::Boolean).unwrap_err();
    assert!(is_cast_err(&e), "unexpected error type: {e:?}");
}

#[test]
fn test_cast_bool_round_trip() {
    let v = cast_value("true", CastType::Boolean).unwrap();
    let back = uncast_value(v, CastType::Boolean).unwrap();
    assert_eq!(back, "true");

    let v = cast_value("false", CastType::Boolean).unwrap();
    let back = uncast_value(v, CastType::Boolean).unwrap();
    assert_eq!(back, "false");
}

// ── CastType::Json ────────────────────────────────────────────────────────────

#[test]
fn test_cast_json_object() {
    let raw = r#"{"name":"Alice","age":30}"#;
    let v = cast_value(raw, CastType::Json).unwrap();
    if let CastedValue::Json(j) = &v {
        assert_eq!(j["name"], "Alice");
        assert_eq!(j["age"], 30);
    } else {
        panic!("expected Json variant");
    }
    // round-trip: serialised JSON should parse back to same structure
    let back = uncast_value(v, CastType::Json).unwrap();
    let reparsed: serde_json::Value = serde_json::from_str(&back).unwrap();
    assert_eq!(reparsed["name"], "Alice");
}

#[test]
fn test_cast_json_array() {
    let raw = r#"[1, 2, 3]"#;
    let v = cast_value(raw, CastType::Json).unwrap();
    if let CastedValue::Json(serde_json::Value::Array(arr)) = v {
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0], 1);
    } else {
        panic!("expected Json(Array) variant");
    }
}

#[test]
fn test_cast_json_invalid_returns_err() {
    let e = cast_value("{not-valid-json}", CastType::Json).unwrap_err();
    assert!(
        matches!(e, CastError::InvalidJson(_)),
        "bad JSON should produce InvalidJson, got {e:?}"
    );
}

#[test]
fn test_cast_json_empty_string_returns_err() {
    assert!(cast_value("", CastType::Json).is_err());
}

// ── CastType::DateTime ────────────────────────────────────────────────────────

#[test]
fn test_cast_datetime_rfc3339() {
    let raw = "2024-06-15T12:30:00Z";
    let v = cast_value(raw, CastType::DateTime).unwrap();
    if let CastedValue::DateTime(dt) = v {
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2024-06-15");
        assert_eq!(dt.format("%H:%M").to_string(), "12:30");
    } else {
        panic!("expected DateTime variant");
    }
}

#[test]
fn test_cast_datetime_naive_sql_format() {
    // Second supported format: "%Y-%m-%d %H:%M:%S" (common SQL timestamp)
    let raw = "2024-01-01 08:00:00";
    let v = cast_value(raw, CastType::DateTime).unwrap();
    if let CastedValue::DateTime(dt) = v {
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2024-01-01");
    } else {
        panic!("expected DateTime variant");
    }
}

#[test]
fn test_cast_datetime_invalid_returns_err() {
    let e = cast_value("not-a-date", CastType::DateTime).unwrap_err();
    assert!(
        matches!(e, CastError::InvalidDate(_)),
        "bad date string should produce InvalidDate, got {e:?}"
    );
}

#[test]
fn test_cast_datetime_round_trip() {
    let raw = "2025-03-20T09:15:00Z";
    let v = cast_value(raw, CastType::DateTime).unwrap();
    let back = uncast_value(v, CastType::DateTime).unwrap();
    // The uncast re-serialises with rfc3339; must parse back to the same instant.
    let v2 = cast_value(&back, CastType::DateTime).unwrap();
    if let (CastedValue::DateTime(_), CastedValue::DateTime(dt2)) =
        (cast_value(raw, CastType::DateTime).unwrap(), v2)
    {
        assert_eq!(dt2.format("%Y-%m-%dT%H:%M").to_string(), "2025-03-20T09:15");
    }
}

// ── CastType::Date ────────────────────────────────────────────────────────────

#[test]
fn test_cast_date_produces_midnight_datetime() {
    let v = cast_value("2024-12-25", CastType::Date).unwrap();
    if let CastedValue::DateTime(dt) = v {
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2024-12-25");
        assert_eq!(dt.format("%H:%M:%S").to_string(), "00:00:00");
    } else {
        panic!("expected DateTime variant for Date cast");
    }
}

#[test]
fn test_cast_date_invalid_returns_err() {
    assert!(cast_value("25/12/2024", CastType::Date).is_err());
}

#[test]
fn test_uncast_date_produces_date_only_string() {
    let v = cast_value("2024-07-04", CastType::Date).unwrap();
    let back = uncast_value(v, CastType::Date).unwrap();
    assert_eq!(back, "2024-07-04");
}

// ── CastType::Array ───────────────────────────────────────────────────────────

#[test]
fn test_cast_array_round_trip() {
    let raw = r#"[1,2,3]"#;
    let v = cast_value(raw, CastType::Array).unwrap();
    let back = uncast_value(v, CastType::Array).unwrap();
    let reparsed: Vec<serde_json::Value> = serde_json::from_str(&back).unwrap();
    assert_eq!(reparsed.len(), 3);
    assert_eq!(reparsed[0], 1);
}

#[test]
fn test_cast_array_not_an_array_returns_err() {
    // A JSON object is not an array → the Array cast should reject it.
    assert!(cast_value(r#"{"key":"value"}"#, CastType::Array).is_err());
}

// ── CastType::Null (CastedValue::Null → empty string on uncast) ──────────────

#[test]
fn test_uncast_null_returns_empty_string() {
    // Any cast type accepts CastedValue::Null and serialises to "".
    let back = uncast_value(CastedValue::Null, CastType::Integer).unwrap();
    assert_eq!(back, "", "null must serialise to empty string");
}

// ── TypeMismatch on uncast_value ─────────────────────────────────────────────

#[test]
fn test_uncast_type_mismatch_returns_err() {
    // Passing an Integer variant when the cast type is Json should fail.
    let e = uncast_value(CastedValue::Integer(1), CastType::Json).unwrap_err();
    assert!(
        matches!(e, CastError::TypeMismatch { .. }),
        "wrong variant should give TypeMismatch, got {e:?}"
    );
}

#[test]
fn test_uncast_string_as_float_returns_err() {
    let e = uncast_value(CastedValue::String("hello".into()), CastType::Float).unwrap_err();
    assert!(matches!(e, CastError::TypeMismatch { .. }));
}

// ── CastRegistry ─────────────────────────────────────────────────────────────

#[test]
fn test_registry_has_get_remove() {
    let mut reg = CastRegistry::new()
        .cast("name", CastType::String)
        .cast("age", CastType::Integer)
        .cast("meta", CastType::Json);

    assert!(reg.has("name"));
    assert_eq!(*reg.get("name").unwrap(), CastType::String);
    assert!(reg.get("missing").is_none());

    let removed = reg.remove("age");
    assert_eq!(removed, Some(CastType::Integer));
    assert!(!reg.has("age"));
}

#[test]
fn test_registry_all_lists_all_casts() {
    let reg = CastRegistry::new()
        .cast("a", CastType::String)
        .cast("b", CastType::Boolean);
    assert_eq!(reg.all().len(), 2);
}

// ── Custom caster ─────────────────────────────────────────────────────────────

struct UpperCaseCaster;

impl Castable for UpperCaseCaster {
    fn get(&self, value: &str) -> rf_eloquent::casting::CastResult<CastedValue> {
        Ok(CastedValue::String(value.to_uppercase()))
    }
    fn set(&self, value: CastedValue) -> rf_eloquent::casting::CastResult<String> {
        match value {
            CastedValue::String(s) => Ok(s.to_uppercase()),
            _ => Ok(String::new()),
        }
    }
}

#[test]
fn test_custom_caster_get_and_set() {
    register_caster("upper", UpperCaseCaster);

    let v = cast_value("hello", CastType::Custom("upper")).unwrap();
    assert_eq!(v.as_string().unwrap(), "HELLO");

    let back = uncast_value(v, CastType::Custom("upper")).unwrap();
    assert_eq!(back, "HELLO");
}

#[test]
fn test_custom_caster_not_found_returns_err() {
    let e = cast_value("x", CastType::Custom("__no_such_caster__")).unwrap_err();
    assert!(
        is_cast_err(&e),
        "missing caster should return CastFailed: {e:?}"
    );
}

// ── Encrypted cast (AES-GCM round-trip) ───────────────────────────────────────

// `set_encryption_key` mutates a PROCESS-GLOBAL key. These tests each install a
// different random key and then encrypt+decrypt, so run in parallel they race
// (test A encrypts with key1, test B swaps in key2, test A decrypts with key2 →
// panic). Serialize them on a shared, poison-tolerant lock held for the whole test.
static ENC_KEY_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn test_encrypted_round_trip_with_explicit_key() {
    let _guard = ENC_KEY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    set_encryption_key(rf_encryption::Encryptor::generate_key());

    let plaintext = "sensitive-data-12345";
    let ciphertext = uncast_value(
        CastedValue::String(plaintext.to_string()),
        CastType::Encrypted,
    )
    .expect("encryption must not fail");

    // The ciphertext must not contain the plaintext.
    assert!(!ciphertext.contains(plaintext));

    // Decryption must recover the original plaintext.
    let recovered = cast_value(&ciphertext, CastType::Encrypted).unwrap();
    assert_eq!(recovered.as_string().unwrap(), plaintext);
}

#[test]
fn test_encrypted_two_encryptions_differ() {
    let _guard = ENC_KEY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    set_encryption_key(rf_encryption::Encryptor::generate_key());

    let ct1 = uncast_value(
        CastedValue::String("hello".to_string()),
        CastType::Encrypted,
    )
    .unwrap();
    let ct2 = uncast_value(
        CastedValue::String("hello".to_string()),
        CastType::Encrypted,
    )
    .unwrap();
    // AES-GCM uses a random nonce → ciphertexts must differ.
    assert_ne!(ct1, ct2, "AES-GCM nonce must be randomised per call");
}
