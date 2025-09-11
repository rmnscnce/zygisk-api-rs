#![warn(clippy::std_instead_of_core)]
#![no_std]
extern crate std;

use api::ZygiskApi;
use jni::JNIEnv;
use raw::ZygiskRaw;

pub mod api;
mod aux;
pub use aux::*;
pub mod error;
pub mod raw;

#[doc(hidden)]
pub mod utils;
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

#[macro_export]
macro_rules! register_module {
    ($module:ty) => {
        const _: () = {
            #[unsafe(export_name = "zygisk_module_entry")]
            unsafe extern "C" fn module_entry(
                api_table: *const (),
                env: *mut $crate::jni::sys::JNIEnv,
            ) {
                if ::std::panic::catch_unwind(
                    #[inline(always)]
                    move || {
                        type Api = <$module as $crate::ZygiskModule>::Api;
                        type RawModule<'a> = $crate::raw::RawModule<'a, Api>;
                        type ModuleAbi<'a> = $crate::raw::ModuleAbi<'a, Api>;

                        #[repr(transparent)]
                        struct AssertSyncUnsafeCell<T>(::core::cell::UnsafeCell<T>);

                        unsafe impl<T> Sync for AssertSyncUnsafeCell<T> {}

                        impl<T> AssertSyncUnsafeCell<T> {
                            #[inline(always)]
                            const fn new(value: T) -> Self {
                                Self(::core::cell::UnsafeCell::new(value))
                            }
                        }

                        static INSTANCE: AssertSyncUnsafeCell<::core::mem::MaybeUninit<$module>> =
                            const { AssertSyncUnsafeCell::new(::core::mem::MaybeUninit::uninit()) };
                        static RAW_MODULE: AssertSyncUnsafeCell<
                            ::core::mem::MaybeUninit<RawModule<'static>>,
                        > = AssertSyncUnsafeCell::new(::core::mem::MaybeUninit::uninit());
                        static MODULE_ABI: AssertSyncUnsafeCell<
                            ::core::mem::MaybeUninit<ModuleAbi<'static>>,
                        > = const { AssertSyncUnsafeCell::new(::core::mem::MaybeUninit::uninit()) };

                        unsafe { &mut *INSTANCE.0.get() }
                            .write(<$module as ::core::default::Default>::default());
                        let api_table =
                            unsafe { $crate::raw::ApiTableRef::from_raw(api_table as *const _) };

                        unsafe { &mut *RAW_MODULE.0.get() }.write($crate::raw::RawModule {
                            dispatch: unsafe { (&*INSTANCE.0.get()).assume_init_ref() },
                            api_table: ::core::clone::Clone::clone(&api_table),
                            jni_env: unsafe {
                                $crate::jni::JNIEnv::from_raw(env).unwrap_unchecked()
                            },
                        });

                        unsafe { &mut *MODULE_ABI.0.get() }.write(
                            <Api as $crate::raw::ZygiskRaw>::abi_from_module(unsafe {
                                (&mut *RAW_MODULE.0.get()).assume_init_mut()
                            }),
                        );

                        let abi = unsafe {
                            $crate::raw::ModuleAbiRef::from_raw(
                                (&mut *MODULE_ABI.0.get()).as_mut_ptr(),
                            )
                        };

                        if unsafe {
                            <Api as $crate::raw::ZygiskRaw>::register_module_fn(api_table)(
                                api_table, abi,
                            )
                        } {
                            unsafe { (&*INSTANCE.0.get()).assume_init_ref() }
                                .on_load($crate::api::ZygiskApi(api_table), unsafe {
                                    $crate::jni::JNIEnv::from_raw(env).unwrap_unchecked()
                                })
                        }
                    },
                )
                .is_err()
                {
                    ::std::process::abort();
                }
            }
        };
    };
}

#[macro_export]
macro_rules! register_companion {
    ($func: expr) => {
        const _: () = {
            #[unsafe(export_name = "zygisk_companion_entry")]
            extern "C" fn companion_entry(sock_fd: ::std::os::fd::OwnedFd) {
                if ::std::panic::catch_unwind(
                    #[inline(always)]
                    move || {
                        let mut stream =
                            <::std::os::unix::net::UnixStream as ::core::convert::From<
                                ::std::os::fd::OwnedFd,
                            >>::from(sock_fd);

                        let func: for<'a> fn(&'a mut ::std::os::unix::net::UnixStream) = $func;
                        func(&mut stream)
                    },
                )
                .is_err()
                {
                    ::std::process::abort();
                }
            }
        };
    };
}

#[cfg(test)]
mod compile_test {
    use crate::{ZygiskModule, api};

    #[derive(Default)]
    struct MyModule;

    impl ZygiskModule for MyModule {
        type Api = api::V5;
    }
    register_module!(MyModule);

    register_companion!(|_| ());
}
