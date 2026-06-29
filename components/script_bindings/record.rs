/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! The `Record` (open-ended dictionary) type.

use std::cmp::Eq;
use std::hash::Hash;
use std::marker::Sized;
use std::ops::{Deref, DerefMut};

use indexmap::IndexMap;
use js::context::{JSContext, RawJSContext};
use js::conversions::{ConversionResult, FromJSValConvertible, ToJSValConvertible};
use js::jsapi::{
    JSITER_HIDDEN, JSITER_OWNONLY, JSITER_SYMBOLS, JSPROP_ENUMERATE, PropertyDescriptor,
};
use js::jsval::{ObjectValue, UndefinedValue};
use js::rust::wrappers2::{
    GetPropertyKeys, JS_DefineUCProperty2, JS_GetOwnPropertyDescriptorById, JS_GetPropertyById,
    JS_IdToValue, JS_NewPlainObject,
};
use js::rust::{HandleId, HandleValue, IdVector, MutableHandleValue};

use crate::conversions::jsid_to_string;
use crate::str::{ByteString, DOMString, USVString};

pub trait RecordKey: Eq + Hash + Sized {
    fn to_utf16_vec(&self) -> Vec<u16>;

    /// Attempt to extract a key from a JS id.
    #[allow(clippy::result_unit_err)]
    fn from_id(cx: &mut JSContext, id: HandleId) -> Result<ConversionResult<Self>, ()>;
}

impl RecordKey for DOMString {
    fn to_utf16_vec(&self) -> Vec<u16> {
        self.str().encode_utf16().collect::<Vec<_>>()
    }

    fn from_id(cx: &mut JSContext, id: HandleId) -> Result<ConversionResult<Self>, ()> {
        match jsid_to_string(cx, id) {
            Some(s) => Ok(ConversionResult::Success(s)),
            None => Ok(ConversionResult::Failure(c"Failed to get DOMString".into())),
        }
    }
}

impl RecordKey for USVString {
    fn to_utf16_vec(&self) -> Vec<u16> {
        self.0.encode_utf16().collect::<Vec<_>>()
    }

    fn from_id(cx: &mut JSContext, id: HandleId) -> Result<ConversionResult<Self>, ()> {
        rooted!(&in(cx) let mut jsid_value = UndefinedValue());
        unsafe { JS_IdToValue(crate::script_runtime::copy_cx(cx), id.get(), jsid_value.handle_mut()) };

        USVString::safe_from_jsval(cx, jsid_value.handle(), ())
    }
}

impl RecordKey for ByteString {
    fn to_utf16_vec(&self) -> Vec<u16> {
        self.iter().map(|&x| x as u16).collect::<Vec<u16>>()
    }

    fn from_id(cx: &mut JSContext, id: HandleId) -> Result<ConversionResult<Self>, ()> {
        rooted!(&in(cx) let mut jsid_value = UndefinedValue());
        unsafe { JS_IdToValue(crate::script_runtime::copy_cx(cx), id.get(), jsid_value.handle_mut()) };

        ByteString::safe_from_jsval(cx, jsid_value.handle(), ())
    }
}

/// The `Record` (open-ended dictionary) type.
#[derive(Clone, JSTraceable)]
pub struct Record<K: RecordKey + crate::JSTraceable, V: crate::JSTraceable> {
    #[custom_trace]
    map: IndexMap<K, V>,
}

impl<K: RecordKey + crate::JSTraceable, V: crate::JSTraceable> Record<K, V> {
    /// Create an empty `Record`.
    pub fn new() -> Self {
        Record {
            map: IndexMap::new(),
        }
    }
}

impl<K: RecordKey + crate::JSTraceable, V: crate::JSTraceable> Deref for Record<K, V> {
    type Target = IndexMap<K, V>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl<K: RecordKey + crate::JSTraceable, V: crate::JSTraceable> DerefMut for Record<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

impl<K, V, C> FromJSValConvertible for Record<K, V>
where
    K: RecordKey + crate::JSTraceable,
    V: FromJSValConvertible<Config = C> + crate::JSTraceable,
    C: Clone,
{
    type Config = C;
    unsafe fn from_jsval(
        _cx: *mut RawJSContext,
        value: HandleValue,
        config: C,
    ) -> Result<ConversionResult<Self>, ()> {
        // TODO https://github.com/servo/mozjs/issues/749
        let mut cx = unsafe { crate::script_runtime::temp_cx() };
        FromJSValConvertible::safe_from_jsval(&mut cx, value, config)
    }

    fn safe_from_jsval(
        cx: &mut JSContext,
        value: HandleValue,
        config: C,
    ) -> Result<ConversionResult<Self>, ()> {
        if !value.is_object() {
            return Ok(ConversionResult::Failure(
                c"Record value was not an object".into(),
            ));
        }

        rooted!(&in(cx) let object = value.to_object());
        let mut ids = unsafe { IdVector::new(cx) };
        if unsafe {
            !GetPropertyKeys(
                crate::script_runtime::copy_cx(cx),
                object.handle(),
                JSITER_OWNONLY | JSITER_HIDDEN | JSITER_SYMBOLS,
                ids.handle_mut(),
            )
        } {
            return Err(());
        }

        let mut map = IndexMap::new();
        for id in &*ids {
            rooted!(&in(cx) let id = *id);
            rooted!(&in(cx) let mut desc = PropertyDescriptor::default());

            let mut is_none = false;
            if unsafe {
                !JS_GetOwnPropertyDescriptorById(
                    crate::script_runtime::copy_cx(cx),
                    object.handle(),
                    id.handle(),
                    desc.handle_mut(),
                    &mut is_none,
                )
            } {
                return Err(());
            }

            if !desc.enumerable_() {
                continue;
            }

            let key = match K::from_id(cx, id.handle())? {
                ConversionResult::Success(key) => key,
                ConversionResult::Failure(message) => {
                    return Ok(ConversionResult::Failure(message));
                },
            };

            rooted!(&in(cx) let mut property = UndefinedValue());
            if unsafe {
                !JS_GetPropertyById(crate::script_runtime::copy_cx(cx), object.handle(), id.handle(), property.handle_mut())
            } {
                return Err(());
            }

            let property = match V::safe_from_jsval(cx, property.handle(), config.clone())? {
                ConversionResult::Success(property) => property,
                ConversionResult::Failure(message) => {
                    return Ok(ConversionResult::Failure(message));
                },
            };
            map.insert(key, property);
        }

        Ok(ConversionResult::Success(Record { map }))
    }
}

impl<K, V> ToJSValConvertible for Record<K, V>
where
    K: RecordKey + crate::JSTraceable,
    V: ToJSValConvertible + crate::JSTraceable,
{
    #[inline]
    unsafe fn to_jsval(&self, _cx: *mut RawJSContext, rval: MutableHandleValue) {
        // TODO: https://github.com/servo/mozjs/issues/764
        // This is needed until the `RawJSContext` version is removed from the trait.
        let mut cx = unsafe { crate::script_runtime::temp_cx() };
        ToJSValConvertible::safe_to_jsval(self, &mut cx, rval);
    }

    fn safe_to_jsval(&self, cx: &mut JSContext, mut rval: MutableHandleValue) {
        rooted!(&in(cx) let js_object = unsafe { JS_NewPlainObject(cx) });
        assert!(!js_object.handle().is_null());

        rooted!(&in(cx) let mut js_value = UndefinedValue());
        for (key, value) in &self.map {
            let key = key.to_utf16_vec();
            value.safe_to_jsval(cx, js_value.handle_mut());

            assert!(unsafe {
                JS_DefineUCProperty2(
                    cx,
                    js_object.handle(),
                    key.as_ptr(),
                    key.len(),
                    js_value.handle(),
                    JSPROP_ENUMERATE as u32,
                )
            });
        }

        rval.set(ObjectValue(js_object.handle().get()));
    }
}

impl<K: RecordKey + crate::JSTraceable, V: crate::JSTraceable> Default for Record<K, V> {
    fn default() -> Self {
        Self::new()
    }
}
