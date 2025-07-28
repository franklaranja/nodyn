use nodyn::nodyn;

nodyn! {
    pub enum Value {
        i32,
    }
    impl InvalidFeature; // Unknown feature
}

fn main() {}
