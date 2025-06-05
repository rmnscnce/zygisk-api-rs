#![warn(clippy::std_instead_of_core)]

use core::ptr::NonNull;

use api::ZygiskApi;
use jni::JNIEnv;
use raw::{RawApiTable, RawModuleAbi, ZygiskRaw};

pub mod api;
mod aux;
pub use aux::*;
use static_alloc::Bump;
use without_alloc::{alloc::LocalAllocLeakExt, Box};
pub mod error;
pub mod raw;

pub(crate) mod impl_sealing {
    pub trait Sealed {}
}

pub trait ZygiskModule<Version>
where
    for<'a> Version: ZygiskRaw<'a>,
{
    fn on_load(&self, _: ZygiskApi<'_, Version>, _: JNIEnv<'_>) {}

    fn pre_app_specialize<'a>(
        &self,
        _: ZygiskApi<'a, Version>,
        _: JNIEnv<'a>,
        _: &'a mut <Version as ZygiskRaw<'_>>::AppSpecializeArgs,
    ) {
    }

    fn post_app_specialize<'a>(
        &self,
        _: ZygiskApi<'a, Version>,
        _: JNIEnv<'a>,
        _: &'a <Version as ZygiskRaw<'_>>::AppSpecializeArgs,
    ) {
    }

    fn pre_server_specialize<'a>(
        &self,
        _: ZygiskApi<'a, Version>,
        _: JNIEnv<'a>,
        _: &'a mut <Version as ZygiskRaw<'_>>::ServerSpecializeArgs,
    ) {
    }

    fn post_server_specialize<'a>(
        &self,
        _: ZygiskApi<'a, Version>,
        _: JNIEnv<'a>,
        _: &'a <Version as ZygiskRaw<'_>>::ServerSpecializeArgs,
    ) {
    }
}

#[doc(hidden)]
pub fn module_entry<'a, Version, ModuleImpl>(
    dispatch: &'a ModuleImpl,
    api_table: RawApiTable<'a, Version>,
    jni_env: JNIEnv<'a>,
) where
    for<'b> Version: ZygiskRaw<'b>,
    ModuleImpl: ZygiskModule<Version>,
{
    // Raw module vtable and ABI has a fixed size, which was done by design to retain compatibility across interface versions
    // That's why we can pin the ABI and module vtable on version 1, and use it for all versions

    static RAW_MODULE_SLAB: Bump<[raw::RawModule<'static, api::V1>; 2]> = const { Bump::uninit() };

    let raw_module = Box::leak(
        RAW_MODULE_SLAB
            .boxed(raw::RawModule::<'a> {
                dispatch,
                api_table,
                jni_env: unsafe { jni_env.unsafe_clone() },
            })
            .unwrap(),
    );

    static ABI_SLAB: Bump<[raw::ModuleAbi<'static, api::V1>; 2]> = const { Bump::uninit() };
    let abi = RawModuleAbi::from_non_null(unsafe {
        NonNull::new_unchecked(Box::leak(
            ABI_SLAB
                .boxed(Version::abi_from_module(raw_module))
                .unwrap(),
        ))
    });

    if unsafe { Version::register_module_fn(api_table.0.as_ref())(api_table.0, abi) } {
        let api = ZygiskApi::<Version>(api_table);
        dispatch.on_load(api, jni_env);
    }
}

#[macro_export]
macro_rules! register_module {
    ($module:expr) => {
        #[doc(hidden)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn zygisk_module_entry<'a>(
            api_table: ::std::ptr::NonNull<::std::marker::PhantomData<&'a ()>>,
            jni_env: $crate::jni::JNIEnv,
        ) {
            if ::std::panic::catch_unwind(|| {
                $crate::module_entry(
                    $module,
                    $crate::raw::RawApiTable::from_non_null(::std::ptr::NonNull::new_unchecked(
                        api_table.as_ptr().cast(),
                    )),
                    jni_env,
                );
            })
            .is_err()
            {
                ::std::process::abort();
            }
        }
    };
}

#[macro_export]
macro_rules! register_companion {
    ($func: expr) => {
        #[doc(hidden)]
        #[unsafe(no_mangle)]
        pub extern "C" fn zygisk_companion_entry(socket_fd: ::std::os::fd::OwnedFd) {
            if ::std::panic::catch_unwind(|| {
                // SAFETY: it is guaranteed by zygiskd that the argument is a valid
                // socket fd.
                let mut stream = unsafe {
                    <::std::os::unix::net::UnixStream as ::std::convert::From<
                        ::std::os::fd::OwnedFd,
                    >>::from(socket_fd)
                };

                let func: for<'a> fn(&'a mut ::std::os::unix::net::UnixStream) = $func;
                func(&mut stream)
            })
            .is_err()
            {
                // Panic messages should be displayed by the default panic hook.
                ::std::process::abort();
            }

            // It is both OK for us to close the fd or not to, since zygiskd
            // makes use of some nasty `fstat` tricks to handle both situations.
        }
    };
}
