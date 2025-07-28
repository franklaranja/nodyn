use nodyn::nodyn;

nodyn! {
    #[derive(Debug, PartialEq)]
    pub enum Custom {
        Number(i32),
        Text(String),
    }
}

fn main() {
    let num: Custom = 42.into();
    let text: Custom = "hello".to_string().into();
    assert_eq!(num, Custom::Number(42));
    assert_eq!(text, Custom::Text("hello".to_string()));
}
