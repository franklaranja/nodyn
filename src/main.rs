// this file is for dev only, changes all the time
use std::fmt;

nodyn::nodyn! {
  /// A test of the `nodyn!` macro
  #[derive(Debug, PartialEq)]
  pub enum Foo {
    i64,
    Null,
  }

    impl fmt::Display {
       fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    }
}

#[derive(Debug, PartialEq)]
pub struct Null;

impl fmt::Display for Null {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "null")
    }
}
fn main() {}
