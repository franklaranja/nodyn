use core::convert::TryFrom;

nodyn::nodyn! {
    #[derive(PartialEq, Debug)]
    pub enum Foo<'a> {
        i64,
        &'a str,
        u32,
        [u8;4],
    }
    impl TryInto;
}

fn main() {
    let t2: Foo = 42u32.into();
    assert_eq!(t2, Foo::U32(42));
    let r2 = u32::try_from(t2);
    assert_eq!(r2, Ok(42u32));
}
