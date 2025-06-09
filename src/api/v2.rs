use core::
    ptr::{self, NonNull}
;
use std::os::{
    fd::{FromRawFd, RawFd},
    unix::net::UnixStream,
};

use jni::{strings::JNIStr, sys::JNINativeMethod, JNIEnv};
use static_alloc::Bump;
use without_alloc::{alloc::LocalAllocLeakExt, Box};

use crate::{error::ZygiskError, impl_sealing::Sealed};

pub use crate::raw::v2::transparent::*;

#[derive(Clone, Copy)]
pub struct V2;

impl Sealed for V2 {}

impl super::ZygiskApi<'_, V2> {
    pub fn connect_companion(&self) -> Result<&mut UnixStream, ZygiskError> {
        let api_dispatch = unsafe { self.dispatch() };

        match unsafe { (api_dispatch.connect_companion_fn)(api_dispatch.base.this) } {
            -1 => Err(ZygiskError::ConnectCompanionError),
            fd => {
                static UNIXSTREAM_SLAB: Bump<[UnixStream; 1]> =
                    const { Bump::uninit() };
                let unix_stream = UNIXSTREAM_SLAB
                    .boxed(unsafe { UnixStream::from_raw_fd(fd) })
                    .unwrap();
                Ok(Box::leak(unix_stream))
            }
        }
    }

    pub fn get_module_dir(&self) -> RawFd {
        let api_dispatch = unsafe { self.dispatch() };

        unsafe { (api_dispatch.get_module_dir_fn)(api_dispatch.base.this) }
    }

    pub fn set_option(&self, option: ZygiskOption) {
        let api_dispatch = unsafe { self.dispatch() };

        unsafe { (api_dispatch.set_option_fn)(api_dispatch.base.this, option) }
    }

    pub fn get_flags(&self) -> Result<StateFlags, ZygiskError> {
        let api_dispatch = unsafe { self.dispatch() };

        match StateFlags::from_bits(unsafe { (api_dispatch.get_flags_fn)(api_dispatch.base.this) })
        {
            Some(flags) => Ok(flags),
            None => Err(ZygiskError::UnrecognizedStateFlag),
        }
    }

    /// # Safety
    ///
    pub unsafe fn hook_jni_native_methods<M: AsMut<[JNINativeMethod]>>(
        &self,
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
    pub unsafe fn plt_hook_register<S: AsRef<str>>(
        &self,
        regex: S,
        symbol: S,
        new_func: NonNull<()>,
    ) -> NonNull<()> {
        let regex = regex.as_ref();
        let symbol = symbol.as_ref();

        let mut old_func = NonNull::dangling();

        unsafe {
            (self.dispatch().plt_hook_register_fn)(
                regex.as_ptr().cast(),
                match symbol.is_empty() {
                    true => ptr::null(),
                    false => symbol.as_ptr().cast(),
                },
                new_func,
                Some(NonNull::new_unchecked(&mut old_func as *mut _)),
            )
        };

        old_func
    }

    /// # Safety
    ///
    pub unsafe fn plt_hook_exclude<S: AsRef<str>>(&self, regex: S, symbol: S) {
        let regex = regex.as_ref();
        let symbol = symbol.as_ref();

        unsafe {
            (self.dispatch().plt_hook_exclude_fn)(
                regex.as_ptr().cast(),
                match symbol.is_empty() {
                    true => ptr::null(),
                    false => symbol.as_ptr().cast(),
                },
            );
        }
    }

    pub fn plt_hook_commit(&self) -> Result<(), ZygiskError> {
        match unsafe { (self.dispatch().plt_hook_commit_fn)() } {
            true => Ok(()),
            _ => Err(ZygiskError::PltHookCommitError),
        }
    }
}
