use nodyn::nodyn;

nodyn! {
    #[derive(Debug, PartialEq)]
    pub enum Value {
        i32,
        String,
        f64,
    }
}

fn main() {
    let values: Vec<Value> = vec![42.into(), "hello".to_string().into(), 3.14.into()];
    assert_eq!(values[0], Value::I32(42));
    assert_eq!(values[1], Value::String("hello".to_string()));
    assert_eq!(values[2], Value::F64(3.14));
}
