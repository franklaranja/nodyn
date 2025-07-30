nodyn::nodyn! {
    #[derive(Debug, Clone)]
    pub enum Generic<'a, T> {
        Vec<T>,
        &'a str,
    }

    impl {
        fn len(&self) -> usize;

    }
}

fn main() {
    let x: Generic<String> = vec!["hi".to_string()].into();
    let y: Generic<f64> = vec![42.0, 3.1].into();
    let z: Generic<f64> = "hello".into();
    assert_eq!(x.len(), 1);
    assert_eq!(y.len(), 2);
    assert_eq!(z.len(), 5);
}
