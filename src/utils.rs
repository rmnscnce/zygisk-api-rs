use core::{cell, marker, mem, ops, ptr};

pub struct ShapeAssertion<T, U>(T, U);
impl<T, U> ShapeAssertion<T, U> {
    pub const ASSERT: () = const {
        assert!(mem::size_of::<T>() == mem::size_of::<U>());
        assert!(mem::align_of::<T>() % mem::align_of::<U>() == 0);
    };
}

pub struct Local<T>(cell::UnsafeCell<mem::MaybeUninit<T>>, cell::Cell<bool>);

impl<T> Local<T> {
    pub const fn new() -> Self {
        Self(
            cell::UnsafeCell::new(mem::MaybeUninit::uninit()),
            cell::Cell::new(false),
        )
    }

    pub const fn boxed(&self, val: T) -> LocalBox<T> {
        let mem_place = unsafe { &mut *self.0.get() };
        let _ = mem_place.write(val);
        let _ = self.1.replace(true);

        LocalBox(
            unsafe { ptr::NonNull::new_unchecked(mem_place.as_mut_ptr()) },
            PhantomLifetime::DEFAULT,
        )
    }

    pub fn boxed_with<F>(&self, f: F) -> LocalBox<T>
    where
        F: FnOnce() -> T,
    {
        self.boxed(f())
    }
}

impl<T> Default for Local<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Local<T>
where
    T: Default,
{
    pub fn default_boxed(&self) -> LocalBox<T> {
        self.boxed(T::default())
    }
}

impl<T> Drop for Local<T> {
    fn drop(&mut self) {
        if self.1.get() {
            // # Safety: We have determined that the value is initialized
            unsafe {
                (&mut *self.0.get()).assume_init_drop();
            }
        }
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
    pub const fn as_ptr(this: &Self) -> *const T {
        this.0.as_ptr() as *const _
    }

    pub const fn as_mut_ptr(this: &Self) -> *mut T {
        this.0.as_ptr()
    }

    pub const fn into_raw(this: Self) -> *mut T {
        Self::as_mut_ptr(&this)
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
