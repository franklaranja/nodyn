nodyn::nodyn! {
    #[derive(Debug, Clone, PartialEq, PartialOrd, Copy)]
    pub enum Number {
        #[into(i16, i32, i64, i128, f32, f64)]
        i8,
        #[into(i16, u16, i32, u32, i64, u64, i128, u128, f32, f64)]
        u8,
        #[into(i32, i64, i128, f32, f64)]
        i16,
        #[into(i32, u32, i64, u64, i128, u128, f32, f64)]
        u16,
        #[into(i64, i128, f64)]
        i32,
        #[into(i64, u64, i128, u128, f64)]
        u32,
        #[into(i128)]
        i64,
        #[into(i128, u128)]
        u64,
        i128,
        u128,
        f32,
        f64,
    }

    impl TryInto;

    vec Numbers;
}

fn main() {
    let x = Number::I16(-42);
    let y: f64 = x.try_into().unwrap();
    let values = numbers![x, y, 67843u32, -56i16, 83.0f32];
    assert_eq!(values.len(), 5);
}
