use crate::{CSHARP_LANGUAGE_CORE_PRODUCER_ID, CSHARP_SOURCE_FACT_PRODUCER_ID};
use nose_il::Lang;

pub(in crate::packs::compiled) const CSHARP_BINDING_LANGS: &[Lang] = &[Lang::CSharp];
pub(in crate::packs::compiled) const CSHARP_LANGUAGE_PRODUCER_IDS: &[&str] = &[
    CSHARP_LANGUAGE_CORE_PRODUCER_ID,
    CSHARP_SOURCE_FACT_PRODUCER_ID,
];
pub(in crate::packs::compiled) const CSHARP_LANGUAGE_SOURCE_FACT_PRODUCER_IDS: &[&str] =
    &[CSHARP_SOURCE_FACT_PRODUCER_ID];
pub(in crate::packs::compiled) const CSHARP_LANGUAGE: &[&str] = &["csharp"];
pub(in crate::packs::compiled) const CSHARP_LANGUAGE_FILE_EXTENSIONS: &[&str] = &["cs"];
