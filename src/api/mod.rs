use crate::{
    impl_sealing::Sealed,
    raw::{RawApiTable, ZygiskRaw},
};

pub mod v1;
pub use v1::V1;

pub mod v2;
pub use v2::V2;

pub mod v3;
pub use v3::V3;

mod v4;

mod v5;

pub trait ZygiskSpec
where
    Self: Clone + Copy + Sealed + Sized,
    <Self as ZygiskSpec>::Spec: ZygiskSpec,
{
    type Spec;
}

#[repr(transparent)]
pub struct ZygiskApi<'a, Version>(pub(crate) RawApiTable<'a, Version>)
where
    Version: ZygiskRaw<'a> + 'a;

impl<'a, Version> ZygiskApi<'a, Version>
where
    Version: ZygiskRaw<'a> + 'a,
{
    pub(crate) unsafe fn dispatch(&self) -> &<Version as ZygiskRaw<'a>>::RawApiTable {
        &*self.0 .0
    }
}
