use crate::lower::Lowering;
use nose_il::{
    NodeId, NodeKind, Payload, RegionKind, SourceGranularity, UnitBodyKind, UnitDomain,
    UnitDomains, UnitEvidenceFlag, UnitKind, UnitOrigin, UnitSubkind,
};
use tree_sitter::Node as TsNode;

pub(super) fn lower_type_decl(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let name = node.child_by_field_name("name").map(|n| lo.sym(lo.text(n)));
    let target = node
        .child_by_field_name("value")
        .or_else(|| node.child_by_field_name("body"))
        .unwrap_or(node);
    let body = lower_type_skeleton(lo, target);
    let block = lo.add(NodeKind::Block, Payload::None, span, &[body]);
    lo.push_unit_with_origin(block, UnitKind::Class, name, ts_type_decl_origin(node));
    block
}

fn ts_type_decl_origin(node: TsNode) -> UnitOrigin {
    match node.kind() {
        "interface_declaration" => UnitOrigin::new(
            UnitDomains::of(UnitDomain::TypeContract),
            UnitSubkind::InterfaceTraitProtocol,
            UnitBodyKind::DeclarationOnly,
            SourceGranularity::WholeUnit,
            RegionKind::Code,
        )
        .with_evidence(UnitEvidenceFlag::TypeOnly)
        .with_evidence(UnitEvidenceFlag::DeclarationOnly),
        "type_alias_declaration" => UnitOrigin::new(
            UnitDomains::of(UnitDomain::TypeContract),
            UnitSubkind::TypeAlias,
            UnitBodyKind::DeclarationOnly,
            SourceGranularity::WholeUnit,
            RegionKind::Code,
        )
        .with_evidence(UnitEvidenceFlag::TypeOnly)
        .with_evidence(UnitEvidenceFlag::AliasDeclaration)
        .with_evidence(UnitEvidenceFlag::DeclarationOnly),
        "enum_declaration" => UnitOrigin::new(
            UnitDomains::of(UnitDomain::TypeContract).with(UnitDomain::Data),
            UnitSubkind::Enum,
            UnitBodyKind::DeclarativeDenotation,
            SourceGranularity::WholeUnit,
            RegionKind::Code,
        )
        .with_evidence(UnitEvidenceFlag::RuntimeValue)
        .with_evidence(UnitEvidenceFlag::DataShapeOnly),
        _ => UnitOrigin::unknown(),
    }
}

/// Recursively skeletonize a type node: identifiers / property names / type keywords →
/// `Var`, literal types → literals, composites → `Seq` of their parts. Captures the
/// type's textual structure (so identical definitions converge, different ones don't)
/// without modeling type semantics.
fn lower_type_skeleton(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    match node.kind() {
        "type_identifier"
        | "property_identifier"
        | "identifier"
        | "predefined_type"
        | "shorthand_property_identifier"
        | "this_type" => lo.var(lo.text(node), span),
        "string" => lo.str_lit(lo.text(node), span),
        "number" => lo.int_lit(lo.text(node).trim(), span),
        _ => {
            let kids: Vec<NodeId> = Lowering::named_children(node)
                .into_iter()
                .map(|c| lower_type_skeleton(lo, c))
                .collect();
            if kids.is_empty() {
                lo.var(node.kind(), span) // keyword leaf (true/null/void/…)
            } else {
                lo.add(NodeKind::Seq, Payload::None, span, &kids)
            }
        }
    }
}

pub(super) fn is_ts_type(k: &str) -> bool {
    matches!(
        k,
        "type_identifier"
            | "predefined_type"
            | "generic_type"
            | "type_annotation"
            | "opting_type_annotation"
            | "omitting_type_annotation"
            | "type_arguments"
            | "type_parameter"
            | "type_parameters"
            | "function_type"
            | "constructor_type"
            | "property_signature"
            | "call_signature"
            | "construct_signature"
            | "index_signature"
            | "method_signature"
            | "abstract_method_signature"
            | "union_type"
            | "intersection_type"
            | "type_predicate"
            | "type_query"
            | "index_type_query"
            | "lookup_type"
            | "literal_type"
            | "tuple_type"
            | "array_type"
            | "object_type"
            | "parenthesized_type"
            | "conditional_type"
            | "mapped_type"
            | "nested_type_identifier"
            | "readonly_type"
            | "infer_type"
            | "template_literal_type"
            | "existential_type"
    )
}
