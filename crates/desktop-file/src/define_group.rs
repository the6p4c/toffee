//! Provides a simple way to define a standardized group in a desktop file with both required and
//! optional keys.
//!
//! # Example
//! ```
//! use desktop_file::define_group::preamble::*;
//! use desktop_file::ParseError;
//!
//! define_group! {
//!     #[derive(Debug)]
//!     #[error(MyError)]
//!     pub struct MyGroup {
//!         pub first_key: Required<String>,
//!         pub second_key: Option<Vec<String>>,
//!         #[key("Is-Fancy")]
//!         pub fancy: Option<bool>,
//!     }
//! }
//!
//! enum MyError {
//!     Parse(ParseError),
//!     RequiredKeyMissing(&'static str),
//! }
//!
//! impl From<ParseError> for MyError {
//!     fn from(value: ParseError) -> Self {
//!         Self::Parse(value)
//!     }
//! }
//!
//! impl From<RequiredKeyMissing> for MyError {
//!     fn from(value: RequiredKeyMissing) -> Self {
//!         Self::RequiredKeyMissing(value.0)
//!     }
//! }
//! ```
use std::marker::PhantomData;

use crate::{FromRaw, Group, ParseError};

pub mod preamble {
    pub use super::{GroupExt, Required, RequiredKeyMissing};
    pub use crate::define_group;
}

pub struct RequiredKeyMissing(pub &'static str);

pub trait GroupExt {
    fn get_optional<V: FromRaw, E: From<RequiredKeyMissing> + From<ParseError>>(
        &self,
        key: &'static str,
    ) -> Result<Option<V>, E>;

    fn get_required<V: FromRaw, E: From<RequiredKeyMissing> + From<ParseError>>(
        &self,
        key: &'static str,
    ) -> Result<V, E>;
}

impl GroupExt for Group<'_> {
    fn get_optional<V: FromRaw, E: From<RequiredKeyMissing> + From<ParseError>>(
        &self,
        key: &'static str,
    ) -> Result<Option<V>, E> {
        let value = self.get::<V>(key).transpose()?;
        Ok(value)
    }

    fn get_required<V: FromRaw, E: From<RequiredKeyMissing> + From<ParseError>>(
        &self,
        key: &'static str,
    ) -> Result<V, E> {
        let value = self.get::<V>(key).ok_or(RequiredKeyMissing(key))??;
        Ok(value)
    }
}

pub trait GroupValue<E: From<RequiredKeyMissing> + From<ParseError>> {
    type Value;

    fn get_from(group: &Group, key: &'static str) -> Result<Self::Value, E>;
}

impl<E: From<RequiredKeyMissing> + From<ParseError>, V: FromRaw> GroupValue<E> for Option<V> {
    type Value = Self;

    fn get_from(group: &Group, key: &'static str) -> Result<Self::Value, E> {
        group.get_optional(key)
    }
}

pub struct Required<T>(PhantomData<T>);

impl<E: From<RequiredKeyMissing> + From<ParseError>, V: FromRaw> GroupValue<E> for Required<V> {
    type Value = V;

    fn get_from(group: &Group, key: &'static str) -> Result<Self::Value, E> {
        group.get_required(key)
    }
}

#[macro_export]
macro_rules! define_group_key {
    ($name:ident) => {
        map_ascii_case!(Case::Pascal, stringify!($name))
    };
    (#[key($key:expr)] $name:ident) => {
        $key
    };
}

#[macro_export]
macro_rules! define_group {
    {
        $(#[$meta:meta])?
        #[error($E:ty)]
        $vis:vis struct $name:ident {
            $(
                $(#[key($field_key:expr)])?
                pub $field_name:ident: $field_type:ty
            ),*$(,)?
        }
    } => {
        $(#[$meta])?
        $vis struct $name {
            $(pub $field_name: <$field_type as $crate::define_group::GroupValue<$E>>::Value),*
        }

        impl $name {
            fn try_from_group(group: &$crate::Group) -> Result<Self, $E> {
                use const_format::{map_ascii_case, Case};

                use $crate::define_group::GroupValue;
                use $crate::define_group_key;

                Ok(Self {
                    $(
                        $field_name: <$field_type as GroupValue<$E>>::get_from(
                            group,
                            define_group_key!($(#[key($field_key)])? $field_name)
                        )?
                    ),*
                })
            }
        }
    }
}
