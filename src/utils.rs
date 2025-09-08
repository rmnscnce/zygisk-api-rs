use core::mem;

pub struct ShapeAssertion<T, U>(T, U);
impl<T, U> ShapeAssertion<T, U> {
    pub const ASSERT: () = {
        assert!(mem::size_of::<T>() == mem::size_of::<U>(), "size mismatch");
        assert!(
            mem::align_of::<T>() % mem::align_of::<U>() == 0,
            "incorrect alignment"
        );
    };
}
