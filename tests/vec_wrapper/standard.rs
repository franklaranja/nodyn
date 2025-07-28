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
    let mut values = ValueVec::default();
    values.push(42);
    values.push("hello".to_string());
    assert_eq!(values.len(), 2);
    assert_eq!(values.first_i32(), Some(&42));
    assert_eq!(values[1], Value::String("hello".to_string()));
    values.dedup();
    assert_eq!(values.len(), 2); // No duplicates
}
