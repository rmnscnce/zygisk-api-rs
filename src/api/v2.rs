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

impl super::ZygiskApi<'_, V2> {
    pub fn connect_companion(&self) -> Result<UnixStream, ZygiskError> {
        let api_dispatch = unsafe { self.dispatch() };

        match api_dispatch
            .connect_companion_fn
            .map(|f| f(api_dispatch.this))
            .unwrap_or(-1)
        {
            -1 => Err(ZygiskError::ConnectCompanionError),
            fd => Ok(unsafe { UnixStream::from_raw_fd(fd) }),
        }
    }

    pub fn get_module_dir(&self) -> RawFd {
        let api_dispatch = unsafe { self.dispatch() };

        api_dispatch.get_module_dir_fn.unwrap()(api_dispatch.this)
    }

    pub fn set_option(&self, option: ZygiskOption) {
        let api_dispatch = unsafe { self.dispatch() };

        if let Some(f) = api_dispatch.set_option_fn {
            f(api_dispatch.this, option);
        }
    }

    pub fn get_flags(&self) -> StateFlags {
        let api_dispatch = unsafe { self.dispatch() };

        api_dispatch
            .get_flags_fn
            .map(|f| f(api_dispatch.this))
            .map(|raw| StateFlags::from_bits(raw).expect("unsupported flag returned by Zygisk"))
            .unwrap_or(StateFlags::empty())
    }

    pub unsafe fn hook_jni_native_methods<'other_local, M: AsMut<[JNINativeMethod]>>(
        &self,
        env: JNIEnv,
        class_name: &'other_local JNIStr,
        mut methods: M,
    ) {
        let methods = methods.as_mut();

        if let Some(func) = self.dispatch().hook_jni_native_methods_fn {
            func(
                env.get_native_interface(),
                class_name.as_ptr(),
                methods.as_mut_ptr(),
                methods.len() as _,
            );
        }
    }

    pub unsafe fn plt_hook_register<S: AsRef<str>>(
        &self,
        regex: S,
        symbol: S,
        new_func: *mut (),
        old_func: Option<*mut *mut ()>,
    ) {
        if let Some(func) = self.dispatch().plt_hook_register_fn {
            func(
                regex.as_ref().as_ptr() as *const _,
                symbol.as_ref().as_ptr() as *const _,
                new_func,
                old_func.unwrap_or(ptr::null_mut()),
            );
        }
    }

    pub unsafe fn plt_hook_exclude<S: AsRef<str>>(&self, regex: S, symbol: S) {
        if let Some(func) = self.dispatch().plt_hook_exclude_fn {
            func(
                regex.as_ref().as_ptr() as *const _,
                symbol.as_ref().as_ptr() as *const _,
            );
        }
    }

    pub fn plt_hook_commit(&self) -> Result<(), ZygiskError> {
        match unsafe { self.dispatch() }.plt_hook_commit_fn.map(|f| f()) {
            Some(true) => Ok(()),
            _ => Err(ZygiskError::PltHookCommitError),
        }
    }
}
