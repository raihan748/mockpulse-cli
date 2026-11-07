//! Unit tests for Radix Trie routing, path parameter extraction, and wildcards.

use mockpulse_cli::router::{HttpMethod, RadixTrieRouter, RouteEndpoint};

fn make_endpoint(method: HttpMethod, path: &str) -> RouteEndpoint {
    RouteEndpoint {
        method,
        path_pattern: path.to_string(),
        summary: None,
        operation_id: None,
        default_status: 200,
        response_schema: None,
        example_response: None,
        content_type: "application/json".to_string(),
    }
}

#[test]
fn test_exact_route_matching() {
    let mut router = RadixTrieRouter::new();
    router.insert(make_endpoint(HttpMethod::Get, "/api/v1/health"));
    router.insert(make_endpoint(HttpMethod::Get, "/api/v1/users"));

    let res = router.lookup(&HttpMethod::Get, "/api/v1/health");
    assert!(res.is_some());
    assert_eq!(res.unwrap().endpoint.path_pattern, "/api/v1/health");

    let res_users = router.lookup(&HttpMethod::Get, "/api/v1/users");
    assert!(res_users.is_some());
    assert_eq!(res_users.unwrap().endpoint.path_pattern, "/api/v1/users");

    let not_found = router.lookup(&HttpMethod::Get, "/api/v1/nonexistent");
    assert!(not_found.is_none());
}

#[test]
fn test_single_path_parameter_extraction() {
    let mut router = RadixTrieRouter::new();
    router.insert(make_endpoint(HttpMethod::Get, "/users/{userId}"));

    let m = router.lookup(&HttpMethod::Get, "/users/user_984").expect("Route should match");
    assert_eq!(m.endpoint.path_pattern, "/users/{userId}");
    assert_eq!(m.params.get("userId"), Some(&"user_984".to_string()));
}

#[test]
fn test_multiple_path_parameters() {
    let mut router = RadixTrieRouter::new();
    router.insert(make_endpoint(
        HttpMethod::Get,
        "/orgs/{orgId}/repos/{repoId}/issues/{issueNum}",
    ));

    let m = router
        .lookup(&HttpMethod::Get, "/orgs/smk-telkom/repos/mockpulse/issues/42")
        .expect("Route should match nested params");

    assert_eq!(m.params.get("orgId"), Some(&"smk-telkom".to_string()));
    assert_eq!(m.params.get("repoId"), Some(&"mockpulse".to_string()));
    assert_eq!(m.params.get("issueNum"), Some(&"42".to_string()));
}

#[test]
fn test_colon_parameter_syntax() {
    let mut router = RadixTrieRouter::new();
    router.insert(make_endpoint(HttpMethod::Post, "/devices/:deviceId/restart"));

    let m = router
        .lookup(&HttpMethod::Post, "/devices/dev-8801/restart")
        .expect("Colon syntax should match");

    assert_eq!(m.params.get("deviceId"), Some(&"dev-8801".to_string()));
}

#[test]
fn test_wildcard_catch_all_matching() {
    let mut router = RadixTrieRouter::new();
    router.insert(make_endpoint(HttpMethod::Get, "/static/*filepath"));

    let m = router
        .lookup(&HttpMethod::Get, "/static/css/themes/dark.css")
        .expect("Wildcard route should match");

    assert_eq!(
        m.params.get("filepath"),
        Some(&"css/themes/dark.css".to_string())
    );
}

#[test]
fn test_http_method_segregation() {
    let mut router = RadixTrieRouter::new();
    router.insert(make_endpoint(HttpMethod::Get, "/items"));
    router.insert(make_endpoint(HttpMethod::Post, "/items"));
    router.insert(make_endpoint(HttpMethod::Delete, "/items"));

    assert!(router.lookup(&HttpMethod::Get, "/items").is_some());
    assert!(router.lookup(&HttpMethod::Post, "/items").is_some());
    assert!(router.lookup(&HttpMethod::Delete, "/items").is_some());
    assert!(router.lookup(&HttpMethod::Put, "/items").is_none());
}

#[test]
fn test_collect_all_routes() {
    let mut router = RadixTrieRouter::new();
    router.insert(make_endpoint(HttpMethod::Get, "/a"));
    router.insert(make_endpoint(HttpMethod::Post, "/b"));
    router.insert(make_endpoint(HttpMethod::Delete, "/c"));

    let all = router.all_routes();
    assert_eq!(all.len(), 3);
    assert_eq!(router.len(), 3);
    assert!(!router.is_empty());
}
