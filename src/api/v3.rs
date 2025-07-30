use core::{ffi, ptr::NonNull};
use std::os::{
    fd::{FromRawFd, RawFd},
    unix::net::UnixStream,
};

use jni::{JNIEnv, strings::JNIStr, sys::JNINativeMethod};

use crate::{error::ZygiskError, impl_sealing::Sealed, utils};

pub use crate::raw::v3::transparent::*;

#[derive(Clone, Copy)]
pub struct V3;

impl Sealed for V3 {}

impl super::ZygiskApi<'_, V3> {
    pub fn with_companion<R>(
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
    pub unsafe fn hook_jni_native_methods<M: AsMut<[JNINativeMethod]>>(
        &mut self,
        env: JNIEnv,
        class_name: &JNIStr,
        mut methods: M,
    ) {
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
    pub unsafe fn plt_hook_register<S, FnPtr>(
        &mut self,
        regex: S,
        symbol: S,
        replacement: NonNull<FnPtr>,
    ) -> NonNull<FnPtr>
    where
        S: AsRef<ffi::CStr>,
    {
        let regex = regex.as_ref();
        let symbol = symbol.as_ref();
        // constexpr assertion <FnPtr>
        let _: () = utils::ShapeAssertion::<FnPtr, extern "C" fn()>::ASSERT;

        let old_func = NonNull::dangling();

        unsafe {
            (self.dispatch().plt_hook_register_fn)(
                regex.to_bytes_with_nul().as_ptr().cast(),
                symbol.to_bytes_with_nul().as_ptr().cast(),
                replacement.as_ptr().cast(),
                Some(NonNull::new_unchecked(old_func.as_ptr() as *mut _)),
            )
        };

        old_func
    }

    /// # Safety
    ///
    pub unsafe fn plt_hook_exclude<S>(&mut self, regex: S, symbol: S)
    where
        S: AsRef<ffi::CStr>,
    {
        let regex = regex.as_ref();
        let symbol = symbol.as_ref();

        unsafe {
            (self.dispatch().plt_hook_exclude_fn)(
                regex.to_bytes_with_nul().as_ptr().cast(),
                symbol.to_bytes_with_nul().as_ptr().cast(),
            )
        }
    }

    pub fn plt_hook_commit(&mut self) -> Result<(), ZygiskError> {
        match unsafe { (self.dispatch().plt_hook_commit_fn)() } {
            true => Ok(()),
            false => Err(ZygiskError::PltHookCommitError),
        }
    }
}
