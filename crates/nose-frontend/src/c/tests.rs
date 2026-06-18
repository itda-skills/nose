use super::*;
use nose_il::EvidenceKind;

/// Collect every `Op` carried by a `UnOp` node in the lowered IL.
fn unary_ops(src: &str) -> Vec<Op> {
    let interner = Interner::new();
    let il = lower(FileId(0), "t.c", src.as_bytes(), &interner).expect("lower");
    il.nodes
        .iter()
        .filter(|n| n.kind == NodeKind::UnOp)
        .filter_map(|n| match n.payload {
            Payload::Op(op) => Some(op),
            _ => None,
        })
        .collect()
}

#[test]
fn unary_operators_lower_to_distinct_ops() {
    // Each C unary operator must lower to its own `Op`; in particular unary
    // plus is `Pos`, not `Neg` (the two were once indistinguishable).
    let ops = unary_ops("int f(int x){ int a=+x; int b=-x; int c=!x; int d=~x; return 0; }");
    assert!(
        ops.contains(&Op::Pos),
        "unary + should lower to Op::Pos, got {ops:?}"
    );
    assert!(
        ops.contains(&Op::Neg),
        "unary - should lower to Op::Neg, got {ops:?}"
    );
    assert!(
        ops.contains(&Op::Not),
        "unary ! should lower to Op::Not, got {ops:?}"
    );
    assert!(
        ops.contains(&Op::BitNot),
        "unary ~ should lower to Op::BitNot, got {ops:?}"
    );
}

#[test]
fn unary_plus_and_minus_are_not_aliased() {
    // `+x` and `-x` must not collapse to the same operator.
    assert_eq!(unary_ops("int f(int x){ return +x; }"), vec![Op::Pos]);
    assert_eq!(unary_ops("int f(int x){ return -x; }"), vec![Op::Neg]);
}

#[test]
fn unsigned_32_byte_lane_cast_emits_source_cast_evidence() {
    let interner = Interner::new();
    let il = lower(
        FileId(0),
        "t.c",
        b"typedef unsigned char u8;\ntypedef unsigned int u32;\nu32 f(const u8 *a){ return ((u32)a[0]) << 24; }",
        &interner,
    )
    .expect("lower");

    let u8_type = il.evidence.iter().find(|record| {
        matches!(
            record.kind,
            EvidenceKind::Type(TypeEvidenceKind::CTypeAlias {
                alias_hash,
                target: CTypeTarget::UnsignedInteger { bits: 8 },
            }) if alias_hash == stable_symbol_hash("u8")
        )
    });
    assert!(u8_type.is_some(), "u8 typedef must emit Type evidence");

    let u32_type = il.evidence.iter().find(|record| {
        matches!(
            record.kind,
            EvidenceKind::Type(TypeEvidenceKind::CTypeAlias {
                alias_hash,
                target: CTypeTarget::UnsignedInteger { bits: 32 },
            }) if alias_hash == stable_symbol_hash("u32")
        )
    });
    let u32_type = u32_type.expect("u32 typedef must emit Type evidence");

    let cast = il
        .evidence
        .iter()
        .find(|record| {
            record.kind == EvidenceKind::Source(SourceFactKind::Cast(SourceCastKind::CUnsigned32))
        })
        .expect("C unsigned 32-bit byte-lane casts must emit source evidence");
    assert_eq!(
        cast.dependencies,
        vec![u32_type.id],
        "alias-based unsigned casts should depend on the alias Type proof"
    );
}

#[test]
fn direct_quote_include_aliases_emit_import_type_and_dependent_facts() {
    let interner = Interner::new();
    let dir = std::env::temp_dir().join(format!(
        "nose_c_include_alias_evidence_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("bytes.h"),
        "typedef unsigned char u8;\ntypedef unsigned int u32;\n",
    )
    .unwrap();
    let source = "#include \"bytes.h\"\nu32 f(const u8 *a){ return (((u32)a[0]) << 24) | (((u32)a[1]) << 16) | (((u32)a[2]) << 8) | ((u32)a[3]); }\n";
    let il = lower(
        FileId(0),
        dir.join("main.c").to_str().unwrap(),
        source.as_bytes(),
        &interner,
    )
    .expect("lower");

    let include = il
        .evidence
        .iter()
        .find(|record| {
            record.kind
                == EvidenceKind::Import(ImportEvidenceKind::CQuoteInclude {
                    include_hash: stable_symbol_hash("bytes.h"),
                })
        })
        .expect("quote include must emit Import evidence");
    let u8_type = il
        .evidence
        .iter()
        .find(|record| {
            matches!(
                record.kind,
                EvidenceKind::Type(TypeEvidenceKind::CTypeAlias {
                    alias_hash,
                    target: CTypeTarget::UnsignedInteger { bits: 8 },
                }) if alias_hash == stable_symbol_hash("u8")
            )
        })
        .expect("included u8 alias must emit Type evidence");
    assert_eq!(u8_type.dependencies, vec![include.id]);
    let u32_type = il
        .evidence
        .iter()
        .find(|record| {
            matches!(
                record.kind,
                EvidenceKind::Type(TypeEvidenceKind::CTypeAlias {
                    alias_hash,
                    target: CTypeTarget::UnsignedInteger { bits: 32 },
                }) if alias_hash == stable_symbol_hash("u32")
            )
        })
        .expect("included u32 alias must emit Type evidence");
    assert_eq!(u32_type.dependencies, vec![include.id]);

    let domain = il
        .evidence
        .iter()
        .find(|record| record.kind == EvidenceKind::Domain(DomainEvidence::ByteArray))
        .expect("u8 pointer parameter must emit ByteArray domain evidence");
    assert_eq!(domain.dependencies, vec![u8_type.id]);
    let cast = il
        .evidence
        .iter()
        .find(|record| {
            record.kind == EvidenceKind::Source(SourceFactKind::Cast(SourceCastKind::CUnsigned32))
        })
        .expect("included u32 cast alias must emit Source cast evidence");
    assert_eq!(cast.dependencies, vec![u32_type.id]);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn direct_quote_include_alias_scan_is_not_hardcoded_to_u8_u32_names() {
    let interner = Interner::new();
    let dir = std::env::temp_dir().join(format!(
        "nose_c_include_generic_alias_evidence_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("bytes.h"),
        "typedef unsigned char byte;\ntypedef uint32_t word;\n",
    )
    .unwrap();
    let source = "#include \"bytes.h\"\nword f(const byte *a){ return (((word)a[0]) << 24) | (((word)a[1]) << 16) | (((word)a[2]) << 8) | ((word)a[3]); }\n";
    let il = lower(
        FileId(0),
        dir.join("main.c").to_str().unwrap(),
        source.as_bytes(),
        &interner,
    )
    .expect("lower");

    let byte_type = il
        .evidence
        .iter()
        .find(|record| {
            matches!(
                record.kind,
                EvidenceKind::Type(TypeEvidenceKind::CTypeAlias {
                    alias_hash,
                    target: CTypeTarget::UnsignedInteger { bits: 8 },
                }) if alias_hash == stable_symbol_hash("byte")
            )
        })
        .expect("included byte alias must emit Type evidence");
    let word_type = il
        .evidence
        .iter()
        .find(|record| {
            matches!(
                record.kind,
                EvidenceKind::Type(TypeEvidenceKind::CTypeAlias {
                    alias_hash,
                    target: CTypeTarget::UnsignedInteger { bits: 32 },
                }) if alias_hash == stable_symbol_hash("word")
            )
        })
        .expect("included word alias must emit Type evidence");

    let domain = il
        .evidence
        .iter()
        .find(|record| record.kind == EvidenceKind::Domain(DomainEvidence::ByteArray))
        .expect("byte pointer parameter must emit ByteArray domain evidence");
    assert_eq!(domain.dependencies, vec![byte_type.id]);
    let cast = il
        .evidence
        .iter()
        .find(|record| {
            record.kind == EvidenceKind::Source(SourceFactKind::Cast(SourceCastKind::CUnsigned32))
        })
        .expect("included word cast alias must emit Source cast evidence");
    assert_eq!(cast.dependencies, vec![word_type.id]);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn byte_array_alias_must_denote_a_plain_type_not_a_struct_tag_or_param_name() {
    let interner = Interner::new();
    let il = lower(
        FileId(0),
        "t.c",
        b"typedef unsigned char u8;\nint f(struct u8 *u8){ return (u8[0] << 8) | u8[1]; }",
        &interner,
    )
    .expect("lower");

    assert!(
        !il.evidence
            .iter()
            .any(|record| record.kind == EvidenceKind::Domain(DomainEvidence::ByteArray)),
        "struct tags or parameter names must not satisfy a typedef alias proof"
    );
}

#[test]
fn scalar_pointer_param_does_not_emit_integer_domain() {
    let interner = Interner::new();
    let il = lower(
        FileId(0),
        "t.c",
        b"int f(int *xs){ return xs[0]; }",
        &interner,
    )
    .expect("lower");

    assert!(
        !il.evidence
            .iter()
            .any(|record| record.kind == EvidenceKind::Domain(DomainEvidence::Integer)),
        "C pointer parameters must not inherit scalar integer domain evidence"
    );
}

/// Collect every `Op` carried by a `BinOp` node in the lowered IL.
fn binops(src: &str) -> Vec<Op> {
    let interner = Interner::new();
    let il = lower(FileId(0), "t.c", src.as_bytes(), &interner).expect("lower");
    il.nodes
        .iter()
        .filter(|n| n.kind == NodeKind::BinOp)
        .filter_map(|n| match n.payload {
            Payload::Op(op) => Some(op),
            _ => None,
        })
        .collect()
}

fn switch_case_rhs_ints(src: &str) -> Vec<i64> {
    let interner = Interner::new();
    let il = lower(FileId(0), "t.c", src.as_bytes(), &interner).expect("lower");
    il.nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.kind == NodeKind::BinOp && n.payload == Payload::Op(Op::Eq))
        .filter_map(|(idx, _)| {
            let kids = il.children(NodeId(idx as u32));
            match kids {
                [_, rhs] => match il.node(*rhs).payload {
                    Payload::LitInt(value) => Some(value),
                    _ => None,
                },
                _ => None,
            }
        })
        .collect()
}

fn expr_stmt_ints(src: &str) -> Vec<i64> {
    let interner = Interner::new();
    let il = lower(FileId(0), "t.c", src.as_bytes(), &interner).expect("lower");
    il.nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.kind == NodeKind::ExprStmt)
        .filter_map(|(idx, _)| {
            let kids = il.children(NodeId(idx as u32));
            match kids {
                [expr] => match il.node(*expr).payload {
                    Payload::LitInt(value) => Some(value),
                    _ => None,
                },
                _ => None,
            }
        })
        .collect()
}

#[test]
fn switch_cases_compare_scrutinee_to_case_literals() {
    let src = "int f(int x){ switch(x){ case 7: return 1; case 8: return 2; default: return 3; } }";
    assert_eq!(switch_case_rhs_ints(src), vec![7, 8]);
    assert!(
        expr_stmt_ints(src).is_empty(),
        "case labels should not remain as stray expression statements"
    );
}

#[test]
fn postfix_increment_with_nested_decrement_in_operand() {
    // `a[i--]++` desugars to `a[i--] = a[i--] + 1`: the OUTER op is increment
    // (`+ 1`). Detecting `--` anywhere in the node text misread the nested `i--`
    // and flipped the outer op to decrement; the operator token, not the text,
    // decides it.
    let ops = binops("int f(){ int a[10]; int i=0; a[i--]++; return 0; }");
    assert!(
        ops.contains(&Op::Add),
        "outer `++` must lower to Op::Add despite the nested `i--`, got {ops:?}"
    );
}
