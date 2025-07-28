use nodyn::nodyn;
use std::convert::TryFrom;

nodyn! {
    pub enum Foo {
        i64,
        #[into(i64)]
        i32,
    }
    impl TryInto;
}

fn main() {
    let foo: Foo = 42i32.into();
    assert_eq!(i64::try_from(foo).unwrap(), 42i64);
}
