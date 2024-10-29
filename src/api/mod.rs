use crate::{impl_sealing::Sealed, raw::ZygiskRaw};

pub mod v1;
pub use v1::V1;

pub mod v2;
pub use v2::V2;

mod v3;

mod v4;

mod v5;

pub trait ZygiskSpec
where
    Self: Sealed,
    <Self as ZygiskSpec>::Spec: ZygiskSpec,
{
    type Spec;
}

pub struct ZygiskApi<'a, Version>(pub(crate) &'a Version::RawApiTable<'a>)
where
    Version: ZygiskRaw;
