use super::*;

fn il_with_interner(src: &str) -> (Il, Interner) {
    let interner = Interner::default();
    let il = lower(FileId(0), "t.swift", src.as_bytes(), &interner).expect("lower swift");
    (il, interner)
}

fn il(src: &str) -> Il {
    il_with_interner(src).0
}

fn raw_names(il: &Il, interner: &Interner) -> Vec<String> {
    il.nodes
        .iter()
        .filter_map(|node| {
            if node.kind != NodeKind::Raw {
                return None;
            }
            let Payload::Name(name) = node.payload else {
                return None;
            };
            Some(interner.resolve(name).to_string())
        })
        .collect()
}

fn seq_names(il: &Il, interner: &Interner) -> Vec<String> {
    il.nodes
        .iter()
        .filter_map(|node| {
            if node.kind != NodeKind::Seq {
                return None;
            }
            let Payload::Name(name) = node.payload else {
                return None;
            };
            Some(interner.resolve(name).to_string())
        })
        .collect()
}

fn has_assign_rhs_seq(il: &Il, interner: &Interner, expected: &str) -> bool {
    il.nodes.iter().enumerate().any(|(idx, node)| {
        if node.kind != NodeKind::Assign {
            return false;
        }
        let children = il.children(NodeId(idx as u32));
        let [_, rhs] = children else {
            return false;
        };
        let rhs = il.node(*rhs);
        if rhs.kind != NodeKind::Seq {
            return false;
        }
        let Payload::Name(name) = rhs.payload else {
            return false;
        };
        interner.resolve(name) == expected
    })
}

#[test]
fn function_lowers_to_unit() {
    let il = il(r#"
func add(_ x: Int, _ y: Int) -> Int {
return x + y
}
"#);
    assert_eq!(il.units.len(), 1);
    assert_eq!(il.meta.lang, Lang::Swift);
}

#[test]
fn foreach_lowers_to_loop() {
    let il = il(r#"
func sumPositive(_ xs: [Int]) -> Int {
var total = 0
for x in xs {
    if x > 0 {
        total += x
    }
}
return total
}
"#);
    assert!(il.nodes.iter().any(|node| {
        node.kind == NodeKind::Loop && node.payload == Payload::Loop(LoopKind::ForEach)
    }));
}

#[test]
fn subscript_lowers_to_index() {
    let il = il(r#"
func get(_ xs: [Int], _ i: Int) -> Int {
return xs[i]
}
"#);
    assert!(il.nodes.iter().any(|node| node.kind == NodeKind::Index));
}

#[test]
fn closure_header_lowers_to_lambda_param() {
    let il = il(r#"
func mapped(_ xs: [Int]) -> [Int] {
return xs.map { x in x + 1 }
}
"#);
    let lambda = il
        .nodes
        .iter()
        .position(|node| node.kind == NodeKind::Lambda)
        .map(|idx| NodeId(idx as u32))
        .expect("lambda");
    let first = il.children(lambda).first().copied().expect("lambda child");
    assert_eq!(il.kind(first), NodeKind::Param);
}

#[test]
fn closure_type_header_dedupes_lambda_params() {
    let il = il(r#"
func mapped(_ xs: [Int]) -> [Int] {
return xs.map { (x: Int) -> Int in x + 1 }
}
"#);
    let lambda = il
        .nodes
        .iter()
        .position(|node| node.kind == NodeKind::Lambda)
        .map(|idx| NodeId(idx as u32))
        .expect("lambda");
    let params = il
        .children(lambda)
        .iter()
        .filter(|&&child| il.kind(child) == NodeKind::Param)
        .count();
    assert_eq!(params, 1);
}

#[test]
fn unparenthesized_comparison_conjunction_lowers_as_boolean_and() {
    let il = il(r#"
func ordered(_ x: Int, _ y: Int) -> Bool {
return x < y && x <= y
}
"#);
    assert!(il.nodes.iter().enumerate().any(|(idx, node)| {
        if node.kind != NodeKind::BinOp || node.payload != Payload::Op(Op::And) {
            return false;
        }
        let kids = il.children(NodeId(idx as u32));
        matches!(
            kids,
            [left, right]
                if il.kind(*left) == NodeKind::BinOp
                    && il.node(*left).payload == Payload::Op(Op::Lt)
                    && il.kind(*right) == NodeKind::BinOp
                    && il.node(*right).payload == Payload::Op(Op::Le)
        )
    }));
}

#[test]
fn dictionary_default_subscript_lowers_with_marker() {
    let (il, interner) = il_with_interner(
        r#"
func lookup(_ dict: Dictionary<String, Int>, _ key: String, _ fallback: Int) -> Int {
return dict[key, default: fallback]
}
"#,
    );
    let marker = interner.intern("swift_subscript_default");
    assert!(il
        .nodes
        .iter()
        .any(|node| { node.kind == NodeKind::Seq && node.payload == Payload::Name(marker) }));
}

#[test]
fn parameter_type_annotation_records_domain() {
    let il = il(r#"
func lookup(_ dict: Dictionary<String, Int>, _ value: Any) -> Int {
return dict["red", default: 0]
}
"#);
    assert_eq!(
        il.evidence
            .iter()
            .filter(|record| record.kind == EvidenceKind::Domain(nose_il::DomainEvidence::Map))
            .count(),
        1,
        "only Dictionary parameters should record a Map domain"
    );
}

#[test]
fn property_type_annotation_records_binding_domain() {
    let il = il(r#"
func build(_ xs: [Int]) -> [Int] {
var out: [Int] = []
for x in xs {
    out.append(x)
}
return out
}
"#);
    assert!(il.evidence.iter().any(|record| {
        matches!(
            record.anchor,
            EvidenceAnchor::Binding { local_hash, .. }
                if local_hash == stable_symbol_hash("out")
        ) && record.kind == EvidenceKind::Domain(nose_il::DomainEvidence::Collection)
    }));
}

#[test]
fn parenthesized_single_expression_does_not_become_tuple() {
    let il = il(r#"
func mapped(_ xs: [Int]) -> [Int] {
return xs.map { x in (x + 1) * 2 }
}
"#);
    assert!(!il
        .nodes
        .iter()
        .any(|node| { node.kind == NodeKind::Seq && matches!(node.payload, Payload::Name(_)) }));
}

#[test]
fn implicit_member_shorthand_lowers_without_raw_prefix() {
    let (il, interner) = il_with_interner(
        r#"
func axis() -> Any {
return .vertical
}

func space() -> Any {
return .named("scroll")
}
"#,
    );
    assert!(
        !raw_names(&il, &interner)
            .iter()
            .any(|name| name == "prefix_expression"),
        "implicit member syntax should not stay as a generic Raw prefix"
    );
    let implicit = interner.intern("swift_implicit_member");
    assert!(il.nodes.iter().enumerate().any(|(idx, node)| {
        if node.kind != NodeKind::Field {
            return false;
        }
        let children = il.children(NodeId(idx as u32));
        matches!(
            children,
            [receiver]
                if il.kind(*receiver) == NodeKind::Var
                    && il.node(*receiver).payload == Payload::Name(implicit)
        )
    }));
}

#[test]
fn protocol_requirements_lower_as_signature_units() {
    let (il, interner) = il_with_interner(
        r#"
protocol Store {
var count: Int { get }
func fetch(_ key: String) async throws -> Int
}
"#,
    );
    let raw = raw_names(&il, &interner);
    assert!(
        !raw.iter().any(|name| matches!(
            name.as_str(),
            "protocol_function_declaration"
                | "protocol_property_declaration"
                | "protocol_property_requirements"
        )),
        "protocol requirements should lower as declaration/signature structure, got {raw:?}"
    );
    assert!(
        il.units.iter().any(
            |unit| unit.kind == UnitKind::Method && unit.name == Some(interner.intern("fetch"))
        ),
        "protocol function requirement should be a method-like unit"
    );
    assert!(il.nodes.iter().any(|node| node.kind == NodeKind::Param));
}

#[test]
fn swift_pattern_and_keypath_surfaces_do_not_fall_to_generic_raw() {
    let (il, interner) = il_with_interner(
        r#"
enum Direction {
case north
case named(String)
}

func classify(_ value: Direction?, _ xs: [Int]) -> Int {
if let value = value {
    switch value {
    case .north:
        return xs[keyPath: \.count] > 0 ? 1 : 2
    case .named(let label):
        return label.count
    default:
        return 0
    }
}
return xs[0..<xs.count].count
}
"#,
    );
    let raw = raw_names(&il, &interner);
    for unexpected in [
        "switch_pattern",
        "value_binding_pattern",
        "enum_entry",
        "key_path_expression",
        "range_expression ..<",
        "ternary_expression",
    ] {
        assert!(
            !raw.iter().any(|name| name == unexpected),
            "{unexpected} should lower to structured IL, got {raw:?}"
        );
    }
}

#[test]
fn macro_invocations_do_not_cascade_raw_call_suffixes() {
    let (il, interner) = il_with_interner(
        r#"
func check(_ value: Int) {
#expect(value == 1)
#warning("generated fixture")
}
"#,
    );
    let raw = raw_names(&il, &interner);
    for unexpected in [
        "macro_invocation",
        "call_suffix",
        "value_arguments",
        "value_argument",
    ] {
        assert!(
            !raw.iter().any(|name| name == unexpected),
            "{unexpected} should not stay Raw in macro calls: {raw:?}"
        );
    }
    let seq = seq_names(&il, &interner);
    assert!(
        seq.iter().any(|name| name == "swift_macro_invocation"),
        "macro invocation should use the exact-closed Swift tag: {seq:?}"
    );
    assert!(
        seq.iter().any(|name| name == "swift_diagnostic_warning"),
        "diagnostic should preserve warning/error kind: {seq:?}"
    );
}

#[test]
fn computed_property_accessors_do_not_fall_to_raw() {
    let (il, interner) = il_with_interner(
        r#"
struct Box {
var storage: Int
var value: Int {
    get { storage }
    set(newValue) { storage = newValue }
}
}
"#,
    );
    let raw = raw_names(&il, &interner);
    for unexpected in [
        "computed_getter",
        "getter_specifier",
        "computed_setter",
        "setter_specifier",
        "computed_property",
    ] {
        assert!(
            !raw.iter().any(|name| name == unexpected),
            "{unexpected} should lower through accessor bodies: {raw:?}"
        );
    }
}

#[test]
fn swift_operator_literal_and_range_surfaces_do_not_fall_to_raw() {
    let (il, interner) = il_with_interner(
        r#"
final class Token {}

func risky() throws -> Int { 1 }

func ops(_ a: Int, _ b: Int, _ left: Token, _ right: Token) -> Any {
var x = a
x &+= b
let y = a &+ b
let z = a &- b
let w = a &* b
let id = left === right
let notId = left !== right
let slice = [a, b][...]
let forced = try! risky()
let info = (#fileID, #line, #function)
return (x, y, z, w, id, notId, slice, forced, info)
}
"#,
    );
    let raw = raw_names(&il, &interner);
    for unexpected in [
        "infix_expression &+=",
        "infix_expression &+",
        "infix_expression &-",
        "infix_expression &*",
        "equality_expression ===",
        "infix_expression !==",
        "fully_open_range",
        "bang",
        "special_literal",
    ] {
        assert!(
            !raw.iter().any(|name| name == unexpected),
            "{unexpected} should lower to Swift-specific structured IL: {raw:?}"
        );
    }
    let seq = seq_names(&il, &interner);
    for expected in [
        "swift_overflow_add",
        "swift_overflow_sub",
        "swift_overflow_mul",
        "swift_identity_eq",
        "swift_identity_ne",
        "swift_range_fully_open",
        "swift_special_literal",
    ] {
        assert!(
            seq.iter().any(|name| name == expected),
            "{expected} should be preserved as an exact-closed Swift tag: {seq:?}"
        );
    }
    assert!(
        has_assign_rhs_seq(&il, &interner, "swift_overflow_add"),
        "&+= should lower as mutation with a Swift overflow-add RHS"
    );
}

#[test]
fn conditional_compilation_directives_do_not_fall_to_raw() {
    let (il, interner) = il_with_interner(
        r#"
#if os(macOS)
let platform = 1
#elseif canImport(Glibc)
let platform = 2
#else
let platform = 3
#endif
"#,
    );
    let raw = raw_names(&il, &interner);
    assert!(
        !raw.iter().any(|name| name == "directive"),
        "conditional compilation directives should lower to Swift-specific markers: {raw:?}"
    );
    let seq = seq_names(&il, &interner);
    for expected in [
        "swift_directive_if",
        "swift_directive_elseif",
        "swift_directive_else",
        "swift_directive_endif",
    ] {
        assert!(
            seq.iter().any(|name| name == expected),
            "{expected} should preserve conditional-compilation shape: {seq:?}"
        );
    }
}
