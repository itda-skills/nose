use super::*;

pub(super) fn record_c_direct_include_type_aliases(path: &str, src: &[u8], lo: &mut Lowering) {
    let Ok(source) = std::str::from_utf8(src) else {
        return;
    };
    let needs_byte_alias =
        c_source_may_contain_u16_byte_pack(source) || c_source_may_contain_u32_byte_pack(source);
    let needs_unsigned_32_alias = c_source_may_contain_u32_byte_pack(source);
    if !needs_byte_alias && !needs_unsigned_32_alias {
        return;
    }
    let Some(dir) = Path::new(path).parent() else {
        return;
    };
    let mut start_byte = 0u32;
    for (line_idx, line) in source.lines().enumerate() {
        let line_span = Span::new(
            lo.b.file(),
            start_byte,
            start_byte.saturating_add(line.len() as u32),
            line_idx as u32 + 1,
            line_idx as u32 + 1,
        );
        start_byte = start_byte.saturating_add(line.len() as u32 + 1);
        let Some(include) = c_direct_quote_include_name(line) else {
            continue;
        };
        if include.is_empty() || include.contains('/') || include.contains('\\') {
            continue;
        }
        let header = dir.join(include);
        let Ok(meta) = fs::metadata(&header) else {
            continue;
        };
        if !meta.is_file() || meta.len() > C_INCLUDE_ALIAS_READ_LIMIT {
            continue;
        }
        let Ok(header_text) = fs::read_to_string(&header) else {
            continue;
        };
        let mut include_evidence = None;
        for header_line in header_text.lines() {
            if needs_byte_alias {
                if let Some(alias) = c_unsigned_char_typedef_alias(header_line) {
                    if contains_c_identifier(source, &alias) {
                        let include_id = *include_evidence.get_or_insert_with(|| {
                            record_c_quote_include_evidence(lo, line_span, include)
                        });
                        let type_id = record_c_type_alias_evidence(
                            lo,
                            line_span,
                            &alias,
                            CTypeTarget::UnsignedInteger { bits: 8 },
                            vec![include_id],
                        );
                        lo.record_type_domain_alias_exact_with_evidence(
                            &alias,
                            DomainEvidence::ByteArray,
                            Some(type_id),
                        );
                    }
                }
            }
            if needs_unsigned_32_alias {
                if let Some(alias) = c_unsigned_32_typedef_alias(header_line) {
                    if contains_c_identifier(source, &alias) {
                        let include_id = *include_evidence.get_or_insert_with(|| {
                            record_c_quote_include_evidence(lo, line_span, include)
                        });
                        let type_id = record_c_type_alias_evidence(
                            lo,
                            line_span,
                            &alias,
                            CTypeTarget::UnsignedInteger { bits: 32 },
                            vec![include_id],
                        );
                        lo.record_unsigned_32_alias_with_evidence(&alias, Some(type_id));
                    }
                }
            }
        }
    }
}
pub(super) fn record_c_quote_include_evidence(
    lo: &mut Lowering,
    span: Span,
    include: &str,
) -> EvidenceId {
    lo.record_evidence(
        EvidenceAnchor::source_span(span),
        EvidenceKind::Import(ImportEvidenceKind::CQuoteInclude {
            include_hash: stable_symbol_hash(include),
        }),
        "c_quote_include",
    )
}
pub(super) fn record_c_type_alias_evidence(
    lo: &mut Lowering,
    span: Span,
    alias: &str,
    target: CTypeTarget,
    dependencies: Vec<EvidenceId>,
) -> EvidenceId {
    lo.record_evidence_with_dependencies(
        EvidenceAnchor::binding(span, stable_symbol_hash(alias)),
        EvidenceKind::Type(TypeEvidenceKind::CTypeAlias {
            alias_hash: stable_symbol_hash(alias),
            target,
        }),
        "c_type_alias",
        dependencies,
    )
}
pub(super) fn c_direct_quote_include_name(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let rest = line.strip_prefix('#')?.trim_start();
    let rest = rest.strip_prefix("include")?.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(&rest[..end])
}
pub(super) fn c_source_may_contain_u16_byte_pack(source: &str) -> bool {
    source.contains("[0]")
        && source.contains("[1]")
        && (source.contains("<<8") || source.contains("<< 8"))
}
pub(super) fn c_source_may_contain_u32_byte_pack(source: &str) -> bool {
    source.contains("[0]")
        && source.contains("[1]")
        && source.contains("[2]")
        && source.contains("[3]")
        && (source.contains("<<24") || source.contains("<< 24"))
}
