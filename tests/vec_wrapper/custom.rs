use nodyn::nodyn;

nodyn! {
    #[derive(Debug, Clone)]
    pub enum Value {
        i32,
        String,
    }

    #[vec(inner_vec)]
    #[derive(Debug, Clone)]
    pub struct CustomValues {
        metadata: String,
    }
}

fn main() {
    let mut values = CustomValues {
        metadata: "test".to_string(),
        inner_vec: vec![],
    };
    values.push(42);
    values.push("hello".to_string());
    assert_eq!(values.metadata, "test");
    assert_eq!(values.len(), 2);
    assert_eq!(values.first_i32(), Some(&42));
}
