#[test]
fn tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/01_generate_enum.rs");
    t.compile_fail("tests/02_empty_and_type_errors.rs");
    t.pass("tests/03_impl_from.rs");
    t.pass("tests/04_impl_try_from.rs");
    t.pass("tests/05_into_attribute.rs");
    t.pass("tests/06_impl_blocks.rs");
    t.pass("tests/07_trait_blocks.rs");
    t.pass("tests/08_named_variants.rs");
    t.pass("tests/09_type_functions.rs");
    t.pass("tests/10_advanced_example.rs");
    t.pass("tests/11_vec_wrapper.rs");

    t.pass("tests/enum/basic.rs");
    t.pass("tests/enum/custom_variants.rs");
    t.pass("tests/enum/from.rs");

    t.pass("tests/features/is_as.rs");
    t.pass("tests/features/introspection.rs");
    t.pass("tests/features/into_attribute.rs");

    t.pass("tests/vec_wrapper/standard.rs");
    t.pass("tests/vec_wrapper/custom.rs");
    t.pass("tests/vec_wrapper/macro.rs");

    t.compile_fail("tests/errors/invalid_impl.rs");
    t.compile_fail("tests/errors/double_type.rs");
}
