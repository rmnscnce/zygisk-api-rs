use jni::sys::{JNIEnv, JNINativeMethod};
use libc::{c_char, c_int, c_long};

use crate::{
    api::{ZygiskApi, V2},
    raw::RawModule,
};

use super::{ModuleAbi, ZygiskRaw};

pub(crate) mod transparent {

    pub use crate::raw::v1::transparent::{
        AppSpecializeArgs, ServerSpecializeArgs, StateFlags, ZygiskOption,
    };
}
#[repr(C)]
pub struct RawApiTable<'a> {
    pub this: *mut (),
    pub register_module_fn: Option<extern "C" fn(*const Self, *mut ModuleAbi<'a, V2>) -> bool>,

    pub hook_jni_native_methods_fn:
        Option<extern "C" fn(*mut JNIEnv, *const c_char, *mut JNINativeMethod, c_int)>,
    pub plt_hook_register_fn:
        Option<extern "C" fn(*const c_char, *const c_char, *mut (), *mut *mut ())>,
    pub plt_hook_exclude_fn: Option<extern "C" fn(*const c_char, *const c_char)>,
    pub plt_hook_commit_fn: Option<extern "C" fn() -> bool>,

    pub connect_companion_fn: Option<extern "C" fn(*const ()) -> c_int>,
    pub set_option_fn: Option<extern "C" fn(*const (), transparent::ZygiskOption)>,
    pub get_module_dir_fn: Option<extern "C" fn(*const ()) -> c_int>,
    pub get_flags_fn: Option<extern "C" fn(*const ()) -> u32>,
}

impl ZygiskRaw for V2 {
    const API_VERSION: c_long = 2;
    type RawApiTable<'a> = RawApiTable<'a>;
    type ModuleAbi<'a> = ModuleAbi<'a, V2>;
    type AppSpecializeArgs<'a> = transparent::AppSpecializeArgs<'a>;
    type ServerSpecializeArgs<'a> = transparent::ServerSpecializeArgs<'a>;

    fn abi_from_module<'a>(module: &'a mut super::RawModule<'a, V2>) -> Self::ModuleAbi<'a> {
        extern "C" fn pre_app_specialize(
            m: &mut RawModule<V2>,
            args: &mut transparent::AppSpecializeArgs<'_>,
        ) {
            m.inner.pre_app_specialize(
                ZygiskApi::<V2>(unsafe { &*m.api_table }),
                unsafe { jni::JNIEnv::from_raw(m.jni_env).unwrap_unchecked() },
                args,
            );
        }

        extern "C" fn post_app_specialize(
            m: &mut RawModule<V2>,
            args: &transparent::AppSpecializeArgs<'_>,
        ) {
            m.inner.post_app_specialize(
                ZygiskApi::<V2>(unsafe { &*m.api_table }),
                unsafe { jni::JNIEnv::from_raw(m.jni_env).unwrap_unchecked() },
                args,
            );
        }

        extern "C" fn pre_server_specialize(
            m: &mut RawModule<V2>,
            args: &mut transparent::ServerSpecializeArgs<'_>,
        ) {
            m.inner.pre_server_specialize(
                ZygiskApi::<V2>(unsafe { &*m.api_table }),
                unsafe { jni::JNIEnv::from_raw(m.jni_env).unwrap_unchecked() },
                args,
            );
        }

        extern "C" fn post_server_specialize(
            m: &mut RawModule<V2>,
            args: &transparent::ServerSpecializeArgs<'_>,
        ) {
            m.inner.post_server_specialize(
                ZygiskApi::<V2>(unsafe { &*m.api_table }),
                unsafe { jni::JNIEnv::from_raw(m.jni_env).unwrap_unchecked() },
                args,
            );
        }

        ModuleAbi {
            api_version: Self::API_VERSION,
            this: module,
            pre_app_specialize_fn: pre_app_specialize,
            post_app_specialize_fn: post_app_specialize,
            pre_server_specialize_fn: pre_server_specialize,
            post_server_specialize_fn: post_server_specialize,
        }
    }

    fn register_module_fn<'a>(
        table: &'a Self::RawApiTable<'a>,
    ) -> Option<extern "C" fn(*const Self::RawApiTable<'a>, *mut ModuleAbi<'a, V2>) -> bool> {
        table.register_module_fn
    }
}
