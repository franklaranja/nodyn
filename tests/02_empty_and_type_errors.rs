nodyn::wrap! {
  #[derive(Debug)]
  TestEmpty {
  }
}

nodyn::wrap! {
  #[derive(Debug)]
  TestTypes<'a> {
        Foo,
        &'a str,
        (i32, bool),
  }
}

#[derive(Debug)]
struct Foo;

fn main() {}
