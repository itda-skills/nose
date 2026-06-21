use super::*;

mod group_0;
mod group_1;
mod group_2;
mod group_3;

#[test]
fn builtin_pack_descriptors_enumerate_declarations_and_conformance_refs() {
    group_0::assert_group();
    group_1::assert_group();
    group_2::assert_group();
    group_3::assert_group();
}
