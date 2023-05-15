use crate::error::{Result, ResultExt};
use classicube_helpers::CellGetSet;
use std::{cell::Cell, fmt::Display, marker::PhantomData, str::FromStr, thread::LocalKey};

pub struct RustOption<T>
where
    T: FromStr + Display + Copy + 'static,
    Self: 'static,
{
    key: &'static str,

    // has to be static str because we use default() in the static clap app
    default: &'static str,

    cache: LocalKey<Cell<Option<T>>>,

    _phantom: PhantomData<T>,
}

impl<T> RustOption<T>
where
    T: FromStr + Display + Copy + 'static,
    Self: 'static,
    <T as FromStr>::Err: std::error::Error + Send + 'static,
{
    pub const fn new(
        key: &'static str,
        default: &'static str,
        cache: LocalKey<Cell<Option<T>>>,
    ) -> Self {
        Self {
            key,
            default,
            cache,
            _phantom: PhantomData,
        }
    }

    pub fn get(&'static self) -> Result<T> {
        if let Some(cache) = self.cache.get() {
            Ok(cache)
        } else {
            let value = if let Some(value) = super::get(self.key) {
                value
                    .parse()
                    .chain_err(|| format!("couldn't parse options key {}", self.key))?
            } else {
                self.default
                    .parse()
                    .chain_err(|| format!("couldn't parse default {}", self.key))?
            };
            self.cache.set(Some(value));

            Ok(value)
        }
    }

    pub fn set(&'static self, value: T) {
        super::set(self.key, format!("{value}"));
        self.cache.set(None);
    }

    pub fn default(&'static self) -> &'static str {
        self.default
    }
}
