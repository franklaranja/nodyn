nodyn::nodyn! {
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    pub enum Value<'a> {
        i32,
        bool,
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
    values.extend(vec![4, 1, 7345]);
    assert_eq!(values.len(), 8);

    let indexes = values
        .enumerate_str_ref()
        .map(|(i, _)| i)
        .collect::<Vec<usize>>();
    assert_eq!(indexes.len(), 3);
    values.sort();
    let search = values.binary_search(&Value::from(7345));
    assert_eq!(search, Ok(4));

    let bools = values![true, true, false, true, true];
    assert!(bools.all_bool());
    assert_eq!(bools.count_bool(), 5);
    assert_eq!(values.count_bool(), 0);
    values.push(42);
}
