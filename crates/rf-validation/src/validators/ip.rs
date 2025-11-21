//! IP address validation

use std::net::IpAddr;

/// Validate IP address (v4 or v6)
pub fn validate_ip(ip: &str) -> bool {
    ip.parse::<IpAddr>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_ipv4() {
        assert!(validate_ip("192.168.1.1"));
        assert!(validate_ip("127.0.0.1"));
        assert!(validate_ip("255.255.255.255"));
    }

    #[test]
    fn test_valid_ipv6() {
        assert!(validate_ip("::1"));
        assert!(validate_ip("2001:db8::8a2e:370:7334"));
        assert!(validate_ip("::ffff:192.0.2.1"));
    }

    #[test]
    fn test_invalid_ips() {
        assert!(!validate_ip(""));
        assert!(!validate_ip("invalid"));
        assert!(!validate_ip("256.1.1.1"));
        assert!(!validate_ip("192.168.1"));
    }
}
