use std::fmt;

#[derive(Debug, Clone)]
pub struct Null;

impl fmt::Display for Null {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "null")
    }
}

#[derive(Debug, Clone)]
pub struct JsonArray(JsonValueVec);

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

    vec;

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
    let values = json_value_vec![
        Null,                // null
        true,                // boolean
        42.0,                // number
        "hello".to_string(), // string
        JsonArray(json_value_vec![Null, false, 33.0, "world".to_string(),]),
    ];
    assert_eq!(values.len(), 5);
}
