use super::*;

#[test]
fn semantic_scan_reports_exact_safe_java_this_field_assignment_fragments() {
    let fixtures = [
        (
            "FieldSelfSquareA.java",
            "class FieldSelfSquareA {\n  int value;\n  void f(int v) {\n    this.value = (v + 1) * (v + 1);\n    audit(this);\n  }\n}\n",
        ),
        (
            "FieldSelfSquareB.java",
            "class FieldSelfSquareB {\n  int value;\n  void f(int w) {\n    this.value = (1 + w) * (1 + w);\n    trace(this);\n  }\n}\n",
        ),
        (
            "FieldSelfSquareWrongValue.java",
            "class FieldSelfSquareWrongValue {\n  int value;\n  void f(int x) {\n    this.value = (x + 2) * (x + 2);\n    audit(this);\n  }\n}\n",
        ),
        (
            "FieldSelfConditionalA.java",
            "class FieldSelfConditionalA {\n  int total;\n  int other;\n  void f(boolean enabled, int a, int b) {\n    if (enabled) {\n      this.total = a + b;\n    }\n    audit(this);\n  }\n}\n",
        ),
        (
            "FieldSelfConditionalB.java",
            "class FieldSelfConditionalB {\n  int total;\n  int other;\n  void f(boolean ready, int c, int d) {\n    if (ready) {\n      this.total = d + c;\n    }\n    trace(this);\n  }\n}\n",
        ),
        (
            "FieldSelfConditionalWrongField.java",
            "class FieldSelfConditionalWrongField {\n  int total;\n  int other;\n  void f(boolean ready, int c, int d) {\n    if (ready) {\n      this.other = d + c;\n    }\n    audit(this);\n  }\n}\n",
        ),
        (
            "FieldSelfNestedA.java",
            "class FieldSelfNestedA {\n  int score;\n  void f(boolean enabled, int a, int b) {\n    if (enabled) {\n      if (a > 0) {\n        this.score = (a + b) * 2;\n      }\n    }\n    audit(this);\n  }\n}\n",
        ),
        (
            "FieldSelfNestedB.java",
            "class FieldSelfNestedB {\n  int score;\n  void f(boolean ready, int c, int d) {\n    if (ready) {\n      if (0 < c) {\n        this.score = 2 * (d + c);\n      }\n    }\n    trace(this);\n  }\n}\n",
        ),
        (
            "FieldSelfNestedWrongReceiver.java",
            "class FieldSelfNestedWrongReceiverBox { int score; }\nclass FieldSelfNestedWrongReceiver {\n  int score;\n  void f(FieldSelfNestedWrongReceiverBox other, boolean ready, int c, int d) {\n    if (ready) {\n      if (0 < c) {\n        other.score = 2 * (d + c);\n      }\n    }\n    audit(this);\n  }\n}\n",
        ),
        (
            "js_this_field_a.js",
            "function jsThisFieldLeft(v) {\n  this.value = (v + 1) * (v + 1);\n  audit(this);\n}\n",
        ),
        (
            "js_this_field_b.js",
            "function jsThisFieldRight(w) {\n  this.value = (1 + w) * (1 + w);\n  trace(this);\n}\n",
        ),
        (
            "py_self_field_a.py",
            "class PyFieldLeft:\n    def f(self, v):\n        self.value = (v + 1) * (v + 1)\n        audit(self)\n",
        ),
        (
            "py_self_field_b.py",
            "class PyFieldRight:\n    def f(self, w):\n        self.value = (1 + w) * (1 + w)\n        trace(self)\n",
        ),
    ];
    let (dir, out, families) =
        scan_fragment_only_fixtures("nose_exact_this_field_assign_fragments", &fixtures);

    let assert_fragment_family = |left: &str, right: &str, negative: &str| {
        let family =
            find_block_pair_family(&families, left, right, negative).unwrap_or_else(|| {
                panic!("missing exact this-field fragment family {left}/{right}: {out}")
            });
        assert!(
            pair_locations(family, left, right)
                .iter()
                .all(|loc| loc["end_line"].as_u64().unwrap_or(0)
                    <= loc["start_line"].as_u64().unwrap_or(0) + 5),
            "this-field fragments should stay tightly scoped: {family:?}"
        );
    };

    let assert_no_pair = |left: &str, right: &str| {
        assert!(
            !has_pair_family(&families, left, right),
            "dynamic or wrong-receiver field assignment must stay outside exact fragments: {left}/{right}: {out}"
        );
    };

    assert_fragment_family(
        "FieldSelfSquareA.java",
        "FieldSelfSquareB.java",
        "FieldSelfSquareWrongValue.java",
    );
    assert_fragment_family(
        "FieldSelfConditionalA.java",
        "FieldSelfConditionalB.java",
        "FieldSelfConditionalWrongField.java",
    );
    assert_fragment_family(
        "FieldSelfNestedA.java",
        "FieldSelfNestedB.java",
        "FieldSelfNestedWrongReceiver.java",
    );
    assert_no_pair("FieldSelfNestedA.java", "FieldSelfNestedWrongReceiver.java");
    assert_no_pair("js_this_field_a.js", "js_this_field_b.js");
    assert_no_pair("py_self_field_a.py", "py_self_field_b.py");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn semantic_scan_reports_exact_safe_ordered_java_this_field_branch_fragments() {
    let fixtures = [
        (
            "FieldBranchOrderedA.java",
            "class FieldBranchOrderedA {\n  int value;\n  int limit;\n  void f(boolean enabled, int a, int b) {\n    if (enabled) {\n      this.value = (a + 1) * (a + 1);\n      this.limit = b - a;\n    }\n    audit(this);\n  }\n}\n",
        ),
        (
            "FieldBranchOrderedB.java",
            "class FieldBranchOrderedB {\n  int value;\n  int limit;\n  void f(boolean ready, int c, int d) {\n    if (ready) {\n      this.value = (1 + c) * (1 + c);\n      this.limit = d - c;\n    }\n    trace(this);\n  }\n}\n",
        ),
        (
            "FieldBranchOrderedWrongReceiver.java",
            "class FieldBranchOrderedWrongReceiverBox { int value; }\nclass FieldBranchOrderedWrongReceiver {\n  int value;\n  int limit;\n  void f(FieldBranchOrderedWrongReceiverBox other, boolean ready, int c, int d) {\n    if (ready) {\n      other.value = (1 + c) * (1 + c);\n      this.limit = d - c;\n    }\n    audit(this);\n  }\n}\n",
        ),
        (
            "FieldBranchTripleA.java",
            "class FieldBranchTripleA {\n  int value;\n  int limit;\n  int score;\n  void f(boolean enabled, int a, int b) {\n    if (enabled) {\n      this.value = a + b;\n      this.limit = (a + b) * 2;\n      this.score = b - a;\n    }\n    audit(this);\n  }\n}\n",
        ),
        (
            "FieldBranchTripleB.java",
            "class FieldBranchTripleB {\n  int value;\n  int limit;\n  int score;\n  void f(boolean ready, int c, int d) {\n    if (ready) {\n      this.value = d + c;\n      this.limit = 2 * (d + c);\n      this.score = d - c;\n    }\n    trace(this);\n  }\n}\n",
        ),
        (
            "FieldBranchTripleWrongReceiver.java",
            "class FieldBranchTripleWrongReceiverBox { int limit; }\nclass FieldBranchTripleWrongReceiver {\n  int value;\n  int limit;\n  int score;\n  void f(FieldBranchTripleWrongReceiverBox other, boolean ready, int c, int d) {\n    if (ready) {\n      this.value = d + c;\n      other.limit = 2 * (d + c);\n      this.score = d - c;\n    }\n    audit(this);\n  }\n}\n",
        ),
    ];
    let (dir, out, families) =
        scan_fragment_only_fixtures("nose_ordered_this_field_branch_fragments", &fixtures);

    let assert_branch_family = |left: &str, right: &str, negative: &str| {
        let family = families
            .iter()
            .find(|family| {
                let files = location_files(family);
                files.iter().any(|file| file.ends_with(left))
                    && files.iter().any(|file| file.ends_with(right))
                    && files.iter().all(|file| !file.ends_with(negative))
                    && family_locations(family)
                        .iter()
                        .all(|loc| loc["fragment_kind"] == "conditional-guard")
            })
            .unwrap_or_else(|| {
                panic!("missing ordered self-field branch fragment family {left}/{right}: {out}")
            });
        assert!(
            pair_locations(family, left, right)
                .iter()
                .all(|loc| loc["end_line"].as_u64().unwrap_or(0)
                    <= loc["start_line"].as_u64().unwrap_or(0) + 5),
            "ordered self-field branch fragments should stay tightly scoped: {family:?}"
        );
    };

    let assert_no_conditional_guard_location = |negative: &str| {
        let has_conditional_guard = families.iter().any(|family| {
            family_locations(family).iter().any(|loc| {
                loc["file"]
                    .as_str()
                    .is_some_and(|file| file.ends_with(negative))
                    && loc["fragment_kind"] == "conditional-guard"
            })
        });
        assert!(
            !has_conditional_guard,
            "wrong receiver must not produce an ordered self-field conditional guard: {negative}: {out}"
        );
    };

    assert_branch_family(
        "FieldBranchOrderedA.java",
        "FieldBranchOrderedB.java",
        "FieldBranchOrderedWrongReceiver.java",
    );
    assert_branch_family(
        "FieldBranchTripleA.java",
        "FieldBranchTripleB.java",
        "FieldBranchTripleWrongReceiver.java",
    );
    assert_no_conditional_guard_location("FieldBranchOrderedWrongReceiver.java");
    assert_no_conditional_guard_location("FieldBranchTripleWrongReceiver.java");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn semantic_scan_reports_exact_safe_java_this_field_assignment_body_fragments() {
    let fixtures = [
        (
            "FieldBodyDirectA.java",
            "class FieldBodyDirectA {\n  int value;\n  int limit;\n  void f(int v, int n) {\n    this.value = (v + 1) * (v + 1);\n    this.limit = n + 3;\n  }\n}\n",
        ),
        (
            "FieldBodyDirectB.java",
            "class FieldBodyDirectB {\n  int value;\n  int limit;\n  void f(int w, int m) {\n    this.value = (1 + w) * (1 + w);\n    this.limit = 3 + m;\n  }\n}\n",
        ),
        (
            "FieldBodyDirectWrongValue.java",
            "class FieldBodyDirectWrongValue {\n  int value;\n  int limit;\n  void f(int x, int m) {\n    this.value = (x + 1) * (x + 1);\n    this.limit = 4 + m;\n  }\n}\n",
        ),
        (
            "FieldBodyConditionalA.java",
            "class FieldBodyConditionalA {\n  int total;\n  int score;\n  void f(boolean enabled, int a, int b) {\n    this.total = a + b;\n    if (enabled) {\n      this.score = (a + b) * 2;\n    }\n  }\n}\n",
        ),
        (
            "FieldBodyConditionalB.java",
            "class FieldBodyConditionalB {\n  int total;\n  int score;\n  void f(boolean ready, int c, int d) {\n    this.total = d + c;\n    if (ready) {\n      this.score = 2 * (d + c);\n    }\n  }\n}\n",
        ),
        (
            "FieldBodyConditionalWrongField.java",
            "class FieldBodyConditionalWrongField {\n  int total;\n  int score;\n  int other;\n  void f(boolean ready, int c, int d) {\n    this.total = d + c;\n    if (ready) {\n      this.other = 2 * (d + c);\n    }\n  }\n}\n",
        ),
        (
            "FieldBodyNestedA.java",
            "class FieldBodyNestedA {\n  int base;\n  int score;\n  void f(boolean enabled, int a, int b) {\n    this.base = a + b;\n    if (enabled) {\n      if (a > 0) {\n        this.score = (a + b) * (a + b);\n      }\n    }\n  }\n}\n",
        ),
        (
            "FieldBodyNestedB.java",
            "class FieldBodyNestedB {\n  int base;\n  int score;\n  void f(boolean ready, int c, int d) {\n    this.base = d + c;\n    if (ready) {\n      if (0 < c) {\n        this.score = (d + c) * (d + c);\n      }\n    }\n  }\n}\n",
        ),
        (
            "FieldBodyNestedWrongReceiver.java",
            "class FieldBodyNestedWrongReceiverBox { int score; }\nclass FieldBodyNestedWrongReceiver {\n  int base;\n  int score;\n  void f(FieldBodyNestedWrongReceiverBox other, boolean ready, int c, int d) {\n    this.base = d + c;\n    if (ready) {\n      if (0 < c) {\n        other.score = (d + c) * (d + c);\n      }\n    }\n  }\n}\n",
        ),
    ];
    let (dir, out, families) =
        scan_fragment_only_fixtures("nose_exact_this_field_body_fragments", &fixtures);

    let assert_body_family = |left: &str, right: &str, negative: &str| {
        let family = find_multiline_block_pair_family(&families, left, right, negative)
            .unwrap_or_else(|| {
                panic!("missing exact this-field body fragment family {left}/{right}: {out}")
            });
        assert!(
            pair_locations(family, left, right)
                .iter()
                .all(|loc| loc["end_line"].as_u64().unwrap_or(0)
                    <= loc["start_line"].as_u64().unwrap_or(0) + 7),
            "this-field body fragments should stay tightly scoped: {family:?}"
        );
    };

    let assert_no_pair = |left: &str, right: &str| {
        assert!(
            !has_pair_family(&families, left, right),
            "wrong-receiver field body must stay outside exact fragments: {left}/{right}: {out}"
        );
    };

    assert_body_family(
        "FieldBodyDirectA.java",
        "FieldBodyDirectB.java",
        "FieldBodyDirectWrongValue.java",
    );
    assert_body_family(
        "FieldBodyConditionalA.java",
        "FieldBodyConditionalB.java",
        "FieldBodyConditionalWrongField.java",
    );
    assert_body_family(
        "FieldBodyNestedA.java",
        "FieldBodyNestedB.java",
        "FieldBodyNestedWrongReceiver.java",
    );
    assert_no_pair("FieldBodyNestedA.java", "FieldBodyNestedWrongReceiver.java");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn semantic_scan_reports_exact_safe_java_this_field_return_this_body_fragments() {
    let fixtures = [
        (
            "FluentBodyDirectA.java",
            "class FluentBodyDirectA {\n  int value;\n  int limit;\n  FluentBodyDirectA f(int v, int n) {\n    this.value = (v + 1) * (v + 1);\n    this.limit = n + 3;\n    return this;\n  }\n}\n",
        ),
        (
            "FluentBodyDirectB.java",
            "class FluentBodyDirectB {\n  int value;\n  int limit;\n  FluentBodyDirectB f(int w, int m) {\n    this.value = (1 + w) * (1 + w);\n    this.limit = 3 + m;\n    return this;\n  }\n}\n",
        ),
        (
            "FluentBodyDirectWrongReturn.java",
            "class FluentBodyDirectWrongReturn {\n  int value;\n  int limit;\n  FluentBodyDirectWrongReturn f(FluentBodyDirectWrongReturn other, int w, int m) {\n    this.value = (1 + w) * (1 + w);\n    this.limit = 3 + m;\n    return other;\n  }\n}\n",
        ),
        (
            "FluentBodyConditionalA.java",
            "class FluentBodyConditionalA {\n  int total;\n  int score;\n  FluentBodyConditionalA f(boolean enabled, int a, int b) {\n    this.total = a + b;\n    if (enabled) {\n      this.score = (a + b) * 2;\n    }\n    return this;\n  }\n}\n",
        ),
        (
            "FluentBodyConditionalB.java",
            "class FluentBodyConditionalB {\n  int total;\n  int score;\n  FluentBodyConditionalB f(boolean ready, int c, int d) {\n    this.total = d + c;\n    if (ready) {\n      this.score = 2 * (d + c);\n    }\n    return this;\n  }\n}\n",
        ),
        (
            "FluentBodyConditionalWrongField.java",
            "class FluentBodyConditionalWrongField {\n  int total;\n  int score;\n  int other;\n  FluentBodyConditionalWrongField f(boolean ready, int c, int d) {\n    this.total = d + c;\n    if (ready) {\n      this.other = 2 * (d + c);\n    }\n    return this;\n  }\n}\n",
        ),
        (
            "FluentBodyNestedA.java",
            "class FluentBodyNestedA {\n  int base;\n  int score;\n  FluentBodyNestedA f(boolean enabled, int a, int b) {\n    this.base = a + b;\n    if (enabled) {\n      if (a > 0) {\n        this.score = (a + b) * (a + b);\n      }\n    }\n    return this;\n  }\n}\n",
        ),
        (
            "FluentBodyNestedB.java",
            "class FluentBodyNestedB {\n  int base;\n  int score;\n  FluentBodyNestedB f(boolean ready, int c, int d) {\n    this.base = d + c;\n    if (ready) {\n      if (0 < c) {\n        this.score = (d + c) * (d + c);\n      }\n    }\n    return this;\n  }\n}\n",
        ),
        (
            "FluentBodyNestedWrongValue.java",
            "class FluentBodyNestedWrongValue {\n  int base;\n  int score;\n  FluentBodyNestedWrongValue f(boolean ready, int c, int d) {\n    this.base = d + c;\n    if (ready) {\n      if (0 < c) {\n        this.score = (d + c) + (d + c);\n      }\n    }\n    return this;\n  }\n}\n",
        ),
    ];
    let (dir, out, families) = scan_fragment_only_fixtures(
        "nose_exact_this_field_return_this_body_fragments",
        &fixtures,
    );

    let assert_body_family = |left: &str, right: &str, negative: &str| {
        let family = find_multiline_block_pair_family(&families, left, right, negative)
            .unwrap_or_else(|| {
                panic!(
                    "missing exact this-field return-this body fragment family {left}/{right}: {out}"
                )
            });
        assert!(
            pair_locations(family, left, right)
                .iter()
                .all(|loc| loc["end_line"].as_u64().unwrap_or(0)
                    <= loc["start_line"].as_u64().unwrap_or(0) + 9),
            "this-field return-this body fragments should stay tightly scoped: {family:?}"
        );
    };

    let assert_no_pair = |left: &str, right: &str| {
        assert!(
            !has_pair_family(&families, left, right),
            "wrong-return field body must stay outside exact fragments: {left}/{right}: {out}"
        );
    };

    assert_body_family(
        "FluentBodyDirectA.java",
        "FluentBodyDirectB.java",
        "FluentBodyDirectWrongReturn.java",
    );
    assert_body_family(
        "FluentBodyConditionalA.java",
        "FluentBodyConditionalB.java",
        "FluentBodyConditionalWrongField.java",
    );
    assert_body_family(
        "FluentBodyNestedA.java",
        "FluentBodyNestedB.java",
        "FluentBodyNestedWrongValue.java",
    );
    assert_no_pair("FluentBodyDirectA.java", "FluentBodyDirectWrongReturn.java");
    let _ = fs::remove_dir_all(&dir);
}
