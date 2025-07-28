
use nodyn::nodyn;

nodyn! {
    pub enum Value {
        FirstName(String),
        LastName(String),
    }
}

fn main() {}
