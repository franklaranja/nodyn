use std::fmt;

#[derive(Debug, Clone)]
pub struct Null;

impl fmt::Display for Null {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "null")
    }
}

#[derive(Debug, Clone)]
pub struct JsonArray(Vec<JsonValue>);

impl fmt::Display for JsonArray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self
            .0
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "[{s}]")
    }
}

nodyn::nodyn! {
    #[derive(Debug, Clone)]
    pub enum JsonValue {
        Null,
        bool,
        f64,
        String,
        JsonArray,
    }

    impl fmt::Display {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    }

    impl {
        // Custom method
        const fn json_type_name(&self) -> &'static str {
            match self {
                Self::Null(_) => "null",
                Self::Bool(_) => "boolean",
                Self::F64(_) => "number",
                Self::String(_) => "string",
                Self::JsonArray(_) => "array",
            }
        }
    }
}

fn main() {
    let values: Vec<JsonValue> = vec![
        Null.into(),                // null
        true.into(),                // boolean
        42.0.into(),                // number
        "hello".to_string().into(), // string
        JsonArray(vec![
            Null.into(),
            false.into(),
            33.0.into(),
            "world".to_string().into(),
        ])
        .into(),
    ];

    for val in &values {
        println!("{}: {}", val.json_type_name(), val);
    }

    // output
    // null: null
    // boolean: true
    // number: 42
    // string: hello
    // array: [null, false, 33, world]
}
