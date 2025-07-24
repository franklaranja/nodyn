use core::convert::TryFrom;

nodyn::nodyn! {
  /// A test of the `nodyn!` macro
  #[derive(PartialEq, Debug, Clone)]
  pub enum Foo<'a> {
    i64,
    /// a &str
    &'a str,
    #[into(i64)]
    u32,
    [u8;4],
  }

  impl TryInto
}

fn main() {
    let t2: Foo = 66u32.into();
    assert_eq!(t2, Foo::U32(66));
    let r2 = u32::try_from(t2.clone());
    assert_eq!(r2, Ok(66u32));
    let r3 = i64::try_from(t2);
    assert_eq!(r3, Ok(66i64));
}
