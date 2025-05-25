// empty no longer causes errors (or anything else but waist)

nodyn::wrap! {
  enum TestTypes {
        *mut u32,
  }
}

fn main() {}
