#![warn(clippy::std_instead_of_core)]

use api::ZygiskApi;
use jni::JNIEnv;
use raw::ZygiskRaw;

pub mod api;
mod aux;
pub use aux::*;
pub mod error;
pub mod raw;

pub(crate) mod impl_sealing {
    pub trait Sealed {}
}

pub(crate) mod utils {
    use core::mem;

    pub struct ShapeAssertion<T, U>(T, U);
    impl<T, U> ShapeAssertion<T, U> {
        pub const ASSERT: () = const {
            assert!(mem::size_of::<T>() == mem::size_of::<U>());
            assert!(mem::align_of::<T>() == mem::align_of::<U>());
        };
    }
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
    ($module:expr) => {
        #[doc(hidden)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn zygisk_module_entry(
            api_table: ::core::ptr::NonNull<::core::marker::PhantomData<&'static ()>>,
            env: $crate::jni::JNIEnv<'static>,
        ) {
            if ::std::panic::catch_unwind(move || {
                let api_table = $crate::raw::RawApiTable::from_non_null(unsafe {
                    ::core::ptr::NonNull::new_unchecked(api_table.as_ptr().cast())
                });

                let dispatch = const { ::core::mem::ManuallyDrop::new($module) };
                let dispatch: &dyn $crate::ZygiskModule<Api = _> =
                    ::core::ops::Deref::deref(&dispatch);

                let mut raw_module = ::core::mem::ManuallyDrop::new($crate::raw::RawModule {
                    dispatch,
                    api_table,
                    jni_env: unsafe { env.unsafe_clone() },
                });

                let mut abi =
                    ::core::mem::ManuallyDrop::new($crate::raw::ZygiskRaw::abi_from_module(
                        ::core::ops::DerefMut::deref_mut(&mut raw_module),
                    ));
                let abi = $crate::raw::RawModuleAbi::from_non_null(unsafe {
                    ::core::ptr::NonNull::new_unchecked(::core::ops::DerefMut::deref_mut(&mut abi))
                });

                if unsafe {
                    $crate::raw::ZygiskRaw::register_module_fn(api_table.0.as_ref())(
                        api_table.0,
                        abi,
                    )
                } {
                    dispatch.on_load($crate::api::ZygiskApi(api_table), env);
                }
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
            if ::std::panic::catch_unwind(move || {
                let mut stream = <::std::os::unix::net::UnixStream as ::core::convert::From<
                    ::std::os::fd::OwnedFd,
                >>::from(socket_fd);

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

    struct MyModule;

    impl ZygiskModule for MyModule {
        type Api = api::V5;
    }
    register_module!(MyModule);

    register_companion!(|_| ());
}
