// this file is for dev only, changes all the time

nodyn::wrap! {
  /// A test of the `wrap!` macro
  #[derive(Debug, PartialEq)]
  pub Foo<'a, 'b> {
        Bar<'a>,
        Bax(Xee<'b>),
  }

  impl {
        #[must_use]
        pub const fn foo(&self) -> &'static str {
            "Foo"
        }
  }

  trait Say {
        fn say(&self, f: &str) -> String;
  }
}

#[derive(Debug, PartialEq)]
pub struct Bar<'a>(&'a str);

impl Say for Bar<'_> {
    fn say(&self, s: &str) -> String {
        format!("{s} {}", self.0)
    }
}

#[derive(Debug, PartialEq)]
pub struct Xee<'a>(&'a str, &'a str);

impl Say for Xee<'_> {
    fn say(&self, s: &str) -> String {
        format!("{s} {} {}", self.0, self.1)
    }
}

pub trait Say {
    fn say(&self, s: &str) -> String;
}

fn main() {
    let b: Foo = Bar("world").into();
    assert_eq!(b.say("hello"), "hello world".to_string());
    assert_eq!(b.foo(), "Foo");
    let x: Foo = Xee("big", "world").into();
    assert_eq!(x.say("hello"), "hello big world".to_string());
    assert_eq!(x.foo(), "Foo");
    assert_eq!(x, Foo::Bax(Xee("big", "world")));
}
