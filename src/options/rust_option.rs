use crate::error::*;
use std::{fmt::Display, marker::PhantomData, str::FromStr};

// TODO cache get, update on set

pub struct RustOption<T>
where
    T: FromStr + Display,
{
    key: &'static str,

    // has to be static str because we use default() in the static clap app
    default: &'static str,

    _phantom: PhantomData<T>,
}

impl<T> RustOption<T>
where
    T: FromStr + Display,
    <T as FromStr>::Err: std::error::Error,
    <T as FromStr>::Err: Send,
    <T as FromStr>::Err: 'static,
{
    pub const fn new(key: &'static str, default: &'static str) -> Self {
        Self {
            key,
            default,
            _phantom: PhantomData,
        }
    }

    pub fn get(&self) -> Result<T> {
        if let Some(value) = super::get(self.key) {
            let value = value
                .parse()
                .chain_err(|| format!("couldn't parse options key {}", self.key))?;
            Ok(value)
        } else {
            let value = self
                .default
                .parse()
                .chain_err(|| format!("couldn't parse default {}", self.key))?;
            Ok(value)
        }
    }

    pub fn set(&self, value: T) {
        super::set(self.key, format!("{}", value));
    }

    pub fn default(&self) -> &'static str {
        self.default
    }
}
