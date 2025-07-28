
use nodyn::nodyn;

nodyn! {
    #[derive(Debug)]
    pub enum Value {
        i32,
        String,
        f64,
    }
    impl introspection;
}

fn main() {
    assert_eq!(Value::count(), 3);
    assert_eq!(Value::types(), ["i32", "String", "f64"]);
    let val: Value = 42.into();
    assert_eq!(val.type_name(), "i32");
}
