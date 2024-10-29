use std::panic::RefUnwindSafe;

use jni::sys::JNIEnv;
use libc::c_long;

use crate::{api::ZygiskSpec, impl_sealing::Sealed, ZygiskModule};

pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;
pub mod v5;

pub struct RawModule<'a, Version>
where
    Version: ZygiskRaw + ?Sized,
{
    pub(crate) inner: &'a dyn ZygiskModule<Version>,
    pub(crate) api_table: *const <Version as ZygiskRaw>::RawApiTable<'a>,
    pub(crate) jni_env: *mut JNIEnv,
}

#[repr(C)]
pub struct ModuleAbi<'a, Version>
where
    Version: ZygiskRaw + ?Sized + 'a,
{
    pub(crate) api_version: c_long,
    pub(crate) this: &'a mut RawModule<'a, Version>,

    pub(crate) pre_app_specialize_fn:
        extern "C" fn(&mut RawModule<Version>, &mut <Version as ZygiskRaw>::AppSpecializeArgs<'_>),
    pub(crate) post_app_specialize_fn:
        extern "C" fn(&mut RawModule<Version>, &<Version as ZygiskRaw>::AppSpecializeArgs<'_>),
    pub(crate) pre_server_specialize_fn: extern "C" fn(
        &mut RawModule<Version>,
        &mut <Version as ZygiskRaw>::ServerSpecializeArgs<'_>,
    ),
    pub(crate) post_server_specialize_fn:
        extern "C" fn(&mut RawModule<Version>, &<Version as ZygiskRaw>::ServerSpecializeArgs<'_>),
}

pub trait ZygiskRaw
where
    Self: ZygiskSpec + Sealed,
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
