use super::*;

impl<'a> Lowering<'a> {
    pub(crate) fn source_fact_evidence_id(
        &self,
        node: NodeId,
        expected: SourceFactKind,
    ) -> Option<EvidenceId> {
        let span = self.b.node(node).span;
        self.evidence.iter().find_map(|record| {
            (record.anchor == EvidenceAnchor::source_span(span)
                && record.kind == EvidenceKind::Source(expected)
                && record.status == EvidenceStatus::Asserted)
                .then_some(record.id)
        })
    }

    pub(crate) fn source_call_evidence_id(
        &self,
        span: Span,
        call: SourceCallKind,
    ) -> Option<EvidenceId> {
        self.evidence.iter().find_map(|record| {
            (record.anchor == EvidenceAnchor::source_span(span)
                && record.kind == EvidenceKind::Source(SourceFactKind::Call(call))
                && record.status == EvidenceStatus::Asserted)
                .then_some(record.id)
        })
    }

    pub(crate) fn record_imported_binding_symbol_for_node(
        &mut self,
        node: NodeId,
        module: &str,
        exported: &str,
    ) -> Option<EvidenceId> {
        let expected = SymbolEvidenceKind::ImportedBinding {
            module_hash: stable_symbol_hash(module),
            exported_hash: stable_symbol_hash(exported),
        };
        let dependency = self.binding_symbol_evidence_id(node, expected)?;
        Some(self.record_evidence_with_dependencies(
            EvidenceAnchor::node(self.b.node(node).span, NodeKind::Var),
            EvidenceKind::Symbol(expected),
            "symbol_imported_binding_occurrence",
            vec![dependency],
        ))
    }

    pub(crate) fn record_imported_namespace_symbol_for_node(
        &mut self,
        node: NodeId,
        module: &str,
    ) -> Option<EvidenceId> {
        let expected = SymbolEvidenceKind::ImportedNamespace {
            module_hash: stable_symbol_hash(module),
        };
        let dependency = self.binding_symbol_evidence_id(node, expected)?;
        Some(self.record_evidence_with_dependencies(
            EvidenceAnchor::node(self.b.node(node).span, NodeKind::Var),
            EvidenceKind::Symbol(expected),
            "symbol_imported_namespace_occurrence",
            vec![dependency],
        ))
    }

    pub(crate) fn binding_symbol_evidence_id(
        &self,
        node: NodeId,
        expected: SymbolEvidenceKind,
    ) -> Option<EvidenceId> {
        if self.b.kind(node) != NodeKind::Var {
            return None;
        }
        let Payload::Name(local) = self.b.payload(node) else {
            return None;
        };
        let local_hash = stable_symbol_hash(self.interner.resolve(local));
        self.evidence.iter().find_map(|record| {
            (matches!(
                record.anchor,
                EvidenceAnchor::Binding {
                    local_hash: anchor_hash,
                    ..
                } if anchor_hash == local_hash
            ) && record.kind == EvidenceKind::Symbol(expected)
                && record.status == EvidenceStatus::Asserted)
                .then_some(record.id)
        })
    }

    pub(crate) fn unshadowed_global_evidence_id(
        &self,
        node: NodeId,
        name: &str,
    ) -> Option<EvidenceId> {
        let span = self.b.node(node).span;
        let kind = self.b.kind(node);
        self.evidence.iter().find_map(|record| {
            (record.anchor == EvidenceAnchor::node(span, kind)
                && record.kind
                    == EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
                        name_hash: stable_symbol_hash(name),
                    })
                && record.status == EvidenceStatus::Asserted)
                .then_some(record.id)
        })
    }

    pub(crate) fn qualified_global_evidence_id(
        &self,
        node: NodeId,
        path: &str,
    ) -> Option<EvidenceId> {
        let span = self.b.node(node).span;
        let kind = self.b.kind(node);
        self.evidence.iter().find_map(|record| {
            (record.anchor == EvidenceAnchor::node(span, kind)
                && record.kind
                    == EvidenceKind::Symbol(SymbolEvidenceKind::QualifiedGlobal {
                        path_hash: stable_symbol_hash(path),
                    })
                && record.status == EvidenceStatus::Asserted)
                .then_some(record.id)
        })
    }

    pub(crate) fn node_is_java_this_var(&self, node: NodeId) -> bool {
        self.lang == Lang::Java
            && self.b.kind(node) == NodeKind::Var
            && matches!(self.b.payload(node), Payload::Name(name) if self.interner.resolve(name) == "this")
    }

    pub(crate) fn self_receiver_evidence_id(&self, node: NodeId) -> Option<EvidenceId> {
        if !self.node_is_java_this_var(node) {
            return None;
        }
        let span = self.b.node(node).span;
        self.evidence.iter().find_map(|record| {
            (record.anchor == EvidenceAnchor::node(span, NodeKind::Var)
                && record.kind == EvidenceKind::Place(PlaceEvidenceKind::SelfReceiver)
                && record.status == EvidenceStatus::Asserted)
                .then_some(record.id)
        })
    }

    pub(crate) fn non_overloadable_index_assignment_target(&self, node: NodeId) -> bool {
        matches!(self.lang, Lang::C | Lang::Go | Lang::Java) && self.b.kind(node) == NodeKind::Index
    }

    pub(crate) fn self_field_assignment_target(&self, node: NodeId) -> Option<(u64, EvidenceId)> {
        if self.lang != Lang::Java || self.b.kind(node) != NodeKind::Field {
            return None;
        }
        let Payload::Name(field) = self.b.payload(node) else {
            return None;
        };
        let span = self.b.node(node).span;
        let field_hash = stable_symbol_hash(self.interner.resolve(field));
        self.evidence.iter().find_map(|record| {
            (record.anchor == EvidenceAnchor::node(span, NodeKind::Field)
                && record.kind == EvidenceKind::Place(PlaceEvidenceKind::SelfField { field_hash })
                && record.status == EvidenceStatus::Asserted)
                .then_some((field_hash, record.id))
        })
    }
}
