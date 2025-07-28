use nodyn::nodyn;
use std::convert::TryFrom;

nodyn! {
    #[derive(Debug, PartialEq)]
    pub enum Value {
        i32,
        String,
    }
    impl TryInto;
}

fn main() {
    let num: Value = 42.into();
    let text: Value = "hello".to_string().into();
    assert_eq!(i32::try_from(num).unwrap(), 42);
    assert_eq!(String::try_from(text).unwrap(), "hello");
}
