use std::{
    os::{
        fd::{FromRawFd, RawFd},
        unix::net::UnixStream,
    },
    ptr,
};

use jni::{strings::JNIStr, sys::JNINativeMethod, JNIEnv};

use crate::{error::ZygiskError, impl_sealing::Sealed};

use super::ZygiskSpec;

pub use crate::raw::v2::transparent::*;

#[derive(Clone, Copy)]
pub struct V2;

impl Sealed for V2 {}

impl ZygiskSpec for V2 {
    type Spec = V2;
}

impl<'local> super::ZygiskApi<'local, V2> {
    pub fn connect_companion(&'local self) -> Result<UnixStream, ZygiskError> {
        match self
            .as_tbl()
            .connect_companion_fn
            .map(|f| f(self.as_tbl().this))
            .unwrap_or(-1)
        {
            -1 => Err(ZygiskError::ConnectCompanionError),
            fd => Ok(unsafe { UnixStream::from_raw_fd(fd) }),
        }
    }

    pub fn get_module_dir(&'local self) -> RawFd {
        self.as_tbl().get_module_dir_fn.unwrap()(self.as_tbl().this)
    }

    pub fn set_option(&'local self, option: ZygiskOption) {
        if let Some(f) = self.as_tbl().set_option_fn {
            f(self.as_tbl().this, option);
        }
    }

    pub fn get_flags(&'local self) -> StateFlags {
        self.as_tbl()
            .get_flags_fn
            .map(|f| f(self.as_tbl().this))
            .map(|raw| StateFlags::from_bits(raw).expect("unsupported flag returned by Zygisk"))
            .unwrap_or(StateFlags::empty())
    }

    pub unsafe fn hook_jni_native_methods<'other_local, M: AsMut<[JNINativeMethod]>>(
        &'local self,
        env: JNIEnv,
        class_name: &'other_local JNIStr,
        mut methods: M,
    ) {
        let methods = methods.as_mut();

        if let Some(func) = self.as_tbl().hook_jni_native_methods_fn {
            func(
                env.get_native_interface(),
                class_name.as_ptr(),
                methods.as_mut_ptr(),
                methods.len() as _,
            );
        }
    }

    pub unsafe fn plt_hook_register<S: AsRef<str>>(
        &'local self,
        regex: S,
        symbol: S,
        new_func: *mut (),
        old_func: Option<*mut *mut ()>,
    ) {
        if let Some(func) = self.as_tbl().plt_hook_register_fn {
            func(
                regex.as_ref().as_ptr() as *const _,
                symbol.as_ref().as_ptr() as *const _,
                new_func,
                old_func.unwrap_or(ptr::null_mut()),
            );
        }
    }

    pub unsafe fn plt_hook_exclude<S: AsRef<str>>(&'local self, regex: S, symbol: S) {
        if let Some(func) = self.as_tbl().plt_hook_exclude_fn {
            func(
                regex.as_ref().as_ptr() as *const _,
                symbol.as_ref().as_ptr() as *const _,
            );
        }
    }

    pub fn plt_hook_commit(&'local self) -> Result<(), ZygiskError> {
        match self.as_tbl().plt_hook_commit_fn.map(|f| f()) {
            Some(true) => Ok(()),
            _ => Err(ZygiskError::PltHookCommitError),
        }
    }
}
