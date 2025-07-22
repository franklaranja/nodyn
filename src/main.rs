nodyn::nodyn! {
  #[derive(Debug, PartialEq, Clone)]
  pub enum Foo<'a> {
    &'a str,
    u32,
    f64,
  }

    impl From;

  impl vec;

  impl vec Bar;

  #[vec_wrapper]
  #[derive(Debug, Default)]
  pub struct MyVec<'b> {
        pub foo: &'b str,
  }
}

fn main() {
    let mut test = MyVec::default();
    test.push(33u32);
    test.push(42u32);
    test.push("hello");
    assert_eq!(test.get(0), Some(&Foo::U32(33)));
    assert_eq!(test[0], Foo::U32(33));
    for x in &test {
        println!("{x:?}");
    }

    for x in &mut test {
        if x == &Foo::StrRef("hello") {
            *x = "hello world".into();
        }
    }
    for x in test {
        println!("{x:?}");
    }
}
