use super::*;

#[path = "exact_fragments/support.rs"]
mod fragment_support;
use fragment_support::*;

#[path = "exact_fragments/java_this_field.rs"]
mod java_this_field;

#[path = "exact_fragments/ordered_effect_branches.rs"]
mod ordered_effect_branches;

#[path = "exact_fragments/ordered_conditional_branches.rs"]
mod ordered_conditional_branches;

#[path = "exact_fragments/ordered_loop_conditional_branches.rs"]
mod ordered_loop_conditional_branches;

#[path = "exact_fragments/conditional_effects.rs"]
mod conditional_effects;
#[path = "exact_fragments/foreach_append.rs"]
mod foreach_append;
#[path = "exact_fragments/foreach_index.rs"]
mod foreach_index;
#[path = "exact_fragments/index_and_throw.rs"]
mod index_and_throw;
#[path = "exact_fragments/ordered_append.rs"]
mod ordered_append;
#[path = "exact_fragments/return_fragments.rs"]
mod return_fragments;

#[test]
fn feature_extraction_keeps_dense_small_functions_and_exact_fragments_but_not_small_control_blocks()
{
    let dir = std::env::temp_dir().join(format!("nose_dense_gate_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("a.py"),
        "def dense(xs):\n    return sum(x for x in xs if x > 0)\n\n\
def blocky(xs):\n    total = 0\n    if xs:\n        total = total + xs[0]\n    return total\n",
    )
    .unwrap();

    let out = run(&[
        "features",
        dir.to_str().unwrap(),
        "--min-lines",
        "20",
        "--min-tokens",
        "60",
    ]);
    let json: serde_json::Value = serde_json::from_str(&out).expect("features JSON");
    let units = json["units"].as_array().expect("features units array");
    assert!(
        units
            .iter()
            .any(|unit| unit["kind"] == "Function" && unit["name"] == "dense"),
        "behaviorally dense functions keep the semantic size-gate escape: {out}"
    );
    let block_units: Vec<&serde_json::Value> = units
        .iter()
        .filter(|unit| unit["kind"] == "Block")
        .collect();
    assert!(
        block_units
            .iter()
            .all(|unit| unit["start_line"] == 2 && unit["end_line"] == 2),
        "small control-flow blocks should stay behind the syntactic gate; exact return fragments may pass: {out}"
    );
    let _ = fs::remove_dir_all(&dir);
}
