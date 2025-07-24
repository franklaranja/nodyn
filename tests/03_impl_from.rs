nodyn::nodyn! {
  #[derive(PartialEq, Debug)]
  pub enum Foo<'a> {
    i64,
    &'a str,
    u32,
    [u8;4],
  }
}

fn main() {
    let t1: Foo = "hello world".into();
    assert_eq!(t1, Foo::StrRef("hello world"));
    let t2: Foo = 66u32.into();
    assert_eq!(t2, Foo::U32(66));
}
