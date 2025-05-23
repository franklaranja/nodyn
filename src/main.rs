use core::convert::TryFrom;

nodyn::wrap! {
  /// A test of the `wrap!` macro
  #[derive(PartialEq, Debug, Clone)]
  pub Foo<'a> {
    i64,
    /// a &str
    &'a str,
    #[into(i64)]
    u32,
    [u8;4],
    /// tuple
    (u8,u8,u8),
  }

  impl {
        pub fn foo(&self) {
            println!("foo");
        }

        #[skip(i64)]
        pub fn bar(&self, baz: &str) -> bool;

        pub fn baz(&self, baz: &str) -> bool;

    }

}

fn main() {
    let t1: Foo = (1, 2, 3).into();
    assert_eq!(t1, Foo::U8U8U8Tuple((1, 2, 3)));
    let r1 = <(u8, u8, u8)>::try_from(t1);
    println!("result: {r1:#?}");
}
