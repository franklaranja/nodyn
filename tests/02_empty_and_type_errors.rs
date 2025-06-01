// empty no longer causes errors (or anything else but waist)

nodyn::nodyn! {
  enum TestTypes {
        *mut u32,
  }
}

fn main() {}
