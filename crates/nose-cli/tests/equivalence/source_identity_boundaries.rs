use super::*;

#[test]
fn python_docstrings_are_function_semantic_noops() {
    let i = Interner::new();
    let plain = "def f(i, j):\n    if i == j:\n        return 1\n    return 0\n";
    let docstring = "def g(i, j):\n    \"\"\"Return one when the indexes match.\"\"\"\n    if i == j:\n        return 1\n    else:\n        return 0\n";
    let other_docstring = "def h(i, j):\n    \"\"\"Different documentation text.\"\"\"\n    if i == j:\n        return 1\n    return 0\n";

    assert_eq!(
        value_fp(&i, plain, Lang::Python),
        value_fp(&i, docstring, Lang::Python),
        "a Python function docstring must not change call behavior"
    );
    assert_eq!(
        value_fp(&i, plain, Lang::Python),
        value_fp(&i, other_docstring, Lang::Python),
        "docstring text is metadata, not function return behavior"
    );

    let returned_red = "def f():\n    return \"red\"\n";
    let returned_blue = "def g():\n    return \"blue\"\n";
    assert_ne!(
        value_fp(&i, returned_red, Lang::Python),
        value_fp(&i, returned_blue, Lang::Python),
        "returned strings are behavior-defining values"
    );

    let f_string = "def f(x):\n    f\"{x}\"\n    return 1\n";
    let no_effect = "def g(x):\n    return 1\n";
    assert_ne!(
        value_fp(&i, f_string, Lang::Python),
        value_fp(&i, no_effect, Lang::Python),
        "a leading f-string expression is not a static docstring proof"
    );
}

#[test]
fn import_named_and_namespace_member_coordinates_converge() {
    let i = Interner::new();
    let js_named = "import { helper } from \"./shared-math\";\nfunction f(value) { return helper(value + 1); }\n";
    let js_namespace = "import * as mathOps from \"./shared-math\";\nfunction f(value) { return mathOps.helper(value + 1); }\n";
    let js_wrong_member = "import * as mathOps from \"./shared-math\";\nfunction f(value) { return mathOps.otherHelper(value + 1); }\n";
    let ts_named = "import { helper } from \"./shared-math\";\nfunction f(value: number): number { return helper(value + 1); }\n";
    let ts_namespace = "import * as mathOps from \"./shared-math\";\nfunction f(value: number): number { return mathOps.helper(value + 1); }\n";
    let ts_type_only =
        "import type { helper } from \"./shared-math\";\nfunction f(value: number): number { return helper(value + 1); }\n";
    let ts_mixed_type_only = "import { helper, type otherHelper } from \"./shared-math\";\nfunction f(value: number): number { return otherHelper(value + 1); }\n";
    let py_named =
        "from shared_math import helper\n\ndef f(value):\n    return helper(value + 1)\n";
    let py_namespace =
        "import shared_math as math_ops\n\ndef f(value):\n    return math_ops.helper(value + 1)\n";
    let py_wrong_member =
        "import shared_math as math_ops\n\ndef f(value):\n    return math_ops.other_helper(value + 1)\n";

    let fp = value_fp(&i, js_named, Lang::JavaScript);
    assert_eq!(fp, value_fp(&i, js_namespace, Lang::JavaScript));
    assert_ne!(fp, value_fp(&i, js_wrong_member, Lang::JavaScript));

    let ts_fp = value_fp(&i, ts_named, Lang::TypeScript);
    assert_eq!(ts_fp, value_fp(&i, ts_namespace, Lang::TypeScript));
    assert_ne!(fp, ts_fp);
    assert_ne!(fp, value_fp(&i, ts_type_only, Lang::TypeScript));
    assert_ne!(fp, value_fp(&i, ts_mixed_type_only, Lang::TypeScript));

    let py_fp = value_fp(&i, py_named, Lang::Python);
    assert_eq!(py_fp, value_fp(&i, py_namespace, Lang::Python));
    assert_ne!(py_fp, value_fp(&i, py_wrong_member, Lang::Python));
}

#[test]
fn js_namespace_imports_ignore_parameter_shadow_mutations_only() {
    let i = Interner::new();
    let plain = r#"
import * as path from "node:path";

export const replaceRootDirInPath = (rootDir: string, filePath: string): string => {
  if (!filePath.startsWith("<rootDir>")) {
    return filePath;
  }

  return path.resolve(
    rootDir,
    path.normalize(`./${filePath.slice("<rootDir>".length)}`),
  );
};
"#;
    let shadowed_param = r#"
import * as path from "node:path";

export const escapeGlobCharacters = (path: string): string =>
  path.replaceAll(/([!()*?[\\\]{}])/g, "\\$1");

export const replaceRootDirInPath = (rootDir: string, filePath: string): string => {
  if (!filePath.startsWith("<rootDir>")) {
    return filePath;
  }

  return path.resolve(
    rootDir,
    path.normalize(`./${filePath.slice("<rootDir>".length)}`),
  );
};
"#;
    let unshadowed_non_mutating_js_method = r#"
import * as path from "node:path";

export const touchPath = (): void => {
  path.replaceAll("x", "y");
};

export const replaceRootDirInPath = (rootDir: string, filePath: string): string => {
  if (!filePath.startsWith("<rootDir>")) {
    return filePath;
  }

  return path.resolve(
    rootDir,
    path.normalize(`./${filePath.slice("<rootDir>".length)}`),
  );
};
"#;
    let fp = value_fp_named(&i, plain, Lang::TypeScript, "replaceRootDirInPath");
    assert_eq!(
        fp,
        value_fp_named(&i, shadowed_param, Lang::TypeScript, "replaceRootDirInPath"),
        "a parameter named like the namespace import must not taint the module binding"
    );
    assert_eq!(
        fp,
        value_fp_named(
            &i,
            unshadowed_non_mutating_js_method,
            Lang::TypeScript,
            "replaceRootDirInPath"
        ),
        "a Java-only mutation-like method name must not taint a TypeScript namespace import"
    );
}

#[test]
fn java_arrays_aslist_single_argument_respects_array_provenance() {
    let i = Interner::new();
    let array_membership = "import java.util.Arrays;\n\nclass C { static boolean f(String[] values, String value) { return Arrays.asList(values).contains(value); } }\n";
    let list_membership = "import java.util.Arrays;\nimport java.util.List;\n\nclass C { static boolean f(List<String> values, String value) { return Arrays.asList(values).contains(value); } }\n";
    let singleton_list_membership = "import java.util.List;\n\nclass C { static boolean f(String[] values, String value) { return List.of(values).contains(value); } }\n";
    let parameter_shadowed_arrays = "import java.util.Arrays;\n\nclass FakeArrays { java.util.List<String> asList(String[] values) { return java.util.List.of(\"green\"); } }\nclass C { static boolean f(FakeArrays Arrays, String[] values, String value) { return Arrays.asList(values).contains(value); } }\n";

    let array_fp = value_fp(&i, array_membership, Lang::Java);
    assert_ne!(array_fp, value_fp(&i, list_membership, Lang::Java));
    assert_ne!(
        array_fp,
        value_fp(&i, singleton_list_membership, Lang::Java)
    );
    assert_ne!(
        array_fp,
        value_fp(&i, parameter_shadowed_arrays, Lang::Java)
    );
}

#[test]
fn typed_empty_checks_keep_array_collection_and_string_domains_distinct() {
    let i = Interner::new();
    let java_list_size =
        "class C { static boolean f(java.util.List<Integer> values) { return values == null || values.size() == 0; } }\n";
    let java_list_named =
        "class C { static boolean f(java.util.List<Integer> values) { return values == null || values.isEmpty(); } }\n";
    let java_queue_named = "import java.util.Queue;\n\nclass C { static boolean f(Queue<String> values) { return values == null || values.isEmpty(); } }\n";
    let java_array_length =
        "class C { static boolean f(Object[] values) { return values == null || values.length == 0; } }\n";
    let java_string_named =
        "class C { static boolean f(String value) { return value == null || value.isEmpty(); } }\n";

    let list_fp = value_fp(&i, java_list_size, Lang::Java);
    assert_eq!(list_fp, value_fp(&i, java_list_named, Lang::Java));
    assert_eq!(list_fp, value_fp(&i, java_queue_named, Lang::Java));
    assert_ne!(list_fp, value_fp(&i, java_array_length, Lang::Java));
    assert_ne!(list_fp, value_fp(&i, java_string_named, Lang::Java));
    assert_ne!(
        value_fp(&i, java_array_length, Lang::Java),
        value_fp(&i, java_string_named, Lang::Java)
    );
}

#[test]
fn swift_import_identity_uses_module_and_export_coordinates() {
    let i = Interner::new();
    let imported = r#"
import Shared

func f(_ value: Int) -> Int {
    return Shared.helper(value + 1)
}
"#;
    let renamed = r#"
import Shared

func g(_ input: Int) -> Int {
    return Shared.helper(input + 1)
}
"#;
    let no_import = r#"
func f(_ value: Int) -> Int {
    return Shared.helper(value + 1)
}
"#;
    let wrong_module = r#"
import Other

func f(_ value: Int) -> Int {
    return Other.helper(value + 1)
}
"#;
    let wrong_member = r#"
import Shared

func f(_ value: Int) -> Int {
    return Shared.other(value + 1)
}
"#;

    let fp = value_fp(&i, imported, Lang::Swift);
    assert_eq!(
        fp,
        value_fp(&i, renamed, Lang::Swift),
        "Swift imported namespace calls should be stable under local alpha-renaming"
    );
    assert_ne!(
        fp,
        value_fp(&i, no_import, Lang::Swift),
        "an imported module coordinate must require the import statement"
    );
    assert_ne!(
        fp,
        value_fp(&i, wrong_module, Lang::Swift),
        "changing the imported module coordinate changes the callee identity"
    );
    assert_ne!(
        fp,
        value_fp(&i, wrong_member, Lang::Swift),
        "changing the imported export/member coordinate changes the callee identity"
    );
}
