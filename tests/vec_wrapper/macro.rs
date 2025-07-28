use nodyn::nodyn;

nodyn! {
    #[derive(Debug, Clone, PartialEq)]
    pub enum Value {
        i32,
        String,
    }
    vec;
}

fn main() {
    let values = value_vec![42, "hello".to_string()];
    assert_eq!(values.len(), 2);
    assert_eq!(values[0], Value::I32(42));
    assert_eq!(values[1], Value::String("hello".to_string()));
}
