use std::{
    os::{fd::FromRawFd, unix::net::UnixStream},
    ptr,
};

use jni::{strings::JNIStr, sys::JNINativeMethod, JNIEnv};

use crate::{error::ZygiskError, impl_sealing::Sealed};

use super::ZygiskSpec;

pub use crate::raw::v1::transparent::*;

#[derive(Clone, Copy)]
pub struct V1;

impl Sealed for V1 {}

impl ZygiskSpec for V1 {
    type Spec = V1;
}

impl<'local> super::ZygiskApi<'local, V1> {
    /// Connect to a root companion process and get a Unix domain socket for IPC.
    ///
    /// This API only works in the `pre[XXX]Specialize` functions due to SELinux restrictions.
    ///
    /// The `pre[XXX]Specialize` functions run with the same privilege of zygote.
    /// If you would like to do some operations with superuser permissions, register a handler
    /// function that would be called in the root process with `zygisk_companion!(handler_func)`.
    /// Another good use case for a companion process is that if you want to share some resources
    /// across multiple processes, hold the resources in the companion process and pass it over.
    ///
    /// The root companion process is ABI aware; that is, when calling this function from a 32-bit
    /// process, you will be connected to a 32-bit companion process, and vice versa for 64-bit.
    ///
    /// Returns a [UnixStream] that is connected to the socket passed to your module's companion
    /// request handler. Returns `Err` if the connection attempt failed.
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

    /// Set various options for your module.
    /// Please note that this function accepts one single option at a time.
    /// Check [ZygiskOption] for the full list of options available.
    pub fn set_option(&'local self, option: ZygiskOption) {
        if let Some(f) = self.as_tbl().set_option_fn {
            f(self.as_tbl().this, option);
        }
    }

    /// Hook JNI native methods for a Java class.
    ///
    /// This looks up all registered JNI native methods and replaces them with your own functions.
    /// The original function pointer will be saved in each `JNINativeMethod`'s `fnPtr` (thus the
    /// `&mut` requirement in the function signature).
    ///
    /// If no matching class, method name, or signature is found, that specific `JNINativeMethod.fnPtr`
    /// will be set to [std::ptr::null_mut()].
    ///
    /// ## Safety
    ///
    /// This function is unsafe, since a badly designed hook or misuse of raw pointers may lead to
    /// memory unsafety.
    pub unsafe fn hook_jni_native_methods<M: AsMut<[JNINativeMethod]>>(
        &'local self,
        env: JNIEnv<'local>,
        class_name: &'local JNIStr,
        mut methods: M,
    ) {
        let methods = methods.as_mut();

        if let Some(func) = self.as_tbl().hook_jni_native_methods_fn {
            func(
                env,
                class_name.as_ptr(),
                methods.as_mut_ptr(),
                methods.len() as _,
            );
        }
    }

    /// Hook functions in the PLT (Procedure Linkage Table) of ELFs loaded in memory.
    ///
    /// Parsing `/proc/[PID]/maps` will give you the memory map of a process. As an example:
    ///
    /// ```text
    ///       <address>       <perms>  <offset>   <dev>  <inode>           <pathname>
    /// 56b4346000-56b4347000  r-xp    00002000   fe:00    235       /system/bin/app_process64
    /// ```
    /// (More details: https://man7.org/linux/man-pages/man5/proc.5.html)
    ///
    /// The `dev` and `inode` pair uniquely identifies a file being mapped into memory.
    /// For matching ELFs loaded in memory, replace function `symbol` with `new_func`.
    /// If `old_func` is not [`None`], the original function pointer will be saved to `old_func`.
    ///
    /// ## Safety
    ///
    /// This function is unsafe, since a badly designed hook or misuse of raw pointers may lead to
    /// memory unsafety.
    pub unsafe fn plt_hook_register<S: AsRef<str>>(
        &'local self,
        regex: S,
        symbol: S,
        new_func: *mut (),
        old_func: Option<*mut *mut ()>,
    ) {
        let regex = regex.as_ref();
        let symbol = symbol.as_ref();

        if let Some(func) = self.as_tbl().plt_hook_register_fn {
            func(
                regex.as_ptr().cast(),
                symbol.as_ptr().cast(),
                new_func,
                old_func.unwrap_or(ptr::null_mut()),
            );
        }
    }

    /// For ELFs loaded in memory matching `regex`, exclude hooks registered for `symbol`.
    /// If symbol is empty, then all symbols will be excluded.
    pub fn plt_hook_exclude<S: AsRef<str>>(&'local self, regex: S, symbol: S) {
        let regex = regex.as_ref();
        let symbol = symbol.as_ref();

        if let Some(func) = self.as_tbl().plt_hook_exclude_fn {
            func(
                regex.as_ptr().cast(),
                match symbol.is_empty() {
                    true => ptr::null(),
                    false => symbol.as_ptr().cast(),
                },
            );
        }
    }

    /// Commit all the hooks that was previously registered.
    ///
    /// Returns [`ZygiskError::PltHookCommitError`] if any error occurs.
    pub fn plt_hook_commit(&'local self) -> Result<(), ZygiskError> {
        match self.as_tbl().plt_hook_commit_fn.map(|f| f()) {
            Some(true) => Ok(()),
            _ => Err(ZygiskError::PltHookCommitError),
        }
    }
}
