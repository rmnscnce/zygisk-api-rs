use jni::{sys::JNINativeMethod, JNIEnv};
use libc::{c_char, c_int, c_long};

use crate::{
    api::{ZygiskApi, V3},
    raw::RawModule,
};

use super::{ModuleAbi, RawModuleAbi, ZygiskRaw};

pub(crate) mod transparent {
    use jni::{
        objects::JString,
        sys::{jboolean, jint, jintArray, jobjectArray},
    };

    pub use crate::raw::v1::transparent::{ServerSpecializeArgs, ZygiskOption};
    pub use crate::raw::v2::transparent::StateFlags;

    #[repr(C)]
    pub struct AppSpecializeArgs<'a> {
        // Required arguments. These arguments are guaranteed to exist on all Android versions.
        pub uid: &'a mut jint,
        pub gid: &'a mut jint,
        pub gids: &'a mut jintArray,
        pub runtime_flags: &'a jint,
        pub rlimits: &'a jobjectArray,
        pub mount_external: &'a jint,
        pub se_info: &'a JString<'a>,
        pub nice_name: &'a JString<'a>,
        pub instruction_set: &'a JString<'a>,
        pub app_data_dir: &'a JString<'a>,

        // Optional arguments. Please check whether the pointer is null before de-referencing
        pub fds_to_ignore: Option<&'a jintArray>,
        pub is_child_zygote: Option<&'a jint>,
        pub is_top_app: Option<&'a jint>,
        pub pkg_data_info_list: Option<&'a jobjectArray>,
        pub whitelisted_data_info_list: Option<&'a jobjectArray>,
        pub mount_data_dirs: Option<&'a jboolean>,
        pub mount_storage_dirs: Option<&'a jboolean>,
    }
}

#[repr(C)]
pub struct RawApiTable {
    pub(crate) this: *mut (),
    pub(crate) register_module_fn:
        Option<for<'b> extern "C" fn(*const Self, RawModuleAbi<'b, V3>) -> bool>,

    pub(crate) hook_jni_native_methods_fn:
        Option<extern "C" fn(JNIEnv<'_>, *const c_char, *mut JNINativeMethod, c_int)>,
    pub(crate) plt_hook_register_fn:
        Option<extern "C" fn(*const c_char, *const c_char, *mut (), *mut *mut ())>,
    pub(crate) plt_hook_exclude_fn: Option<extern "C" fn(*const c_char, *const c_char)>,
    pub(crate) plt_hook_commit_fn: Option<extern "C" fn() -> bool>,

    pub(crate) connect_companion_fn: Option<extern "C" fn(*const ()) -> c_int>,
    pub(crate) set_option_fn: Option<extern "C" fn(*const (), transparent::ZygiskOption)>,
    pub(crate) get_module_dir_fn: Option<extern "C" fn(*const ()) -> c_int>,
    pub(crate) get_flags_fn: Option<extern "C" fn(*const ()) -> u32>,
}

impl<'a> ZygiskRaw<'a> for V3 {
    const API_VERSION: c_long = 3;
    type RawApiTable = RawApiTable;
    type ModuleAbi = ModuleAbi<'a, V3>;
    type AppSpecializeArgs = transparent::AppSpecializeArgs<'a>;
    type ServerSpecializeArgs = transparent::ServerSpecializeArgs<'a>;

    fn abi_from_module(
        module: &'a mut super::RawModule<'a, Self>,
    ) -> <Self as ZygiskRaw<'a>>::ModuleAbi {
        extern "C" fn pre_app_specialize<'a>(
            m: &mut RawModule<'a, V3>,
            args: &'a mut transparent::AppSpecializeArgs<'a>,
        ) {
            m.dispatch.pre_app_specialize(
                ZygiskApi::<V3>(m.api_table),
                unsafe { m.jni_env.unsafe_clone() },
                args,
            );
        }

        extern "C" fn post_app_specialize<'a>(
            m: &mut RawModule<'a, V3>,
            args: &'a transparent::AppSpecializeArgs<'a>,
        ) {
            m.dispatch.post_app_specialize(
                ZygiskApi::<V3>(m.api_table),
                unsafe { m.jni_env.unsafe_clone() },
                args,
            );
        }

        extern "C" fn pre_server_specialize<'a>(
            m: &mut RawModule<'a, V3>,
            args: &'a mut transparent::ServerSpecializeArgs<'a>,
        ) {
            m.dispatch.pre_server_specialize(
                ZygiskApi::<V3>(m.api_table),
                unsafe { m.jni_env.unsafe_clone() },
                args,
            );
        }

        extern "C" fn post_server_specialize<'a>(
            m: &mut RawModule<'a, V3>,
            args: &'a transparent::ServerSpecializeArgs<'a>,
        ) {
            m.dispatch.post_server_specialize(
                ZygiskApi::<V3>(m.api_table),
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
        table: &'a <Self as ZygiskRaw<'a>>::RawApiTable,
    ) -> Option<for<'b> extern "C" fn(*const Self::RawApiTable, RawModuleAbi<'b, V3>) -> bool> {
        table.register_module_fn
    }
}
