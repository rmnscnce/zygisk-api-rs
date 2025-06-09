use core::{marker::PhantomData, ptr::NonNull};

use jni::JNIEnv;
use libc::c_long;

use crate::{impl_sealing::Sealed, ZygiskModule};

pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;
pub mod v5;

pub struct RawModule<'a, Version>
where
    Version: ZygiskRaw<'a> + 'a,
{
    pub(crate) dispatch: &'a dyn ZygiskModule<Api = Version>,
    pub(crate) api_table: RawApiTable<'a, Version>,
    pub(crate) jni_env: JNIEnv<'a>,
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct RawApiTable<'a, Version>(
    // Note: This is a pointer to const
    pub(crate) NonNull<<Version as ZygiskRaw<'a>>::ApiTable>,
    PhantomData<&'a Version>,
)
where
    Version: ZygiskRaw<'a> + 'a;

impl<'a, Version> RawApiTable<'a, Version>
where
    Version: ZygiskRaw<'a> + 'a,
{
    #[doc(hidden)]
    pub const fn from_non_null(non_null: NonNull<<Version as ZygiskRaw<'a>>::ApiTable>) -> Self {
        Self(non_null, PhantomData)
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

/// Opaque type representing the API instance handle pointer
#[repr(transparent)]
pub(crate) struct Instance(());

#[repr(C)]
pub(crate) struct BaseApi<V>
where
    for<'a> V: ZygiskRaw<'a>,
{
    pub(crate) this: NonNull<Instance>,
    pub(crate) register_module_fn: for<'a> unsafe extern "C" fn(
        NonNull<<V as ZygiskRaw>::ApiTable>,
        RawModuleAbi<'a, V>,
    ) -> bool,
}

#[repr(transparent)]
pub struct RawModuleAbi<'a, Version>(
    pub(crate) NonNull<ModuleAbi<'a, Version>>,
    PhantomData<&'a Version>,
)
where
    Version: ZygiskRaw<'a>;

impl<'a, Version> RawModuleAbi<'a, Version>
where
    Version: ZygiskRaw<'a>,
{
    pub(crate) fn from_non_null(non_null: NonNull<ModuleAbi<'a, Version>>) -> Self {
        Self(non_null, PhantomData)
    }
}

pub trait ZygiskRaw<'a>
where
    Self: Sealed + Copy + Sized + 'a,
{
    const API_VERSION: c_long;
    type ApiTable: 'a;
    type AppSpecializeArgs: 'a;
    type ServerSpecializeArgs: 'a;

    fn abi_from_module(module: &'a mut RawModule<'a, Self>) -> ModuleAbi<'a, Self>;

    fn register_module_fn(
        table: &'a <Self as ZygiskRaw<'a>>::ApiTable,
    ) -> for<'b> unsafe extern "C" fn(
        NonNull<<Self as ZygiskRaw<'a>>::ApiTable>,
        RawModuleAbi<'b, Self>,
    ) -> bool;
}
