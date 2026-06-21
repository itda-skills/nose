use super::*;

impl<'a> Lowering<'a> {
    pub(crate) fn record_param_domain(&mut self, span: Span, domain: DomainEvidence) {
        self.record_param_domain_with_dependencies(span, domain, Vec::new());
    }

    pub(crate) fn record_param_domain_with_dependencies(
        &mut self,
        span: Span,
        domain: DomainEvidence,
        dependencies: Vec<EvidenceId>,
    ) {
        self.record_param_domain_with_provenance(
            span,
            domain,
            dependencies,
            first_party_param_domain_provenance(),
        );
    }

    pub(crate) fn record_param_domain_resolution(
        &mut self,
        span: Span,
        domain: ResolvedTypeDomain,
    ) {
        self.record_param_domain_with_provenance(
            span,
            domain.domain,
            domain.dependencies,
            domain.provenance,
        );
    }

    pub(crate) fn record_param_domain_with_provenance(
        &mut self,
        span: Span,
        domain: DomainEvidence,
        dependencies: Vec<EvidenceId>,
        provenance: TypeDomainEvidenceProvenance,
    ) {
        self.record_evidence_with_pack_dependencies(
            EvidenceAnchor::param(span),
            EvidenceKind::Domain(domain),
            provenance.pack_id,
            provenance.rule,
            dependencies,
        );
    }

    pub(crate) fn record_source_fact(&mut self, span: Span, kind: SourceFactKind) {
        let (pack_id, producer_id) = nose_semantics::language_source_fact_provenance(self.lang);
        self.record_evidence_with_pack_dependencies(
            EvidenceAnchor::source_span(span),
            EvidenceKind::Source(kind),
            pack_id,
            producer_id,
            Vec::new(),
        );
    }

    pub(crate) fn record_evidence(
        &mut self,
        anchor: EvidenceAnchor,
        kind: EvidenceKind,
        rule: &str,
    ) -> EvidenceId {
        self.record_evidence_with_dependencies(anchor, kind, rule, Vec::new())
    }

    pub(crate) fn record_evidence_with_dependencies(
        &mut self,
        anchor: EvidenceAnchor,
        kind: EvidenceKind,
        rule: &str,
        dependencies: Vec<EvidenceId>,
    ) -> EvidenceId {
        self.record_evidence_with_pack_dependencies(
            anchor,
            kind,
            nose_semantics::FIRST_PARTY_PACK_ID,
            rule,
            dependencies,
        )
    }

    pub(crate) fn record_evidence_with_pack_dependencies(
        &mut self,
        anchor: EvidenceAnchor,
        kind: EvidenceKind,
        pack_id: &str,
        rule: &str,
        dependencies: Vec<EvidenceId>,
    ) -> EvidenceId {
        let id = EvidenceId(self.evidence.len() as u32);
        self.evidence.push(EvidenceRecord {
            id,
            anchor,
            kind,
            provenance: EvidenceProvenance {
                emitter: EvidenceEmitter::FirstParty,
                pack_hash: Some(stable_symbol_hash(pack_id)),
                rule_hash: Some(stable_symbol_hash(rule)),
            },
            dependencies,
            status: EvidenceStatus::Asserted,
        });
        id
    }

    pub(crate) fn record_type_domain_alias_with_pack_evidence(
        &mut self,
        local: &str,
        domain: DomainEvidence,
        evidence: Option<EvidenceId>,
        provenance: TypeDomainEvidenceProvenance,
    ) {
        self.type_domain_aliases
            .record_normalized(local, domain, evidence, provenance);
    }

    pub(crate) fn record_type_domain_alias_exact_with_evidence(
        &mut self,
        local: &str,
        domain: DomainEvidence,
        evidence: Option<EvidenceId>,
    ) {
        self.type_domain_aliases.record_exact(
            local,
            domain,
            evidence,
            first_party_param_domain_provenance(),
        );
    }

    pub(crate) fn clear_type_domain_alias(&mut self, local: &str) {
        self.type_domain_aliases.clear_normalized(local);
    }

    pub(crate) fn record_unsigned_32_alias_with_evidence(
        &mut self,
        local: &str,
        evidence: Option<EvidenceId>,
    ) {
        let alias = local.trim().to_string();
        if alias.is_empty() {
            return;
        }
        if let Some(existing) = self
            .unsigned_32_aliases
            .iter_mut()
            .find(|known| known.alias == alias)
        {
            if evidence.is_some() {
                existing.evidence = evidence;
            }
            return;
        }
        self.unsigned_32_aliases
            .push(Unsigned32Alias { alias, evidence });
    }

    pub(crate) fn type_domain_from_text_with_dependencies(
        &self,
        text: &str,
    ) -> Option<ResolvedTypeDomain> {
        self.type_domain_aliases.resolve_text(text).or_else(|| {
            type_domain_from_source_text(self.lang, text).map(|domain| ResolvedTypeDomain {
                domain,
                dependencies: Vec::new(),
                provenance: first_party_param_domain_provenance(),
            })
        })
    }
}

fn first_party_param_domain_provenance() -> TypeDomainEvidenceProvenance {
    TypeDomainEvidenceProvenance {
        pack_id: nose_semantics::FIRST_PARTY_PACK_ID,
        rule: "param_domain",
    }
}
