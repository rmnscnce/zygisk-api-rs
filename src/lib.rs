use std::cell::OnceCell;

use api::ZygiskApi;
use jni::JNIEnv;
use raw::ZygiskRawApi;

pub mod api;
pub mod aux;
pub mod error;
pub mod raw;

pub(crate) mod impl_sealing {
    pub trait Sealed {}
}

pub trait ZygiskModule<Version>
where
    Version: ZygiskRawApi,
{
    /// ### Memory-safety API
    fn prepare(&mut self);

    /// ### Memory-safety API
    #[allow(clippy::mut_from_ref)]
    fn raw_module(&self) -> &mut OnceCell<raw::RawModule<Version>>;

    fn on_load(&self, _: ZygiskApi<Version>, _: JNIEnv) {}

    fn pre_app_specialize(
        &self,
        _: ZygiskApi<Version>,
        _: JNIEnv,
        _: &mut Version::AppSpecializeArgs<'_>,
    ) {
    }

    fn post_app_specialize(
        &self,
        _: ZygiskApi<Version>,
        _: JNIEnv,
        _: &Version::AppSpecializeArgs<'_>,
    ) {
    }

    fn pre_server_specialize(
        &self,
        _: ZygiskApi<Version>,
        _: JNIEnv,
        _: &mut Version::ServerSpecializeArgs<'_>,
    ) {
    }

    fn post_server_specialize(
        &self,
        _: ZygiskApi<Version>,
        _: JNIEnv,
        _: &Version::ServerSpecializeArgs<'_>,
    ) {
    }
}

#[doc(hidden)]
#[inline(always)]
pub fn module_entry<'a, Version>(
    inner: &'a mut dyn ZygiskModule<Version>,
    api_table: *const Version::RawApiTable<'a>,
    jni_env: *mut jni::sys::JNIEnv,
) where
    Version: ZygiskRawApi<ModuleAbi<'a> = raw::ModuleAbi<'a, Version>> + 'a,
{
    inner.prepare();

    inner.raw_module().get_or_init(|| raw::RawModule {
        inner,
        api_table,
        jni_env,
    });

    let api_table = unsafe { &*api_table.cast() };
    let env = unsafe { JNIEnv::from_raw(jni_env.cast()).unwrap_unchecked() };
    let mut abi = Version::abi_from_module(inner.raw_module().get_mut().unwrap());

    if let Some(f) = Version::register_module_fn(api_table) {
        if f(api_table, &mut abi) {
            let api = ZygiskApi::<Version>(api_table);
            inner.on_load(api, env);
        }
    }
}

#[macro_export]
macro_rules! register_module {
    ($module:expr) => {
        #[allow(no_mangle_generic_items)]
        #[no_mangle]
        pub extern "C" fn zygisk_module_entry(
            api_table: *const (),
            jni_env: *mut $crate::aux::jni::sys::JNIEnv,
        ) {
            let mut module = $module;

            if ::std::panic::catch_unwind(|| {
                $crate::module_entry(module, api_table.cast(), jni_env);
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
        #[no_mangle]
        extern "C" fn zygisk_companion_entry(socket_fd: ::std::os::unix::io::RawFd) {
            // SAFETY: it is guaranteed by zygiskd that the argument is a valid
            // socket fd.
            let stream = unsafe {
                <::std::os::unix::net::UnixStream as ::std::os::fd::FromRawFd>::from_raw_fd(
                    socket_fd,
                )
            };

            // Call the actual function.
            let _type_check: fn(::std::os::unix::net::UnixStream) = $func;
            if ::std::panic::catch_unwind(|| _type_check(stream)).is_err() {
                // Panic messages should be displayed by the default panic hook.
                ::std::process::abort();
            }

            // It is both OK for us to close the fd or not to, since zygiskd
            // makes use of some nasty `fstat` tricks to handle both situations.
        }
    };
}
