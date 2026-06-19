use super::*;

/// Per-unit value fingerprints by unit name, via the hidden `features` command.
fn unit_value_fingerprints(dir: &Path) -> std::collections::HashMap<String, String> {
    let out = run(&[
        "features",
        dir.to_str().unwrap(),
        "--min-lines",
        "1",
        "--min-tokens",
        "1",
    ]);
    let json: serde_json::Value = serde_json::from_str(&out).expect("features JSON");
    json["units"]
        .as_array()
        .expect("units")
        .iter()
        .filter(|u| u["kind"] == "Function" || u["kind"] == "Method")
        .map(|u| {
            (
                u["name"].as_str().unwrap_or_default().to_string(),
                u["value"].to_string(),
            )
        })
        .collect()
}

/// #210 hard negative: a Python try/except/`else` wrapper whose success path
/// returns an opaque call must NOT merge with the bare identity function. The
/// `else` clause used to be dropped at lowering, erasing the success path from
/// the value fingerprint and collapsing the wrapper onto its `except: return x`
/// arm (black's `wrap_stream_for_windows` ≡ `optimize(self): return self`).
#[test]
fn query_mode_semantic_keeps_python_try_else_wrapper_apart_from_identity() {
    let dir = std::env::temp_dir().join(format!("nose_try_else_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let wrapper = "def wrap(f):\n    try:\n        from colorama.initialise import wrap_stream\n    except ImportError:\n        return f\n    else:\n        return wrap_stream(f)\n";
    fs::write(dir.join("wrapper_a.py"), wrapper).unwrap();
    fs::write(
        dir.join("wrapper_b.py"),
        wrapper.replace("def wrap", "def wrap_again"),
    )
    .unwrap();
    fs::write(dir.join("ident.py"), "def optimize(f):\n    return f\n").unwrap();

    // The wrappers are exact-unsafe (opaque call), so the property to pin is the
    // FINGERPRINT itself: identical wrappers share one, the identity never does.
    let fps = unit_value_fingerprints(&dir);
    assert_eq!(
        fps["wrap"], fps["wrap_again"],
        "identical try/else wrappers must share a fingerprint"
    );
    assert_ne!(
        fps["wrap"], fps["optimize"],
        "try/else wrapper must not share a fingerprint with the identity function"
    );
}

/// #210 hard negative, Ruby spelling: `begin/rescue/else` — the `else` clause is
/// the success path, not a handler; in handler position the no-throw fingerprint
/// convention erased it entirely.
#[test]
fn query_mode_semantic_keeps_ruby_begin_else_wrapper_apart_from_identity() {
    let dir = std::env::temp_dir().join(format!("nose_ruby_else_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let wrapper = "def wrap(f)\n  begin\n    probe\n  rescue StandardError\n    return f\n  else\n    return transform(f)\n  end\nend\n";
    fs::write(dir.join("wrapper_a.rb"), wrapper).unwrap();
    fs::write(
        dir.join("wrapper_b.rb"),
        wrapper.replace("def wrap", "def wrap_again"),
    )
    .unwrap();
    fs::write(
        dir.join("ident.rb"),
        "def passthrough(f)\n  return f\nend\n",
    )
    .unwrap();

    let fps = unit_value_fingerprints(&dir);
    assert_eq!(
        fps["wrap"], fps["wrap_again"],
        "identical begin/else wrappers must share a fingerprint"
    );
    assert_ne!(
        fps["wrap"], fps["passthrough"],
        "begin/rescue/else wrapper must not share a fingerprint with the identity function"
    );
}

/// #210 oracle fidelity: the 3-way selection canon (if-chain ≡ nested 2-arg
/// `Math.max`) is sound, and the oracle must AGREE — its 2-arg scalar min/max
/// used to fall into the 1-arg collection fold on a List operand
/// (`max([1,2,3,4], 7)` returned 4), making the merged twins disagree on
/// battery rows and flagging a proof-backed canon as a false merge.
#[test]
fn verify_oracle_agrees_with_three_way_selection_canon() {
    let dir = std::env::temp_dir().join(format!("nose_minmax_oracle_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("Chain.java"),
        "class Chain {\n    static int max3(int a, int b, int c) {\n        if (b > a) {\n            a = b;\n        }\n        if (c > a) {\n            a = c;\n        }\n        return a;\n    }\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("Nested.java"),
        "class Nested {\n    static int max3(int a, int b, int c) {\n        return Math.max(Math.max(a, b), c);\n    }\n}\n",
    )
    .unwrap();

    let out = run(&["verify", dir.to_str().unwrap()]);
    assert!(
        out.contains("SOUND: no false merges"),
        "the selection canon's twins must agree behaviorally: {out}"
    );
}
