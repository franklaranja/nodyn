use core::convert::TryFrom;

nodyn::wrap! {
  /// A test of the `wrap!` macro
  #[derive(PartialEq, Debug)]
  pub Foo<'a> {
    i64,
    &'a str,
    u32,
    [u8;4],
  }

}

fn main() {
    let t2: Foo = 66u32.into();
    assert_eq!(t2, Foo::U32(66));
    let r2 = u32::try_from(t2);
    assert_eq!(r2, Ok(66u32));
}
