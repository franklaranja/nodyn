use nodyn::nodyn;

nodyn! {
    #[derive(Debug, PartialEq)]
    pub enum Container {
        String,
        Vec<u8>,
    }
    impl is_as;
}

fn main() {
    let container: Container = "hello".to_string().into();
    assert!(container.is_string());
    assert!(!container.is_vec_u8());
    if let Some(s) = container.try_as_string_ref() {
        assert_eq!(s, "hello");
    }
    let mut container: Container = vec![1u8, 2].into();
    if let Some(v) = container.try_as_vec_u8_mut() {
        v.push(3);
    }
    assert_eq!(container, Container::VecU8(vec![1, 2, 3]));
}
