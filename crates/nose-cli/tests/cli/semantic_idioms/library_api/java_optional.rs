use crate::*;

#[test]
fn cli_normalized_il_proves_java_optional_value_channel() {
    let dir = std::env::temp_dir().join(format!("nose_java_optional_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("present_fq.java"),
        "class PresentFq { static boolean f(java.util.Optional<String> value) { return value.isPresent(); } }\n",
    )
    .unwrap();
    fs::write(
        dir.join("present_renamed.java"),
        "class PresentRenamed { static boolean f(java.util.Optional<String> maybe) { return maybe.isPresent(); } }\n",
    )
    .unwrap();
    fs::write(
        dir.join("present_bare.java"),
        "import java.util.Optional;\nclass PresentBare { static boolean f(Optional<String> value) { return value.isPresent(); } }\n",
    )
    .unwrap();
    fs::write(
        dir.join("default_fq.java"),
        "class DefaultFq { static String f(java.util.Optional<String> value, String fallback) { return value.orElse(fallback); } }\n",
    )
    .unwrap();
    fs::write(
        dir.join("default_bare.java"),
        "import java.util.Optional;\nclass DefaultBare { static String f(Optional<String> value, String fallback) { return value.orElse(fallback); } }\n",
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
    let present = normalized("present_fq.java");
    let present_renamed = normalized("present_renamed.java");
    assert_eq!(
        present, present_renamed,
        "fully-qualified java.util.Optional receivers should canonicalize across local names"
    );
    assert!(
        present.contains("@IsNotNull"),
        "Optional.isPresent should lower to the option presence predicate: {present}"
    );
    let present_bare = normalized("present_bare.java");
    assert!(
        !present_bare.contains("@IsNotNull"),
        "bare Optional remains closed until import-backed type-domain proof exists: {present_bare}"
    );

    let default = normalized("default_fq.java");
    assert!(
        default.contains("@ValueOrDefault"),
        "Optional.orElse should lower to the option default channel: {default}"
    );
    let default_bare = normalized("default_bare.java");
    assert!(
        !default_bare.contains("@ValueOrDefault"),
        "bare Optional.orElse remains closed until import-backed type-domain proof exists: {default_bare}"
    );

    let _ = fs::remove_dir_all(&dir);
}
