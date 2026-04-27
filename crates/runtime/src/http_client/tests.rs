use super::*;

// Note: HTTP tests require network access and a running server
// Unit tests here focus on the response building logic

#[test]
fn test_build_response_map_success() {
    let response = build_response_map(200, "Hello".to_string(), true, None);

    match response {
        Value::Map(map_data) => {
            let map = map_data.as_ref();

            // Check status
            let status_key = MapKey::String(global_string("status".to_string()));
            assert!(matches!(map.get(&status_key), Some(Value::Int(200))));

            // Check body
            let body_key = MapKey::String(global_string("body".to_string()));
            if let Some(Value::String(s)) = map.get(&body_key) {
                assert_eq!(s.as_str_or_empty(), "Hello");
            } else {
                panic!("Expected body to be String");
            }

            // Check ok
            let ok_key = MapKey::String(global_string("ok".to_string()));
            assert!(matches!(map.get(&ok_key), Some(Value::Bool(true))));

            // Check no error key
            let error_key = MapKey::String(global_string("error".to_string()));
            assert!(map.get(&error_key).is_none());
        }
        _ => panic!("Expected Map"),
    }
}

#[test]
fn test_build_response_map_error() {
    let response = build_response_map(404, String::new(), false, Some("Not Found".to_string()));

    match response {
        Value::Map(map_data) => {
            let map = map_data.as_ref();

            // Check status
            let status_key = MapKey::String(global_string("status".to_string()));
            assert!(matches!(map.get(&status_key), Some(Value::Int(404))));

            // Check ok is false
            let ok_key = MapKey::String(global_string("ok".to_string()));
            assert!(matches!(map.get(&ok_key), Some(Value::Bool(false))));

            // Check error message
            let error_key = MapKey::String(global_string("error".to_string()));
            if let Some(Value::String(s)) = map.get(&error_key) {
                assert_eq!(s.as_str_or_empty(), "Not Found");
            } else {
                panic!("Expected error to be String");
            }
        }
        _ => panic!("Expected Map"),
    }
}

#[test]
fn test_error_response() {
    let response = error_response("Connection refused".to_string());

    match response {
        Value::Map(map_data) => {
            let map = map_data.as_ref();

            // Check status is 0
            let status_key = MapKey::String(global_string("status".to_string()));
            assert!(matches!(map.get(&status_key), Some(Value::Int(0))));

            // Check ok is false
            let ok_key = MapKey::String(global_string("ok".to_string()));
            assert!(matches!(map.get(&ok_key), Some(Value::Bool(false))));

            // Check error message
            let error_key = MapKey::String(global_string("error".to_string()));
            if let Some(Value::String(s)) = map.get(&error_key) {
                assert_eq!(s.as_str_or_empty(), "Connection refused");
            } else {
                panic!("Expected error to be String");
            }
        }
        _ => panic!("Expected Map"),
    }
}

// SSRF protection tests

#[test]
fn test_ssrf_blocks_localhost() {
    assert!(validate_url_for_ssrf("http://localhost/").is_err());
    assert!(validate_url_for_ssrf("http://localhost:8080/").is_err());
    assert!(validate_url_for_ssrf("http://LOCALHOST/").is_err());
    assert!(validate_url_for_ssrf("http://test.localhost/").is_err());
}

#[test]
fn test_ssrf_blocks_loopback_ip() {
    assert!(validate_url_for_ssrf("http://127.0.0.1/").is_err());
    assert!(validate_url_for_ssrf("http://127.0.0.1:8080/").is_err());
    assert!(validate_url_for_ssrf("http://127.1.2.3/").is_err());
}

#[test]
fn test_ssrf_blocks_private_ranges() {
    // 10.0.0.0/8
    assert!(validate_url_for_ssrf("http://10.0.0.1/").is_err());
    assert!(validate_url_for_ssrf("http://10.255.255.255/").is_err());

    // 172.16.0.0/12
    assert!(validate_url_for_ssrf("http://172.16.0.1/").is_err());
    assert!(validate_url_for_ssrf("http://172.31.255.255/").is_err());

    // 192.168.0.0/16
    assert!(validate_url_for_ssrf("http://192.168.0.1/").is_err());
    assert!(validate_url_for_ssrf("http://192.168.255.255/").is_err());
}

#[test]
fn test_ssrf_blocks_link_local() {
    // Cloud metadata endpoint
    assert!(validate_url_for_ssrf("http://169.254.169.254/").is_err());
    assert!(validate_url_for_ssrf("http://169.254.0.1/").is_err());
}

#[test]
fn test_ssrf_blocks_invalid_schemes() {
    assert!(validate_url_for_ssrf("file:///etc/passwd").is_err());
    assert!(validate_url_for_ssrf("ftp://example.com/").is_err());
    assert!(validate_url_for_ssrf("gopher://example.com/").is_err());
}

#[test]
fn test_ssrf_allows_public_urls() {
    // These should be allowed (public IPs)
    assert!(validate_url_for_ssrf("https://example.com/").is_ok());
    assert!(validate_url_for_ssrf("https://httpbin.org/get").is_ok());
    assert!(validate_url_for_ssrf("http://8.8.8.8/").is_ok());
}

#[test]
fn test_dangerous_ipv4() {
    use std::net::Ipv4Addr;

    // Loopback
    assert!(is_dangerous_ipv4(Ipv4Addr::new(127, 0, 0, 1)));
    assert!(is_dangerous_ipv4(Ipv4Addr::new(127, 1, 2, 3)));

    // Private 10.x.x.x
    assert!(is_dangerous_ipv4(Ipv4Addr::new(10, 0, 0, 1)));
    assert!(is_dangerous_ipv4(Ipv4Addr::new(10, 255, 255, 255)));

    // Private 172.16-31.x.x
    assert!(is_dangerous_ipv4(Ipv4Addr::new(172, 16, 0, 1)));
    assert!(is_dangerous_ipv4(Ipv4Addr::new(172, 31, 255, 255)));
    assert!(!is_dangerous_ipv4(Ipv4Addr::new(172, 15, 0, 1))); // Not private
    assert!(!is_dangerous_ipv4(Ipv4Addr::new(172, 32, 0, 1))); // Not private

    // Private 192.168.x.x
    assert!(is_dangerous_ipv4(Ipv4Addr::new(192, 168, 0, 1)));
    assert!(is_dangerous_ipv4(Ipv4Addr::new(192, 168, 255, 255)));

    // Link-local (cloud metadata)
    assert!(is_dangerous_ipv4(Ipv4Addr::new(169, 254, 169, 254)));

    // Public IPs - should NOT be dangerous
    assert!(!is_dangerous_ipv4(Ipv4Addr::new(8, 8, 8, 8)));
    assert!(!is_dangerous_ipv4(Ipv4Addr::new(1, 1, 1, 1)));
    assert!(!is_dangerous_ipv4(Ipv4Addr::new(93, 184, 216, 34)));
}

#[test]
fn test_dangerous_ipv6() {
    use std::net::Ipv6Addr;

    // Loopback
    assert!(is_dangerous_ipv6(Ipv6Addr::LOCALHOST));

    // Link-local fe80::/10
    assert!(is_dangerous_ipv6(Ipv6Addr::new(
        0xfe80, 0, 0, 0, 0, 0, 0, 1
    )));

    // Unique local fc00::/7
    assert!(is_dangerous_ipv6(Ipv6Addr::new(
        0xfc00, 0, 0, 0, 0, 0, 0, 1
    )));
    assert!(is_dangerous_ipv6(Ipv6Addr::new(
        0xfd00, 0, 0, 0, 0, 0, 0, 1
    )));

    // Public - should NOT be dangerous
    assert!(!is_dangerous_ipv6(Ipv6Addr::new(
        0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888
    ))); // Google DNS
}
