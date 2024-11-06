use std::{
    os::{
        fd::{FromRawFd, RawFd},
        unix::net::UnixStream,
    },
    ptr::NonNull,
};

use jni::{strings::JNIStr, sys::JNINativeMethod, JNIEnv};
use libc::{dev_t, ino_t};

use crate::{error::ZygiskError, impl_sealing::Sealed};

pub use crate::raw::v5::transparent::*;

#[derive(Clone, Copy)]
pub struct V5;

impl Sealed for V5 {}

impl super::ZygiskApi<'_, V5> {
    pub fn connect_companion(&self) -> Result<&mut UnixStream, ZygiskError> {
        let api_dispatch = unsafe { self.dispatch() };

        match unsafe { (api_dispatch.connect_companion_fn)(api_dispatch.base.this) } {
            -1 => Err(ZygiskError::ConnectCompanionError),
            fd => {
                let unix_stream = Box::new(unsafe { UnixStream::from_raw_fd(fd) });
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

    pub unsafe fn hook_jni_native_methods<'other_local, M: AsMut<[JNINativeMethod]>>(
        &self,
        env: JNIEnv,
        class_name: &'other_local JNIStr,
        mut methods: M,
    ) {
        let methods = methods.as_mut();

        (self.dispatch().hook_jni_native_methods_fn)(
            env,
            class_name.as_ptr(),
            NonNull::new_unchecked(methods.as_mut_ptr()),
            methods.len() as _,
        );
    }

    pub unsafe fn plt_hook_register<S: AsRef<str>>(
        &self,
        device: dev_t,
        inode: ino_t,
        symbol: S,
        replacement: NonNull<()>,
    ) -> NonNull<()> {
        let symbol = symbol.as_ref();

        let mut original = NonNull::dangling();

        (self.dispatch().plt_hook_register_fn)(
            device,
            inode,
            match symbol.is_empty() {
                true => std::ptr::null(),
                false => symbol.as_ptr().cast(),
            },
            replacement,
            Some(NonNull::new_unchecked(&mut original as *mut _)),
        );

        original
    }
}
