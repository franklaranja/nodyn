nodyn::nodyn! {
  /// A test of the `nodyn!` macro
  pub enum Foo<'a, 'b> {
        Bar<'a>,
        Xee<'b>,
  }

  impl {
        #[must_use]
        pub const fn foo(&self) -> &'static str {
            "Foo"
        }

        pub fn say(&self, f: &str) -> String;

    }

}

pub struct Bar<'a>(&'a str);

impl Bar<'_> {
    fn say(&self, s: &str) -> String {
        format!("{s} {}", self.0)
    }
}

pub struct Xee<'a>(&'a str, &'a str);

impl Xee<'_> {
    fn say(&self, s: &str) -> String {
        format!("{s} {} {}", self.0, self.1)
    }
}

fn main() {
    let b: Foo = Bar("world").into();
    assert_eq!(b.say("hello"), "hello world".to_string());
    assert_eq!(b.foo(), "Foo");
    let x: Foo = Xee("big", "world").into();
    assert_eq!(x.say("hello"), "hello big world".to_string());
    assert_eq!(x.foo(), "Foo");
}
