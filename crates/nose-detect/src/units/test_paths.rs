pub(super) fn is_test_path(path: &str) -> bool {
    let p = path.to_ascii_lowercase();
    p.contains("/test/")
        || p.contains("/tests/")
        || p.contains("/__tests__/")
        || p.contains("/spec/")
        || p.starts_with("test/")
        || p.starts_with("tests/")
        || p.ends_with("_test.go")
        || p.ends_with("conftest.py")
        || ["_test.", ".test.", ".spec.", "_spec."]
            .iter()
            .any(|m| p.contains(m))
        || p.rsplit('/')
            .next()
            .unwrap_or(&p)
            .split('.')
            .next()
            .unwrap_or("")
            .starts_with("test_")
}
