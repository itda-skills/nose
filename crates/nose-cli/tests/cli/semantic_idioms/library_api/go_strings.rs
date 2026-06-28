use crate::*;

#[test]
fn cli_normalized_il_proves_go_strings_contains_namespace_calls() {
    let dir = std::env::temp_dir().join(format!("nose_go_strings_contains_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("contains_std.go"),
        "package p\n\nimport \"strings\"\n\nfunc ContainsStd(value string) bool {\n    return strings.Contains(value, \"pre\")\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("contains_alias.go"),
        "package p\n\nimport str \"strings\"\n\nfunc ContainsAlias(value string) bool {\n    return str.Contains(value, \"pre\")\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("contains_other_needle.go"),
        "package p\n\nimport \"strings\"\n\nfunc ContainsOtherNeedle(value string) bool {\n    return strings.Contains(value, \"alt\")\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("contains_slice.go"),
        "package p\n\nimport \"slices\"\n\nfunc ContainsSlice(xs []string) bool {\n    return slices.Contains(xs, \"pre\")\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("contains_shadow.go"),
        "package p\n\ntype matcher struct{}\n\nfunc (m matcher) Contains(value string, needle string) bool { return true }\n\nfunc ContainsShadow(strings matcher, value string) bool {\n    return strings.Contains(value, \"pre\")\n}\n",
    )
    .unwrap();

    let normalized = |name: &str| {
        run_raw(&[
            "il",
            dir.join(name).to_str().unwrap(),
            "--normalized",
            "--format",
            "sexpr",
        ])
    };
    let std = normalized("contains_std.go");
    let alias = normalized("contains_alias.go");
    assert_eq!(
        std, alias,
        "Go strings.Contains should canonicalize import aliases through namespace evidence"
    );
    assert!(
        std.contains("@StringContains"),
        "strings.Contains should lower to substring membership, not collection membership: {std}"
    );

    let other_needle = normalized("contains_other_needle.go");
    assert_ne!(
        std, other_needle,
        "different substring needles must remain distinct"
    );
    let slice = normalized("contains_slice.go");
    assert!(
        slice.contains("@Contains") && !slice.contains("@StringContains"),
        "slices.Contains should stay collection membership: {slice}"
    );
    let shadow = normalized("contains_shadow.go");
    assert!(
        !shadow.contains("@StringContains") && !shadow.contains("@Contains"),
        "a local value named strings must not prove any stdlib Contains semantic: {shadow}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cli_normalized_il_proves_go_strings_join_namespace_calls() {
    let dir = std::env::temp_dir().join(format!("nose_go_strings_join_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("join_std.go"),
        "package p\n\nimport \"strings\"\n\nfunc JoinStd(parts []string) string {\n    return strings.Join(parts, \",\")\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("join_alias.go"),
        "package p\n\nimport str \"strings\"\n\nfunc JoinAlias(parts []string) string {\n    return str.Join(parts, \",\")\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("join_other_sep.go"),
        "package p\n\nimport \"strings\"\n\nfunc JoinOtherSep(parts []string) string {\n    return strings.Join(parts, \";\")\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("join_shadow.go"),
        "package p\n\ntype joiner struct{}\n\nfunc (j joiner) Join(parts []string, sep string) string { return sep }\n\nfunc JoinShadow(strings joiner, parts []string) string {\n    return strings.Join(parts, \",\")\n}\n",
    )
    .unwrap();

    let normalized = |name: &str| {
        run_raw(&[
            "il",
            dir.join(name).to_str().unwrap(),
            "--normalized",
            "--format",
            "sexpr",
        ])
    };
    let std = normalized("join_std.go");
    let alias = normalized("join_alias.go");
    assert_eq!(
        std, alias,
        "Go strings.Join should canonicalize import aliases through namespace evidence"
    );
    assert!(
        std.contains("@Join"),
        "strings.Join should lower to ordered string join: {std}"
    );

    let other_sep = normalized("join_other_sep.go");
    assert_ne!(std, other_sep, "different separators must remain distinct");
    let shadow = normalized("join_shadow.go");
    assert!(
        !shadow.contains("@Join"),
        "a local value named strings must not prove stdlib Join semantic: {shadow}"
    );

    let _ = fs::remove_dir_all(&dir);
}
