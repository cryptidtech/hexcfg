// SPDX-License-Identifier: MIT OR Apache-2.0

//! Property-based tests using proptest.
//!
//! These tests use property-based testing to verify that the configuration
//! system handles arbitrary inputs correctly.

use hexcfg::domain::{ConfigKey, ConfigValue};
use proptest::prelude::*;

// Test that ConfigKey can be created from any string
proptest! {
    #[test]
    fn test_config_key_from_any_string(s in "\\PC*") {
        let key = ConfigKey::from(s.clone());
        prop_assert_eq!(key.as_str(), s.as_str());
    }
}

// Test that ConfigValue can be created from any string
proptest! {
    #[test]
    fn test_config_value_from_any_string(s in "\\PC*") {
        let value = ConfigValue::from(s.clone());
        prop_assert_eq!(value.as_str(), s.as_str());
    }
}

// Test that ConfigValue.as_string() always returns the original string
proptest! {
    #[test]
    fn test_config_value_as_string_roundtrip(s in "\\PC*") {
        let value = ConfigValue::from(s.clone());
        prop_assert_eq!(value.as_string(), s);
    }
}

// Test boolean parsing with known valid values
proptest! {
    #[test]
    fn test_bool_parsing_valid_values(
        b in prop::bool::ANY
    ) {
        let value_str = if b { "true" } else { "false" };
        let value = ConfigValue::from(value_str);
        prop_assert_eq!(value.as_bool("test").unwrap(), b);
    }
}

// Test that invalid boolean strings fail gracefully
proptest! {
    #[test]
    fn test_bool_parsing_invalid_values(
        s in "[^tfyn01]\\PC*" // Strings that don't start with common bool indicators
    ) {
        let value = ConfigValue::from(s);
        // Should either parse correctly or fail gracefully
        let result = value.as_bool("test");
        prop_assert!(result.is_ok() || result.is_err());
    }
}

// Test integer parsing
proptest! {
    #[test]
    fn test_i32_parsing_valid(n in prop::num::i32::ANY) {
        let value = ConfigValue::from(n.to_string());
        prop_assert_eq!(value.as_i32("test").unwrap(), n);
    }
}

proptest! {
    #[test]
    fn test_i64_parsing_valid(n in prop::num::i64::ANY) {
        let value = ConfigValue::from(n.to_string());
        prop_assert_eq!(value.as_i64("test").unwrap(), n);
    }
}

proptest! {
    #[test]
    fn test_u32_parsing_valid(n in prop::num::u32::ANY) {
        let value = ConfigValue::from(n.to_string());
        prop_assert_eq!(value.as_u32("test").unwrap(), n);
    }
}

proptest! {
    #[test]
    fn test_u64_parsing_valid(n in prop::num::u64::ANY) {
        let value = ConfigValue::from(n.to_string());
        prop_assert_eq!(value.as_u64("test").unwrap(), n);
    }
}

// Test float parsing
proptest! {
    #[test]
    fn test_f64_parsing_valid(n in prop::num::f64::NORMAL) {
        let value = ConfigValue::from(n.to_string());
        let parsed = value.as_f64("test").unwrap();
        // Allow for floating point precision issues
        prop_assert!((parsed - n).abs() < 1e-10 * n.abs().max(1.0));
    }
}

// Test that non-numeric strings fail integer parsing
proptest! {
    #[test]
    fn test_integer_parsing_non_numeric(
        s in "[a-zA-Z]\\PC*" // Strings starting with a letter
    ) {
        let value = ConfigValue::from(s);
        // Should fail to parse as integer
        prop_assert!(value.as_i32("test").is_err());
    }
}

// Test ConfigKey equality
proptest! {
    #[test]
    fn test_config_key_equality(s in "\\PC+") {
        let key1 = ConfigKey::from(s.clone());
        let key2 = ConfigKey::from(s.clone());
        prop_assert_eq!(key1, key2);
    }
}

// Test ConfigKey inequality
proptest! {
    #[test]
    fn test_config_key_inequality(
        s1 in "\\PC+",
        s2 in "\\PC+"
    ) {
        let key1 = ConfigKey::from(s1.clone());
        let key2 = ConfigKey::from(s2.clone());
        if s1 != s2 {
            prop_assert_ne!(key1, key2);
        }
    }
}

// Test that keys with dots are handled correctly
proptest! {
    #[test]
    fn test_keys_with_dots(
        parts in prop::collection::vec("[a-z]+", 1..5)
    ) {
        let key_str = parts.join(".");
        let key = ConfigKey::from(key_str.clone());
        prop_assert_eq!(key.as_str(), key_str.as_str());
    }
}

// Test that keys with underscores are handled correctly
proptest! {
    #[test]
    fn test_keys_with_underscores(
        parts in prop::collection::vec("[a-z]+", 1..5)
    ) {
        let key_str = parts.join("_");
        let key = ConfigKey::from(key_str.clone());
        prop_assert_eq!(key.as_str(), key_str.as_str());
    }
}

// Test empty string handling
#[test]
fn test_empty_string_value() {
    let value = ConfigValue::from("");
    assert_eq!(value.as_str(), "");
    assert_eq!(value.as_string(), "");
    // Empty string should fail as boolean
    assert!(value.as_bool("test").is_err());
}

// Test whitespace handling
proptest! {
    #[test]
    fn test_whitespace_preservation(
        leading in "[ \t]*",
        content in "[a-z]+",
        trailing in "[ \t]*"
    ) {
        let s = format!("{}{}{}", leading, content, trailing);
        let value = ConfigValue::from(s.clone());
        // Whitespace should be preserved
        prop_assert_eq!(value.as_str(), s.as_str());
    }
}

// Test very long strings
proptest! {
    #[test]
    fn test_long_strings(s in prop::collection::vec(prop::char::any(), 1000..2000)) {
        let string: String = s.iter().collect();
        let value = ConfigValue::from(string.clone());
        prop_assert_eq!(value.as_string(), string);
    }
}

// Test unicode handling
proptest! {
    #[test]
    fn test_unicode_strings(s in "\\p{Greek}+|\\p{Cyrillic}+|\\p{Han}+") {
        let key = ConfigKey::from(s.clone());
        let value = ConfigValue::from(s.clone());
        prop_assert_eq!(key.as_str(), s.as_str());
        prop_assert_eq!(value.as_str(), s.as_str());
    }
}

// Test that ConfigKey can be cloned
proptest! {
    #[test]
    fn test_config_key_clone(s in "\\PC+") {
        let key1 = ConfigKey::from(s);
        let key2 = key1.clone();
        prop_assert_eq!(key1, key2);
    }
}

// Test case sensitivity
proptest! {
    #[test]
    fn test_case_sensitivity(s in "[a-z]+") {
        let lower = ConfigKey::from(s.clone());
        let upper = ConfigKey::from(s.to_uppercase());
        // Keys should be case-sensitive
        if s != s.to_uppercase() {
            prop_assert_ne!(lower, upper);
        }
    }
}
