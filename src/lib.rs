#![warn(clippy::std_instead_of_core)]

use core::ptr::NonNull;

use api::ZygiskApi;
use jni::JNIEnv;
use raw::{RawApiTable, RawModuleAbi, ZygiskRaw};
use static_alloc::Bump;
use without_alloc::{alloc::LocalAllocLeakExt, Box};

pub mod api;
mod aux;
pub use aux::*;
pub mod error;
pub mod raw;

pub(crate) mod impl_sealing {
    pub trait Sealed {}
}

pub trait ZygiskModule {
    type Api: for<'a> ZygiskRaw<'a>;

    fn on_load(&self, _: ZygiskApi<'_, Self::Api>, _: JNIEnv<'_>) {}

    fn pre_app_specialize<'a>(
        &self,
        _: ZygiskApi<'a, Self::Api>,
        _: JNIEnv<'a>,
        _: &'a mut <Self::Api as ZygiskRaw<'_>>::AppSpecializeArgs,
    ) {
    }

    fn post_app_specialize<'a>(
        &self,
        _: ZygiskApi<'a, Self::Api>,
        _: JNIEnv<'a>,
        _: &'a <Self::Api as ZygiskRaw<'_>>::AppSpecializeArgs,
    ) {
    }

    fn pre_server_specialize<'a>(
        &self,
        _: ZygiskApi<'a, Self::Api>,
        _: JNIEnv<'a>,
        _: &'a mut <Self::Api as ZygiskRaw<'_>>::ServerSpecializeArgs,
    ) {
    }

    fn post_server_specialize<'a>(
        &self,
        _: ZygiskApi<'a, Self::Api>,
        _: JNIEnv<'a>,
        _: &'a <Self::Api as ZygiskRaw<'_>>::ServerSpecializeArgs,
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
    ModuleImpl: ZygiskModule<Api = Version>,
{
    // RawModule<Version> size and alignment are consistent across all versions, hence we can just use a slab for the first version
    static RAW_MODULE_SLAB: Bump<[raw::RawModule<api::V1>; 1]> = const { Bump::uninit() };
    let raw_module = Box::leak(
        RAW_MODULE_SLAB
            .boxed(raw::RawModule::<'a> {
                dispatch,
                api_table,
                jni_env: unsafe { jni_env.unsafe_clone() },
            })
            .unwrap(),
    );

    // RawModuleAbi<Version> size and alignment are *also* consistent across all versions, hence we can just use a slab for the first version
    static RAW_MODULE_ABI_SLAB: Bump<[raw::ModuleAbi<api::V1>; 1]> = const { Bump::uninit() };
    let abi = RawModuleAbi::from_non_null(unsafe {
        NonNull::new_unchecked(Box::leak(
            RAW_MODULE_ABI_SLAB
                .boxed(Version::abi_from_module(raw_module))
                .unwrap(),
        ))
    });

    if unsafe { Version::register_module_fn(api_table.0.as_ref())(api_table.0, abi) } {
        dispatch.on_load(ZygiskApi::<Version>(api_table), jni_env);
    }
}

#[macro_export]
macro_rules! register_module {
    ($module:expr) => {
        #[doc(hidden)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn zygisk_module_entry(
            api_table: ::core::ptr::NonNull<::core::marker::PhantomData<&()>>,
            jni_env: $crate::jni::JNIEnv,
        ) {
            if ::std::panic::catch_unwind(|| {
                $crate::module_entry(
                    $module,
                    $crate::raw::RawApiTable::from_non_null(unsafe {
                        ::core::ptr::NonNull::new_unchecked(api_table.as_ptr().cast())
                    }),
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
                let mut stream = <::std::os::unix::net::UnixStream as ::core::convert::From<
                    ::std::os::fd::OwnedFd,
                >>::from(socket_fd);

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
