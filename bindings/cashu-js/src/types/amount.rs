use std::ops::Deref;

use cashu::Amount;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = Amount)]
pub struct JsAmount {
    inner: Amount,
}

impl Deref for JsAmount {
    type Target = Amount;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<Amount> for JsAmount {
    fn from(inner: Amount) -> JsAmount {
        JsAmount { inner }
    }
}

#[wasm_bindgen(js_class = Amount)]
impl JsAmount {
    #[wasm_bindgen(constructor)]
    pub fn new(sats: u64) -> Self {
        Self {
            inner: Amount::from_sat(sats),
        }
    }

    /// From Sats
    #[wasm_bindgen(js_name = fromSat)]
    pub fn from_sat(sats: u64) -> Self {
        Self {
            inner: Amount::from_sat(sats),
        }
    }

    /// From Msats
    #[wasm_bindgen(js_name = fromMSat)]
    pub fn from_msat(msats: u64) -> Self {
        Self {
            inner: Amount::from_msat(msats),
        }
    }

    /// Get as sats
    #[wasm_bindgen(js_name = toSat)]
    pub fn to_sat(&self) -> u64 {
        self.inner.to_sat()
    }

    /// Get as msats
    #[wasm_bindgen(js_name = toMSat)]
    pub fn to_msat(&self) -> u64 {
        self.inner.to_msat()
    }

    /// Split amount returns sat vec of sats
    // REVIEW: https://github.com/rustwasm/wasm-bindgen/issues/111
    #[wasm_bindgen(js_name = split)]
    pub fn split(&self) -> Vec<u64> {
        let split = self.inner.split();
        split.into_iter().map(|a| a.to_sat()).collect()
    }
}
