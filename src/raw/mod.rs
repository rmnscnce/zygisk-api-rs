use std::marker::PhantomData;

use jni::JNIEnv;
use libc::c_long;

use crate::{api::ZygiskSpec, ZygiskModule};

pub mod v1;
pub mod v2;
pub mod v3;
mod v4;
mod v5;

pub struct RawModule<'a, Version>
where
    Version: ZygiskRaw<'a> + 'a,
{
    pub(crate) dispatch: &'a dyn ZygiskModule<Version>,
    pub(crate) api_table: RawApiTable<'a, Version>,
    pub(crate) jni_env: JNIEnv<'a>,
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct RawApiTable<'a, Version>(
    pub(crate) *const <Version as ZygiskRaw<'a>>::RawApiTable,
    PhantomData<&'a Version>,
)
where
    Version: ZygiskRaw<'a> + 'a;

impl<'a, Version> RawApiTable<'a, Version>
where
    Version: ZygiskRaw<'a> + 'a,
{
    #[doc(hidden)]
    pub fn from_ptr(ptr: *const <Version as ZygiskRaw<'a>>::RawApiTable) -> Self {
        Self(ptr, PhantomData)
    }
}

#[repr(C)]
pub struct ModuleAbi<'a, Version>
where
    Version: ZygiskRaw<'a> + 'a,
{
    pub(crate) api_version: c_long,
    pub(crate) this: &'a mut RawModule<'a, Version>,

    pub(crate) pre_app_specialize_fn: for<'c, 'd> extern "C" fn(
        &'d mut RawModule<'c, Version>,
        &'c mut <Version as ZygiskRaw<'c>>::AppSpecializeArgs,
    ),
    pub(crate) post_app_specialize_fn: for<'c, 'd> extern "C" fn(
        &'d mut RawModule<'c, Version>,
        &'c <Version as ZygiskRaw<'c>>::AppSpecializeArgs,
    ),
    pub(crate) pre_server_specialize_fn: for<'c, 'd> extern "C" fn(
        &'d mut RawModule<'c, Version>,
        &'c mut <Version as ZygiskRaw<'c>>::ServerSpecializeArgs,
    ),
    pub(crate) post_server_specialize_fn: for<'c, 'd> extern "C" fn(
        &'d mut RawModule<'c, Version>,
        &'c <Version as ZygiskRaw<'c>>::ServerSpecializeArgs,
    ),
}

#[repr(transparent)]
pub struct RawModuleAbi<'a, Version>(
    pub(crate) *mut ModuleAbi<'a, Version>,
    PhantomData<&'a Version>,
)
where
    Version: ZygiskRaw<'a>;

impl<'a, Version> RawModuleAbi<'a, Version>
where
    Version: ZygiskRaw<'a>,
{
    pub(crate) fn from_ptr(ptr: *mut ModuleAbi<'a, Version>) -> Self {
        Self(ptr, PhantomData)
    }
}

pub trait ZygiskRaw<'a>
where
    Self: ZygiskSpec,
{
    const API_VERSION: c_long;
    type RawApiTable: 'a;
    type ModuleAbi: 'a;
    type AppSpecializeArgs: 'a;
    type ServerSpecializeArgs: 'a;

    fn abi_from_module(module: &'a mut RawModule<'a, Self>) -> <Self as ZygiskRaw<'a>>::ModuleAbi;

    fn register_module_fn(
        table: &'a <Self as ZygiskRaw<'a>>::RawApiTable,
    ) -> Option<
        extern "C" fn(*const <Self as ZygiskRaw<'a>>::RawApiTable, RawModuleAbi<'_, Self>) -> bool,
    >;
}
