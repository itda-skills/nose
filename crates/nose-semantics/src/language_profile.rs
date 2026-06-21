//! First-party language profile facade.

use super::*;

pub const C_LANGUAGE_PACK_ID: &str = "nose.lang.c";
pub const PYTHON_LANGUAGE_PACK_ID: &str = "nose.lang.python";
pub const JS_TS_LANGUAGE_PACK_ID: &str = "nose.lang.javascript-typescript";
pub const GO_LANGUAGE_PACK_ID: &str = "nose.lang.go";
pub const RUST_LANGUAGE_PACK_ID: &str = "nose.lang.rust";
pub const JAVA_LANGUAGE_PACK_ID: &str = "nose.lang.java";
pub const RUBY_LANGUAGE_PACK_ID: &str = "nose.lang.ruby";
pub const SWIFT_LANGUAGE_PACK_ID: &str = "nose.lang.swift";
pub const CSS_LANGUAGE_PACK_ID: &str = "nose.lang.css";
pub const HTML_EMBEDDED_LANGUAGE_PACK_ID: &str = "nose.lang.html";
pub const PYTHON_SOURCE_FACT_PRODUCER_ID: &str = "python.source.fact";
pub const JS_TS_SOURCE_FACT_PRODUCER_ID: &str = "javascript-typescript.source.fact";
pub const GO_SOURCE_FACT_PRODUCER_ID: &str = "go.source.fact";
pub const RUST_SOURCE_FACT_PRODUCER_ID: &str = "rust.source.fact";
pub const JAVA_SOURCE_FACT_PRODUCER_ID: &str = "java.source.fact";
pub const C_SOURCE_FACT_PRODUCER_ID: &str = "c.source.fact";
pub const RUBY_SOURCE_FACT_PRODUCER_ID: &str = "ruby.source.fact";
pub const SWIFT_SOURCE_FACT_PRODUCER_ID: &str = "swift.source.fact";
pub const CSS_SOURCE_FACT_PRODUCER_ID: &str = "css.source.fact";
pub const HTML_EMBEDDED_SOURCE_FACT_PRODUCER_ID: &str = "html-embedded.source.fact";
pub const C_UNSIGNED_32_CAST_SOURCE_PRODUCER_ID: &str = "c.source.cast.unsigned32";

/// A first-party language profile. Keep this cheap and copyable; callers use it as a
/// named semantic boundary around currently-supported language behavior.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LanguageProfile {
    lang: Lang,
}

pub fn semantics(lang: Lang) -> LanguageProfile {
    LanguageProfile { lang }
}

pub fn language_source_fact_provenance(lang: Lang) -> (&'static str, &'static str) {
    match lang {
        Lang::Python => (PYTHON_LANGUAGE_PACK_ID, PYTHON_SOURCE_FACT_PRODUCER_ID),
        Lang::JavaScript | Lang::TypeScript => {
            (JS_TS_LANGUAGE_PACK_ID, JS_TS_SOURCE_FACT_PRODUCER_ID)
        }
        Lang::Go => (GO_LANGUAGE_PACK_ID, GO_SOURCE_FACT_PRODUCER_ID),
        Lang::Rust => (RUST_LANGUAGE_PACK_ID, RUST_SOURCE_FACT_PRODUCER_ID),
        Lang::Java => (JAVA_LANGUAGE_PACK_ID, JAVA_SOURCE_FACT_PRODUCER_ID),
        Lang::C => (C_LANGUAGE_PACK_ID, C_SOURCE_FACT_PRODUCER_ID),
        Lang::Ruby => (RUBY_LANGUAGE_PACK_ID, RUBY_SOURCE_FACT_PRODUCER_ID),
        Lang::Swift => (SWIFT_LANGUAGE_PACK_ID, SWIFT_SOURCE_FACT_PRODUCER_ID),
        Lang::Css => (CSS_LANGUAGE_PACK_ID, CSS_SOURCE_FACT_PRODUCER_ID),
        Lang::Vue | Lang::Svelte | Lang::Html => (
            HTML_EMBEDDED_LANGUAGE_PACK_ID,
            HTML_EMBEDDED_SOURCE_FACT_PRODUCER_ID,
        ),
    }
}

impl LanguageProfile {
    pub fn lang(self) -> Lang {
        self.lang
    }

    /// Whether the language is dynamically typed — a bare parameter carries no static type, so
    /// it could be a float at runtime (#342). Used to decide that an untyped `+`/`*` chain is
    /// POSSIBLY float and must not be reassociated (float `+`/`*` is non-associative). The
    /// statically-typed languages (Rust/Go/C/Java) instead carry per-param domain evidence, so
    /// their float-ness is decided by the proven domain, not by this.
    pub fn is_dynamically_typed(self) -> bool {
        matches!(
            self.lang,
            Lang::Python
                | Lang::Ruby
                | Lang::JavaScript
                | Lang::TypeScript
                | Lang::Vue
                | Lang::Svelte
                | Lang::Html
        )
    }

    pub fn pack_id(self) -> &'static str {
        match self.lang {
            Lang::C => C_LANGUAGE_PACK_ID,
            _ => FIRST_PARTY_PACK_ID,
        }
    }

    pub fn trust(self) -> PackTrust {
        PackTrust::DefaultFirstParty
    }

    pub fn operators(self) -> OperatorSemantics {
        OperatorSemantics { lang: self.lang }
    }

    pub fn effects(self) -> EffectSemantics {
        EffectSemantics { lang: self.lang }
    }

    pub fn modules(self) -> ModuleSemantics {
        ModuleSemantics { lang: self.lang }
    }

    pub fn stdlib(self) -> StdlibSemantics {
        StdlibSemantics { lang: self.lang }
    }

    pub fn collections(self) -> CollectionSemantics {
        CollectionSemantics { lang: self.lang }
    }

    pub fn exact_fragments(self) -> FragmentSemantics {
        FragmentSemantics { lang: self.lang }
    }
}

pub(crate) fn js_like_lang(lang: Lang) -> bool {
    matches!(
        lang,
        Lang::JavaScript | Lang::TypeScript | Lang::Vue | Lang::Svelte | Lang::Html
    )
}
