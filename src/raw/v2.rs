use jni::sys::{JNIEnv, JNINativeMethod};
use libc::{c_char, c_int, c_long};

use crate::{
    api::{ZygiskApi, V2},
    raw::RawModule,
};

use super::{ModuleAbi, ZygiskRaw};

pub(crate) mod transparent {

    pub use crate::raw::v1::transparent::{AppSpecializeArgs, ServerSpecializeArgs, ZygiskOption};

    bitflags::bitflags! {
        pub struct StateFlags: u32 {
            const PROCESS_GRANTED_ROOT = (1 << 0);
            const PROCESS_ON_DENYLIST = (1 << 1);
        }
    }
}
#[repr(C)]
pub struct RawApiTable {
    pub this: *mut (),
    pub register_module_fn: Option<for<'b> extern "C" fn(*const Self, ModuleAbi<'b, V2>) -> bool>,

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

impl<'a> ZygiskRaw<'a> for V2 {
    const API_VERSION: c_long = 2;
    type RawApiTable = RawApiTable;
    type ModuleAbi = ModuleAbi<'a, V2>;
    type AppSpecializeArgs = transparent::AppSpecializeArgs<'a>;
    type ServerSpecializeArgs = transparent::ServerSpecializeArgs<'a>;

    fn abi_from_module(module: &'a mut super::RawModule<'a, V2>) -> Self::ModuleAbi {
        extern "C" fn pre_app_specialize<'a>(
            m: &mut RawModule<'a, V2>,
            args: &'a mut transparent::AppSpecializeArgs<'a>,
        ) {
            m.dispatch.pre_app_specialize(
                ZygiskApi::<V2>(m.api_table),
                unsafe { m.jni_env.unsafe_clone() },
                args,
            );
        }

        extern "C" fn post_app_specialize<'a>(
            m: &mut RawModule<'a, V2>,
            args: &'a transparent::AppSpecializeArgs<'a>,
        ) {
            m.dispatch.post_app_specialize(
                ZygiskApi::<V2>(m.api_table),
                unsafe { m.jni_env.unsafe_clone() },
                args,
            );
        }

        extern "C" fn pre_server_specialize<'a>(
            m: &mut RawModule<'a, V2>,
            args: &'a mut transparent::ServerSpecializeArgs<'a>,
        ) {
            m.dispatch.pre_server_specialize(
                ZygiskApi::<V2>(m.api_table),
                unsafe { m.jni_env.unsafe_clone() },
                args,
            );
        }

        extern "C" fn post_server_specialize<'a>(
            m: &mut RawModule<'a, V2>,
            args: &'a transparent::ServerSpecializeArgs<'a>,
        ) {
            m.dispatch.post_server_specialize(
                ZygiskApi::<V2>(m.api_table),
                unsafe { m.jni_env.unsafe_clone() },
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

    fn register_module_fn(
        table: &'a Self::RawApiTable,
    ) -> Option<for<'b> extern "C" fn(*const Self::RawApiTable, ModuleAbi<'b, V2>) -> bool> {
        table.register_module_fn
    }
}
