// this file is for dev only, changes all the time

nodyn::nodyn! {
  /// A test of the `nodyn!` macro
  #[derive(Debug, PartialEq)]
  pub enum Foo<'a> {
    i64,
    &'a str,
    u32,
    [u8;4],
  }
}

fn main() {
    let t: Foo = "hello world".into();
    assert_eq!(Foo::count(), 4usize);
    assert_eq!(Foo::types(), ["i64", "&'a str", "u32", "[u8; 4]"]);
    assert_eq!(t.ty(), "&'a str")
}
