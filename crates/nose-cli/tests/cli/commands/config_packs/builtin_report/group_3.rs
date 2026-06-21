use super::*;

pub(super) fn assert_group(json: &serde_json::Value) {
    let js_collections =
        semantic_pack_by_id(&json, "nose.javascript.builtins.collection_constructors");
    assert_eq!(js_collections["hash"], "38f71dd71d3585c5");
    assert_eq!(js_collections["kind"], "StdlibPack");
    assert_eq!(
        js_collections["display_name"],
        "nose JavaScript builtins collection constructor pack"
    );
    assert_eq!(js_collections["source"], "compiled-builtin");
    assert_eq!(js_collections["influence"], "evidence-and-contracts");
    assert_eq!(js_collections["trust"], "builtin-default");
    assert_eq!(js_collections["enabled_by_default"], true);
    assert_eq!(js_collections["path"], serde_json::Value::Null);
    assert_eq!(js_collections["provider"], "Corca, Inc.");
    assert_eq!(
        js_collections["repository"],
        "https://github.com/corca-ai/nose"
    );
    assert_eq!(js_collections["license"], "MIT");
    assert_eq!(
        json_array_strings(js_collections, "supported_languages"),
        vec!["javascript", "typescript"]
    );
    assert_eq!(js_collections["counts"]["evidence_producers"], 1);
    assert_eq!(js_collections["counts"]["contracts"], 2);
    assert_eq!(js_collections["counts"]["value_laws"], 0);
    assert_eq!(js_collections["counts"]["positive_fixtures"], 2);
    assert_eq!(js_collections["counts"]["hard_negatives"], 3);

    let stdlib = semantic_pack_by_id(&json, "nose.python.stdlib.type_domain");
    assert_eq!(stdlib["kind"], "StdlibPack");
    assert_eq!(
        json_array_strings(stdlib, "supported_languages"),
        vec!["python"]
    );
    assert_eq!(stdlib["counts"]["evidence_producers"], 1);
    assert_eq!(stdlib["counts"]["contracts"], 1);

    let laws = semantic_pack_by_id(&json, "nose.value_graph.laws");
    assert_eq!(laws["kind"], "LawPack");
    assert_eq!(laws["source"], "compiled-builtin");
}
