use super::*;

impl<'a> Lowering<'a> {
    pub(crate) fn record_core_semantic_evidence(
        &mut self,
        kind: NodeKind,
        payload: Payload,
        span: Span,
        children: &[NodeId],
    ) {
        match kind {
            NodeKind::Var if self.lang == Lang::Java => {
                if matches!(payload, Payload::Name(name) if self.interner.resolve(name) == "this") {
                    self.record_evidence(
                        EvidenceAnchor::node(span, kind),
                        EvidenceKind::Place(PlaceEvidenceKind::SelfReceiver),
                        "place_self_receiver",
                    );
                }
            }
            NodeKind::Call => {
                if matches!(payload, Payload::None) {
                    self.record_call_mutation_evidence(span, kind, children);
                    self.record_library_api_evidence_for_call(span, children);
                }
            }
            NodeKind::Field if self.lang == Lang::Java => {
                if let (Payload::Name(field), [receiver]) = (payload, children) {
                    if let Some(receiver_evidence) = self.self_receiver_evidence_id(*receiver) {
                        let field_hash = stable_symbol_hash(self.interner.resolve(field));
                        self.record_evidence_with_dependencies(
                            EvidenceAnchor::node(span, kind),
                            EvidenceKind::Place(PlaceEvidenceKind::SelfField { field_hash }),
                            "place_self_field",
                            vec![receiver_evidence],
                        );
                    }
                }
            }
            NodeKind::Assign => {
                if let [target, _value] = children {
                    self.record_evidence(
                        EvidenceAnchor::node(span, kind),
                        EvidenceKind::Effect(EffectEvidenceKind::BindingWrite),
                        "effect_binding_write",
                    );
                    if self.non_overloadable_index_assignment_target(*target) {
                        self.record_evidence(
                            EvidenceAnchor::node(span, kind),
                            EvidenceKind::Effect(EffectEvidenceKind::NonOverloadableIndexWrite),
                            "effect_non_overloadable_index_write",
                        );
                    } else if let Some((field_hash, place_evidence)) =
                        self.self_field_assignment_target(*target)
                    {
                        self.record_evidence_with_dependencies(
                            EvidenceAnchor::node(span, kind),
                            EvidenceKind::Effect(EffectEvidenceKind::SelfFieldWrite { field_hash }),
                            "effect_self_field_write",
                            vec![place_evidence],
                        );
                    }
                }
            }
            _ => {}
        }
    }

    fn record_call_mutation_evidence(&mut self, span: Span, kind: NodeKind, children: &[NodeId]) {
        if children.len() > 1 {
            self.record_evidence(
                EvidenceAnchor::node(span, kind),
                EvidenceKind::Effect(EffectEvidenceKind::OpaqueArgumentEscape),
                "effect_opaque_argument_escape",
            );
        }
        let Some(&callee) = children.first() else {
            return;
        };
        if self.b.node(callee).kind != NodeKind::Field {
            return;
        }
        let Payload::Name(method) = self.b.node(callee).payload else {
            return;
        };
        let arg_count = children.len().saturating_sub(1);
        if let Some(contract) = module_binding_mutating_method_contract(
            self.lang,
            self.interner.resolve(method),
            arg_count,
        ) {
            self.record_evidence(
                EvidenceAnchor::node(span, kind),
                EvidenceKind::Effect(contract.effect),
                "effect_receiver_mutation",
            );
        }
    }
}
