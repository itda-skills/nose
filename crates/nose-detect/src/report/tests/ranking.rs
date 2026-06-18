use crate::{Group, Loc};

use super::super::{rank_families, ranking::family_of};
use super::support::{loc, report};

#[test]
fn dup_lines_and_module_spread() {
    // 3 copies of a ~10-line unit across 3 modules
    let g = Group {
        score: 0.9,
        members: vec![
            loc("x/a.rs", 1, 10, "rust"),
            loc("y/b.rs", 1, 10, "rust"),
            loc("z/c.rs", 1, 10, "rust"),
        ],
        semantic_laws: Vec::new(),
        abstraction_witness: None,
        witness: None,
    };
    let f = &rank_families(&report(vec![g]))[0];
    assert_eq!(f.members, 3);
    assert_eq!(f.modules, 3);
    assert_eq!(f.mean_lines, 10);
    assert_eq!(f.dup_lines, 20, "(members-1) * mean_lines");
}

#[test]
fn ranks_by_value_design_level_first() {
    // big cross-module family should outrank a small local pair
    let big = Group {
        score: 0.8,
        members: (0..10)
            .map(|i| loc(&format!("m{i}/f.rs"), 1, 30, "rust"))
            .collect(),
        semantic_laws: Vec::new(),
        abstraction_witness: None,
        witness: None,
    };
    let small = Group {
        score: 1.0,
        members: vec![loc("p/a.rs", 1, 6, "rust"), loc("p/b.rs", 1, 6, "rust")],
        semantic_laws: Vec::new(),
        abstraction_witness: None,
        witness: None,
    };
    let fams = rank_families(&report(vec![small, big]));
    assert!(
        fams[0].members == 10,
        "the large cross-module family ranks first"
    );
    assert!(fams[0].value > fams[1].value);
}

#[test]
fn cross_language_bonus() {
    let mono = Group {
        score: 0.9,
        members: vec![loc("a.py", 1, 10, "python"), loc("b.py", 1, 10, "python")],
        semantic_laws: Vec::new(),
        abstraction_witness: None,
        witness: None,
    };
    let cross = Group {
        score: 0.9,
        members: vec![
            loc("a.py", 1, 10, "python"),
            loc("b.ts", 1, 10, "typescript"),
        ],
        semantic_laws: Vec::new(),
        abstraction_witness: None,
        witness: None,
    };
    let fm = family_of(&mono);
    let fc = family_of(&cross);
    assert_eq!(fc.languages, 2);
    assert!(
        fc.value > fm.value,
        "cross-language family is weighted higher"
    );
}

#[test]
fn test_code_duplication_is_not_discounted() {
    // Duplication in tests is a real smell too — a test-only family with the same
    // metrics as a prod family gets the same value (only tagged, not penalised).
    let prod = Group {
        score: 1.0,
        members: vec![
            loc("src/a.rs", 1, 30, "rust"),
            loc("src/b.rs", 1, 30, "rust"),
        ],
        semantic_laws: Vec::new(),
        abstraction_witness: None,
        witness: None,
    };
    let test = Group {
        score: 1.0,
        members: vec![
            loc("tests/a.rs", 1, 30, "rust"),
            loc("tests/b.rs", 1, 30, "rust"),
        ],
        semantic_laws: Vec::new(),
        abstraction_witness: None,
        witness: None,
    };
    let fp = family_of(&prod);
    let ft = family_of(&test);
    assert_eq!(ft.scope, "test");
    assert_eq!(fp.scope, "prod");
    assert_eq!(
        ft.value, fp.value,
        "test-code duplication is ranked like any other (tag only, no penalty)"
    );
}

#[test]
fn mixed_test_prod_is_not_discounted() {
    // Logic duplicated *across* the test boundary is a real smell — keep it.
    // Use a test *name* marker (not a test path) so the two families share the
    // same module/file/spread metrics and differ only in scope.
    let test_named = Loc {
        name: Some("test_thing".into()),
        ..loc("src/b.rs", 1, 30, "rust")
    };
    let mixed = Group {
        score: 1.0,
        members: vec![loc("src/a.rs", 1, 30, "rust"), test_named],
        semantic_laws: Vec::new(),
        abstraction_witness: None,
        witness: None,
    };
    let pure = Group {
        score: 1.0,
        members: vec![
            loc("src/a.rs", 1, 30, "rust"),
            loc("src/b.rs", 1, 30, "rust"),
        ],
        semantic_laws: Vec::new(),
        abstraction_witness: None,
        witness: None,
    };
    let fmixed = family_of(&mixed);
    let fpure = family_of(&pure);
    assert_eq!(fmixed.scope, "mixed");
    assert_eq!(
        fmixed.value, fpure.value,
        "test↔prod duplication is not discounted"
    );
}
