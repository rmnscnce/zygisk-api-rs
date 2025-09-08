#![warn(clippy::std_instead_of_core)]

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
        #[doc(hidden)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn zygisk_module_entry(
            api_table: *const (),
            env: *mut $crate::jni::sys::JNIEnv,
        ) {
            if ::std::panic::catch_unwind(move || {
                type Api = <$module as $crate::ZygiskModule>::Api;
                type RawModule<'a> = $crate::raw::RawModule<'a, Api>;
                type ModuleAbi<'a> = $crate::raw::ModuleAbi<'a, Api>;

                struct ModuleInstance {
                    instance: ::core::cell::OnceCell<$module>,
                    raw_module:
                        ::core::cell::OnceCell<::core::cell::UnsafeCell<RawModule<'static>>>,
                    module_abi:
                        ::core::cell::OnceCell<::core::cell::UnsafeCell<ModuleAbi<'static>>>,
                }

                impl ModuleInstance {
                    const fn new() -> Self {
                        Self {
                            instance: ::core::cell::OnceCell::new(),
                            raw_module: ::core::cell::OnceCell::new(),
                            module_abi: ::core::cell::OnceCell::new(),
                        }
                    }
                }

                ::std::thread_local! {
                    static INSTANCE: ModuleInstance = const { ModuleInstance::new() };
                }

                INSTANCE.with(move |inst| {
                    let _ = inst
                        .instance
                        .set(<$module as ::core::default::Default>::default());
                    let api_table =
                        unsafe { $crate::raw::ApiTableRef::from_raw(api_table as *const _) };
                    #[allow(clippy::missing_transmute_annotations)]
                    let _ = inst.raw_module.set(unsafe {
                        ::core::mem::transmute(::core::cell::UnsafeCell::new(
                            $crate::raw::RawModule {
                                dispatch: inst.instance.get().unwrap(),
                                api_table: ::core::clone::Clone::clone(&api_table),
                                jni_env: $crate::jni::JNIEnv::from_raw(env).unwrap_unchecked(),
                            },
                        ))
                    });
                    let _ = inst.module_abi.set(::core::cell::UnsafeCell::new(
                        <Api as $crate::raw::ZygiskRaw>::abi_from_module(unsafe {
                            &mut *inst.raw_module.get().unwrap_unchecked().get()
                        }),
                    ));

                    let abi = unsafe {
                        $crate::raw::ModuleAbiRef::from_raw(
                            inst.module_abi.get().unwrap_unchecked().get(),
                        )
                    };

                    if unsafe {
                        <Api as $crate::raw::ZygiskRaw>::register_module_fn(api_table)(
                            api_table, abi,
                        )
                    } {
                        inst.instance
                            .get()
                            .unwrap()
                            .on_load($crate::api::ZygiskApi(api_table), unsafe {
                                $crate::jni::JNIEnv::from_raw(env).unwrap_unchecked()
                            })
                    }
                });
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
        pub extern "C" fn zygisk_companion_entry(sock_fd: ::std::os::fd::OwnedFd) {
            if ::std::panic::catch_unwind(move || {
                let mut stream = <::std::os::unix::net::UnixStream as ::core::convert::From<
                    ::std::os::fd::OwnedFd,
                >>::from(sock_fd);

                let func: for<'a> fn(&'a mut ::std::os::unix::net::UnixStream) = $func;
                func(&mut stream)
            })
            .is_err()
            {
                ::std::process::abort();
            }
        }
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
