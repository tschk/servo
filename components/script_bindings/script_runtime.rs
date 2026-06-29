/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::cell::Cell;
use std::marker::PhantomData;

use js::context::JSContext as JsContext;
use js::jsapi::JSContext as RawJSContext;
use js::realm::{AutoRealm, CurrentRealm};

/// Servo bindings view of the active JS context.
pub type JSContext = JsContext;
pub type SafeJSContext = JsContext;

/// Copy a mutable JS context reference for APIs that take `JSContext` by value.
#[inline]
pub fn copy_cx(cx: &mut JsContext) -> JsContext {
    unsafe { JsContext::from_raw_ptr(cx.raw_cx()) }
}

/// Construct a `JSContext` from a realm guard.
#[inline]
pub unsafe fn cx_from_realm<'a>(realm: &mut CurrentRealm<'a>) -> JsContext {
    JsContext::from_raw_ptr(realm.raw_cx())
}

/// Construct a `JSContext` from an auto-realm guard.
#[inline]
pub unsafe fn cx_from_auto_realm<'a>(realm: &mut AutoRealm<'a>) -> JsContext {
    JsContext::from_raw_ptr(realm.raw_cx())
}

thread_local!(
    static THREAD_ACTIVE: Cell<bool> = const { Cell::new(true) };
);

pub fn runtime_is_alive() -> bool {
    THREAD_ACTIVE.with(|t| t.get())
}

pub fn mark_runtime_dead() {
    THREAD_ACTIVE.with(|t| t.set(false));
}

/// Get the current JSContext for the running thread.
///
/// ## Safety
/// Using this function is unsafe because no other JSContext may be constructed apart from initial ones,
/// but because we are still working on passing down &mut SafeJSContext references,
/// this function is provided as temporary workaround/placeholder.
///
/// As such all it's usages will need to be eventually replaced with proper &mut SafeJSContext references.
pub unsafe fn temp_cx() -> SafeJSContext {
    unsafe { JSContext::from_raw_ptr(js::rust::Runtime::get().unwrap().as_ptr()) }
}

#[derive(Clone, Copy, Debug)]
/// A compile-time marker that there are operations that could trigger a JS garbage collection
/// operation within the current stack frame. It is trivially copyable, so it should be passed
/// as a function argument and reused when calling other functions whenever possible. Since it
/// is only meaningful within the current stack frame, it is impossible to move it to a different
/// thread or into a task that will execute asynchronously.
pub struct CanGc(PhantomData<*mut ()>);

impl CanGc {
    /// Create a new CanGc value, representing that a GC operation is possible within the
    /// current stack frame.
    ///
    /// Deprecrated: do not use. Instead, use [`CanGc::from_cx(cx)`] with a `cx` from a task
    /// callback or by declaring it in Bindings.conf.
    pub fn deprecated_note() -> CanGc {
        CanGc(PhantomData)
    }

    /// &mut SafeJSContext is always an indication that GC is possible.
    pub fn from_cx<T>(_cx: &mut T) -> CanGc {
        CanGc::deprecated_note()
    }
}