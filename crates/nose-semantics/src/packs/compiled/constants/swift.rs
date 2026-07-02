use crate::{
    SWIFT_LANGUAGE_CORE_PRODUCER_ID, SWIFT_SOURCE_FACT_PRODUCER_ID,
    SWIFT_STDLIB_COLLECTION_FACTORY_ARRAY_CONTRACT_ID, SWIFT_STDLIB_COLLECTION_FACTORY_PRODUCER_ID,
    SWIFT_STDLIB_COLLECTION_FACTORY_SET_CONTRACT_ID,
    SWIFT_STDLIB_DICTIONARY_UNIQUE_KEYS_CONTRACT_ID,
};
use nose_il::Lang;

pub(in crate::packs::compiled) const SWIFT_BINDING_LANGS: &[Lang] = &[Lang::Swift];
pub(in crate::packs::compiled) const SWIFT_LANGUAGE_PRODUCER_IDS: &[&str] = &[
    SWIFT_LANGUAGE_CORE_PRODUCER_ID,
    SWIFT_SOURCE_FACT_PRODUCER_ID,
];
pub(in crate::packs::compiled) const SWIFT_LANGUAGE_SOURCE_FACT_PRODUCER_IDS: &[&str] =
    &[SWIFT_SOURCE_FACT_PRODUCER_ID];
pub(in crate::packs::compiled) const SWIFT_LANGUAGE: &[&str] = &["swift"];
pub(in crate::packs::compiled) const SWIFT_LANGUAGE_FILE_EXTENSIONS: &[&str] = &["swift"];

pub(in crate::packs::compiled) const SWIFT_STDLIB_COLLECTION_FACTORY_PACKAGES: &[&str] =
    &["Array", "Set", "Dictionary", "Swift"];
pub(in crate::packs::compiled) const SWIFT_STDLIB_COLLECTION_FACTORY_PRODUCER_IDS: &[&str] =
    &[SWIFT_STDLIB_COLLECTION_FACTORY_PRODUCER_ID];
pub(in crate::packs::compiled) const SWIFT_STDLIB_COLLECTION_FACTORY_CONTRACT_IDS: &[&str] = &[
    SWIFT_STDLIB_COLLECTION_FACTORY_ARRAY_CONTRACT_ID,
    SWIFT_STDLIB_COLLECTION_FACTORY_SET_CONTRACT_ID,
    SWIFT_STDLIB_DICTIONARY_UNIQUE_KEYS_CONTRACT_ID,
];
pub(in crate::packs::compiled) const SWIFT_STDLIB_COLLECTION_FACTORY_CONFORMANCE_REFS: &[&str] = &[
    "swift-array-sequence-factory-positive",
    "swift-set-sequence-factory-positive",
    "swift-dictionary-unique-keys-with-values-positive",
    "swift-array-shadowed-hard-negative",
    "swift-set-shadowed-hard-negative",
    "swift-dictionary-wrong-label-hard-negative",
    "swift-dictionary-implicit-entry-shape-hard-negative",
];
