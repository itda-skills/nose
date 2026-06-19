use super::support::{lower_ts_with_interner, raw_names};

#[test]
fn ts_object_opaque_surfaces_do_not_cascade_raw_wrappers() {
    let (il, interner) = lower_ts_with_interner(
        r#"
function build(rest: object, key: string, target: Record<string, number>) {
  return {
    ...rest,
    [key]: 1,
    run(value: number) {
      return void value;
    },
    drop() {
      return delete target[key];
    }
  };
}
"#,
    );
    let raw = raw_names(&il, &interner);
    for unexpected in [
        "spread_element",
        "method_definition",
        "formal_parameters",
        "statement_block",
        "return_statement",
        "computed_property_name",
        "void",
        "delete",
    ] {
        assert!(
            !raw.iter().any(|name| name == unexpected),
            "{unexpected} should lower to exact-closed structured IL, got {raw:?}"
        );
    }
}
