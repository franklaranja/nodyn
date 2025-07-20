nodyn::nodyn! {
  #[derive(Debug)]
  pub enum Foo<'a> {
    I64,
    Str<'a>,
    F64,
    u32,
  }

  impl vec;

  impl vec Bar;
  #[vec_wrapper]
  pub struct MyVec<'b> {
        pub foo: &'b str,
  }
}

#[derive(Debug)]
pub struct I64(pub i64);

#[derive(Debug)]
pub struct Str<'a>(pub &'a str);

#[derive(Debug)]
pub struct F64(pub f64);

fn main() {}
