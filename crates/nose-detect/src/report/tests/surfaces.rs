use crate::Group;
use nose_il::UnitKind::{Class, Function};

use super::super::ranking::family_of;
use super::support::{fam, fragment_loc, fragment_loc_k, loc, loc_k, test_fragment_loc_k};

#[test]
fn pack_facing_laws_become_family_provenance_only_when_registered() {
    let proven = Group {
        score: 1.0,
        members: vec![
            loc("src/a.py", 1, 30, "python"),
            loc("src/b.py", 1, 30, "python"),
        ],
        semantic_laws: vec![nose_semantics::ValueLaw::NumericFactorDistribution],
        abstraction_witness: None,
        witness: None,
    };
    let family = family_of(&proven);
    assert_eq!(family.semantic_laws.len(), 1);
    assert_eq!(
        family.semantic_laws[0].pack_id,
        nose_semantics::FIRST_PARTY_VALUE_LAW_PACK_ID
    );
    assert_eq!(
        family.semantic_laws[0].law_id,
        "value-graph.factor-distribute.numeric-common-factor"
    );
    assert_eq!(
        family.semantic_laws[0].proof_obligation_id,
        "normalize.value_graph.factor_distribute"
    );

    let internal_only = Group {
        score: 1.0,
        members: vec![
            loc("src/c.py", 1, 30, "python"),
            loc("src/d.py", 1, 30, "python"),
        ],
        semantic_laws: vec![nose_semantics::ValueLaw::AddCommutativity],
        abstraction_witness: None,
        witness: None,
    };
    assert!(
        family_of(&internal_only).semantic_laws.is_empty(),
        "historical internal value-law gates must not be over-reported as pack-facing laws"
    );
}

#[test]
fn tiny_all_fragment_family_is_hidden_and_downranked() {
    // mean_lines above the `trivial` floor so the whole unit is a real default
    // candidate — the contrast this test is about is fragment-vs-whole, not size.
    let whole = fam(
        10.0,
        10,
        8,
        0,
        vec![
            loc("src/a.rs", 1, 10, "rust"),
            loc("src/b.rs", 1, 10, "rust"),
        ],
    );
    let fragment = fam(
        10.0,
        2,
        2,
        0,
        vec![
            fragment_loc("src/a.rs", 3, 4),
            fragment_loc("src/b.rs", 3, 4),
        ],
    );

    assert_eq!(whole.recommended_surface(), "default");
    assert_eq!(fragment.recommended_surface(), "hidden");
    assert!(
        fragment.extractability() < whole.extractability(),
        "tiny exact fragments remain present but should not outrank whole-unit candidates"
    );
}

#[test]
fn tiny_mixed_fragment_family_is_hidden() {
    let mixed = fam(
        10.0,
        1,
        0,
        0,
        vec![
            loc("src/a.rs", 1, 3, "rust"),
            fragment_loc_k("src/b.rs", 9, 9, crate::FragmentKind::ExprEffect),
            fragment_loc_k("src/c.rs", 12, 12, crate::FragmentKind::ExprEffect),
        ],
    );

    assert_eq!(
        mixed.recommended_surface(),
        "hidden",
        "one whole-unit site must not promote a one-line proof fragment family"
    );
}

#[test]
fn fragment_surface_uses_kind_scope_and_spread() {
    let divergence_effect = fam(
        10.0,
        6,
        6,
        0,
        vec![
            fragment_loc_k("src/a.rs", 1, 6, crate::FragmentKind::LoopEffect),
            fragment_loc_k("src/b.rs", 1, 6, crate::FragmentKind::LoopEffect),
        ],
    );
    assert_eq!(
        divergence_effect.recommended_surface(),
        "divergence",
        "medium effect fragments are synchronization hazards before default refactors"
    );

    let tiny_test_expr = fam(
        10.0,
        3,
        3,
        0,
        vec![
            test_fragment_loc_k("tests/a.py", 1, 3, crate::FragmentKind::ExprEffect),
            test_fragment_loc_k("tests/b.py", 1, 3, crate::FragmentKind::ExprEffect),
        ],
    );
    assert_eq!(
        tiny_test_expr.recommended_surface(),
        "hidden",
        "tiny test-only expression-effect scaffolding stays diagnostic-only"
    );

    let tiny_test_self_field = fam(
        10.0,
        4,
        3,
        1,
        vec![
            test_fragment_loc_k("tests/A.java", 10, 13, crate::FragmentKind::SelfFieldBody),
            test_fragment_loc_k("tests/B.java", 20, 23, crate::FragmentKind::SelfFieldBody),
        ],
    );
    assert_eq!(
        tiny_test_self_field.recommended_surface(),
        "hidden",
        "small test fixture constructor bodies stay out of divergence output"
    );

    let medium_test_expr = fam(
        10.0,
        8,
        7,
        0,
        vec![
            test_fragment_loc_k("tests/a.py", 10, 17, crate::FragmentKind::ExprEffect),
            test_fragment_loc_k("tests/b.py", 20, 27, crate::FragmentKind::ExprEffect),
        ],
    );
    assert_eq!(
        medium_test_expr.recommended_surface(),
        "divergence",
        "larger test setup fragments can still be useful divergence context"
    );

    let default_guard = fam(
        10.0,
        14,
        14,
        0,
        vec![
            fragment_loc("src/a.rs", 1, 14),
            fragment_loc("src/b.rs", 1, 14),
        ],
    );
    assert_eq!(
        default_guard.recommended_surface(),
        "default",
        "substantial cross-file guard fragments can be default candidates"
    );

    let generated = fam(
        10.0,
        20,
        20,
        0,
        vec![
            fragment_loc("target/generated/a.rs", 1, 20),
            fragment_loc("target/generated/b.rs", 1, 20),
        ],
    );
    assert_eq!(
        generated.recommended_surface(),
        "hidden",
        "generated-looking exact fragments stay out of default output"
    );
}

#[test]
fn value_poor_typedef_class_is_discounted() {
    // A field-only type definition (low value-graph) matches on shape alone.
    let typedef = Group {
        score: 1.0,
        members: vec![
            loc_k("src/a.rs", 1, 30, Class, 5),
            loc_k("src/b.rs", 1, 30, Class, 5),
        ],
        semantic_laws: Vec::new(),
        abstraction_witness: None,
        witness: None,
    };
    // A behavior-rich class of the same size is a genuine candidate.
    let rich = Group {
        score: 1.0,
        members: vec![
            loc_k("src/c.rs", 1, 30, Class, 80),
            loc_k("src/d.rs", 1, 30, Class, 80),
        ],
        semantic_laws: Vec::new(),
        abstraction_witness: None,
        witness: None,
    };
    let ftd = family_of(&typedef);
    let frich = family_of(&rich);
    assert!(
        frich.value > ftd.value,
        "value-poor type-def class is discounted below a behavior-rich one"
    );
    // A function family of the same low sem is NOT a type-def → not discounted.
    let func = Group {
        score: 1.0,
        members: vec![
            loc_k("src/e.rs", 1, 30, Function, 5),
            loc_k("src/f.rs", 1, 30, Function, 5),
        ],
        semantic_laws: Vec::new(),
        abstraction_witness: None,
        witness: None,
    };
    assert!(
        family_of(&func).value > ftd.value,
        "the type-def discount applies only to all-Class families"
    );
}

#[test]
fn vendored_family_is_discounted() {
    // All sites in vendored/generated paths → not the maintainer's to dedupe.
    let vendored = Group {
        score: 1.0,
        members: vec![
            loc("a/vendor/x.go", 1, 30, "go"),
            loc("b/vendor/y.go", 1, 30, "go"),
        ],
        semantic_laws: Vec::new(),
        abstraction_witness: None,
        witness: None,
    };
    let owned = Group {
        score: 1.0,
        members: vec![loc("src/x.go", 1, 30, "go"), loc("src/y.go", 1, 30, "go")],
        semantic_laws: Vec::new(),
        abstraction_witness: None,
        witness: None,
    };
    assert!(
        family_of(&owned).value > family_of(&vendored).value,
        "vendored duplication is discounted below owned-code duplication"
    );
}
