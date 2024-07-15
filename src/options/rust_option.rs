use std::{cell::Cell, fmt::Display, str::FromStr, thread::LocalKey};

use crate::error::{Result, ResultExt};

pub struct RustOption<T>
where
    T: FromStr + Display + Copy + 'static,
    Self: 'static,
{
    key: &'static str,

    default: T,

    cache: LocalKey<Cell<Option<T>>>,
}

impl<T> RustOption<T>
where
    T: FromStr + Display + Copy + 'static,
    Self: 'static,
    <T as FromStr>::Err: std::error::Error + Send + 'static,
{
    pub const fn new(key: &'static str, default: T, cache: LocalKey<Cell<Option<T>>>) -> Self {
        Self {
            key,
            default,
            cache,
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
            };
            self.cache.set(Some(value));

            Ok(value)
        }
    }

    pub fn set(&'static self, value: T) {
        super::set(self.key, format!("{value}"));
        self.cache.set(None);
    }

    pub const fn default(&'static self) -> T {
        self.default
    }
}
