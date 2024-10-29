use std::panic::RefUnwindSafe;

use jni::sys::JNIEnv;
use libc::c_long;

use crate::{api::ZygiskApiSpec, impl_sealing::Sealed, ZygiskModule};

pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;
pub mod v5;

pub struct RawModule<'a, Version>
where
    Version: ZygiskRawApi + ?Sized,
{
    pub inner: &'a dyn ZygiskModule<Version>,
    pub api_table: *const <Version as ZygiskRawApi>::RawApiTable<'a>,
    pub jni_env: *mut JNIEnv,
}

#[repr(C)]
pub struct ModuleAbi<'a, Version>
where
    Version: ZygiskRawApi + ?Sized + 'a,
{
    pub api_version: c_long,
    pub this: &'a mut RawModule<'a, Version>,

    pub pre_app_specialize_fn: extern "C" fn(
        &mut RawModule<Version>,
        &mut <Version as ZygiskRawApi>::AppSpecializeArgs<'_>,
    ),
    pub post_app_specialize_fn:
        extern "C" fn(&mut RawModule<Version>, &<Version as ZygiskRawApi>::AppSpecializeArgs<'_>),
    pub pre_server_specialize_fn: extern "C" fn(
        &mut RawModule<Version>,
        &mut <Version as ZygiskRawApi>::ServerSpecializeArgs<'_>,
    ),
    pub post_server_specialize_fn: extern "C" fn(
        &mut RawModule<Version>,
        &<Version as ZygiskRawApi>::ServerSpecializeArgs<'_>,
    ),
}

pub trait ZygiskRawApi
where
    Self: ZygiskApiSpec + Sealed,
{
    const API_VERSION: c_long;
    type RawApiTable<'a>: RefUnwindSafe;
    type ModuleAbi<'a>
    where
        Self: 'a;
    type AppSpecializeArgs<'a>;
    type ServerSpecializeArgs<'a>;

    fn abi_from_module<'a>(module: &'a mut RawModule<'a, Self>) -> Self::ModuleAbi<'a>;

    fn register_module_fn<'a>(
        table: &'a Self::RawApiTable<'a>,
    ) -> Option<extern "C" fn(*const Self::RawApiTable<'a>, *mut ModuleAbi<'a, Self>) -> bool>;
}
