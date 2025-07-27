nodyn::nodyn! {
    #[derive(Debug, Clone, PartialEq, PartialOrd)]
    pub enum Value<'a> {
        i32,
        bool,
        f64,
        &'a str,
    }
    vec Values;
}

fn main() {
    let mut values = values![2, "a", 5, "foo", "z"];
    assert_eq!(values.first_str_ref(), Some(&"a"));
    if let Some(s) = values.last_str_ref_mut() {
        *s = "omega";
    }
    assert_eq!(values.last_str_ref(), Some(&"omega"));
    assert_eq!(values.last_i32(), Some(&5));
    assert!(values.any_i32());
    assert!(!values.any_f64());
    values.extend(vec![4.3, 1.61, 7345.2]);
    assert!(values.any_f64());

    let indexes = values
        .enumerate_str_ref()
        .map(|(i, _)| i)
        .collect::<Vec<usize>>();
    assert_eq!(indexes.len(), 3);
    let bools = values![true, true, false, true, true];
    assert!(bools.all_bool());
    assert_eq!(bools.count_bool(), 5);
    assert_eq!(values.count_bool(), 0);
    values.push(42);
}
