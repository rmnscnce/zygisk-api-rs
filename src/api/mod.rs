use crate::{impl_sealing::Sealed, raw::ZygiskRawApi};

mod v1;
pub use v1::V1;

mod v2;
pub use v2::V2;

mod v3;

mod v4;

mod v5;

pub trait ZygiskApiSpec
where
    Self: Sealed,
{
}

pub struct ZygiskApi<'a, Version>(pub(crate) &'a Version::RawApiTable<'a>)
where
    Version: ZygiskRawApi;
