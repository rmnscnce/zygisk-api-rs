use jni::sys::{JNIEnv, JNINativeMethod};
use libc::{c_char, c_int, c_long};

use crate::api::{ZygiskApi, V1};

use super::{ModuleAbi, RawModule, ZygiskRaw};

pub(crate) mod transparent {
    use jni::{
        objects::JString,
        sys::{jboolean, jint, jintArray, jlong, jobjectArray},
    };

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ZygiskOption {
        ForceDenylistUnmount = 0,
        DlCloseModuleLibrary = 1,
    }

    bitflags::bitflags! {
        pub struct StateFlags: u32 {
            const PROCESS_GRANTED_ROOT = (1 << 0);
            const PROCESS_ON_DENYLIST = (1 << 1);
        }
    }

    #[repr(C)]
    pub struct AppSpecializeArgs<'a> {
        // Required arguments. These arguments are guaranteed to exist on all Android versions.
        pub uid: &'a mut jint,
        pub gid: &'a mut jint,
        pub gids: &'a mut jintArray,
        pub runtime_flags: &'a jint,
        pub mount_external: &'a jint,
        pub se_info: &'a JString<'a>,
        pub nice_name: &'a JString<'a>,
        pub instruction_set: &'a JString<'a>,
        pub app_data_dir: &'a JString<'a>,

        // Optional arguments. Please check whether the pointer is null before de-referencing
        pub is_child_zygote: Option<&'a jint>,
        pub is_top_app: Option<&'a jint>,
        pub pkg_data_info_list: Option<&'a jobjectArray>,
        pub whitelisted_data_info_list: Option<&'a jobjectArray>,
        pub mount_data_dirs: Option<&'a jboolean>,
        pub mount_storage_dirs: Option<&'a jboolean>,
    }

    #[repr(C)]
    pub struct ServerSpecializeArgs<'a> {
        pub uid: &'a mut jint,
        pub gid: &'a mut jint,
        pub gids: &'a mut jintArray,
        pub runtime_flags: &'a jint,
        pub permitted_capabilities: &'a jlong,
        pub effective_capabilities: &'a jlong,
    }
}

#[repr(C)]
pub struct RawApiTable<'a> {
    pub(crate) this: *mut (),
    pub(crate) register_module_fn:
        Option<extern "C" fn(*const Self, *mut ModuleAbi<'a, V1>) -> bool>,

    pub(crate) hook_jni_native_methods_fn:
        Option<extern "C" fn(*mut JNIEnv, *const c_char, *mut JNINativeMethod, c_int)>,
    pub(crate) plt_hook_register_fn:
        Option<extern "C" fn(*const c_char, *const c_char, *mut (), *mut *mut ())>,
    pub(crate) plt_hook_exclude_fn: Option<extern "C" fn(*const c_char, *const c_char)>,
    pub(crate) plt_hook_commit_fn: Option<extern "C" fn() -> bool>,

    pub(crate) connect_companion_fn: Option<extern "C" fn(*const ()) -> c_int>,
    pub(crate) set_option_fn: Option<extern "C" fn(*const (), transparent::ZygiskOption)>,
}

impl ZygiskRaw for V1 {
    const API_VERSION: c_long = 1;
    type RawApiTable<'a> = RawApiTable<'a>;
    type ModuleAbi<'a> = ModuleAbi<'a, V1>;
    type AppSpecializeArgs<'a> = transparent::AppSpecializeArgs<'a>;
    type ServerSpecializeArgs<'a> = transparent::ServerSpecializeArgs<'a>;

    fn abi_from_module<'a>(module: &'a mut RawModule<'a, V1>) -> Self::ModuleAbi<'a> {
        extern "C" fn pre_app_specialize(
            m: &mut RawModule<V1>,
            args: &mut transparent::AppSpecializeArgs,
        ) {
            m.inner.pre_app_specialize(
                unsafe { ZygiskApi::<V1>(&*m.api_table) },
                unsafe { jni::JNIEnv::from_raw(m.jni_env).unwrap_unchecked() },
                args,
            );
        }

        extern "C" fn post_app_specialize(
            m: &mut RawModule<V1>,
            args: &transparent::AppSpecializeArgs,
        ) {
            m.inner.post_app_specialize(
                unsafe { ZygiskApi::<V1>(&*m.api_table) },
                unsafe { jni::JNIEnv::from_raw(m.jni_env).unwrap_unchecked() },
                args,
            );
        }

        extern "C" fn pre_server_specialize(
            m: &mut RawModule<V1>,
            args: &mut transparent::ServerSpecializeArgs,
        ) {
            m.inner.pre_server_specialize(
                unsafe { ZygiskApi::<V1>(&*m.api_table) },
                unsafe { jni::JNIEnv::from_raw(m.jni_env).unwrap_unchecked() },
                args,
            );
        }

        extern "C" fn post_server_specialize(
            m: &mut RawModule<V1>,
            args: &transparent::ServerSpecializeArgs,
        ) {
            m.inner.post_server_specialize(
                unsafe { ZygiskApi::<V1>(&*m.api_table) },
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
    ) -> Option<extern "C" fn(*const Self::RawApiTable<'a>, *mut ModuleAbi<'a, V1>) -> bool> {
        table.register_module_fn
    }
}
