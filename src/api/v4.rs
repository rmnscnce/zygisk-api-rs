use core::{
    ops::Deref,
    ptr::{self, NonNull},
};
use std::{
    ffi,
    os::{
        fd::{FromRawFd, RawFd},
        unix::net::UnixStream,
    },
};

use jni::{JNIEnv, strings::JNIStr, sys::JNINativeMethod};
use libc::{dev_t, ino_t};

use crate::{error::ZygiskError, impl_sealing::Sealed, utils};

pub use crate::raw::v4::transparent::*;

#[derive(Clone, Copy)]
pub struct V4;

impl Sealed for V4 {}

impl super::ZygiskApi<'_, V4> {
    pub fn with_companion<F, R>(
        &mut self,
        f: impl FnOnce(&mut UnixStream) -> R,
    ) -> Result<R, ZygiskError> {
        let api_dispatch = unsafe { self.dispatch() };

        match unsafe { (api_dispatch.connect_companion_fn)(api_dispatch.base.this) } {
            -1 => Err(ZygiskError::ConnectCompanionError),
            fd => {
                let mut companion_sock = unsafe { UnixStream::from_raw_fd(fd) };
                Ok(f(&mut companion_sock))
            }
        }
    }

    pub fn get_module_dir(&self) -> RawFd {
        let api_dispatch = unsafe { self.dispatch() };

        unsafe { (api_dispatch.get_module_dir_fn)(api_dispatch.base.this) }
    }

    pub fn set_option(&mut self, option: ZygiskOption) {
        let api_dispatch = unsafe { self.dispatch() };

        unsafe { (api_dispatch.set_option_fn)(api_dispatch.base.this, option) }
    }

    pub fn get_flags(&self) -> Result<StateFlags, ZygiskError> {
        let api_dispatch = unsafe { self.dispatch() };

        let flags = unsafe { (api_dispatch.get_flags_fn)(api_dispatch.base.this) };

        match StateFlags::from_bits(flags) {
            Some(flags) => Ok(flags),
            None => Err(ZygiskError::UnrecognizedStateFlag(flags)),
        }
    }

    /// # Safety
    ///
    pub unsafe fn hook_jni_native_methods(
        &mut self,
        env: JNIEnv,
        class_name: impl Deref<Target = JNIStr>,
        mut methods: impl AsMut<[JNINativeMethod]>,
    ) {
        let class_name = class_name.deref();
        let methods = methods.as_mut();

        unsafe {
            (self.dispatch().hook_jni_native_methods_fn)(
                env,
                class_name.as_ptr(),
                NonNull::new_unchecked(methods.as_mut_ptr()),
                methods.len() as _,
            )
        };
    }

    /// # Safety
    ///
    pub unsafe fn plt_hook_register<FnPtr>(
        &mut self,
        device: dev_t,
        inode: ino_t,
        symbol: impl AsRef<str>,
        replacement: NonNull<FnPtr>,
    ) -> Result<NonNull<FnPtr>, ZygiskError> {
        let symbol = symbol.as_ref();
        // constexpr assertion <FnPtr>
        let _: () = utils::ShapeAssertion::<FnPtr, extern "C" fn()>::ASSERT;

        let symbol = match symbol.is_empty() {
            true => ptr::null(),
            false => match ffi::CString::new(symbol) {
                Ok(symbol) => symbol.as_bytes_with_nul().as_ptr().cast(),
                Err(e) => return Err(ZygiskError::IncompatibleWithCStr(e)),
            },
        };

        let original = NonNull::dangling();

        unsafe {
            (self.dispatch().plt_hook_register_fn)(
                device,
                inode,
                symbol,
                replacement.as_ptr().cast(),
                Some(NonNull::new_unchecked(original.as_ptr() as *mut _)),
            )
        };

        Ok(original)
    }
}
