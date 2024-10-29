use crate::{
    impl_sealing::Sealed,
    raw::{RawApiTable, ZygiskRaw},
};

pub mod v1;
pub use v1::V1;

pub mod v2;
pub use v2::V2;

mod v3;

mod v4;

mod v5;

pub trait ZygiskSpec
where
    Self: Clone + Copy + Sealed + Sized,
    <Self as ZygiskSpec>::Spec: ZygiskSpec,
{
    type Spec;
}

pub struct ZygiskApi<'a, Version>(pub(crate) RawApiTable<'a, Version>)
where
    for<'b> Version: ZygiskRaw<'b> + 'b;

impl<'a, Version> ZygiskApi<'a, Version>
where
    for<'b> Version: ZygiskRaw<'b> + 'b,
{
    pub(crate) fn as_tbl(&self) -> &<Version as ZygiskRaw<'_>>::RawApiTable {
        unsafe { &*self.0 .0.cast() }
    }
}
