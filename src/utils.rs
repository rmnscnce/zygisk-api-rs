use core::{cell, marker, mem, ops, ptr};

pub struct ShapeAssertion<T, U>(T, U);
impl<T, U> ShapeAssertion<T, U> {
    pub const ASSERT: () = const {
        assert!(mem::size_of::<T>() == mem::size_of::<U>());
        assert!(mem::align_of::<T>() % mem::align_of::<U>() == 0);
    };
}

pub struct Local<T>(cell::UnsafeCell<mem::MaybeUninit<T>>);

impl<T> Local<T> {
    pub const fn uninit() -> Self {
        Self(cell::UnsafeCell::new(mem::MaybeUninit::uninit()))
    }

    pub fn boxed(&self, val: T) -> LocalBox<T> {
        let mem_place = unsafe { &mut *self.0.get() };
        let _ = mem_place.write(val);

        LocalBox(
            unsafe { ptr::NonNull::new_unchecked((&mut *self.0.get()).as_mut_ptr()) },
            PhantomLifetime::DEFAULT,
        )
    }
}

unsafe impl<T> Sync for Local<T> {}

unsafe impl<T> Send for Local<T> where T: Send {}

#[derive(Default, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhantomLifetime<'a>(marker::PhantomData<&'a fn(&'a ())>);

impl PhantomLifetime<'_> {
    const DEFAULT: Self = Self(marker::PhantomData);
}

pub struct LocalBox<'local, T>(ptr::NonNull<T>, PhantomLifetime<'local>)
where
    T: ?Sized;

impl<'local, T> LocalBox<'local, T>
where
    T: ?Sized,
{
    pub const fn into_raw(this: Self) -> *mut T {
        this.0.as_ptr()
    }

    pub const fn leak<'a>(this: Self) -> &'a mut T
    where
        'local: 'a,
    {
        unsafe { &mut *Self::into_raw(this) }
    }
}

impl<'local, T> ops::Deref for LocalBox<'local, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}
