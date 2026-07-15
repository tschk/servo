// V8-backed JS engine bridge — replaces mozjs SpiderMonkey FFI.
// Uses rusty_v8 with thread-local isolates and persistent handles.

#[macro_use]
mod macros;

#[cfg(feature = "v8")]
pub mod v8_glue;

use std::cell::RefCell;
use std::collections::HashMap;
use std::ptr;
use std::sync::Once;

#[cfg(feature = "v8")]
use rusty_v8 as v8;
#[cfg(feature = "v8")]
static V8_INIT: Once = Once::new();
#[cfg(feature = "v8")]
thread_local! {
    static V8_ISOLATE: RefCell<Option<v8::OwnedIsolate>> = RefCell::new(None);
    static V8_CONTEXTS: RefCell<HashMap<usize, v8::Global<v8::Context>>> = RefCell::new(HashMap::new());
    static V8_SLOTS: RefCell<Vec<Box<dyn std::any::Any>>> = RefCell::new(Vec::new());
}

#[cfg(feature = "v8")]
pub fn ensure_v8() {
    V8_INIT.call_once(|| {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
    });
}

// ── Core types ──────────────────────────────────────────────────────────────

pub mod glue {
    use super::jsapi;
    use std::cell::RefCell;
    use std::collections::HashMap;

    // ── type definitions ──

    #[repr(C)]
    pub struct ProxyTraps {
        pub enter:
            Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject) -> bool>,
        pub getOwnPropertyDescriptor: Option<
            unsafe extern "C" fn(
                *mut jsapi::JSContext,
                jsapi::HandleObject<'_>,
                jsapi::HandleId<'_>,
                jsapi::MutableHandle<'_, jsapi::PropertyDescriptor>,
                *mut bool,
            ) -> bool,
        >,
        pub defineProperty: Option<
            unsafe extern "C" fn(
                *mut jsapi::JSContext,
                jsapi::HandleObject<'_>,
                jsapi::HandleId<'_>,
                jsapi::Handle<'_, jsapi::PropertyDescriptor>,
                *mut jsapi::ObjectOpResult,
            ) -> bool,
        >,
        pub ownPropertyKeys: Option<
            unsafe extern "C" fn(
                *mut jsapi::JSContext,
                jsapi::HandleObject<'_>,
                jsapi::MutableHandleIdVector,
            ) -> bool,
        >,
        pub delete_: Option<
            unsafe extern "C" fn(
                *mut jsapi::JSContext,
                jsapi::HandleObject<'_>,
                jsapi::HandleId<'_>,
                *mut jsapi::ObjectOpResult,
            ) -> bool,
        >,
        pub enumerate: Option<
            unsafe extern "C" fn(
                *mut jsapi::JSContext,
                *mut jsapi::JSObject,
                *mut jsapi::ObjectOpResult,
            ) -> bool,
        >,
        pub getPrototypeIfOrdinary: Option<
            unsafe extern "C" fn(
                *mut jsapi::JSContext,
                jsapi::HandleObject<'_>,
                *mut bool,
                jsapi::MutableHandleObject<'_>,
            ) -> bool,
        >,
        pub getPrototype: Option<
            for<'a, 'b> unsafe extern "C" fn(
                *mut jsapi::JSContext,
                jsapi::HandleObject<'a>,
                jsapi::MutableHandleObject<'b>,
            ) -> bool,
        >,
        pub setPrototype: Option<
            for<'a, 'b> unsafe extern "C" fn(
                *mut jsapi::JSContext,
                jsapi::HandleObject<'a>,
                jsapi::HandleObject<'b>,
                *mut jsapi::ObjectOpResult,
            ) -> bool,
        >,
        pub setImmutablePrototype: Option<
            unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, *mut bool) -> bool,
        >,
        pub preventExtensions: Option<
            unsafe extern "C" fn(
                *mut jsapi::JSContext,
                jsapi::HandleObject<'_>,
                *mut jsapi::ObjectOpResult,
            ) -> bool,
        >,
        pub isExtensible: Option<
            unsafe extern "C" fn(*mut jsapi::JSContext, jsapi::HandleObject<'_>, *mut bool) -> bool,
        >,
        pub has: Option<
            unsafe extern "C" fn(
                *mut jsapi::JSContext,
                jsapi::HandleObject<'_>,
                jsapi::HandleId<'_>,
                *mut bool,
            ) -> bool,
        >,
        pub get: Option<
            unsafe extern "C" fn(
                *mut jsapi::JSContext,
                jsapi::HandleObject<'_>,
                jsapi::HandleValue<'_>,
                jsapi::HandleId<'_>,
                jsapi::MutableHandleValue<'_>,
            ) -> bool,
        >,
        pub set: Option<
            for<'a, 'b, 'c, 'd> unsafe extern "C" fn(
                *mut jsapi::JSContext,
                jsapi::HandleObject<'a>,
                jsapi::HandleId<'b>,
                jsapi::HandleValue<'c>,
                jsapi::HandleValue<'d>,
                *mut jsapi::ObjectOpResult,
            ) -> bool,
        >,
        pub call: Option<
            unsafe extern "C" fn(
                *mut jsapi::JSContext,
                *mut jsapi::JSObject,
                *const jsapi::JSVal,
                *mut jsapi::JSVal,
            ) -> bool,
        >,
        pub construct: Option<
            unsafe extern "C" fn(
                *mut jsapi::JSContext,
                *mut jsapi::JSObject,
                *const jsapi::JSVal,
                *mut jsapi::JSVal,
            ) -> bool,
        >,
        pub hasOwn: Option<
            unsafe extern "C" fn(
                *mut jsapi::JSContext,
                jsapi::HandleObject<'_>,
                jsapi::HandleId<'_>,
                *mut bool,
            ) -> bool,
        >,
        pub getOwnEnumerablePropertyKeys: Option<
            unsafe extern "C" fn(
                *mut jsapi::JSContext,
                jsapi::HandleObject<'_>,
                jsapi::MutableHandleIdVector,
            ) -> bool,
        >,
        pub nativeCall: Option<
            unsafe extern "C" fn(
                *mut jsapi::JSContext,
                *mut jsapi::JSObject,
                bool,
                *const jsapi::JSVal,
                *mut jsapi::JSVal,
            ) -> bool,
        >,
        pub objectClassIs: Option<
            unsafe extern "C" fn(
                *mut jsapi::JSContext,
                *mut jsapi::JSObject,
                u32,
                *mut bool,
            ) -> bool,
        >,
        pub className: Option<
            unsafe extern "C" fn(
                *mut jsapi::JSContext,
                jsapi::HandleObject<'_>,
            ) -> *const std::os::raw::c_char,
        >,
        pub fun_toString: Option<
            unsafe extern "C" fn(
                *mut jsapi::JSContext,
                *mut jsapi::JSObject,
                bool,
            ) -> *mut jsapi::JSString,
        >,
        pub boxedValue_unbox: Option<
            unsafe extern "C" fn(
                *mut jsapi::JSContext,
                *mut jsapi::JSObject,
                *mut jsapi::JSVal,
            ) -> bool,
        >,
        pub defaultValue: Option<
            unsafe extern "C" fn(
                *mut jsapi::JSContext,
                *mut jsapi::JSObject,
                u32,
                *mut jsapi::JSVal,
            ) -> bool,
        >,
        pub trace: Option<unsafe extern "C" fn(*mut jsapi::JSTracer, *mut jsapi::JSObject)>,
        pub finalize: Option<unsafe extern "C" fn(*mut jsapi::GCContext, *mut jsapi::JSObject)>,
        pub objectMoved: Option<unsafe extern "C" fn(*mut jsapi::JSObject, *const jsapi::JSObject)>,
        pub isCallable: Option<unsafe extern "C" fn(*mut jsapi::JSObject) -> bool>,
        pub isConstructor: Option<unsafe extern "C" fn(*mut jsapi::JSObject) -> bool>,
    }

    #[derive(Default)]
    pub struct ServoSizes {
        pub gcHeapUsed: usize,
        pub gcHeapUnused: usize,
        pub gcHeapAdmin: usize,
        pub gcHeapDecommitted: usize,
        pub mallocHeap: usize,
        pub nonHeap: usize,
    }

    pub type DispatchablePointer = *mut std::ffi::c_void;

    pub struct JobQueueTraps {
        pub getHostDefinedData: Option<
            unsafe extern "C" fn(
                *const std::ffi::c_void,
                *mut jsapi::JSContext,
                jsapi::MutableHandleObject<'_>,
            ) -> bool,
        >,
        pub enqueuePromiseJob: Option<
            unsafe extern "C" fn(
                *const std::ffi::c_void,
                *mut jsapi::JSContext,
                jsapi::HandleObject<'_>,
                jsapi::HandleObject<'_>,
                jsapi::HandleObject<'_>,
                jsapi::HandleObject<'_>,
            ) -> bool,
        >,
        pub runJobs: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut jsapi::JSContext)>,
        pub empty: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> bool>,
        pub pushNewInterruptQueue:
            Option<unsafe extern "C" fn(*mut std::ffi::c_void) -> *const std::ffi::c_void>,
        pub popInterruptQueue:
            Option<unsafe extern "C" fn(*mut std::ffi::c_void) -> *const std::ffi::c_void>,
        pub dropInterruptQueues: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
    }
    unsafe impl Sync for JobQueueTraps {}

    pub struct JSPrincipalsCallbacks {
        pub write: Option<
            unsafe extern "C" fn(
                *mut jsapi::JSPrincipals,
                *mut jsapi::JSContext,
                *mut jsapi::JSStructuredCloneWriter,
            ) -> bool,
        >,
        pub isSystemOrAddonPrincipal:
            Option<unsafe extern "C" fn(*mut jsapi::JSPrincipals) -> bool>,
    }

    pub trait ToSlotValue {
        fn to_slot_value(self) -> jsapi::JSVal;
    }

    impl ToSlotValue for jsapi::JSVal {
        fn to_slot_value(self) -> jsapi::JSVal {
            self
        }
    }

    impl ToSlotValue for &jsapi::JSVal {
        fn to_slot_value(self) -> jsapi::JSVal {
            *self
        }
    }

    // ── stub functions (return default zero/null/false) ──
    // These compile but return placeholder values. When the `v8` feature is
    // active the real V8-backed implementations in `v8_glue` replace them.

    glue_stub!(pub fn IsWrapper(_obj: *mut jsapi::JSObject) -> bool);
    pub fn UnwrapObjectDynamic<O, C>(_obj: O, _cx: C, _stop: bool) -> *mut jsapi::JSObject {
        ::std::ptr::null_mut()
    }
    #[cfg(not(feature = "v8"))]
    glue_stub!(pub fn RUST_JSID_TO_STRING(_cx: *mut jsapi::JSContext, _id: *const jsapi::jsid) -> *mut jsapi::JSString);
    #[cfg(feature = "v8")]
    pub fn RUST_JSID_TO_STRING(
        _cx: *mut jsapi::JSContext,
        id: *const jsapi::jsid,
    ) -> *mut jsapi::JSString {
        if id.is_null() {
            return std::ptr::null_mut();
        }
        // SAFETY: non-null jsid pointer checked above.
        crate::v8_glue::id_to_string(unsafe { *id })
    }
    pub fn AppendToIdVector<V>(_v: V, _id: jsapi::jsid) -> bool {
        false
    }
    glue_stub!(pub fn GetProxyHandler(_proxy: *mut jsapi::JSObject) -> *const std::ffi::c_void);

    // ---- proxy / DOM global (V8-backed when v8 feature is enabled) ----
    #[cfg(not(feature = "v8"))]
    pub fn NewProxyObject<P, O>(
        _cx: *mut jsapi::JSContext,
        _handler: *const std::ffi::c_void,
        _priv: P,
        _proto: *mut jsapi::JSObject,
        _options: O,
        _flag: bool,
    ) -> *mut jsapi::JSObject {
        ::std::ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn NewProxyObject<P, O>(
        cx: *mut jsapi::JSContext,
        handler: *const std::ffi::c_void,
        _priv: P,
        proto: *mut jsapi::JSObject,
        _options: O,
        flag: bool,
    ) -> *mut jsapi::JSObject {
        crate::v8_glue::new_proxy_object(cx, handler, proto, flag)
    }

    #[cfg(not(feature = "v8"))]
    pub fn GetProxyPrivate<O, V>(_proxy: O, _out: V) {}
    #[cfg(feature = "v8")]
    pub fn GetProxyPrivate<O, V>(_proxy: O, _out: V) {
        crate::v8_glue::get_proxy_private()
    }

    #[cfg(not(feature = "v8"))]
    pub fn SetProxyPrivate<O, V>(_proxy: O, _priv: V) {}
    #[cfg(feature = "v8")]
    pub fn SetProxyPrivate<O, V>(_proxy: O, _priv: V) {
        crate::v8_glue::set_proxy_private()
    }

    #[cfg(not(feature = "v8"))]
    glue_stub!(pub fn DeletePropertyIgnoringResult(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _prop: *const u8));
    #[cfg(feature = "v8")]
    pub fn DeletePropertyIgnoringResult(
        cx: *mut jsapi::JSContext,
        obj: *mut jsapi::JSObject,
        prop: *const u8,
    ) {
        crate::v8_glue::delete_property_ignoring_result(cx, obj, prop)
    }

    #[cfg(not(feature = "v8"))]
    glue_stub!(pub fn DefinePropertyById(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _id: *const jsapi::jsid, _desc: *const jsapi::JSPropertySpec) -> bool);
    #[cfg(feature = "v8")]
    pub fn DefinePropertyById(
        cx: *mut jsapi::JSContext,
        obj: *mut jsapi::JSObject,
        id: *const jsapi::jsid,
        desc: *const jsapi::JSPropertySpec,
    ) -> bool {
        crate::v8_glue::define_property_by_id(cx, obj, id, desc)
    }

    #[cfg(not(feature = "v8"))]
    pub fn SetDataPropertyDescriptor<D, V>(_desc: D, _value: V, _attrs: u32) {}
    #[cfg(feature = "v8")]
    pub fn SetDataPropertyDescriptor<D, V>(_desc: D, _value: V, _attrs: u32) {
        crate::v8_glue::set_data_property_descriptor()
    }

    #[cfg(not(feature = "v8"))]
    glue_stub!(pub fn AtomizeStringN(_cx: *mut jsapi::JSContext, _s: *const u8, _len: usize) -> *mut jsapi::JSString);
    #[cfg(feature = "v8")]
    pub fn AtomizeStringN(
        cx: *mut jsapi::JSContext,
        s: *const u8,
        len: usize,
    ) -> *mut jsapi::JSString {
        crate::v8_glue::atomize_string_n(cx, s, len)
    }

    #[cfg(not(feature = "v8"))]
    glue_stub!(pub fn CreateDOMGlobal(_cx: *mut jsapi::JSContext, _clasp: *const jsapi::JSClass, _principal: *mut std::ffi::c_void) -> *mut jsapi::JSObject);
    #[cfg(feature = "v8")]
    pub fn CreateDOMGlobal(
        cx: *mut jsapi::JSContext,
        clasp: *const jsapi::JSClass,
        principal: *mut std::ffi::c_void,
    ) -> *mut jsapi::JSObject {
        crate::v8_glue::create_dom_global(cx, clasp, principal)
    }

    #[cfg(not(feature = "v8"))]
    pub unsafe fn CallJitGetterOp<'a>(
        _info: *const jsapi::JSJitInfo,
        _cx: &mut crate::context::JSContext,
        _obj: jsapi::HandleObject<'a>,
        _this: *mut std::ffi::c_void,
        _argc: u32,
        _vp: *mut jsapi::JSVal,
    ) -> bool {
        false
    }
    #[cfg(feature = "v8")]
    pub unsafe fn CallJitGetterOp<'a>(
        info: *const jsapi::JSJitInfo,
        cx: &mut crate::context::JSContext,
        obj: jsapi::HandleObject<'a>,
        this: *mut std::ffi::c_void,
        argc: u32,
        vp: *mut jsapi::JSVal,
    ) -> bool {
        crate::v8_glue::call_jit_getter_op(info, cx.raw_cx(), obj, this, argc, vp)
    }

    #[cfg(not(feature = "v8"))]
    pub unsafe fn CallJitMethodOp<'a>(
        _info: *const jsapi::JSJitInfo,
        _cx: &mut crate::context::JSContext,
        _obj: jsapi::HandleObject<'a>,
        _this: *mut std::ffi::c_void,
        _argc: u32,
        _vp: *mut jsapi::JSVal,
    ) -> bool {
        false
    }
    #[cfg(feature = "v8")]
    pub unsafe fn CallJitMethodOp<'a>(
        info: *const jsapi::JSJitInfo,
        cx: &mut crate::context::JSContext,
        obj: jsapi::HandleObject<'a>,
        this: *mut std::ffi::c_void,
        argc: u32,
        vp: *mut jsapi::JSVal,
    ) -> bool {
        crate::v8_glue::call_jit_method_op(info, cx.raw_cx(), obj, this, argc, vp)
    }

    #[cfg(not(feature = "v8"))]
    pub fn CallJitSetterOp<O>(
        _info: *const jsapi::JSJitInfo,
        _cx: *mut jsapi::JSContext,
        _obj: O,
        _this: *mut std::ffi::c_void,
        _argc: u32,
        _vp: *mut jsapi::JSVal,
    ) -> bool {
        false
    }
    #[cfg(feature = "v8")]
    pub fn CallJitSetterOp<O>(
        info: *const jsapi::JSJitInfo,
        cx: *mut jsapi::JSContext,
        obj: O,
        this: *mut std::ffi::c_void,
        argc: u32,
        vp: *mut jsapi::JSVal,
    ) -> bool {
        crate::v8_glue::call_jit_setter_op(info, cx, obj, this, argc, vp)
    }

    #[cfg(not(feature = "v8"))]
    glue_stub!(pub fn CreateProxyHandler(_traps: &ProxyTraps, _extra: *const std::ffi::c_void) -> *mut std::ffi::c_void);
    #[cfg(feature = "v8")]
    pub fn CreateProxyHandler(
        traps: &ProxyTraps,
        extra: *const std::ffi::c_void,
    ) -> *mut std::ffi::c_void {
        crate::v8_glue::create_proxy_handler(traps, extra)
    }

    #[cfg(not(feature = "v8"))]
    glue_stub!(pub fn GetProxyReservedSlot(_proxy: *mut jsapi::JSObject, _slot: u32, _out: *mut jsapi::JSVal));
    #[cfg(feature = "v8")]
    pub fn GetProxyReservedSlot(proxy: *mut jsapi::JSObject, slot: u32, out: *mut jsapi::JSVal) {
        crate::v8_glue::get_proxy_reserved_slot(proxy, slot, out)
    }

    #[cfg(not(feature = "v8"))]
    pub fn JS_GetReservedSlot<O>(_obj: O, _slot: u32, _out: *mut jsapi::JSVal) {}
    #[cfg(feature = "v8")]
    pub fn JS_GetReservedSlot<O>(obj: O, slot: u32, out: *mut jsapi::JSVal)
    where
        O: Into<*mut jsapi::JSObject>,
    {
        if out.is_null() {
            return;
        }
        // SAFETY: caller supplied non-null out pointer for JSAPI out-param.
        unsafe {
            *out = crate::v8_glue::get_reserved_slot(obj.into(), slot);
        }
    }

    #[cfg(not(feature = "v8"))]
    pub fn SetProxyReservedSlot<V>(_proxy: *mut jsapi::JSObject, _slot: u32, _val: V) {}
    #[cfg(feature = "v8")]
    pub fn SetProxyReservedSlot<V: ToSlotValue>(_proxy: *mut jsapi::JSObject, _slot: u32, _val: V) {
        crate::v8_glue::set_reserved_slot(_proxy, _slot, _val.to_slot_value())
    }

    #[cfg(not(feature = "v8"))]
    pub fn CreateJobQueue<T, Q, I>(
        _traps: T,
        _queue: Q,
        _interrupt_queues: I,
    ) -> *mut jsapi::JobQueue {
        ::std::ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn CreateJobQueue<T, Q, I>(
        _traps: T,
        _queue: Q,
        _interrupt_queues: I,
    ) -> *mut jsapi::JobQueue {
        crate::v8_glue::create_job_queue()
    }

    #[cfg(not(feature = "v8"))]
    glue_stub!(pub fn DeleteJobQueue(_queue: *mut jsapi::JobQueue));
    #[cfg(feature = "v8")]
    pub fn DeleteJobQueue(queue: *mut jsapi::JobQueue) {
        crate::v8_glue::delete_job_queue(queue)
    }

    #[cfg(not(feature = "v8"))]
    pub unsafe fn DispatchableRun<D, M>(
        _cx: *mut jsapi::JSContext,
        _dispatchable: D,
        _maybe_shutting_down: M,
    ) {
    }
    #[cfg(feature = "v8")]
    pub unsafe fn DispatchableRun<D, M>(
        cx: *mut jsapi::JSContext,
        _dispatchable: D,
        _maybe_shutting_down: M,
    ) {
        crate::v8_glue::dispatchable_run(cx)
    }

    // ── pure stubs (no V8 equivalent needed / not yet implemented) ──
    pub fn RUST_JSID_IS_VOID<I>(_id: I) -> bool {
        false
    }
    glue_stub!(pub fn CallObjectTracer(_trc: *mut jsapi::JSTracer, _obj: *mut jsapi::JSObject, _name: *const u8));
    pub fn UncheckedUnwrapObject(obj: *mut jsapi::JSObject, _stopAtOuter: bool) -> *mut jsapi::JSObject {
        obj
    }
    #[cfg(not(feature = "v8"))]
    glue_stub!(pub fn IsProxyHandlerFamily(_obj: *mut jsapi::JSObject) -> bool);
    #[cfg(feature = "v8")]
    pub fn IsProxyHandlerFamily(obj: *mut jsapi::JSObject) -> bool {
        !obj.is_null()
    }
    glue_stub!(pub fn GetProxyHandlerFamily() -> *const std::ffi::c_void);
    pub fn CreateRustJSPrincipals<C>(
        _callbacks: C,
        private: *mut std::ffi::c_void,
    ) -> *mut std::ffi::c_void {
        let principal = Box::into_raw(Box::new(0_u8)) as *mut std::ffi::c_void;
        RUST_JS_PRINCIPALS.with(|principals| {
            principals.borrow_mut().insert(principal as usize, private);
        });
        principal
    }
    thread_local! {
        static RUST_JS_PRINCIPALS: RefCell<HashMap<usize, *mut std::ffi::c_void>> = RefCell::new(HashMap::new());
    }
    pub fn GetRustJSPrincipalsPrivate(p: *mut std::ffi::c_void) -> *mut std::ffi::c_void {
        RUST_JS_PRINCIPALS.with(|principals| {
            principals
                .borrow()
                .get(&(p as usize))
                .copied()
                .unwrap_or(::std::ptr::null_mut())
        })
    }
    pub fn DestroyRustJSPrincipals(p: *mut std::ffi::c_void) {
        if p.is_null() {
            return;
        }
        RUST_JS_PRINCIPALS.with(|principals| {
            principals.borrow_mut().remove(&(p as usize));
        });
        unsafe {
            drop(Box::from_raw(p as *mut u8));
        }
    }
    pub fn UnwrapObjectStatic<O>(_obj: O) -> *mut jsapi::JSObject {
        ::std::ptr::null_mut()
    }
    pub fn CopyJSStructuredCloneData<D, B>(_data: D, _dest: B) -> bool {
        false
    }
    pub fn GetLengthOfJSStructuredCloneData<D>(_data: D) -> usize {
        0
    }
    pub fn WriteBytesToJSStructuredCloneData<B, L, D>(_bytes: B, _len: L, _data: D) -> bool {
        false
    }
    glue_stub!(pub fn CallScriptTracer(_trc: *mut jsapi::JSTracer, _script: *mut std::ffi::c_void, _name: *const u8));
    glue_stub!(pub fn CallStringTracer(_trc: *mut jsapi::JSTracer, _string: *mut jsapi::JSString, _name: *const u8));
    glue_stub!(pub fn CallValueTracer(_trc: *mut jsapi::JSTracer, _value: *mut jsapi::JSVal, _name: *const u8));
    pub fn CreateWrapperProxyHandler<T>(_traps: T) -> *mut std::ffi::c_void {
        Box::into_raw(Box::new(0_u8)) as *mut std::ffi::c_void
    }
    pub trait ToWrapperProxyHandlerPtr {
        fn to_wrapper_proxy_handler_ptr(self) -> *mut std::ffi::c_void;
    }
    impl ToWrapperProxyHandlerPtr for *mut std::ffi::c_void {
        fn to_wrapper_proxy_handler_ptr(self) -> *mut std::ffi::c_void {
            self
        }
    }
    impl ToWrapperProxyHandlerPtr for *const std::ffi::c_void {
        fn to_wrapper_proxy_handler_ptr(self) -> *mut std::ffi::c_void {
            self as *mut std::ffi::c_void
        }
    }
    pub fn DeleteWrapperProxyHandler<H: ToWrapperProxyHandlerPtr>(handler: H) {
        let handler = handler.to_wrapper_proxy_handler_ptr();
        if !handler.is_null() {
            unsafe {
                drop(Box::from_raw(handler as *mut u8));
            }
        }
    }
    pub fn DumpJSStack<C, A, B, D>(_cx: C, _show_args: A, _show_locals: B, _show_this_props: D) {}
    pub fn InitializeMemoryReporter<F>(_is_dom_object: F) {}
    pub fn CollectServoSizes<C, F>(_cx: C, _sizes: &mut ServoSizes, _get_size: F) -> bool {
        true
    }
    glue_stub!(pub fn RUST_js_GetErrorMessage(_user_ref: *mut std::ffi::c_void, _error_number: u32) -> *const std::os::raw::c_char);
    pub fn RegisterScriptEnvironmentPreparer<C, F>(_cx: C, _callback: F) {}
    pub fn RunScriptEnvironmentPreparerClosure<C, D>(_cx: C, _closure: D) -> bool {
        true
    }
    pub fn SetBuildId<B, P>(_build_id: B, _ptr: P, _len: usize) -> bool {
        true
    }
    pub fn StreamConsumerConsumeChunk<C, B>(_consumer: C, _data: B, _len: usize) -> bool {
        true
    }
    pub fn StreamConsumerNoteResponseURLs<C, U, S>(_consumer: C, _url: U, _source_map_url: S) {}
    pub fn StreamConsumerStreamEnd<C>(_consumer: C) {}
    pub fn StreamConsumerStreamError<C, E>(_consumer: C, _error: E) {}
    glue_stub!(pub fn GetWindowProxyClass() -> *const jsapi::JSClass);
    #[cfg(not(feature = "v8"))]
    glue_stub!(pub fn GetProxyHandlerExtra(_proxy: *mut jsapi::JSObject) -> *mut std::ffi::c_void);
    #[cfg(feature = "v8")]
    pub fn GetProxyHandlerExtra(proxy: *mut jsapi::JSObject) -> *mut std::ffi::c_void {
        crate::v8_glue::get_proxy_handler_extra(proxy)
    }
    pub fn RUST_FUNCTION_VALUE_TO_JITINFO<V>(_value: V) -> *const jsapi::JSJitInfo {
        ::std::ptr::null()
    }
}

pub mod jsapi {
    use super::*;

    #[derive(Debug, Copy, Clone)]
    #[repr(C)]
    pub struct JSContext {
        _unused: [u8; 0],
    }
    #[derive(Debug, Copy, Clone)]
    #[repr(C)]
    pub struct JSObject {
        _unused: [u8; 0],
    }
    #[derive(Debug, Copy, Clone)]
    #[repr(C)]
    pub struct JSString {
        _unused: [u8; 0],
    }

    pub trait ToBytePtr {
        fn to_byte_ptr(self) -> *const u8;
    }

    impl ToBytePtr for *const u8 {
        fn to_byte_ptr(self) -> *const u8 {
            self
        }
    }

    impl ToBytePtr for *mut u8 {
        fn to_byte_ptr(self) -> *const u8 {
            self as *const u8
        }
    }

    impl ToBytePtr for *const std::os::raw::c_char {
        fn to_byte_ptr(self) -> *const u8 {
            self as *const u8
        }
    }

    impl ToBytePtr for *mut std::os::raw::c_char {
        fn to_byte_ptr(self) -> *const u8 {
            self as *const u8
        }
    }

    #[derive(Debug, Copy, Clone)]
    #[repr(C)]
    pub struct JSStringData {
        pub data_: *const std::os::raw::c_char,
    }
    #[derive(Debug, Copy, Clone)]
    #[repr(C)]
    pub struct ColumnNumber {
        pub _base: u32,
    }
    #[derive(Debug, Copy, Clone)]
    #[repr(C)]
    pub struct JSErrorReportBase {
        pub filename: JSStringData,
        pub lineno: u32,
        pub column: ColumnNumber,
        pub message_: JSStringData,
    }
    #[derive(Debug, Copy, Clone)]
    #[repr(C)]
    pub struct JSErrorReport {
        pub _base: JSErrorReportBase,
    }
    #[derive(Debug, Copy, Clone)]
    #[repr(C)]
    pub struct JSFunction {
        _unused: [u8; 0],
    }
    #[derive(Debug, Copy, Clone)]
    #[repr(C)]
    pub struct JSTracer {
        _unused: [u8; 0],
    }
    #[derive(Debug, Copy, Clone)]
    #[repr(C)]
    pub struct JSRuntime {
        _unused: [u8; 0],
    }
    pub type JSPrincipals = std::ffi::c_void;
    pub type JSClass = JSClassDef;
    #[derive(Copy, Clone, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct jsid {
        pub asBits_: u64,
    }
    pub const fn jsid(asBits_: u64) -> jsid {
        jsid { asBits_ }
    }
    impl std::ops::Deref for jsid {
        type Target = u64;
        fn deref(&self) -> &u64 {
            &self.asBits_
        }
    }
    impl std::fmt::Debug for jsid {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "jsid({})", self.asBits_)
        }
    }
    impl From<u64> for jsid {
        fn from(v: u64) -> jsid {
            jsid(v)
        }
    }
    impl From<*const jsid> for jsid {
        fn from(p: *const jsid) -> jsid {
            unsafe { *p }
        }
    }
    impl From<&jsid> for *const jsid {
        fn from(p: &jsid) -> *const jsid {
            p as *const jsid
        }
    }
    impl jsid {
        const STRING_TAG: u64 = 0x1000_0000_0000_0000;
        const INT_TAG: u64 = 0x2000_0000_0000_0000;
        const TAG_MASK: u64 = 0xf000_0000_0000_0000;
        const PAYLOAD_MASK: u64 = 0x0fff_ffff_ffff_ffff;

        pub fn from_string(s: *mut JSString) -> Self {
            Self {
                asBits_: Self::STRING_TAG | ((s as u64) & Self::PAYLOAD_MASK),
            }
        }
        pub fn from_int(i: i32) -> Self {
            Self {
                asBits_: Self::INT_TAG | (i as u32 as u64),
            }
        }
        pub fn is_string(&self) -> bool {
            (self.asBits_ & Self::TAG_MASK) == Self::STRING_TAG
        }
        pub fn is_int(&self) -> bool {
            (self.asBits_ & Self::TAG_MASK) == Self::INT_TAG
        }
        pub fn is_void(&self) -> bool {
            self.asBits_ == 0
        }
        pub fn is_symbol(&self) -> bool {
            false
        }
        pub fn to_int(&self) -> i32 {
            (self.asBits_ & u32::MAX as u64) as u32 as i32
        }
        pub fn to_string(&self) -> *mut JSString {
            if self.is_string() {
                (self.asBits_ & Self::PAYLOAD_MASK) as *mut JSString
            } else {
                ptr::null_mut()
            }
        }
    }
    #[derive(Debug, Copy, Clone, Default, PartialEq)]
    #[repr(C)]
    pub struct JSVal {
        pub asBits_: u64,
    }
    impl JSVal {
        const TAG_MASK: u64 = 0xff00_0000_0000_0000;
        const PAYLOAD_MASK: u64 = 0x00ff_ffff_ffff_ffff;
        const NULL: u64 = 0x0100_0000_0000_0000;
        const FALSE: u64 = 0x0200_0000_0000_0000;
        const TRUE: u64 = 0x0300_0000_0000_0000;
        const OBJECT: u64 = 0x0400_0000_0000_0000;
        const STRING: u64 = 0x0500_0000_0000_0000;
        const PRIVATE: u64 = 0x0600_0000_0000_0000;
        const INT32: u64 = 0x0700_0000_0000_0000;
        const UINT32: u64 = 0x0800_0000_0000_0000;

        pub fn from_object(obj: *const JSObject) -> Self {
            Self {
                asBits_: Self::OBJECT | ((obj as u64) & Self::PAYLOAD_MASK),
            }
        }
        pub fn from_string(s: *const JSString) -> Self {
            Self {
                asBits_: Self::STRING | ((s as u64) & Self::PAYLOAD_MASK),
            }
        }
        pub fn from_private(p: *const std::ffi::c_void) -> Self {
            Self {
                asBits_: Self::PRIVATE | ((p as u64) & Self::PAYLOAD_MASK),
            }
        }
        pub fn from_bool(b: bool) -> Self {
            Self {
                asBits_: if b { Self::TRUE } else { Self::FALSE },
            }
        }
        pub fn from_int32(i: i32) -> Self {
            Self {
                asBits_: Self::INT32 | (i as u32 as u64),
            }
        }
        pub fn from_uint32(u: u32) -> Self {
            Self {
                asBits_: Self::UINT32 | (u as u64),
            }
        }
        pub fn null() -> Self {
            Self {
                asBits_: Self::NULL,
            }
        }
        pub fn undefined() -> Self {
            Self::default()
        }
        pub fn get(&self) -> Self {
            *self
        }
        pub fn is_object(&self) -> bool {
            (self.asBits_ & Self::TAG_MASK) == Self::OBJECT
        }
        pub fn is_object_or_null(&self) -> bool {
            self.is_object() || self.is_null()
        }
        pub fn is_string(&self) -> bool {
            (self.asBits_ & Self::TAG_MASK) == Self::STRING
        }
        pub fn is_null(&self) -> bool {
            self.asBits_ == Self::NULL
        }
        pub fn is_null_or_undefined(&self) -> bool {
            self.is_null() || self.is_undefined()
        }
        pub fn is_number(&self) -> bool {
            matches!(self.asBits_ & Self::TAG_MASK, Self::INT32 | Self::UINT32)
        }
        pub fn is_boolean(&self) -> bool {
            matches!(self.asBits_, Self::TRUE | Self::FALSE)
        }
        pub fn is_markable(&self) -> bool {
            self.is_object() || self.is_string()
        }
        pub fn is_undefined(&self) -> bool {
            self.asBits_ == 0
        }
        pub fn to_boolean(&self) -> bool {
            self.asBits_ == Self::TRUE
        }
        pub fn to_number(&self) -> f64 {
            match self.asBits_ & Self::TAG_MASK {
                Self::INT32 => self.to_int32() as f64,
                Self::UINT32 => (self.asBits_ & u32::MAX as u64) as u32 as f64,
                _ => 0.0,
            }
        }
        pub fn to_int32(&self) -> i32 {
            (self.asBits_ & u32::MAX as u64) as u32 as i32
        }
        pub fn to_private(&self) -> *mut std::ffi::c_void {
            if (self.asBits_ & Self::TAG_MASK) == Self::PRIVATE {
                (self.asBits_ & Self::PAYLOAD_MASK) as *mut std::ffi::c_void
            } else {
                ptr::null_mut()
            }
        }
        pub fn to_object(&self) -> *mut JSObject {
            if self.is_object() {
                (self.asBits_ & Self::PAYLOAD_MASK) as *mut JSObject
            } else {
                ptr::null_mut()
            }
        }
        pub fn to_string(&self) -> *mut JSString {
            if self.is_string() {
                (self.asBits_ & Self::PAYLOAD_MASK) as *mut JSString
            } else {
                ptr::null_mut()
            }
        }
        pub fn to_object_or_null(&self) -> *mut JSObject {
            if self.is_null() {
                ptr::null_mut()
            } else {
                self.to_object()
            }
        }
        pub fn trace_kind(&self) -> TraceKind {
            if self.is_string() {
                TraceKind::String
            } else {
                TraceKind::Object
            }
        }
    }
    pub type Value = JSVal;

    pub trait ToJsapiPropertyValue {
        fn to_jsapi_property_value(self) -> JSVal;
    }

    impl ToJsapiPropertyValue for JSVal {
        fn to_jsapi_property_value(self) -> JSVal {
            self
        }
    }

    impl ToJsapiPropertyValue for &JSVal {
        fn to_jsapi_property_value(self) -> JSVal {
            *self
        }
    }

    impl ToJsapiPropertyValue for HandleValue<'_> {
        fn to_jsapi_property_value(self) -> JSVal {
            self.get()
        }
    }

    impl ToJsapiPropertyValue for *mut JSObject {
        fn to_jsapi_property_value(self) -> JSVal {
            JSVal::from_object(self)
        }
    }

    impl<T> ToJsapiPropertyValue for *const T {
        fn to_jsapi_property_value(self) -> JSVal {
            JSVal::from_private(self as *const std::ffi::c_void)
        }
    }

    impl ToJsapiPropertyValue for *mut std::ffi::c_void {
        fn to_jsapi_property_value(self) -> JSVal {
            JSVal::from_private(self as *const std::ffi::c_void)
        }
    }

    impl ToJsapiPropertyValue for HandleObject<'_> {
        fn to_jsapi_property_value(self) -> JSVal {
            JSVal::from_object(self.get())
        }
    }

    impl ToJsapiPropertyValue for Handle<'_, PropertyDescriptor> {
        fn to_jsapi_property_value(self) -> JSVal {
            self.get().value_
        }
    }

    pub trait SetJsapiValOut {
        fn set_jsapi_val_out(self, val: JSVal);
    }

    impl SetJsapiValOut for *mut JSVal {
        fn set_jsapi_val_out(self, val: JSVal) {
            if !self.is_null() {
                // SAFETY: non-null JSAPI out-param checked above.
                unsafe { *self = val };
            }
        }
    }

    impl SetJsapiValOut for MutableHandleValue<'_> {
        fn set_jsapi_val_out(mut self, val: JSVal) {
            self.set(val);
        }
    }

    pub trait SetJsapiBoolOut {
        fn set_jsapi_bool_out(self, val: bool);
    }

    pub trait SetJsapiIdOut {
        fn set_jsapi_id_out(self, val: jsid);
    }

    impl SetJsapiIdOut for *mut jsid {
        fn set_jsapi_id_out(self, val: jsid) {
            if !self.is_null() {
                // SAFETY: non-null JSAPI out-param checked above.
                unsafe { *self = val };
            }
        }
    }

    impl SetJsapiIdOut for MutableHandleId<'_> {
        fn set_jsapi_id_out(mut self, val: jsid) {
            self.set(val);
        }
    }

    pub trait SetJsapiObjectOut {
        fn set_jsapi_object_out(self, val: *mut JSObject);
    }

    pub trait SetObjectOpResultOut {
        fn set_object_op_success(self);
    }

    impl SetObjectOpResultOut for *mut ObjectOpResult {
        fn set_object_op_success(self) {
            if !self.is_null() {
                // SAFETY: non-null JSAPI out-param checked above.
                unsafe { (*self).succeed() };
            }
        }
    }

    impl SetObjectOpResultOut for &mut ObjectOpResult {
        fn set_object_op_success(self) {
            self.succeed();
        }
    }

    impl SetObjectOpResultOut for () {
        fn set_object_op_success(self) {}
    }

    impl SetJsapiObjectOut for *mut *mut JSObject {
        fn set_jsapi_object_out(self, val: *mut JSObject) {
            if !self.is_null() {
                // SAFETY: non-null JSAPI out-param checked above.
                unsafe { *self = val };
            }
        }
    }

    impl SetJsapiObjectOut for MutableHandleObject<'_> {
        fn set_jsapi_object_out(mut self, val: *mut JSObject) {
            self.set(val);
        }
    }

    pub trait ToFunctionObjectPtr {
        fn to_function_object_ptr(self) -> *mut JSObject;
    }

    impl ToFunctionObjectPtr for *mut JSObject {
        fn to_function_object_ptr(self) -> *mut JSObject {
            self
        }
    }

    #[cfg(feature = "v8")]
    impl ToFunctionObjectPtr for *mut JSFunction {
        fn to_function_object_ptr(self) -> *mut JSObject {
            crate::v8_glue::get_function_object(self)
        }
    }

    impl ToFunctionObjectPtr for HandleObject<'_> {
        fn to_function_object_ptr(self) -> *mut JSObject {
            self.get()
        }
    }

    impl ToFunctionObjectPtr for JSVal {
        fn to_function_object_ptr(self) -> *mut JSObject {
            self.to_object()
        }
    }

    impl ToFunctionObjectPtr for HandleValue<'_> {
        fn to_function_object_ptr(self) -> *mut JSObject {
            self.get().to_object()
        }
    }

    pub trait ToCallArgs {
        fn to_call_args(self) -> HandleValueArray;
    }

    pub trait ToArrayObjectInit {
        fn to_array_values(self) -> Option<Vec<JSVal>>;
    }

    impl ToArrayObjectInit for HandleValueArray {
        fn to_array_values(self) -> Option<Vec<JSVal>> {
            if self.length_ == 0 {
                return Some(Vec::new());
            }
            if self.elements_.is_null() {
                return None;
            }
            // SAFETY: HandleValueArray promises `elements_` points to `length_` values.
            Some(unsafe { std::slice::from_raw_parts(self.elements_, self.length_) }.to_vec())
        }
    }

    impl ToArrayObjectInit for &HandleValueArray {
        fn to_array_values(self) -> Option<Vec<JSVal>> {
            (*self).to_array_values()
        }
    }

    impl ToArrayObjectInit for usize {
        fn to_array_values(self) -> Option<Vec<JSVal>> {
            Some(vec![JSVal::undefined(); self])
        }
    }

    impl ToArrayObjectInit for u32 {
        fn to_array_values(self) -> Option<Vec<JSVal>> {
            (self as usize).to_array_values()
        }
    }

    impl ToArrayObjectInit for i32 {
        fn to_array_values(self) -> Option<Vec<JSVal>> {
            if self < 0 {
                None
            } else {
                (self as usize).to_array_values()
            }
        }
    }

    impl ToCallArgs for HandleValueArray {
        fn to_call_args(self) -> HandleValueArray {
            self
        }
    }

    impl ToCallArgs for &HandleValueArray {
        fn to_call_args(self) -> HandleValueArray {
            *self
        }
    }

    impl ToCallArgs for &Vec<JSVal> {
        fn to_call_args(self) -> HandleValueArray {
            HandleValueArray::from(self)
        }
    }

    impl SetJsapiBoolOut for *mut bool {
        fn set_jsapi_bool_out(self, val: bool) {
            if !self.is_null() {
                // SAFETY: non-null JSAPI out-param checked above.
                unsafe { *self = val };
            }
        }
    }

    impl SetJsapiBoolOut for &mut bool {
        fn set_jsapi_bool_out(self, val: bool) {
            *self = val;
        }
    }

    pub struct JSAutoRealm;
    impl JSAutoRealm {
        pub fn new<C, O>(_cx: C, _obj: O) -> Self {
            Self
        }
    }
    pub type JSAutoCompartment = *mut std::ffi::c_void;
    pub type GCContext = *mut std::ffi::c_void;

    #[derive(Debug)]
    #[repr(transparent)]
    pub struct Handle<'a, T> {
        pub ptr: *const T,
        _phantom: std::marker::PhantomData<&'a T>,
    }
    impl<'a, T> Copy for Handle<'a, T> {}
    impl<'a, T> Clone for Handle<'a, T> {
        fn clone(&self) -> Self {
            *self
        }
    }
    impl<'a, T> Handle<'a, T> {
        pub unsafe fn from_raw<P: Into<*const T>>(ptr: P) -> Self {
            Self {
                ptr: ptr.into(),
                _phantom: std::marker::PhantomData,
            }
        }
        pub fn null() -> Self {
            Self {
                ptr: ptr::null(),
                _phantom: std::marker::PhantomData,
            }
        }
        pub fn as_ptr(self) -> *const T {
            self.ptr
        }
        pub fn handle(&self) -> Handle<'_, T> {
            unsafe { Handle::from_raw(self.ptr) }
        }
        pub fn into_handle(self) -> Handle<'a, T> {
            self
        }
        pub fn get(self) -> T
        where
            T: Copy,
        {
            if self.ptr.is_null() {
                unsafe { std::mem::zeroed() }
            } else {
                unsafe { *self.ptr }
            }
        }
    }
    impl<'a, T> std::ops::Deref for Handle<'a, T> {
        type Target = T;
        fn deref(&self) -> &T {
            unsafe { &*self.ptr }
        }
    }
    impl<'a, T: Copy> Handle<'a, Vec<T>> {
        pub fn len(&self) -> u32 {
            std::ops::Deref::deref(self).len() as u32
        }
        pub fn at(&self, index: u32) -> Option<Handle<'_, T>> {
            let vec = std::ops::Deref::deref(self);
            if (index as usize) >= vec.len() {
                return None;
            }
            // SAFETY: index is in bounds for this rooted vector handle.
            Some(unsafe { Handle::from_raw(vec.as_ptr().add(index as usize)) })
        }
    }
    impl<'a, T: PartialEq + Copy> PartialEq for Handle<'a, T> {
        fn eq(&self, other: &Self) -> bool {
            self.get() == other.get()
        }
    }
    impl<'a, T> From<Handle<'a, T>> for *const T {
        fn from(handle: Handle<'a, T>) -> Self {
            handle.ptr
        }
    }
    impl<'a, T> From<Handle<'a, T>> for *mut T {
        fn from(handle: Handle<'a, T>) -> Self {
            handle.ptr as *mut T
        }
    }
    impl<'a> From<Handle<'a, JSVal>> for JSVal {
        fn from(handle: Handle<'a, JSVal>) -> Self {
            handle.get()
        }
    }
    impl<'a> From<Handle<'a, jsid>> for jsid {
        fn from(handle: Handle<'a, jsid>) -> Self {
            handle.get()
        }
    }
    impl<'a> Handle<'a, jsid> {
        pub fn is_symbol(&self) -> bool {
            self.get().is_symbol()
        }
    }
    impl<'a> From<Handle<'a, *mut JSObject>> for *mut JSObject {
        fn from(handle: Handle<'a, *mut JSObject>) -> Self {
            handle.get()
        }
    }
    impl<'a> Handle<'a, JSVal> {
        pub fn undefined() -> Self {
            Self::null()
        }
        pub fn is_boolean(&self) -> bool {
            self.get().is_boolean()
        }
        pub fn is_primitive(&self) -> bool {
            !self.get().is_object()
        }
        pub fn to_boolean(&self) -> bool {
            self.get().to_boolean()
        }
        pub fn to_string(&self) -> *mut JSString {
            self.get().to_string()
        }
        pub unsafe fn from_marked_location(ptr: *const JSVal) -> Self {
            Self::from_raw(ptr)
        }
    }

    #[derive(Debug, Copy, Clone)]
    #[repr(transparent)]
    pub struct MutableHandle<'a, T> {
        pub ptr: *mut T,
        _phantom: std::marker::PhantomData<&'a mut T>,
    }
    impl<'a, T> MutableHandle<'a, T> {
        pub unsafe fn from_raw<P: Into<*mut T>>(ptr: P) -> Self {
            Self {
                ptr: ptr.into(),
                _phantom: std::marker::PhantomData,
            }
        }
        pub fn null() -> Self {
            Self {
                ptr: ptr::null_mut(),
                _phantom: std::marker::PhantomData,
            }
        }
        pub fn as_ptr(self) -> *mut T {
            self.ptr
        }
        pub fn handle(&self) -> Handle<'_, T> {
            unsafe { Handle::from_raw(self.ptr as *const T) }
        }
        pub fn into_handle(self) -> Handle<'a, T> {
            unsafe { Handle::from_raw(self.ptr as *const T) }
        }
        pub fn get(self) -> T
        where
            T: Copy,
        {
            if self.ptr.is_null() {
                unsafe { std::mem::zeroed() }
            } else {
                unsafe { *self.ptr }
            }
        }
        pub fn set(&mut self, val: T) {
            if !self.ptr.is_null() {
                unsafe {
                    *self.ptr = val;
                }
            }
        }
        pub fn reborrow(&mut self) -> MutableHandle<'_, T> {
            MutableHandle {
                ptr: self.ptr,
                _phantom: std::marker::PhantomData,
            }
        }
    }
    impl<'a, T> std::ops::Deref for MutableHandle<'a, T> {
        type Target = T;
        fn deref(&self) -> &T {
            unsafe { &*self.ptr }
        }
    }
    impl<'a, T> std::ops::DerefMut for MutableHandle<'a, T> {
        fn deref_mut(&mut self) -> &mut T {
            unsafe { &mut *self.ptr }
        }
    }
    impl<'a, T> From<MutableHandle<'a, T>> for *mut T {
        fn from(handle: MutableHandle<'a, T>) -> Self {
            handle.ptr
        }
    }
    impl<'a> From<MutableHandle<'a, JSVal>> for JSVal {
        fn from(handle: MutableHandle<'a, JSVal>) -> Self {
            handle.get()
        }
    }

    // from_raw helpers — accessed as Handle::from_raw / MutableHandle::from_raw
    // These live in a nested module to avoid conflicting with the type alias at this level.
    pub mod handle_from_raw {
        pub unsafe fn Handle<T>(_: super::Handle<'static, T>, raw: *const T) -> *const T {
            raw
        }
        pub unsafe fn MutableHandle<T>(_: super::MutableHandle<'static, T>, raw: *mut T) -> *mut T {
            raw
        }
    }
    pub unsafe fn handle_from_raw<T>(raw: *const T) -> *const T {
        raw
    }
    pub unsafe fn mutable_handle_from_raw<T>(raw: *mut T) -> *mut T {
        raw
    }

    pub type HandleId<'a> = Handle<'a, jsid>;
    pub type HandleObject<'a> = Handle<'a, *mut JSObject>;
    pub type HandleValue<'a> = Handle<'a, JSVal>;
    #[derive(Debug, Copy, Clone)]
    #[repr(C)]
    pub struct HandleValueArray {
        pub length_: usize,
        pub elements_: *const JSVal,
    }
    impl HandleValueArray {
        pub fn empty() -> Self {
            Self {
                length_: 0,
                elements_: ptr::null(),
            }
        }
    }
    impl<'a> From<&crate::gc::RootedGuard<'a, Vec<JSVal>>> for HandleValueArray {
        fn from(values: &crate::gc::RootedGuard<'a, Vec<JSVal>>) -> Self {
            Self {
                length_: values.len(),
                elements_: values.as_ptr(),
            }
        }
    }
    impl From<&Vec<JSVal>> for HandleValueArray {
        fn from(values: &Vec<JSVal>) -> Self {
            Self {
                length_: values.len(),
                elements_: values.as_ptr(),
            }
        }
    }
    impl<'a> From<&crate::gc::RootedVec<'a, JSVal>> for HandleValueArray {
        fn from(values: &crate::gc::RootedVec<'a, JSVal>) -> Self {
            Self {
                length_: values.len(),
                elements_: values.as_ptr(),
            }
        }
    }
    pub type MutableHandleIdVector = *mut Vec<jsid>;
    pub type MutableHandleId<'a> = MutableHandle<'a, jsid>;
    pub type MutableHandleObject<'a> = MutableHandle<'a, *mut JSObject>;
    pub type MutableHandleValue<'a> = MutableHandle<'a, JSVal>;
    pub fn UndefinedHandleValue() -> HandleValue<'static> {
        HandleValue::null()
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct CallArgs {
        pub argc_: u32,
        _constructing_: bool,
        _pad_: [u8; 3],
        argv_: *const JSVal,
        rval_: *mut JSVal,
    }
    impl CallArgs {
        pub unsafe fn from_vp(vp: *mut JSVal, argc: u32) -> Self {
            CallArgs {
                argc_: argc,
                _constructing_: false,
                _pad_: [0u8; 3],
                // In this shim, vp[0] is rval, vp[1] is callee, and argv starts at vp[2].
                argv_: unsafe { vp.offset(2) },
                rval_: vp,
            }
        }
        pub fn get(&self, index: u32) -> HandleValue<'_> {
            if (index as usize) < self.argc_ as usize {
                unsafe { HandleValue::from_raw(self.argv_.offset(index as isize)) }
            } else {
                HandleValue::null()
            }
        }
        pub fn rval(&self) -> MutableHandleValue<'_> {
            unsafe { MutableHandleValue::from_raw(self.rval_) }
        }
        pub fn new_target(&self) -> HandleValue<'_> {
            HandleValue::null()
        }
        pub fn callee(&self) -> HandleValue<'_> {
            if self.rval_.is_null() {
                HandleValue::null()
            } else {
                unsafe { HandleValue::from_raw(self.rval_.add(1) as *const JSVal) }
            }
        }
        pub fn thisv(&self) -> HandleValue<'_> {
            HandleValue::null()
        }
        pub fn is_constructing(&self) -> bool {
            false
        }
    }
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    #[repr(usize)]
    pub enum JSErrNum {
        JSMSG_CANT_PREVENT_EXTENSIONS = 1,
        JSMSG_CANT_SET_PROTO = 2,
        JSMSG_CANT_DEFINE_WINDOW_NAMED_PROPERTY = 3,
        JSMSG_CANT_DELETE_WINDOW_NAMED_PROPERTY = 4,
        JSMSG_CANT_DEFINE_WINDOW_ELEMENT = 5,
        JSMSG_READ_ONLY = 6,
    }

    #[derive(Debug, Copy, Clone)]
    #[repr(C)]
    pub struct ObjectOpResult {
        pub code_: usize,
    }
    impl ObjectOpResult {
        pub fn succeed(&mut self) -> bool {
            self.code_ = 0;
            true
        }
        pub fn failNoIndexedSetter(&mut self) -> bool {
            false
        }
        pub fn fail_no_named_setter(&mut self) -> bool {
            false
        }
    }

    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct PropertyDescriptor {
        pub obj: *mut JSObject,
        pub attrs: u32,
        pub getter_: *mut JSObject,
        pub setter_: *mut JSObject,
        pub value_: JSVal,
        has_getter_: bool,
        has_setter_: bool,
        has_writable_: bool,
        has_value_: bool,
    }
    impl PropertyDescriptor {
        pub fn set_data_descriptor(&mut self, obj: *mut JSObject, value: JSVal, attrs: u32) {
            self.obj = obj;
            self.attrs = attrs;
            self.getter_ = ptr::null_mut();
            self.setter_ = ptr::null_mut();
            self.value_ = value;
            self.has_getter_ = false;
            self.has_setter_ = false;
            self.has_writable_ = true;
            self.has_value_ = true;
        }

        pub fn clear(&mut self) {
            *self = Self::default();
        }

        pub fn value(&self) -> JSVal {
            self.value_
        }

        pub fn hasGetter_(&self) -> bool {
            self.has_getter_
        }
        pub fn hasSetter_(&self) -> bool {
            self.has_setter_
        }
        pub fn hasWritable_(&self) -> bool {
            self.has_writable_
        }
        pub fn writable_(&self) -> bool {
            (self.attrs & JSPROP_READONLY) == 0
        }
        pub fn hasConfigurable_(&self) -> bool {
            self.obj != ptr::null_mut()
        }
        pub fn configurable_(&self) -> bool {
            (self.attrs & JSPROP_PERMANENT) == 0
        }
        pub fn hasEnumerable_(&self) -> bool {
            self.obj != ptr::null_mut()
        }
        pub fn hasValue_(&self) -> bool {
            self.has_value_
        }
        pub fn enumerable_(&self) -> bool {
            (self.attrs & JSPROP_ENUMERATE) != 0
        }
    }
    impl Default for PropertyDescriptor {
        fn default() -> Self {
            Self {
                obj: ptr::null_mut(),
                attrs: 0,
                getter_: ptr::null_mut(),
                setter_: ptr::null_mut(),
                value_: JSVal::default(),
                has_getter_: false,
                has_setter_: false,
                has_writable_: false,
                has_value_: false,
            }
        }
    }

    pub struct HeapPtr<T>(*mut T);
    impl<T> HeapPtr<T> {
        pub fn get(&self) -> *mut T {
            self.0
        }
    }
    pub struct Heap<T> {
        pub ptr: HeapPtr<T>,
        cell: RefCell<Option<Box<T>>>,
    }
    impl<T> Heap<T> {
        pub fn new(val: T) -> Self {
            Self {
                ptr: HeapPtr(ptr::null_mut()),
                cell: RefCell::new(Some(Box::new(val))),
            }
        }
        pub fn boxed(val: T) -> Box<Self> {
            Box::new(Self::new(val))
        }
        pub fn set(&self, val: T) {
            *self.cell.borrow_mut() = Some(Box::new(val));
        }
        pub fn get(&self) -> T
        where
            T: Copy,
        {
            self.cell
                .borrow()
                .as_ref()
                .map(|value| **value)
                .unwrap_or_else(|| unsafe { std::mem::zeroed() })
        }
        pub fn handle(&self) -> *const T {
            self.cell
                .borrow()
                .as_ref()
                .map(|value| &**value as *const T)
                .unwrap_or(ptr::null())
        }
        pub unsafe fn get_unsafe(&self) -> *mut T {
            self.cell
                .borrow_mut()
                .as_mut()
                .map(|value| &mut **value as *mut T)
                .unwrap_or(ptr::null_mut())
        }
        pub unsafe fn unbarriered_get(&self) -> *const T {
            self.handle()
        }
    }
    impl<T> Default for Heap<T> {
        fn default() -> Self {
            Self {
                ptr: HeapPtr(ptr::null_mut()),
                cell: RefCell::new(None),
            }
        }
    }
    impl<T> std::fmt::Debug for Heap<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Heap").finish()
        }
    }
    impl<T: Copy + PartialEq> PartialEq for Heap<T> {
        fn eq(&self, other: &Self) -> bool {
            self.get() == other.get()
        }
    }

    #[repr(u32)]
    #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
    pub enum JSValueType {
        JSVAL_TYPE_DOUBLE = 0,
        JSVAL_TYPE_INT32 = 1,
        JSVAL_TYPE_BOOLEAN = 2,
        JSVAL_TYPE_UNDEFINED = 3,
        JSVAL_TYPE_NULL = 4,
        JSVAL_TYPE_MAGIC = 5,
        JSVAL_TYPE_STRING = 6,
        JSVAL_TYPE_SYMBOL = 7,
        JSVAL_TYPE_PRIVATE_GCTHING = 8,
        JSVAL_TYPE_BIGINT = 9,
        JSVAL_TYPE_OBJECT = 12,
        JSVAL_TYPE_UNKNOWN = 32,
    }

    #[repr(u32)]
    #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
    pub enum JSJitInfo_OpType {
        Getter = 0,
        Setter = 1,
        Method = 2,
        StaticMethod = 3,
        InlinableNative = 4,
        TrampolineNative = 5,
        IgnoresReturnValueNative = 6,
        OpTypeCount = 7,
    }

    #[repr(u32)]
    #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
    pub enum JSJitInfo_AliasSet {
        AliasNone = 0,
        AliasDOMSets = 1,
        AliasEverything = 2,
        AliasSetCount = 3,
    }

    #[repr(i32)]
    #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
    pub enum JSJitInfo_ArgType {
        Null = 0,
        String = 1,
        Integer = 2,
        Double = 4,
        Boolean = 8,
        Object = 16,
        Any = 32,
        ArgTypeListEnd = 64,
    }

    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct JSJitGetterCallArgs {
        pub _base: *mut JSVal,
    }
    impl JSJitGetterCallArgs {
        pub fn get(&self, _index: u32) -> HandleValue<'_> {
            HandleValue::null()
        }
        pub fn rval(&self) -> MutableHandleValue<'_> {
            unsafe { MutableHandleValue::from_raw(self._base) }
        }
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct JSJitMethodCallArgs {
        pub _base: *mut JSVal,
        pub argc_: u32,
    }
    impl JSJitMethodCallArgs {
        pub fn get(&self, _index: u32) -> HandleValue<'_> {
            HandleValue::null()
        }
        pub fn rval(&self) -> MutableHandleValue<'_> {
            unsafe { MutableHandleValue::from_raw(self._base) }
        }
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct JSJitSetterCallArgs {
        pub _base: *mut JSVal,
        pub argc_: u32,
    }
    impl JSJitSetterCallArgs {
        pub fn get(&self, _index: u32) -> HandleValue<'_> {
            HandleValue::null()
        }
        pub fn rval(&self) -> MutableHandleValue<'_> {
            unsafe { MutableHandleValue::from_raw(self._base) }
        }
    }

    #[repr(C)]
    pub struct JSJitInfo__bindgen_ty_1 {
        pub method: Option<
            for<'a> unsafe extern "C" fn(
                *mut JSContext,
                HandleObject<'a>,
                *mut std::ffi::c_void,
                *const JSJitMethodCallArgs,
            ) -> bool,
        >,
        pub getter: Option<
            for<'a> unsafe extern "C" fn(
                *mut JSContext,
                HandleObject<'a>,
                *mut std::ffi::c_void,
                JSJitGetterCallArgs,
            ) -> bool,
        >,
        pub setter: Option<
            for<'a> unsafe extern "C" fn(
                *mut JSContext,
                HandleObject<'a>,
                *mut std::ffi::c_void,
                JSJitSetterCallArgs,
            ) -> bool,
        >,
        pub staticMethod: Option<unsafe extern "C" fn(*mut JSContext, u32, *mut JSVal) -> bool>,
        pub staticGetter: Option<
            for<'a> unsafe extern "C" fn(
                *mut JSContext,
                HandleObject<'a>,
                *mut std::ffi::c_void,
                JSJitGetterCallArgs,
            ) -> bool,
        >,
        pub staticSetter: Option<
            for<'a> unsafe extern "C" fn(
                *mut JSContext,
                HandleObject<'a>,
                *mut std::ffi::c_void,
                JSJitSetterCallArgs,
            ) -> bool,
        >,
    }
    impl Default for JSJitInfo__bindgen_ty_1 {
        fn default() -> Self {
            JSJitInfo__bindgen_ty_1 {
                method: None,
                getter: None,
                setter: None,
                staticMethod: None,
                staticGetter: None,
                staticSetter: None,
            }
        }
    }
    #[repr(C)]
    pub struct JSJitInfo__bindgen_ty_2 {
        pub protoID: u16,
    }
    #[repr(C)]
    pub struct JSJitInfo__bindgen_ty_3 {
        pub depth: u16,
    }

    #[repr(C)]
    pub struct JSJitInfo {
        pub __bindgen_anon_1: JSJitInfo__bindgen_ty_1,
        pub __bindgen_anon_2: JSJitInfo__bindgen_ty_2,
        pub __bindgen_anon_3: JSJitInfo__bindgen_ty_3,
        pub _bitfield_align_1: [u8; 0],
        pub _bitfield_1: __BindgenBitfieldUnit<[u8; 4usize]>,
    }

    #[repr(C)]
    pub struct JSTypedMethodJitInfo {
        pub base: JSJitInfo,
        pub argTypes: *const JSJitInfo_ArgType,
    }
    pub struct CompileOptions {
        pub asmJSOption_: AsmJSOption,
        importAttributes_: bool,
    }
    impl Default for CompileOptions {
        fn default() -> Self {
            Self {
                asmJSOption_: AsmJSOption::DisabledByAsmJSPref,
                importAttributes_: false,
            }
        }
    }
    impl CompileOptions {
        pub fn set_importAttributes_(&mut self, value: bool) {
            self.importAttributes_ = value;
        }
    }
    pub struct ContextOptions {
        pub compileOptions_: CompileOptions,
        wasm_: bool,
        wasmBaseline_: bool,
        wasmIon_: bool,
    }
    impl Default for ContextOptions {
        fn default() -> Self {
            Self {
                compileOptions_: CompileOptions::default(),
                wasm_: false,
                wasmBaseline_: false,
                wasmIon_: false,
            }
        }
    }
    impl ContextOptions {
        pub fn set_wasm_(&mut self, value: bool) {
            self.wasm_ = value;
        }
        pub fn set_wasmBaseline_(&mut self, value: bool) {
            self.wasmBaseline_ = value;
        }
        pub fn set_wasmIon_(&mut self, value: bool) {
            self.wasmIon_ = value;
        }
    }
    thread_local! {
        static CONTEXT_OPTIONS: RefCell<ContextOptions> = RefCell::new(ContextOptions::default());
    }
    pub fn context_options_ref() -> *mut ContextOptions {
        CONTEXT_OPTIONS.with(|options| options.as_ptr())
    }

    pub type NativeCallback = unsafe extern "C" fn(*mut JSContext, u32, *mut JSVal) -> bool;

    pub trait ToNativeCallback {
        fn to_native_callback(self) -> Option<NativeCallback>;
    }

    impl ToNativeCallback for Option<NativeCallback> {
        fn to_native_callback(self) -> Option<NativeCallback> {
            self
        }
    }

    impl ToNativeCallback for NativeCallback {
        fn to_native_callback(self) -> Option<NativeCallback> {
            Some(self)
        }
    }

    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct JSNativeWrapper {
        pub op: Option<NativeCallback>,
        pub info: *const JSJitInfo,
    }

    #[repr(C)]
    pub struct __BindgenBitfieldUnit<Storage> {
        storage: Storage,
    }
    impl<Storage> __BindgenBitfieldUnit<Storage> {
        pub const fn new(storage: Storage) -> Self {
            Self { storage }
        }
    }

    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct ProxyClassExtensionDef {
        _private: u8,
    }

    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct JSClassDef {
        pub name: *const std::os::raw::c_char,
        pub flags: u32,
        pub cOps: *const JSClassOps,
        pub spec: *const std::ffi::c_void,
        pub ext: *const ProxyClassExtensionDef,
        pub oOps: *const ObjectOps,
    }

    #[repr(C)]
    pub struct JSClassOps {
        pub addProperty:
            Option<unsafe extern "C" fn(*mut JSContext, *mut JSObject, jsid, *const JSVal)>,
        pub delProperty:
            Option<unsafe extern "C" fn(*mut JSContext, *mut JSObject, jsid, *const JSVal)>,
        pub enumerate: Option<unsafe extern "C" fn(*mut JSContext, *mut JSObject)>,
        pub newEnumerate: Option<
            for<'a> unsafe extern "C" fn(
                *mut JSContext,
                HandleObject<'a>,
                MutableHandleIdVector,
                bool,
            ) -> bool,
        >,
        pub resolve: Option<
            for<'a, 'b> unsafe extern "C" fn(
                *mut JSContext,
                HandleObject<'a>,
                HandleId<'b>,
                *mut bool,
            ) -> bool,
        >,
        pub mayResolve:
            Option<unsafe extern "C" fn(*const JSAtomState, PropertyKey, *mut JSObject) -> bool>,
        pub finalize: Option<unsafe extern "C" fn(*mut GCContext, *mut JSObject)>,
        pub call: Option<unsafe extern "C" fn(*mut JSContext, u32, *mut JSVal) -> bool>,
        pub construct: Option<unsafe extern "C" fn(*mut JSContext, u32, *mut JSVal) -> bool>,
        pub trace: Option<unsafe extern "C" fn(*mut JSTracer, *mut JSObject)>,
    }
    unsafe impl Sync for JSClassDef {}
    unsafe impl Sync for JSClassOps {}
    pub static ProxyClassOps: JSClassOps = JSClassOps {
        addProperty: None,
        delProperty: None,
        enumerate: None,
        newEnumerate: None,
        resolve: None,
        mayResolve: None,
        finalize: None,
        call: None,
        construct: None,
        trace: None,
    };
    pub static ProxyClassExtension: ProxyClassExtensionDef = ProxyClassExtensionDef { _private: 0 };
    pub static ProxyClass: JSClassDef = JSClassDef {
        name: std::ptr::null(),
        flags: 1 << 3,
        cOps: &ProxyClassOps,
        spec: std::ptr::null(),
        ext: &ProxyClassExtension,
        oOps: std::ptr::null(),
    };

    #[derive(Copy, Clone)]
    #[repr(C)]
    pub union JSPropertySpec_Name {
        pub string_: *const std::os::raw::c_char,
        pub symbol_: usize,
    }

    #[derive(Copy, Clone)]
    #[repr(C)]
    pub union JSPropertySpec_Accessor {
        pub native: JSNativeWrapper,
    }

    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct JSPropertySpec_AccessorsOrValue_Accessors {
        pub getter: JSPropertySpec_Accessor,
        pub setter: JSPropertySpec_Accessor,
    }

    #[derive(Copy, Clone)]
    #[repr(u8)]
    pub enum JSPropertySpec_Kind {
        Value = 0,
        SelfHostedAccessor = 1,
        NativeAccessor = 2,
    }

    #[derive(Copy, Clone)]
    #[repr(u8)]
    pub enum JSPropertySpec_ValueWrapper_Type {
        String = 0,
        Int32 = 1,
        Double = 2,
    }

    #[derive(Copy, Clone)]
    #[repr(C)]
    pub union JSPropertySpec_ValueWrapper__bindgen_ty_1 {
        pub string: *const std::os::raw::c_char,
        pub int32: i32,
        pub double_: f64,
    }

    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct JSPropertySpec_ValueWrapper {
        pub type_: JSPropertySpec_ValueWrapper_Type,
        pub __bindgen_anon_1: JSPropertySpec_ValueWrapper__bindgen_ty_1,
    }

    #[derive(Copy, Clone)]
    #[repr(C)]
    pub union JSPropertySpec_AccessorsOrValue {
        pub accessors: JSPropertySpec_AccessorsOrValue_Accessors,
        pub value: JSPropertySpec_ValueWrapper,
    }

    #[repr(C)]
    pub struct JSPropertySpec {
        pub name: JSPropertySpec_Name,
        pub attributes_: u32,
        pub kind_: JSPropertySpec_Kind,
        pub u: JSPropertySpec_AccessorsOrValue,
    }
    unsafe impl Sync for JSPropertySpec {}
    impl JSPropertySpec {
        pub const ZERO: Self = JSPropertySpec {
            name: JSPropertySpec_Name {
                string_: ptr::null(),
            },
            attributes_: 0,
            kind_: JSPropertySpec_Kind::NativeAccessor,
            u: JSPropertySpec_AccessorsOrValue {
                accessors: JSPropertySpec_AccessorsOrValue_Accessors {
                    getter: JSPropertySpec_Accessor {
                        native: JSNativeWrapper {
                            op: None,
                            info: ptr::null(),
                        },
                    },
                    setter: JSPropertySpec_Accessor {
                        native: JSNativeWrapper {
                            op: None,
                            info: ptr::null(),
                        },
                    },
                },
            },
        };
    }

    #[repr(C)]
    pub struct JSFunctionSpec {
        pub name: JSPropertySpec_Name,
        pub call: JSNativeWrapper,
        pub nargs: u16,
        pub flags: u16,
        pub selfHostedName: *const std::os::raw::c_char,
    }
    unsafe impl Sync for JSFunctionSpec {}

    pub const JSCLASS_IS_DOMJSCLASS: u32 = 1 << 4;
    pub const JSCLASS_IS_GLOBAL: u32 = 1 << 5;
    pub const JSCLASS_FOREGROUND_FINALIZE: u32 = 1 << 6;
    pub const JSCLASS_RESERVED_SLOTS_SHIFT: u32 = 8;
    pub const JSCLASS_RESERVED_SLOTS_MASK: u32 = 0xff << 8;
    pub const JSCLASS_GLOBAL_SLOT_COUNT: u32 = 4;
    pub const JSPROP_RESOLVING: u32 = 0x8000;
    pub const JSPROP_ENUMERATE: u32 = 0x01;
    pub const JSPROP_READONLY: u32 = 0x02;
    pub const JSPROP_PERMANENT: u32 = 0x04;
    pub const JSFUN_STUB_GSOPS: u32 = 0;
    pub const JSITER_HIDDEN: u32 = 0x1;
    pub const JSITER_OWNONLY: u32 = 0x8;
    pub const JSITER_SYMBOLS: u32 = 0x100;
    pub const JS_CALLEE: u32 = 0;

    pub struct JSProtoKey(pub u32);

    pub mod JS {
        pub const ProtoKey: u32 = 0;
        pub type CompartmentIterResult = super::CompartmentIterResult;
        #[derive(Copy, Clone)]
        pub struct TaggedColumnNumberOneOrigin {
            pub value_: u32,
        }
    }

    pub fn IsCallable<T>(_v: T) -> bool {
        false
    }
    pub fn GetWellKnownSymbol<C, W>(_cx: C, _which: W) -> JSVal {
        JSVal::default()
    }
    #[cfg(not(feature = "v8"))]
    pub fn GetRealmErrorPrototype<C: ?Sized>(_cx: &C) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn GetRealmErrorPrototype<C: ?Sized>(_cx: &C) -> *mut JSObject {
        crate::v8_glue::get_realm_error_prototype()
    }
    #[cfg(not(feature = "v8"))]
    pub fn GetRealmFunctionPrototype<C: ?Sized>(_cx: &C) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn GetRealmFunctionPrototype<C: ?Sized>(_cx: &C) -> *mut JSObject {
        crate::v8_glue::get_realm_function_prototype()
    }
    #[cfg(not(feature = "v8"))]
    pub fn GetRealmIteratorPrototype<C: ?Sized>(_cx: &C) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn GetRealmIteratorPrototype<C: ?Sized>(_cx: &C) -> *mut JSObject {
        crate::v8_glue::get_realm_iterator_prototype()
    }
    #[cfg(not(feature = "v8"))]
    pub fn GetRealmObjectPrototype<C: ?Sized>(_cx: &C) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn GetRealmObjectPrototype<C: ?Sized>(_cx: &C) -> *mut JSObject {
        crate::v8_glue::get_realm_object_prototype()
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_AtomizeAndPinString<C, S>(_cx: C, _s: S) -> *mut JSString {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn JS_AtomizeAndPinString<C, S: ToBytePtr>(_cx: C, s: S) -> *mut JSString {
        let bytes = s.to_byte_ptr();
        if bytes.is_null() {
            return ptr::null_mut();
        }
        // SAFETY: JSAPI string name inputs are null-terminated C strings.
        let len = unsafe { std::ffi::CStr::from_ptr(bytes as *const std::os::raw::c_char) }
            .to_bytes()
            .len();
        crate::v8_glue::atomize_string_n(ptr::null_mut(), bytes, len)
    }
    #[cfg(not(feature = "v8"))]
    pub fn ArrayBufferClone<C>(
        _cx: C,
        _obj: HandleObject<'_>,
        _byte_offset: usize,
        _byte_length: usize,
    ) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn ArrayBufferClone<C>(
        _cx: C,
        obj: HandleObject<'_>,
        byte_offset: usize,
        byte_length: usize,
    ) -> *mut JSObject {
        crate::v8_glue::clone_array_buffer(obj.get(), byte_offset, byte_length)
    }
    pub type BuildIdCharVector = std::ffi::c_void;
    pub struct CloneDataPolicy {
        pub allowIntraClusterClonableSharedObjects_: bool,
        pub allowSharedMemoryObjects_: bool,
    }
    #[derive(Copy, Clone)]
    pub struct ClippedTime {
        pub t: f64,
    }
    #[derive(Copy, Clone)]
    pub struct DOMCallbacks {
        pub instanceClassMatchesProto:
            Option<unsafe extern "C" fn(*const JSClass, u32, u32) -> bool>,
        pub instanceClassIsError: Option<unsafe extern "C" fn(*const JSClass) -> bool>,
    }
    #[derive(Copy, Clone)]
    pub struct Dispatchable_MaybeShuttingDown;
    impl Dispatchable_MaybeShuttingDown {
        pub const NotShuttingDown: Self = Self;
    }
    pub type DispatchablePointer = std::ffi::c_void;
    pub struct GCDescription {
        pub isZone_: bool,
        pub options_: GCOptions,
    }
    #[derive(Copy, Clone)]
    #[repr(u32)]
    pub enum GCOptions {
        Normal = 0,
        Shrink = 1,
        Shutdown = 2,
    }
    #[derive(Copy, Clone)]
    #[repr(u32)]
    pub enum GCProgress {
        GC_CYCLE_BEGIN = 0,
        GC_SLICE_BEGIN = 1,
        GC_SLICE_END = 2,
        GC_CYCLE_END = 3,
    }
    #[derive(Clone, Copy)]
    pub struct GCReason;
    impl GCReason {
        pub const API: Self = Self;
        pub const DOM_TESTUTILS: Self = Self;
    }
    pub struct JSGCParamKey;
    impl JSGCParamKey {
        pub const JSGC_MAX_BYTES: u32 = 0;
        pub const JSGC_INCREMENTAL_GC_ENABLED: u32 = 1;
        pub const JSGC_PER_ZONE_GC_ENABLED: u32 = 2;
        pub const JSGC_SLICE_TIME_BUDGET_MS: u32 = 3;
        pub const JSGC_COMPACTING_ENABLED: u32 = 4;
        pub const JSGC_HIGH_FREQUENCY_TIME_LIMIT: u32 = 5;
        pub const JSGC_LOW_FREQUENCY_HEAP_GROWTH: u32 = 6;
        pub const JSGC_HIGH_FREQUENCY_LARGE_HEAP_GROWTH: u32 = 7;
        pub const JSGC_HIGH_FREQUENCY_SMALL_HEAP_GROWTH: u32 = 8;
        pub const JSGC_SMALL_HEAP_SIZE_MAX: u32 = 9;
        pub const JSGC_LARGE_HEAP_SIZE_MIN: u32 = 10;
        pub const JSGC_MIN_EMPTY_CHUNK_COUNT: u32 = 11;
        pub const JSGC_BYTES: u32 = 12;
    }
    #[derive(Copy, Clone)]
    #[repr(u32)]
    pub enum JSGCStatus {
        JSGC_BEGIN = 0,
        JSGC_END = 1,
    }
    pub struct JSJitCompilerOption;
    impl JSJitCompilerOption {
        pub const JSJITCOMPILER_BASELINE_INTERPRETER_ENABLE: u32 = 0;
        pub const JSJITCOMPILER_BASELINE_ENABLE: u32 = 1;
        pub const JSJITCOMPILER_ION_ENABLE: u32 = 2;
        pub const JSJITCOMPILER_NATIVE_REGEXP_ENABLE: u32 = 3;
        pub const JSJITCOMPILER_BASELINE_WARMUP_TRIGGER: u32 = 4;
        pub const JSJITCOMPILER_ION_NORMAL_WARMUP_TRIGGER: u32 = 5;
    }
    pub type JSScript = std::ffi::c_void;
    pub struct JSSecurityCallbacks {
        pub contentSecurityPolicyAllows: Option<
            unsafe extern "C" fn(
                *mut JSContext,
                RuntimeCode,
                HandleString<'_>,
                CompilationType,
                Handle<'_, Vec<*mut JSString>>,
                HandleString<'_>,
                Handle<'_, Vec<JSVal>>,
                HandleValue<'_>,
                *mut bool,
            ) -> bool,
        >,
        pub codeForEvalGets: Option<
            unsafe extern "C" fn(*mut JSContext, HandleObject<'_>, MutableHandleString<'_>) -> bool,
        >,
        pub subsumes: Option<unsafe extern "C" fn(*mut JSPrincipals, *mut JSPrincipals) -> bool>,
    }
    unsafe impl Sync for JSSecurityCallbacks {}
    pub struct JSStructuredCloneCallbacks {
        pub read: Option<
            unsafe extern "C" fn(
                *mut JSContext,
                *mut JSStructuredCloneReader,
                *const CloneDataPolicy,
                u32,
                u32,
                *mut std::ffi::c_void,
            ) -> *mut JSObject,
        >,
        pub write: Option<
            unsafe extern "C" fn(
                *mut JSContext,
                *mut JSStructuredCloneWriter,
                HandleObject<'_>,
                *mut bool,
                *mut std::ffi::c_void,
            ) -> bool,
        >,
        pub reportError: Option<
            unsafe extern "C" fn(
                *mut JSContext,
                u32,
                *mut std::ffi::c_void,
                *const std::os::raw::c_char,
            ),
        >,
        pub readTransfer: Option<
            unsafe extern "C" fn(
                *mut JSContext,
                *mut JSStructuredCloneReader,
                *const CloneDataPolicy,
                u32,
                *mut std::ffi::c_void,
                u64,
                *mut std::ffi::c_void,
                MutableHandleObject<'_>,
            ) -> bool,
        >,
        pub writeTransfer: Option<
            unsafe extern "C" fn(
                *mut JSContext,
                HandleObject<'_>,
                *mut std::ffi::c_void,
                *mut u32,
                *mut TransferableOwnership,
                *mut *mut std::ffi::c_void,
                *mut u64,
            ) -> bool,
        >,
        pub freeTransfer: Option<
            unsafe extern "C" fn(
                u32,
                TransferableOwnership,
                *mut std::ffi::c_void,
                u64,
                *mut std::ffi::c_void,
            ),
        >,
        pub canTransfer: Option<
            unsafe extern "C" fn(
                *mut JSContext,
                HandleObject<'_>,
                *mut bool,
                *mut std::ffi::c_void,
            ) -> bool,
        >,
        pub sabCloned:
            Option<unsafe extern "C" fn(*mut JSContext, bool, *mut std::ffi::c_void) -> bool>,
    }
    unsafe impl Sync for JSStructuredCloneCallbacks {}
    pub type JSStructuredCloneReader = std::ffi::c_void;
    pub type JSStructuredCloneWriter = std::ffi::c_void;
    pub type JobQueue = std::ffi::c_void;
    pub type MimeType = std::ffi::c_void;
    #[derive(Copy, Clone, PartialEq, Eq)]
    #[repr(u32)]
    pub enum ModuleErrorBehaviour {
        ThrowModuleErrorsSync = 0,
    }
    #[derive(Copy, Clone, PartialEq, Eq, Hash)]
    #[repr(u32)]
    pub enum ModuleType {
        Unknown = 0,
        JavaScript = 1,
        JSON = 2,
    }
    #[derive(Copy, Clone)]
    #[repr(u32)]
    pub enum PromiseRejectionHandlingState {
        Unhandled = 0,
        Handled = 1,
    }
    #[derive(Copy, Clone, PartialEq, Eq)]
    #[repr(u32)]
    pub enum PromiseState {
        Pending = 0,
        Fulfilled = 1,
        Rejected = 2,
    }
    #[derive(Copy, Clone, PartialEq, Eq)]
    #[repr(u32)]
    pub enum PromiseUserInputEventHandlingState {
        DontCare = 0,
        HadUserInteractionAtCreation = 1,
        DidntHaveUserInteractionAtCreation = 2,
    }
    #[derive(Copy, Clone)]
    pub struct RegExpFlags {
        pub flags_: u32,
    }
    #[derive(Copy, Clone)]
    #[repr(u32)]
    pub enum RuntimeCode {
        JS = 0,
        WASM = 1,
    }
    pub type ScriptEnvironmentPreparer_Closure = std::ffi::c_void;
    pub fn SetProcessBuildIdOp<F>(_op: F) {}
    pub type StreamConsumer = std::ffi::c_void;
    #[derive(Copy, Clone)]
    #[repr(u32)]
    pub enum StructuredCloneScope {
        DifferentProcess = 0,
    }
    #[derive(Copy, Clone)]
    #[repr(u32)]
    pub enum TransferableOwnership {
        SCTAG_TMO_CUSTOM = 0,
    }
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub enum Type {
        Int8,
        Uint8,
        Uint8Clamped,
        Int16,
        Uint16,
        Float16,
        Int32,
        Uint32,
        Float32,
        Int64,
        Float64,
        BigInt64,
        BigUint64,
        Simd128,
        MaxTypedArrayViewType,
    }
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub enum JSType {
        JSTYPE_FUNCTION,
        JSTYPE_STRING,
        JSTYPE_OBJECT,
    }
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub enum ESClass {
        Other,
        Object,
        Array,
        Function,
        Map,
        Set,
        Date,
        RegExp,
    }
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub enum CompilationType {
        Function,
        Script,
        Module,
    }
    #[derive(Copy, Clone)]
    pub enum AsmJSOption {
        Enabled,
        DisabledByAsmJSPref,
    }
    #[derive(Copy, Clone)]
    pub enum SupportUnscopables {
        Yes,
        No,
    }
    #[derive(Copy, Clone)]
    pub enum SavedFrameSelfHosted {
        Include,
        Exclude,
    }
    pub type HandleString<'a> = Handle<'a, *mut JSString>;
    pub type MutableHandleString<'a> = MutableHandle<'a, *mut JSString>;
    pub const JS_STRUCTURED_CLONE_VERSION: u32 = 0;
    pub const JSCLASS_DELAY_METADATA_BUILDER: u32 = 0;
    pub const JSClass_NON_NATIVE: u32 = 0;
    pub const RegExpFlag_UnicodeSets: u32 = 0;
    pub fn ArrayBufferCopyData<C>(
        _cx: C,
        _to: HandleObject<'_>,
        _to_offset: usize,
        _from: HandleObject<'_>,
        _from_offset: usize,
        _len: usize,
    ) -> bool {
        false
    }
    #[cfg(not(feature = "v8"))]
    pub fn GetArrayBufferByteLength<O>(_obj: O) -> usize {
        0
    }
    #[cfg(feature = "v8")]
    pub fn GetArrayBufferByteLength<O>(obj: O) -> usize
    where
        O: Into<*mut JSObject>,
    {
        crate::v8_glue::array_buffer_len(obj.into())
    }
    pub fn HasDefinedArrayBufferDetachKey<C>(
        _cx: C,
        _obj: HandleObject<'_>,
        is_defined: &mut bool,
    ) -> bool {
        *is_defined = false;
        false
    }
    #[cfg(not(feature = "v8"))]
    pub fn IsArrayBufferObject<O>(_obj: O) -> bool {
        false
    }
    #[cfg(feature = "v8")]
    pub fn IsArrayBufferObject<O>(obj: O) -> bool
    where
        O: Into<*mut JSObject>,
    {
        crate::v8_glue::is_array_buffer(obj.into())
    }
    #[cfg(not(feature = "v8"))]
    pub fn IsDetachedArrayBufferObject<O>(_obj: O) -> bool {
        false
    }
    #[cfg(feature = "v8")]
    pub fn IsDetachedArrayBufferObject<O>(obj: O) -> bool
    where
        O: Into<*mut JSObject>,
    {
        crate::v8_glue::is_detached_array_buffer(obj.into())
    }
    pub fn IsConstructor<O>(_obj: O) -> bool {
        false
    }
    pub fn IsCyclicModule<O>(_obj: O) -> bool {
        false
    }
    #[cfg(not(feature = "v8"))]
    pub fn IsPromiseObject<O>(_obj: O) -> bool {
        false
    }
    #[cfg(feature = "v8")]
    pub fn IsPromiseObject<O>(obj: O) -> bool
    where
        O: Into<*mut JSObject>,
    {
        crate::v8_glue::is_promise_object(obj.into())
    }
    pub fn JobQueueMayNotBeEmpty<Q>(_queue: Q) {}
    pub fn JS_AddInterruptCallback<C, F>(_cx: C, _callback: F) -> bool {
        false
    }
    pub fn JS_FreezeObject<C, O>(_cx: C, _obj: O) -> bool {
        false
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_ForwardSetPropertyTo<C, O, I, V, R, S>(
        _cx: C,
        _obj: O,
        _id: I,
        _v: V,
        _receiver: R,
        _result: S,
    ) -> bool {
        false
    }
    #[cfg(feature = "v8")]
    pub fn JS_ForwardSetPropertyTo<C, O, I, V, R, S>(
        _cx: C,
        obj: O,
        id: I,
        v: V,
        _receiver: R,
        _result: S,
    ) -> bool
    where
        O: Into<*mut JSObject>,
        I: Into<jsid>,
        V: ToJsapiPropertyValue,
    {
        crate::v8_glue::set_property_by_jsid(obj.into(), id.into(), v.to_jsapi_property_value())
    }
    pub fn JS_GC<C, R>(_cx: C, _reason: R) {}
    #[cfg(not(feature = "v8"))]
    pub fn JS_GetArrayBufferViewBuffer<C>(
        _cx: C,
        _obj: HandleObject<'_>,
        is_shared: &mut bool,
    ) -> *mut JSObject {
        *is_shared = false;
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn JS_GetArrayBufferViewBuffer<C>(
        _cx: C,
        obj: HandleObject<'_>,
        is_shared: &mut bool,
    ) -> *mut JSObject {
        *is_shared = false;
        crate::v8_glue::array_view_buffer(obj.get())
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_GetArrayBufferViewByteLength<O>(_obj: O) -> usize {
        0
    }
    #[cfg(feature = "v8")]
    pub fn JS_GetArrayBufferViewByteLength<O>(obj: O) -> usize
    where
        O: Into<*mut JSObject>,
    {
        crate::v8_glue::array_view_byte_length(obj.into())
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_GetArrayBufferViewByteOffset<O>(_obj: O) -> usize {
        0
    }
    #[cfg(feature = "v8")]
    pub fn JS_GetArrayBufferViewByteOffset<O>(obj: O) -> usize
    where
        O: Into<*mut JSObject>,
    {
        crate::v8_glue::array_view_byte_offset(obj.into())
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_GetArrayBufferViewType<O>(_obj: O) -> Type {
        Type::MaxTypedArrayViewType
    }
    #[cfg(feature = "v8")]
    pub fn JS_GetArrayBufferViewType<O>(obj: O) -> Type
    where
        O: Into<*mut JSObject>,
    {
        crate::v8_glue::array_view_type(obj.into())
    }
    pub fn JS_GetFunctionArity<F>(_fun: F) -> u16 {
        0
    }
    pub fn JS_GetFunctionDisplayId<C, F, O>(_cx: C, _fun: F, _out: O) -> bool {
        false
    }
    pub fn JS_GetFunctionId<C, F, O>(_cx: C, _fun: F, _out: O) -> bool {
        false
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_GetOwnPropertyDescriptorById<C, O, I, D, F>(
        _cx: C,
        _obj: O,
        _id: I,
        _desc: D,
        _found: F,
    ) -> bool {
        false
    }
    #[cfg(feature = "v8")]
    pub fn JS_GetOwnPropertyDescriptorById<C, O, I, D, F>(
        _cx: C,
        obj: O,
        id: I,
        desc: D,
        is_none: F,
    ) -> bool
    where
        O: Into<*mut JSObject>,
        I: Into<jsid>,
        D: Into<*mut PropertyDescriptor>,
        F: SetJsapiBoolOut,
    {
        crate::v8_glue::get_property_descriptor_by_jsid(obj.into(), id.into(), desc.into(), is_none)
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_GetPendingException<C>(_cx: C, _vp: MutableHandleValue<'_>) -> bool {
        false
    }
    #[cfg(feature = "v8")]
    pub fn JS_GetPendingException<C>(_cx: C, vp: MutableHandleValue<'_>) -> bool {
        if let Some(val) = crate::v8_glue::get_pending_exception() {
            vp.set_jsapi_val_out(val);
            true
        } else {
            false
        }
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_GetPropertyById<C, O, I, V>(_cx: C, _obj: O, _id: I, _vp: V) -> bool {
        false
    }
    #[cfg(feature = "v8")]
    pub fn JS_GetPropertyById<C, O, I, V>(_cx: C, obj: O, id: I, vp: V) -> bool
    where
        O: Into<*mut JSObject>,
        I: Into<jsid>,
        V: SetJsapiValOut,
    {
        match crate::v8_glue::get_property_by_jsid(obj.into(), id.into()) {
            Some(val) => {
                vp.set_jsapi_val_out(val);
                true
            },
            None => false,
        }
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_GetStringLength<S>(_s: S) -> usize {
        0
    }
    #[cfg(feature = "v8")]
    pub fn JS_GetStringLength<S: Into<*mut JSString>>(s: S) -> usize {
        crate::v8_glue::string_len(s.into())
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_GetTypedArrayLength<O>(_obj: O) -> usize {
        0
    }
    #[cfg(feature = "v8")]
    pub fn JS_GetTypedArrayLength<O>(obj: O) -> usize
    where
        O: Into<*mut JSObject>,
    {
        crate::v8_glue::typed_array_length(obj.into())
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_HasOwnPropertyById<C, O, I, F>(_cx: C, _obj: O, _id: I, _found: F) -> bool {
        false
    }
    #[cfg(feature = "v8")]
    pub fn JS_HasOwnPropertyById<C, O, I, F>(_cx: C, obj: O, id: I, found: F) -> bool
    where
        O: Into<*mut JSObject>,
        I: Into<jsid>,
        F: SetJsapiBoolOut,
    {
        found.set_jsapi_bool_out(crate::v8_glue::has_property_by_jsid(obj.into(), id.into()));
        true
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_IsArrayBufferViewObject<O>(_obj: O) -> bool {
        false
    }
    #[cfg(feature = "v8")]
    pub fn JS_IsArrayBufferViewObject<O>(obj: O) -> bool
    where
        O: Into<*mut JSObject>,
    {
        crate::v8_glue::is_array_buffer_view(obj.into())
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_IsTypedArrayObject<O>(_obj: O) -> bool {
        false
    }
    #[cfg(feature = "v8")]
    pub fn JS_IsTypedArrayObject<O>(obj: O) -> bool
    where
        O: Into<*mut JSObject>,
    {
        crate::v8_glue::is_array_buffer_view(obj.into())
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_NewDataView<C>(
        _cx: C,
        _obj: HandleObject<'_>,
        _offset: usize,
        _len: usize,
    ) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn JS_NewDataView<C>(
        _cx: C,
        obj: HandleObject<'_>,
        offset: usize,
        len: usize,
    ) -> *mut JSObject {
        crate::v8_glue::new_typed_array_with_buffer(obj.get(), offset, len as i64, Type::Uint8)
    }
    #[cfg(not(feature = "v8"))]
    pub fn CurrentGlobalOrNull<C: ?Sized>(_cx: &C) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn CurrentGlobalOrNull<C: ?Sized>(_cx: &C) -> *mut JSObject {
        crate::v8_glue::current_global_object()
    }
    pub fn DisableJitBackend() {}
    #[cfg(not(feature = "v8"))]
    pub fn GetObjectRealmOrNull<O>(_obj: O) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn GetObjectRealmOrNull<O>(_obj: O) -> *mut JSObject {
        crate::v8_glue::current_global_object()
    }
    #[cfg(not(feature = "v8"))]
    pub fn GetPropertyKeys<C, O, F, V>(_cx: C, _obj: O, _flags: F, _props: V) -> bool {
        false
    }
    #[cfg(feature = "v8")]
    pub fn GetPropertyKeys<C, O, F, V>(_cx: C, obj: O, _flags: F, props: V) -> bool
    where
        O: Into<*mut JSObject>,
        V: Into<MutableHandleIdVector>,
    {
        crate::v8_glue::get_property_keys(obj.into(), props.into())
    }
    pub fn GetPromiseUserInputEventHandlingState<P: ?Sized>(_promise: &P) -> PromiseUserInputEventHandlingState {
        PromiseUserInputEventHandlingState::DontCare
    }
    pub fn GetRealmPrincipals<R>(_realm: R) -> *mut JSPrincipals {
        ptr::null_mut()
    }
    pub fn GetScriptedCallerGlobal<C: ?Sized>(_cx: &C) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(not(feature = "v8"))]
    pub fn NewFunctionWithReserved<C, N>(
        _cx: C,
        _native: Option<NativeCallback>,
        _nargs: u32,
        _flags: u32,
        _name: N,
    ) -> *mut JSFunction {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn NewFunctionWithReserved<C, N>(
        _cx: C,
        native: Option<NativeCallback>,
        nargs: u32,
        flags: u32,
        name: N,
    ) -> *mut JSFunction
    where
        N: ToBytePtr,
    {
        let name = crate::v8_glue::property_name_from_raw(name.to_byte_ptr());
        crate::v8_glue::js_new_function(ptr::null_mut(), native, nargs, flags, name)
    }
    pub fn SetModuleDynamicImportHook<C, F>(_cx: C, _hook: F) {}
    pub fn SetModuleMetadataHook<C, F>(_cx: C, _hook: F) {}
    pub fn SetScriptPrivateReferenceHooks<C, A, B>(_cx: C, _add: A, _release: B) {}
    pub fn ToPrimitive<C, H>(_cx: C, _obj: *mut JSObject, _ty: JSType, _rval: H) -> bool {
        false
    }
    pub fn JS_WriteBytes<W>(_writer: W, _data: *const std::ffi::c_void, _len: usize) -> bool {
        false
    }
    pub fn JS_ReadBytes<R>(_reader: R, _data: *mut std::ffi::c_void, _len: usize) -> bool {
        false
    }
    pub fn GetSavedFrameFunctionDisplayName<C, O, S, R>(
        _cx: *mut JSContext,
        _principals: C,
        _obj: O,
        _name: S,
        _self_hosted: R,
    ) -> bool {
        false
    }
    pub fn GetSavedFrameSource<C, O, S, R>(
        _cx: *mut JSContext,
        _principals: C,
        _obj: O,
        _source: S,
        _self_hosted: R,
    ) -> bool {
        false
    }
    pub fn GetSavedFrameLine<C, O, L, R>(
        _cx: *mut JSContext,
        _principals: C,
        _obj: O,
        _line: L,
        _self_hosted: R,
    ) -> bool {
        false
    }
    pub fn GetSavedFrameColumn<C, O, L, R>(
        _cx: *mut JSContext,
        _principals: C,
        _obj: O,
        _column: L,
        _self_hosted: R,
    ) -> bool {
        false
    }
    pub fn JS_NewStringCopyUTF8N<C, S>(_cx: C, _s: S) -> *mut JSString {
        ptr::null_mut()
    }
    pub fn JS_ReadUint32Pair<R>(_reader: R, _a: *mut u32, _b: *mut u32) -> bool {
        false
    }
    pub fn JS_SetImmutablePrototype<C, O>(_cx: C, _obj: O, succeeded: *mut bool) -> bool {
        if !succeeded.is_null() {
            unsafe {
                *succeeded = true;
            }
        }
        true
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_SetPendingException<C, B>(_cx: C, _val: JSVal, _behavior: B) {}
    #[cfg(feature = "v8")]
    pub fn JS_SetPendingException<C, B>(_cx: C, val: JSVal, _behavior: B) {
        crate::v8_glue::set_pending_exception(val);
    }
    pub fn JS_ValueToFunction<C>(_cx: C, _val: JSVal) -> *mut JSObject {
        ptr::null_mut()
    }
    pub fn JS_WriteUint32Pair<W>(_writer: W, _a: u32, _b: u32) -> bool {
        false
    }
    #[cfg(not(feature = "v8"))]
    pub fn NewArrayBuffer<C>(_cx: C, _len: usize) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn NewArrayBuffer<C>(_cx: C, len: usize) -> *mut JSObject {
        crate::v8_glue::new_array_buffer(len)
    }
    #[cfg(not(feature = "v8"))]
    pub fn NewArrayBufferWithContents<C, P>(_cx: C, _len: usize, _data: P) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn NewArrayBufferWithContents<C, P>(_cx: C, len: usize, data: P) -> *mut JSObject
    where
        P: Into<*mut std::ffi::c_void>,
    {
        crate::v8_glue::new_array_buffer_with_contents(len, data.into() as *const u8)
    }
    #[cfg(not(feature = "v8"))]
    pub unsafe fn NewExternalArrayBuffer(
        _cx: *mut JSContext,
        _len: usize,
        _data: *mut std::ffi::c_void,
        _free_func: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void)>,
        _free_user_data: *mut std::ffi::c_void,
    ) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub unsafe fn NewExternalArrayBuffer(
        _cx: *mut JSContext,
        len: usize,
        data: *mut std::ffi::c_void,
        _free_func: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void)>,
        _free_user_data: *mut std::ffi::c_void,
    ) -> *mut JSObject {
        crate::v8_glue::new_array_buffer_with_contents(len, data as *const u8)
    }
    #[cfg(not(feature = "v8"))]
    pub unsafe fn DetachArrayBuffer<C, O>(_cx: C, _obj: O) -> bool {
        false
    }
    #[cfg(feature = "v8")]
    pub unsafe fn DetachArrayBuffer<C, O>(_cx: C, obj: O) -> bool
    where
        O: Into<*mut JSObject>,
    {
        crate::v8_glue::detach_array_buffer(obj.into())
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_NewBigInt64ArrayWithBuffer<C>(
        _cx: C,
        _buffer: HandleObject<'_>,
        _offset: usize,
        _len: i64,
    ) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn JS_NewBigInt64ArrayWithBuffer<C>(
        _cx: C,
        buffer: HandleObject<'_>,
        offset: usize,
        len: i64,
    ) -> *mut JSObject {
        crate::v8_glue::new_typed_array_with_buffer(buffer.get(), offset, len, Type::BigInt64)
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_NewBigUint64ArrayWithBuffer<C>(
        _cx: C,
        _buffer: HandleObject<'_>,
        _offset: usize,
        _len: i64,
    ) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn JS_NewBigUint64ArrayWithBuffer<C>(
        _cx: C,
        buffer: HandleObject<'_>,
        offset: usize,
        len: i64,
    ) -> *mut JSObject {
        crate::v8_glue::new_typed_array_with_buffer(buffer.get(), offset, len, Type::BigUint64)
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_NewFloat16ArrayWithBuffer<C>(
        _cx: C,
        _buffer: HandleObject<'_>,
        _offset: usize,
        _len: i64,
    ) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn JS_NewFloat16ArrayWithBuffer<C>(
        _cx: C,
        buffer: HandleObject<'_>,
        offset: usize,
        len: i64,
    ) -> *mut JSObject {
        crate::v8_glue::new_typed_array_with_buffer(buffer.get(), offset, len, Type::Float16)
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_NewFloat32ArrayWithBuffer<C>(
        _cx: C,
        _buffer: HandleObject<'_>,
        _offset: usize,
        _len: i64,
    ) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn JS_NewFloat32ArrayWithBuffer<C>(
        _cx: C,
        buffer: HandleObject<'_>,
        offset: usize,
        len: i64,
    ) -> *mut JSObject {
        crate::v8_glue::new_typed_array_with_buffer(buffer.get(), offset, len, Type::Float32)
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_NewFloat64ArrayWithBuffer<C>(
        _cx: C,
        _buffer: HandleObject<'_>,
        _offset: usize,
        _len: i64,
    ) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn JS_NewFloat64ArrayWithBuffer<C>(
        _cx: C,
        buffer: HandleObject<'_>,
        offset: usize,
        len: i64,
    ) -> *mut JSObject {
        crate::v8_glue::new_typed_array_with_buffer(buffer.get(), offset, len, Type::Float64)
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_NewInt8ArrayWithBuffer<C>(
        _cx: C,
        _buffer: HandleObject<'_>,
        _offset: usize,
        _len: i64,
    ) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn JS_NewInt8ArrayWithBuffer<C>(
        _cx: C,
        buffer: HandleObject<'_>,
        offset: usize,
        len: i64,
    ) -> *mut JSObject {
        crate::v8_glue::new_typed_array_with_buffer(buffer.get(), offset, len, Type::Int8)
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_NewInt16ArrayWithBuffer<C>(
        _cx: C,
        _buffer: HandleObject<'_>,
        _offset: usize,
        _len: i64,
    ) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn JS_NewInt16ArrayWithBuffer<C>(
        _cx: C,
        buffer: HandleObject<'_>,
        offset: usize,
        len: i64,
    ) -> *mut JSObject {
        crate::v8_glue::new_typed_array_with_buffer(buffer.get(), offset, len, Type::Int16)
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_NewInt32ArrayWithBuffer<C>(
        _cx: C,
        _buffer: HandleObject<'_>,
        _offset: usize,
        _len: i64,
    ) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn JS_NewInt32ArrayWithBuffer<C>(
        _cx: C,
        buffer: HandleObject<'_>,
        offset: usize,
        len: i64,
    ) -> *mut JSObject {
        crate::v8_glue::new_typed_array_with_buffer(buffer.get(), offset, len, Type::Int32)
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_NewUint8ArrayWithBuffer<C>(
        _cx: C,
        _buffer: HandleObject<'_>,
        _offset: usize,
        _len: i64,
    ) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn JS_NewUint8ArrayWithBuffer<C>(
        _cx: C,
        buffer: HandleObject<'_>,
        offset: usize,
        len: i64,
    ) -> *mut JSObject {
        crate::v8_glue::new_typed_array_with_buffer(buffer.get(), offset, len, Type::Uint8)
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_NewUint8ClampedArrayWithBuffer<C>(
        _cx: C,
        _buffer: HandleObject<'_>,
        _offset: usize,
        _len: i64,
    ) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn JS_NewUint8ClampedArrayWithBuffer<C>(
        _cx: C,
        buffer: HandleObject<'_>,
        offset: usize,
        len: i64,
    ) -> *mut JSObject {
        crate::v8_glue::new_typed_array_with_buffer(buffer.get(), offset, len, Type::Uint8Clamped)
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_NewUint16ArrayWithBuffer<C>(
        _cx: C,
        _buffer: HandleObject<'_>,
        _offset: usize,
        _len: i64,
    ) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn JS_NewUint16ArrayWithBuffer<C>(
        _cx: C,
        buffer: HandleObject<'_>,
        offset: usize,
        len: i64,
    ) -> *mut JSObject {
        crate::v8_glue::new_typed_array_with_buffer(buffer.get(), offset, len, Type::Uint16)
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_NewUint32ArrayWithBuffer<C>(
        _cx: C,
        _buffer: HandleObject<'_>,
        _offset: usize,
        _len: i64,
    ) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn JS_NewUint32ArrayWithBuffer<C>(
        _cx: C,
        buffer: HandleObject<'_>,
        offset: usize,
        len: i64,
    ) -> *mut JSObject {
        crate::v8_glue::new_typed_array_with_buffer(buffer.get(), offset, len, Type::Uint32)
    }
    #[cfg(not(feature = "v8"))]
    pub fn NewArrayObject<C, A>(_cx: C, _args: A) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn NewArrayObject<C, A>(_cx: C, args: A) -> *mut JSObject
    where
        A: ToArrayObjectInit,
    {
        match args.to_array_values() {
            Some(values) => crate::v8_glue::new_array_object(values),
            None => ptr::null_mut(),
        }
    }
    #[cfg(not(feature = "v8"))]
    pub fn NewArrayObject1<C, A>(_cx: C, _args: A) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn NewArrayObject1<C, A>(_cx: C, args: A) -> *mut JSObject
    where
        A: ToArrayObjectInit,
    {
        NewArrayObject(_cx, args)
    }
    pub fn NewDateObject<C, T>(_cx: C, _time: T) -> *mut JSObject {
        ptr::null_mut()
    }
    pub fn NewUCRegExpObject<C, S, F>(_cx: C, _source: S, _len: usize, _flags: F) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(not(feature = "v8"))]
    pub fn SetFunctionNativeReserved<F, V>(_fun: F, _which: usize, _val: V) {}
    #[cfg(feature = "v8")]
    pub fn SetFunctionNativeReserved<F, V>(fun: F, which: usize, val: V)
    where
        F: ToFunctionObjectPtr,
        V: ToJsapiPropertyValue,
    {
        crate::v8_glue::set_function_native_reserved(
            fun.to_function_object_ptr(),
            which,
            val.to_jsapi_property_value(),
        )
    }
    #[cfg(not(feature = "v8"))]
    pub fn GetFunctionNativeReserved<F>(_fun: F, _which: usize) -> JSVal {
        JSVal::default()
    }
    #[cfg(feature = "v8")]
    pub fn GetFunctionNativeReserved<F>(fun: F, which: usize) -> JSVal
    where
        F: ToFunctionObjectPtr,
    {
        crate::v8_glue::get_function_native_reserved(fun.to_function_object_ptr(), which)
    }
    pub fn SetModulePrivate<M, V>(_module: M, _value: V) {}
    pub fn GetModuleResolveHook<C>(_rt: C) -> Option<*mut std::ffi::c_void> {
        None
    }
    pub fn SetModulePrivateReferenceHooks<C, G, S>(_cx: C, _get: G, _set: S) {}
    pub fn SetModuleResolveHook<C, H>(_cx: C, _hook: H) {}
    #[cfg(not(feature = "v8"))]
    pub fn SetScriptPrivate<S, V>(_script: S, _value: V) {}
    #[cfg(feature = "v8")]
    pub fn SetScriptPrivate<S, V>(_script: S, value: V)
    where
        S: Into<*mut JSScript>,
        V: IntoScriptPrivateValue,
    {
        crate::v8_glue::set_script_private(_script.into(), value.into_script_private_value());
    }

    pub trait IntoScriptPrivateValue {
        fn into_script_private_value(self) -> JSVal;
    }

    impl IntoScriptPrivateValue for JSVal {
        fn into_script_private_value(self) -> JSVal {
            self
        }
    }

    impl IntoScriptPrivateValue for &JSVal {
        fn into_script_private_value(self) -> JSVal {
            *self
        }
    }

    impl IntoScriptPrivateValue for HandleValue<'_> {
        fn into_script_private_value(self) -> JSVal {
            self.get()
        }
    }

    impl IntoScriptPrivateValue for &HandleValue<'_> {
        fn into_script_private_value(self) -> JSVal {
            self.get()
        }
    }
    pub fn StealArrayBufferContents<C>(_cx: C, _obj: HandleObject<'_>) -> *mut std::ffi::c_void {
        ptr::null_mut()
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_ForwardGetPropertyTo<C, O, R, V>(
        _cx: C,
        _obj: O,
        _id: impl Into<jsid>,
        _receiver: R,
        _vp: V,
    ) -> bool {
        false
    }
    #[cfg(feature = "v8")]
    pub fn JS_ForwardGetPropertyTo<C, O, R, V>(
        _cx: C,
        obj: O,
        id: impl Into<jsid>,
        _receiver: R,
        vp: V,
    ) -> bool
    where
        O: Into<*mut JSObject>,
        V: SetJsapiValOut,
    {
        match crate::v8_glue::get_property_by_jsid(obj.into(), id.into()) {
            Some(val) => {
                vp.set_jsapi_val_out(val);
                true
            },
            None => false,
        }
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_GetPropertyDescriptorById<C, O, D, Ign>(
        _cx: C,
        _obj: O,
        _id: impl Into<jsid>,
        _desc: D,
        _ignored: Ign,
        _found: *mut bool,
    ) -> bool
    where
        O: Into<*mut JSObject>,
        D: Into<*mut PropertyDescriptor>,
    {
        false
    }
    #[cfg(feature = "v8")]
    pub fn JS_GetPropertyDescriptorById<C, O, D, Ign>(
        _cx: C,
        obj: O,
        id: impl Into<jsid>,
        desc: D,
        _ignored: Ign,
        is_none: *mut bool,
    ) -> bool
    where
        O: Into<*mut JSObject>,
        D: Into<*mut PropertyDescriptor>,
    {
        crate::v8_glue::get_property_descriptor_by_jsid(obj.into(), id.into(), desc.into(), is_none)
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_HasPropertyById<C, O>(
        _cx: C,
        _obj: O,
        _id: impl Into<jsid>,
        _found: *mut bool,
    ) -> bool {
        false
    }
    #[cfg(feature = "v8")]
    pub fn JS_HasPropertyById<C, O>(_cx: C, obj: O, id: impl Into<jsid>, found: *mut bool) -> bool
    where
        O: Into<*mut JSObject>,
    {
        found.set_jsapi_bool_out(crate::v8_glue::has_property_by_jsid(obj.into(), id.into()));
        true
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_NewPlainObject<C: ?Sized>(_cx: &C) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn JS_NewPlainObject<C: ?Sized>(_cx: &C) -> *mut JSObject {
        crate::v8_glue::js_new_object()
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_SetReservedSlot<V>(_obj: *mut JSObject, _index: u32, _val: V) {}
    #[cfg(feature = "v8")]
    pub fn JS_SetReservedSlot<V>(_obj: *mut JSObject, _index: u32, _val: V)
    where
        V: crate::glue::ToSlotValue,
    {
        crate::v8_glue::set_reserved_slot(_obj, _index, _val.to_slot_value())
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_NewObject<C>(_cx: C, _clasp: *const JSClass) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn JS_NewObject<C>(_cx: C, clasp: *const JSClass) -> *mut JSObject {
        crate::v8_glue::js_new_object_with_class(clasp)
    }
    #[repr(u32)]
    #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
    pub enum SymbolCode {
        isConcatSpreadable = 0,
        iterator = 1,
        match_ = 2,
        replace = 3,
        search = 4,
        species = 5,
        hasInstance = 6,
        split = 7,
        toPrimitive = 8,
        toStringTag = 9,
        unscopables = 10,
        asyncIterator = 11,
        matchAll = 12,
        Limit = 13,
        WellKnownAPILimit = 2147483648,
        PrivateNameSymbol = 4294967293,
        InSymbolRegistry = 4294967294,
        UniqueSymbol = 4294967295,
    }

    pub fn AddAssociatedMemory(_obj: *mut JSObject, _sz: usize, _assoc: u32) {}
    pub unsafe extern "C" fn JS_GlobalObjectTraceHook(_trc: *mut JSTracer, _global: *mut JSObject) {
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_DeprecatedStringHasLatin1Chars(_s: *mut JSString) -> bool {
        false
    }
    #[cfg(feature = "v8")]
    pub fn JS_DeprecatedStringHasLatin1Chars(s: *mut JSString) -> bool {
        crate::v8_glue::has_latin1_chars(s)
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_GetTwoByteLatin1Chars(_s: *mut JSString) -> *const u8 {
        ptr::null()
    }
    #[cfg(feature = "v8")]
    pub fn JS_GetTwoByteLatin1Chars(s: *mut JSString) -> *const u8 {
        crate::v8_glue::latin1_chars_and_len(s, ptr::null_mut())
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_GetTwoByteStringChars(_s: *mut JSString) -> *const u16 {
        ptr::null()
    }
    #[cfg(feature = "v8")]
    pub fn JS_GetTwoByteStringChars(s: *mut JSString) -> *const u16 {
        crate::v8_glue::two_byte_chars_and_len(s, ptr::null_mut())
    }
    pub const JSCLASS_IS_PROXY: u32 = 1 << 3;
    pub const JSCLASS_USERBIT1: u32 = 1 << 14;

    pub fn AddRawValueRoot(
        _cx: *mut JSContext,
        _vp: *mut JSVal,
        _name: *const std::os::raw::c_char,
    ) -> bool {
        true
    }
    pub fn RemoveRawValueRoot(_cx: *mut JSContext, _vp: *mut JSVal) {}
    pub fn RemoveAssociatedMemory(_obj: *mut JSObject, _sz: usize, _assoc: u32) {}
    #[cfg(not(feature = "v8"))]
    pub fn IsWindowProxy(_obj: *mut JSObject) -> bool {
        false
    }
    #[cfg(feature = "v8")]
    pub fn IsWindowProxy(obj: *mut JSObject) -> bool {
        crate::v8_glue::is_window_proxy(obj)
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_GetLatin1StringCharsAndLength<C>(
        _cx: C,
        _nogc: *const std::ffi::c_void,
        _s: *mut JSString,
        _len: *mut usize,
    ) -> *const u8 {
        ptr::null()
    }
    #[cfg(feature = "v8")]
    pub fn JS_GetLatin1StringCharsAndLength<C>(
        _cx: C,
        _nogc: *const std::ffi::c_void,
        s: *mut JSString,
        len: *mut usize,
    ) -> *const u8 {
        crate::v8_glue::latin1_chars_and_len(s, len)
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_GetTwoByteStringCharsAndLength<C>(
        _cx: C,
        _nogc: *const std::ffi::c_void,
        _s: *mut JSString,
        _len: *mut usize,
    ) -> *const u16 {
        ptr::null()
    }
    #[cfg(feature = "v8")]
    pub fn JS_GetTwoByteStringCharsAndLength<C>(
        _cx: C,
        _nogc: *const std::ffi::c_void,
        s: *mut JSString,
        len: *mut usize,
    ) -> *const u16 {
        crate::v8_glue::two_byte_chars_and_len(s, len)
    }

    #[cfg(not(feature = "v8"))]
    pub fn JS_NewStringCopyN<C, S>(_cx: C, _s: S, _len: usize) -> *mut JSString {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn JS_NewStringCopyN<C, S: ToBytePtr>(_cx: C, s: S, len: usize) -> *mut JSString {
        crate::v8_glue::atomize_string_n(ptr::null_mut(), s.to_byte_ptr(), len)
    }
    pub fn CheckedUnwrapStatic(_obj: *mut JSObject) -> *mut JSObject {
        ptr::null_mut()
    }
    pub type Compartment = std::ffi::c_void;
    pub enum CompartmentSpecifier {
        NewCompartmentAndZone,
        NewCompartmentInSystemZone,
        NewCompartmentInExistingZone,
        ExistingCompartment,
    }
    pub enum CompartmentIterResult {
        KeepGoing,
        Stop,
    }
    #[repr(C)]
    pub union RealmCreationOptionsCompartment {
        pub comp_: *mut Compartment,
        pub zone_: *mut std::ffi::c_void,
    }
    #[repr(C)]
    pub struct RealmCreationOptions {
        pub traceGlobal_: Option<unsafe extern "C" fn(*mut JSTracer, *mut JSObject)>,
        pub compSpec_: CompartmentSpecifier,
        pub __bindgen_anon_1: RealmCreationOptionsCompartment,
        pub sharedMemoryAndAtomics_: bool,
    }
    #[repr(C)]
    pub struct RealmOptions {
        pub creationOptions_: RealmCreationOptions,
    }
    impl RealmOptions {
        pub fn new() -> Self {
            Self {
                creationOptions_: RealmCreationOptions {
                    traceGlobal_: None,
                    compSpec_: CompartmentSpecifier::NewCompartmentAndZone,
                    __bindgen_anon_1: RealmCreationOptionsCompartment {
                        zone_: ptr::null_mut(),
                    },
                    sharedMemoryAndAtomics_: false,
                },
            }
        }
    }
    impl Default for RealmOptions {
        fn default() -> Self {
            Self::new()
        }
    }
    impl std::ops::Deref for RealmOptions {
        type Target = RealmOptions;
        fn deref(&self) -> &RealmOptions {
            self
        }
    }
    impl std::ops::DerefMut for RealmOptions {
        fn deref_mut(&mut self) -> &mut RealmOptions {
            self
        }
    }
    #[cfg(not(feature = "v8"))]
    pub fn GetNonCCWObjectGlobal(_obj: *mut JSObject) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn GetNonCCWObjectGlobal(obj: *mut JSObject) -> *mut JSObject {
        crate::v8_glue::get_non_ccw_object_global(obj)
    }
    #[cfg(not(feature = "v8"))]
    pub fn GetRealmGlobalOrNull<C: ?Sized>(_cx: &C) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn GetRealmGlobalOrNull<C: ?Sized>(_cx: &C) -> *mut JSObject {
        crate::v8_glue::current_global_object()
    }
    pub fn IsSharableCompartment(_comp: *mut std::ffi::c_void) -> bool {
        false
    }
    pub fn IsSystemCompartment(_comp: *mut std::ffi::c_void) -> bool {
        false
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_GetFunctionObject<F>(_fun: F) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn JS_GetFunctionObject<F>(fun: F) -> *mut JSObject
    where
        F: ToFunctionObjectPtr,
    {
        fun.to_function_object_ptr()
    }
    pub fn JS_IterateCompartments<C>(
        _cx: *mut JSContext,
        _data: *mut std::ffi::c_void,
        _callback: C,
    ) {
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_NewFunction<N>(
        _cx: *mut JSContext,
        _call: Option<NativeCallback>,
        _nargs: u32,
        _flags: u16,
        _name: N,
    ) -> *mut JSFunction {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn JS_NewFunction<N>(
        cx: *mut JSContext,
        call: Option<NativeCallback>,
        nargs: u32,
        flags: u16,
        name: N,
    ) -> *mut JSFunction
    where
        N: ToBytePtr,
    {
        let name = crate::v8_glue::property_name_from_raw(name.to_byte_ptr());
        crate::v8_glue::js_new_function(cx, call, nargs, flags as u32, name)
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_NewGlobalObject<C, O>(
        _cx: C,
        _clasp: *const JSClass,
        _principal: *mut std::ffi::c_void,
        _hook: OnNewGlobalHookOption,
        _options: O,
    ) -> *mut JSObject {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn JS_NewGlobalObject<C, O>(
        _cx: C,
        clasp: *const JSClass,
        principal: *mut std::ffi::c_void,
        _hook: OnNewGlobalHookOption,
        _options: O,
    ) -> *mut JSObject {
        crate::v8_glue::create_dom_global(std::ptr::NonNull::dangling().as_ptr(), clasp, principal)
    }
    pub fn JS_SetTrustedPrincipals(_cx: *mut JSContext, _p: *mut std::ffi::c_void) -> bool {
        false
    }
    pub const JSFUN_CONSTRUCTOR: u16 = 0x01;
    #[repr(C)]
    pub struct ObjectOps {
        pub lookupProperty: Option<unsafe extern "C" fn()>,
        pub defineProperty: Option<unsafe extern "C" fn()>,
        pub hasProperty: Option<unsafe extern "C" fn()>,
        pub getProperty: Option<unsafe extern "C" fn()>,
        pub setProperty: Option<unsafe extern "C" fn()>,
        pub getOwnPropertyDescriptor: Option<unsafe extern "C" fn()>,
        pub deleteProperty: Option<unsafe extern "C" fn()>,
        pub getElements: Option<unsafe extern "C" fn()>,
        pub funToString:
            Option<unsafe extern "C" fn(*mut JSContext, HandleObject<'_>, bool) -> *mut JSString>,
    }
    pub static ProxyObjectOps: ObjectOps = ObjectOps {
        lookupProperty: None,
        defineProperty: None,
        hasProperty: None,
        getProperty: None,
        setProperty: None,
        getOwnPropertyDescriptor: None,
        deleteProperty: None,
        getElements: None,
        funToString: None,
    };
    pub enum OnNewGlobalHookOption {
        FireOnNewGlobalHook,
        DontFireOnNewGlobalHook,
    }
    pub const TrueHandleValue: *const JSVal = std::ptr::null();
    pub enum TraceKind {
        Object,
        String,
        Symbol,
        BigInt,
        Script,
        Shape,
        BaseShape,
        JitCode,
    }
    pub fn GCTraceKindToAscii(_kind: TraceKind) -> *const u8 {
        b"Object\0".as_ptr()
    }
    pub fn StringIsArrayIndex(_s: *mut JSString, _indexp: *mut u32) -> bool {
        false
    }
    pub type PropertyKey = jsid;
    #[cfg(not(feature = "v8"))]
    pub fn JS_IsExceptionPending(_cx: *mut JSContext) -> bool {
        false
    }
    #[cfg(feature = "v8")]
    pub fn JS_IsExceptionPending(_cx: *mut JSContext) -> bool {
        crate::v8_glue::is_exception_pending()
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_ClearPendingException(_cx: *mut JSContext) {}
    #[cfg(feature = "v8")]
    pub fn JS_ClearPendingException(_cx: *mut JSContext) {
        crate::v8_glue::clear_pending_exception();
    }
    pub fn JS_IsGlobalObject(_obj: *mut JSObject) -> bool {
        false
    }
    pub fn JS_MayResolveStandardClass<N, I, O>(_names: N, _id: I, _maybe_obj: O) -> bool {
        false
    }
    pub fn JS_NewEnumerateStandardClasses<C, O, P>(
        _cx: C,
        _obj: O,
        _props: P,
        _enum_op: bool,
    ) -> bool {
        false
    }
    pub fn JS_ResolveStandardClass<C, O, I, R>(_cx: C, _obj: O, _id: I, _resolved: R) -> bool {
        false
    }
    pub fn JS_DropPrincipals(_cx: *mut JSContext, _p: *mut std::ffi::c_void) {}
    pub fn JS_HoldPrincipals<P>(_p: P) {}
    #[cfg(not(feature = "v8"))]
    pub fn JS_DefinePropertyById<C, I, V, R>(
        _cx: C,
        _obj: *mut JSObject,
        _id: I,
        _val: V,
        _result: R,
    ) -> bool {
        false
    }
    #[cfg(feature = "v8")]
    pub fn JS_DefinePropertyById<C, I, V, R>(
        _cx: C,
        obj: *mut JSObject,
        id: I,
        val: V,
        result: R,
    ) -> bool
    where
        I: Into<jsid>,
        V: ToJsapiPropertyValue,
        R: SetObjectOpResultOut,
    {
        let ok =
            crate::v8_glue::set_property_by_jsid(obj, id.into(), val.to_jsapi_property_value());
        if ok {
            result.set_object_op_success();
        }
        ok
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_IdToValue(_cx: *mut JSContext, _id: jsid, _vp: *mut JSVal) -> bool {
        false
    }
    #[cfg(feature = "v8")]
    pub fn JS_IdToValue(_cx: *mut JSContext, id: jsid, vp: *mut JSVal) -> bool {
        crate::v8_glue::id_to_value(id, vp)
    }
    pub enum DOMProxyShadowsResult {
        Shadows,
        DoesntShadow,
        DoesntShadowUnique,
        ShadowsViaDirectExpando,
        ShadowsViaIndirectExpando,
        ShadowCheckFailed,
    }
    pub fn GetStaticPrototype(_obj: *mut JSObject) -> *mut JSObject {
        ptr::null_mut()
    }
    pub fn SetDOMProxyInformation<F, C>(
        _domProxyHandlerFamily: F,
        _callback: C,
        _class: *const std::ffi::c_void,
    ) {
    }
    pub fn HideScriptedCaller(_cx: *mut JSContext) {}
    pub fn UnhideScriptedCaller(_cx: *mut JSContext) {}
    pub struct MemoryUse;
    impl MemoryUse {
        pub const DOMBinding: u32 = 0;
    }
    pub type JSAtom = *mut std::ffi::c_void;
    pub type JSAtomState = *mut std::ffi::c_void;

    pub trait ToLinearStringPtr {
        fn to_linear_string_ptr(self) -> *mut JSString;
    }

    impl ToLinearStringPtr for *mut JSString {
        fn to_linear_string_ptr(self) -> *mut JSString {
            self
        }
    }

    impl ToLinearStringPtr for *mut std::ffi::c_void {
        fn to_linear_string_ptr(self) -> *mut JSString {
            self as *mut JSString
        }
    }

    impl ToLinearStringPtr for *mut *mut std::ffi::c_void {
        fn to_linear_string_ptr(self) -> *mut JSString {
            self as *mut JSString
        }
    }

    pub fn AtomToLinearString<A: ToLinearStringPtr>(atom: A) -> *mut JSString {
        atom.to_linear_string_ptr()
    }
    #[cfg(not(feature = "v8"))]
    pub fn GetLinearStringCharAt(_s: *mut JSString, _index: usize) -> u16 {
        0
    }
    #[cfg(feature = "v8")]
    pub fn GetLinearStringCharAt(s: *mut JSString, index: usize) -> u16 {
        crate::v8_glue::linear_string_char_at(s, index)
    }
    #[cfg(not(feature = "v8"))]
    pub fn GetLinearStringLength(_s: *mut JSString) -> usize {
        0
    }
    #[cfg(feature = "v8")]
    pub fn GetLinearStringLength(s: *mut JSString) -> usize {
        crate::v8_glue::string_len(s)
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_AtomizeStringN<S>(_cx: *mut JSContext, _s: S, _len: usize) -> *mut JSString {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn JS_AtomizeStringN<S: ToBytePtr>(cx: *mut JSContext, s: S, len: usize) -> *mut JSString {
        crate::v8_glue::atomize_string_n(cx, s.to_byte_ptr(), len)
    }
    pub enum ExceptionStackBehavior {
        Capture,
        DoNotCapture,
    }
    #[cfg(not(feature = "v8"))]
    pub fn GetCurrentRealmOrNull(_cx: *mut JSContext) -> *mut std::ffi::c_void {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn GetCurrentRealmOrNull(_cx: *mut JSContext) -> *mut std::ffi::c_void {
        crate::v8_glue::current_global_object() as *mut std::ffi::c_void
    }
    #[cfg(not(feature = "v8"))]
    pub fn JS_ValueToSource(_cx: *mut JSContext, _val: JSVal) -> *mut JSString {
        ptr::null_mut()
    }
    #[cfg(feature = "v8")]
    pub fn JS_ValueToSource(_cx: *mut JSContext, val: JSVal) -> *mut JSString {
        crate::v8_glue::value_to_source(val)
    }
    pub fn GetObjectProto<C, O>(_cx: C, _obj: O, _result: *mut *mut JSObject) -> bool {
        false
    }

    pub mod glue {
        pub use crate::rust::wrappers2::JS_GetOwnPropertyDescriptorById;
    }
}

// ── V8-backed runtime types ─────────────────────────────────────────────────

#[cfg(feature = "v8")]
pub struct V8Runtime {
    isolate: v8::OwnedIsolate,
}

#[cfg(feature = "v8")]
impl V8Runtime {
    pub fn new() -> Self {
        ensure_v8();
        let isolate = V8_ISOLATE.with(|cell| {
            let mut iso = cell.borrow_mut();
            if iso.is_none() {
                *iso = Some(v8::Isolate::new(v8::CreateParams::default()));
            }
            iso.take().unwrap()
        });
        Self { isolate }
    }
}

#[cfg(feature = "v8")]
pub struct V8Context {
    global: v8::Global<v8::Context>,
}

#[cfg(feature = "v8")]
impl V8Context {
    pub fn new(isolate: &mut v8::OwnedIsolate) -> Self {
        let scope = &mut v8::HandleScope::new(isolate);
        let context = v8::Context::new(scope);
        let global = v8::Global::new(scope, context);
        Self { global }
    }
}

#[cfg(feature = "v8")]
pub struct V8Object {
    handle: Option<v8::Global<v8::Object>>,
}

#[cfg(feature = "v8")]
impl V8Object {
    pub fn new(scope: &mut v8::HandleScope, obj: v8::Local<v8::Object>) -> Self {
        Self {
            handle: Some(v8::Global::new(scope, obj)),
        }
    }
}

pub struct V8String;
pub struct V8Value;
pub struct V8Tracer;

#[cfg(feature = "v8")]
pub struct JSAutoRealmDef {
    _cx: *mut V8Context,
    _saved: Option<v8::Global<v8::Context>>,
}

// ── Rust wrappers ───────────────────────────────────────────────────────────
pub mod rust {
    use std::ptr;

    pub mod wrappers {
        use super::super::jsapi;
        use std::ptr;

        pub trait ToPropertyValue {
            fn to_property_value(self) -> jsapi::JSVal;
        }

        impl ToPropertyValue for jsapi::JSVal {
            fn to_property_value(self) -> jsapi::JSVal {
                self
            }
        }

        impl ToPropertyValue for &jsapi::JSVal {
            fn to_property_value(self) -> jsapi::JSVal {
                *self
            }
        }

        impl ToPropertyValue for jsapi::HandleValue<'_> {
            fn to_property_value(self) -> jsapi::JSVal {
                self.get()
            }
        }

        impl ToPropertyValue for *mut jsapi::JSObject {
            fn to_property_value(self) -> jsapi::JSVal {
                jsapi::JSVal::from_object(self)
            }
        }

        impl ToPropertyValue for jsapi::HandleObject<'_> {
            fn to_property_value(self) -> jsapi::JSVal {
                jsapi::JSVal::from_object(self.get())
            }
        }

        impl ToPropertyValue for *mut jsapi::JSString {
            fn to_property_value(self) -> jsapi::JSVal {
                jsapi::JSVal::from_string(self)
            }
        }

        impl ToPropertyValue for jsapi::Handle<'_, *mut jsapi::JSString> {
            fn to_property_value(self) -> jsapi::JSVal {
                jsapi::JSVal::from_string(self.get())
            }
        }

        impl ToPropertyValue for i32 {
            fn to_property_value(self) -> jsapi::JSVal {
                jsapi::JSVal::from_int32(self)
            }
        }

        impl ToPropertyValue for u32 {
            fn to_property_value(self) -> jsapi::JSVal {
                jsapi::JSVal::from_uint32(self)
            }
        }

        pub trait SetPropertyOut {
            fn set_property_out(self, val: jsapi::JSVal);
        }

        pub trait SetBoolOut {
            fn set_bool_out(self, val: bool);
        }

        impl SetBoolOut for *mut bool {
            fn set_bool_out(self, val: bool) {
                if !self.is_null() {
                    // SAFETY: non-null JSAPI out-param checked above.
                    unsafe { *self = val };
                }
            }
        }

        impl SetPropertyOut for *mut jsapi::JSVal {
            fn set_property_out(self, val: jsapi::JSVal) {
                if !self.is_null() {
                    // SAFETY: non-null JSAPI out-param checked above.
                    unsafe { *self = val };
                }
            }
        }

        impl SetPropertyOut for jsapi::MutableHandleValue<'_> {
            fn set_property_out(mut self, val: jsapi::JSVal) {
                self.set(val);
            }
        }

        pub unsafe fn JS_GetClass(_obj: *mut jsapi::JSObject) -> *const jsapi::JSClass {
            ptr::null()
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_GetReservedSlot<O>(_obj: O, _index: u32, _out: *mut jsapi::JSVal) {}
        #[cfg(feature = "v8")]
        pub unsafe fn JS_GetReservedSlot<O>(obj: O, index: u32, out: *mut jsapi::JSVal)
        where
            O: Into<*mut jsapi::JSObject>,
        {
            if out.is_null() {
                return;
            }
            // SAFETY: caller supplied non-null out pointer for JSAPI out-param.
            unsafe {
                *out = crate::v8_glue::get_reserved_slot(obj.into(), index);
            }
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_SetReservedSlot(
            _obj: impl Into<*mut jsapi::JSObject>,
            _index: u32,
            _val: jsapi::JSVal,
        ) {
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_SetReservedSlot(
            obj: impl Into<*mut jsapi::JSObject>,
            index: u32,
            val: jsapi::JSVal,
        ) {
            crate::v8_glue::set_reserved_slot(obj.into(), index, val)
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_GetPrivate(_obj: *mut jsapi::JSObject) -> *mut std::ffi::c_void {
            ptr::null_mut()
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_GetPrivate(obj: *mut jsapi::JSObject) -> *mut std::ffi::c_void {
            crate::v8_glue::get_private(obj)
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_SetPrivate(_obj: *mut jsapi::JSObject, _data: *mut std::ffi::c_void) {}
        #[cfg(feature = "v8")]
        pub unsafe fn JS_SetPrivate(obj: *mut jsapi::JSObject, data: *mut std::ffi::c_void) {
            crate::v8_glue::set_private(obj, data)
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_GetPrototype<C, O, R>(_cx: &C, _obj: O, _result: R) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_GetPrototype<C, O, R>(_cx: &C, obj: O, result: R) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
            R: jsapi::SetJsapiObjectOut,
        {
            result.set_jsapi_object_out(crate::v8_glue::get_prototype(obj.into()));
            true
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_NewGlobalObject<C: ToJsContextPtr, O>(
            _cx: C,
            _clasp: *const jsapi::JSClass,
            _principal: *mut std::ffi::c_void,
            _hook: jsapi::OnNewGlobalHookOption,
            _options: O,
        ) -> *mut jsapi::JSObject {
            ptr::null_mut()
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_NewGlobalObject<C: ToJsContextPtr, O>(
            cx: C,
            clasp: *const jsapi::JSClass,
            principal: *mut std::ffi::c_void,
            _hook: jsapi::OnNewGlobalHookOption,
            _options: O,
        ) -> *mut jsapi::JSObject {
            crate::v8_glue::create_dom_global(
                cx.to_js_context_ptr(),
                clasp,
                principal,
            )
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_DefineProperty<C: ?Sized, O, N, V>(
            _cx: &C,
            _obj: O,
            _name: N,
            _val: V,
            _attrs: u32,
        ) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_DefineProperty<C: ?Sized, O, N, V>(
            _cx: &C,
            obj: O,
            name: N,
            val: V,
            _attrs: u32,
        ) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
            N: jsapi::ToBytePtr,
            V: ToPropertyValue,
        {
            crate::v8_glue::set_property_by_name(
                obj.into(),
                name.to_byte_ptr(),
                val.to_property_value(),
            )
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_GetProperty<C, O, N, V>(_cx: &C, _obj: O, _name: N, _vp: V) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_GetProperty<C, O, N, V>(_cx: &C, obj: O, name: N, vp: V) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
            N: jsapi::ToBytePtr,
            V: SetPropertyOut,
        {
            match crate::v8_glue::get_property_by_name(obj.into(), name.to_byte_ptr()) {
                Some(val) => {
                    vp.set_property_out(val);
                    true
                },
                None => {
                    vp.set_property_out(jsapi::JSVal::undefined());
                    true
                },
            }
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_SetProperty<C, O, N, V>(_cx: &C, _obj: O, _name: N, _vp: V) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_SetProperty<C, O, N, V>(_cx: &C, obj: O, name: N, vp: V) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
            N: jsapi::ToBytePtr,
            V: ToPropertyValue,
        {
            crate::v8_glue::set_property_by_name(
                obj.into(),
                name.to_byte_ptr(),
                vp.to_property_value(),
            )
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_NewPlainObject(_cx: *mut jsapi::JSContext) -> *mut jsapi::JSObject {
            ptr::null_mut()
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_NewPlainObject(_cx: *mut jsapi::JSContext) -> *mut jsapi::JSObject {
            crate::v8_glue::js_new_object()
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_NewFunction<N>(
            _cx: *mut jsapi::JSContext,
            _call: Option<jsapi::NativeCallback>,
            _nargs: u32,
            _flags: u16,
            _name: N,
        ) -> *mut jsapi::JSFunction {
            ptr::null_mut()
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_NewFunction<N>(
            cx: *mut jsapi::JSContext,
            call: Option<jsapi::NativeCallback>,
            nargs: u32,
            flags: u16,
            name: N,
        ) -> *mut jsapi::JSFunction
        where
            N: jsapi::ToBytePtr,
        {
            let name = crate::v8_glue::property_name_from_raw(name.to_byte_ptr());
            crate::v8_glue::js_new_function(cx, call, nargs, flags as u32, name)
        }

        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_GetFunctionObject(_fun: *mut jsapi::JSFunction) -> *mut jsapi::JSObject {
            ptr::null_mut()
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_GetFunctionObject(fun: *mut jsapi::JSFunction) -> *mut jsapi::JSObject {
            crate::v8_glue::get_function_object(fun)
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_LinkConstructorAndPrototype<C, P>(
            _cx: *mut jsapi::JSContext,
            _ctor: C,
            _proto: P,
        ) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_LinkConstructorAndPrototype<C, P>(
            _cx: *mut jsapi::JSContext,
            ctor: C,
            proto: P,
        ) -> bool
        where
            C: Into<*mut jsapi::JSObject>,
            P: Into<*mut jsapi::JSObject>,
        {
            crate::v8_glue::link_constructor_and_prototype(ctor.into(), proto.into())
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_NewStringCopyN(
            _cx: *mut jsapi::JSContext,
            _s: *const u8,
            _len: usize,
        ) -> *mut jsapi::JSString {
            ptr::null_mut()
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_NewStringCopyN(
            cx: *mut jsapi::JSContext,
            s: *const u8,
            len: usize,
        ) -> *mut jsapi::JSString {
            crate::v8_glue::atomize_string_n(cx, s, len)
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_GetTwoByteStringCharsAndLength(
            _cx: *mut jsapi::JSContext,
            _s: *mut jsapi::JSString,
            _len: *mut usize,
        ) -> *const u16 {
            ptr::null()
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_GetTwoByteStringCharsAndLength(
            _cx: *mut jsapi::JSContext,
            s: *mut jsapi::JSString,
            len: *mut usize,
        ) -> *const u16 {
            crate::v8_glue::two_byte_chars_and_len(s, len)
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_AtomizeStringN<S>(
            _cx: *mut jsapi::JSContext,
            _s: S,
            _len: usize,
        ) -> *mut jsapi::JSString {
            ptr::null_mut()
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_AtomizeStringN<S: jsapi::ToBytePtr>(
            cx: *mut jsapi::JSContext,
            s: S,
            len: usize,
        ) -> *mut jsapi::JSString {
            crate::v8_glue::atomize_string_n(cx, s.to_byte_ptr(), len)
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn Call<C, T, F, A, R>(_cx: &C, _this: T, _fun: F, _args: A, _rval: R) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn Call<C, T, F, A, R>(_cx: &C, _this: T, fun: F, args: A, rval: R) -> bool
        where
            F: jsapi::ToFunctionObjectPtr,
            A: jsapi::ToCallArgs,
            R: Into<*mut jsapi::JSVal>,
        {
            crate::v8_glue::call_function(
                ptr::null_mut(),
                jsapi::JSVal::from_object(fun.to_function_object_ptr()),
                args.to_call_args(),
                rval.into(),
            )
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn AppendToIdVector<V, I>(_v: V, _id: I) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn AppendToIdVector<V, I>(v: V, id: I) -> bool
        where
            V: Into<jsapi::MutableHandleIdVector>,
            I: Into<jsapi::jsid>,
        {
            crate::v8_glue::append_to_id_vector(v.into(), id.into())
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn GetPropertyKeys<C, O, I>(_cx: &C, _obj: O, _flags: u32, _ids: I) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn GetPropertyKeys<C, O, I>(_cx: &C, obj: O, _flags: u32, ids: I) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
            I: Into<jsapi::MutableHandleIdVector>,
        {
            crate::v8_glue::get_property_keys(obj.into(), ids.into())
        }
        pub unsafe fn JS_CopyOwnPropertiesAndPrivateFields<C: ?Sized, T, O>(
            _cx: &C,
            _target: T,
            _obj: O,
        ) -> bool {
            true
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_DefinePropertyById2<C: ?Sized, O, I, V, A>(
            _cx: &C,
            _obj: O,
            _id: I,
            _val: V,
            _attrs: A,
        ) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_DefinePropertyById2<C: ?Sized, O, I, V, A>(
            _cx: &C,
            obj: O,
            id: I,
            val: V,
            _attrs: A,
        ) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
            I: Into<jsapi::jsid>,
            V: ToPropertyValue,
        {
            crate::v8_glue::set_property_by_jsid(obj.into(), id.into(), val.to_property_value())
        }
        pub unsafe fn JS_InitializePropertiesFromCompatibleNativeObject<C: ?Sized, D, S>(
            _cx: &C,
            _dst: D,
            _src: S,
        ) -> bool {
            true
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_NewObjectWithGivenProto<C: ?Sized, P>(
            _cx: &C,
            _clasp: *const jsapi::JSClass,
            _proto: P,
        ) -> *mut jsapi::JSObject {
            ptr::null_mut()
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_NewObjectWithGivenProto<C: ?Sized, P>(
            _cx: &C,
            clasp: *const jsapi::JSClass,
            proto: P,
        ) -> *mut jsapi::JSObject
        where
            P: Into<*mut jsapi::JSObject>,
        {
            crate::v8_glue::js_new_object_with_proto(clasp, proto.into())
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_NewObjectWithoutMetadata<C: ?Sized, P>(
            _cx: &C,
            _clasp: *const jsapi::JSClass,
            _proto: P,
        ) -> *mut jsapi::JSObject {
            ptr::null_mut()
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_NewObjectWithoutMetadata<C: ?Sized, P>(
            _cx: &C,
            clasp: *const jsapi::JSClass,
            proto: P,
        ) -> *mut jsapi::JSObject
        where
            P: Into<*mut jsapi::JSObject>,
        {
            crate::v8_glue::js_new_object_with_proto(clasp, proto.into())
        }
        pub unsafe fn JS_SetImmutablePrototype<C: ?Sized, O>(
            _cx: &C,
            _obj: O,
            succeeded: *mut bool,
        ) -> bool {
            if !succeeded.is_null() {
                *succeeded = true;
            }
            true
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_SetPrototype<C, O, P>(_cx: &C, _obj: O, _proto: P) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_SetPrototype<C, O, P>(_cx: &C, obj: O, proto: P) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
            P: Into<*mut jsapi::JSObject>,
        {
            !crate::v8_glue::set_prototype(obj.into(), proto.into()).is_null()
        }
        pub unsafe fn JS_WrapObject<C, O>(_cx: &C, _obj: O) -> bool {
            false
        }
        pub trait ToJsContextPtr {
            fn to_js_context_ptr(&self) -> *mut jsapi::JSContext;
        }
        impl ToJsContextPtr for *mut jsapi::JSContext {
            fn to_js_context_ptr(&self) -> *mut jsapi::JSContext {
                *self
            }
        }
        impl ToJsContextPtr for crate::context::JSContext {
            fn to_js_context_ptr(&self) -> *mut jsapi::JSContext {
                unsafe { self.raw_cx_no_gc() }
            }
        }
        pub unsafe fn NewProxyObject<C: ?Sized, Pr, P>(
            cx: &C,
            handler: *const std::ffi::c_void,
            _priv: Pr,
            proto: P,
            _options: *const std::ffi::c_void,
            flag: bool,
        ) -> *mut jsapi::JSObject
        where
            C: ToJsContextPtr,
            P: Into<*mut jsapi::JSObject>,
        {
            crate::v8_glue::new_proxy_object(cx.to_js_context_ptr(), handler, proto.into(), flag)
        }
        pub fn RUST_INTERNED_STRING_TO_JSID<C, S, O>(_cx: &C, _s: S, _out: O) -> jsapi::jsid {
            jsapi::jsid(0)
        }
        #[cfg(not(feature = "v8"))]
        pub fn RUST_SYMBOL_TO_JSID(
            _sym: jsapi::JSVal,
            _out: jsapi::MutableHandleId<'_>,
        ) -> jsapi::jsid {
            jsapi::jsid(0)
        }
        #[cfg(feature = "v8")]
        pub fn RUST_SYMBOL_TO_JSID(
            _sym: jsapi::JSVal,
            mut out: jsapi::MutableHandleId<'_>,
        ) -> jsapi::jsid {
            let name = b"@@unscopables";
            let string = crate::v8_glue::atomize_string_n(
                std::ptr::null_mut(),
                name.as_ptr(),
                name.len(),
            );
            let id = jsapi::jsid::from_string(string);
            out.set(id);
            id
        }
        pub fn int_to_jsid<O>(i: i32, _out: O) -> jsapi::jsid {
            jsapi::jsid::from_int(i)
        }

        pub type Handle<'a, T> = jsapi::Handle<'a, T>;
        pub type HandleObject<'a> = jsapi::Handle<'a, *mut jsapi::JSObject>;
        pub type HandleValue<'a> = jsapi::Handle<'a, jsapi::JSVal>;
        pub type MutableHandle<'a, T> = jsapi::MutableHandle<'a, T>;
        pub type MutableHandleObject<'a> = jsapi::MutableHandle<'a, *mut jsapi::JSObject>;

        pub trait IntoHandle {
            type Target;
            fn into_handle(self) -> Self::Target;
        }
        impl<T> IntoHandle for *const T {
            type Target = *const T;
            fn into_handle(self) -> *const T {
                self
            }
        }

        #[cfg(not(feature = "v8"))]
        pub fn IsArrayObject<C, V>(_cx: &C, _val: V, _out: *mut bool) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub fn IsArrayObject<C, V>(_cx: &C, val: V, out: *mut bool) -> bool
        where
            V: jsapi::ToFunctionObjectPtr,
        {
            if !out.is_null() {
                // SAFETY: non-null JSAPI out-param checked above.
                unsafe { *out = crate::v8_glue::is_array_object(val.to_function_object_ptr()) };
            }
            true
        }
        pub fn JS_DefineProperty3<C: ?Sized, O, N, V>(
            _cx: &C,
            obj: O,
            name: N,
            val: V,
            _attrs: u32,
        ) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
            N: jsapi::ToBytePtr,
            V: ToPropertyValue,
        {
            crate::v8_glue::set_property_by_name(
                obj.into(),
                name.to_byte_ptr(),
                val.to_property_value(),
            )
        }
        pub fn JS_DefineProperty4<C: ?Sized, O, N, V>(
            _cx: &C,
            obj: O,
            name: N,
            val: V,
            _attrs: u32,
        ) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
            N: jsapi::ToBytePtr,
            V: ToPropertyValue,
        {
            crate::v8_glue::set_property_by_name(
                obj.into(),
                name.to_byte_ptr(),
                val.to_property_value(),
            )
        }
        pub fn JS_DefineProperty5<C: ?Sized, O, N, V>(
            _cx: &C,
            obj: O,
            name: N,
            val: V,
            _attrs: u32,
        ) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
            N: jsapi::ToBytePtr,
            V: ToPropertyValue,
        {
            crate::v8_glue::set_property_by_name(
                obj.into(),
                name.to_byte_ptr(),
                val.to_property_value(),
            )
        }
        #[cfg(not(feature = "v8"))]
        pub fn JS_DefinePropertyById5<C: ?Sized, O, I, V>(
            _cx: &C,
            _obj: O,
            _id: I,
            _val: V,
            _attrs: u32,
        ) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub fn JS_DefinePropertyById5<C: ?Sized, O, I, V>(
            _cx: &C,
            obj: O,
            id: I,
            val: V,
            _attrs: u32,
        ) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
            I: Into<jsapi::jsid>,
            V: ToPropertyValue,
        {
            crate::v8_glue::set_property_by_jsid(obj.into(), id.into(), val.to_property_value())
        }
        pub fn JS_FireOnNewGlobalObject<O>(_cx: *mut jsapi::JSContext, _obj: O) {}
        #[cfg(not(feature = "v8"))]
        pub fn JS_AlreadyHasOwnPropertyById<C: ?Sized, O, I>(
            _cx: &C,
            _obj: O,
            _id: I,
            _found: *mut bool,
        ) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub fn JS_AlreadyHasOwnPropertyById<C: ?Sized, O, I>(
            _cx: &C,
            obj: O,
            id: I,
            found: *mut bool,
        ) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
            I: Into<jsapi::jsid>,
        {
            found.set_bool_out(crate::v8_glue::has_property_by_jsid(obj.into(), id.into()));
            true
        }
        pub fn SetDataPropertyDescriptor<D, V>(_desc: D, _value: V, _attrs: u32) {}
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_GetPropertyById<C, O, I, V>(_cx: &C, _obj: O, _id: I, _vp: V) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_GetPropertyById<C, O, I, V>(_cx: &C, obj: O, id: I, vp: V) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
            I: Into<jsapi::jsid>,
            V: SetPropertyOut,
        {
            match crate::v8_glue::get_property_by_jsid(obj.into(), id.into()) {
                Some(val) => {
                    vp.set_property_out(val);
                    true
                },
                None => {
                    vp.set_property_out(jsapi::JSVal::undefined());
                    true
                },
            }
        }
        pub unsafe fn JS_HasProperty<C: ?Sized, O, N>(
            _cx: &C,
            _obj: O,
            _name: N,
            _found: *mut bool,
        ) -> bool {
            false
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_HasPropertyById<C: ?Sized, O, I>(
            _cx: &C,
            _obj: O,
            _id: I,
            _found: *mut bool,
        ) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_HasPropertyById<C, O, I>(_cx: &C, obj: O, id: I, found: *mut bool) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
            I: Into<jsapi::jsid>,
        {
            found.set_bool_out(crate::v8_glue::has_property_by_jsid(obj.into(), id.into()));
            true
        }
        pub unsafe fn JS_HasOwnProperty<C: ?Sized, O, N>(
            _cx: &C,
            _obj: O,
            _name: N,
            _found: *mut bool,
        ) -> bool {
            false
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_ForwardGetPropertyTo<C: ?Sized, O, I, R, V>(
            _cx: &C,
            _obj: O,
            _id: I,
            _receiver: R,
            _vp: V,
        ) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_ForwardGetPropertyTo<C: ?Sized, O, I, R, V>(
            _cx: &C,
            obj: O,
            id: I,
            _receiver: R,
            vp: V,
        ) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
            I: Into<jsapi::jsid>,
            V: SetPropertyOut,
        {
            match crate::v8_glue::get_property_by_jsid(obj.into(), id.into()) {
                Some(val) => {
                    vp.set_property_out(val);
                    true
                },
                None => false,
            }
        }
        pub unsafe fn JS_DeletePropertyById<C: ?Sized, O, I, R>(
            _cx: &C,
            _obj: O,
            _id: I,
            _result: R,
        ) -> bool {
            false
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_GetPendingException<C, V>(_cx: &C, _vp: V) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_GetPendingException<C, V>(_cx: &C, vp: V) -> bool
        where
            V: SetPropertyOut,
        {
            if let Some(val) = crate::v8_glue::get_pending_exception() {
                vp.set_property_out(val);
                true
            } else {
                false
            }
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_SetPendingException<C, V, B>(_cx: &C, _val: V, _behavior: B) {}
        #[cfg(feature = "v8")]
        pub unsafe fn JS_SetPendingException<C, V, B>(_cx: &C, val: V, _behavior: B)
        where
            V: Into<jsapi::JSVal>,
        {
            crate::v8_glue::set_pending_exception(val.into());
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_IdToValue<C, I, V>(_cx: &C, _id: I, _vp: V) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_IdToValue<C, I, V>(_cx: &C, id: I, vp: V) -> bool
        where
            I: Into<jsapi::jsid>,
            V: jsapi::SetJsapiValOut,
        {
            crate::v8_glue::id_to_value(id.into(), vp)
        }
        pub unsafe fn CallOriginalPromiseReject<C, V>(_cx: &C, _value: V) -> *mut jsapi::JSObject {
            ptr::null_mut()
        }
        pub unsafe fn JS_DefineUCProperty2<C: ?Sized, O, N, V>(
            _cx: &C,
            _obj: O,
            _name: N,
            _namelen: usize,
            _val: V,
            _attrs: u32,
        ) -> bool {
            false
        }
        pub unsafe fn ToJSON<C: ?Sized, V, O, T, W, D>(
            _cx: &C,
            _val: V,
            _obj: O,
            _replacer: T,
            _callback: W,
            _data: D,
        ) -> bool {
            false
        }
        pub unsafe fn JS_GetOwnPropertyDescriptorById<C: ?Sized, O, I, D>(
            _cx: &C,
            _obj: O,
            _id: I,
            _desc: D,
            _found: *mut bool,
        ) -> bool {
            false
        }
        pub unsafe fn AddPromiseReactions<C: ?Sized, P, R, J>(
            _cx: &C,
            _promise: P,
            _resolve: R,
            _reject: J,
        ) -> bool {
            false
        }
        pub unsafe fn CallOriginalPromiseResolve<C, V>(_cx: &C, _value: V) -> *mut jsapi::JSObject {
            ptr::null_mut()
        }
        pub unsafe fn CheckRegExpSyntax<C: ?Sized, S, L, R>(
            _cx: &C,
            _source: S,
            _len: L,
            _flags: crate::jsapi::RegExpFlags,
            _result: R,
        ) -> bool {
            false
        }
        pub unsafe fn Construct1<C, F, A, R>(_cx: &C, _fun: F, _args: A, _rval: R) -> bool {
            false
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn DetachArrayBuffer<C, O>(_cx: &C, _obj: O) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn DetachArrayBuffer<C, O>(_cx: &C, obj: O) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
        {
            crate::v8_glue::detach_array_buffer(obj.into())
        }
        pub unsafe fn ExecuteRegExpNoStatics<C: ?Sized, R, S, I, B, V>(
            _cx: &C,
            _regexp: R,
            _input: S,
            _len: usize,
            _index: I,
            _sticky: B,
            _rval: V,
        ) -> bool {
            false
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn GetArrayLength<C, O>(_cx: &C, _obj: O, _len: *mut u32) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn GetArrayLength<C, O>(_cx: &C, obj: O, len: *mut u32) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
        {
            if !len.is_null() {
                // SAFETY: non-null JSAPI out-param checked above.
                unsafe { *len = crate::v8_glue::array_length(obj.into()) };
            }
            true
        }
        pub unsafe fn GetBuiltinClass<C, O>(_cx: &C, obj: O, class: *mut jsapi::ESClass) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
        {
            if !class.is_null() {
                let obj = obj.into();
                // SAFETY: non-null JSAPI out-param checked above.
                unsafe {
                    *class = if crate::v8_glue::is_array_object(obj) {
                        jsapi::ESClass::Array
                    } else {
                        jsapi::ESClass::Object
                    };
                }
            }
            true
        }
        pub unsafe fn GetPromiseIsHandled<O>(_obj: O) -> bool {
            false
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn GetPromiseState<O>(_obj: O) -> jsapi::PromiseState {
            jsapi::PromiseState::Pending
        }
        #[cfg(feature = "v8")]
        pub unsafe fn GetPromiseState<O>(obj: O) -> jsapi::PromiseState
        where
            O: Into<*mut jsapi::JSObject>,
        {
            crate::v8_glue::get_promise_state(obj.into())
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn IsPromiseObject<O>(_obj: O) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn IsPromiseObject<O>(obj: O) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
        {
            crate::v8_glue::is_promise_object(obj.into())
        }
        pub unsafe fn JS_CallFunctionName<C: ?Sized, O, N, A, R>(
            _cx: &C,
            _obj: O,
            _name: N,
            _args: A,
            _rval: R,
        ) -> bool {
            false
        }
        pub unsafe fn JS_ErrorFromException<C, V>(_cx: &C, _value: V) -> *mut jsapi::JSErrorReport {
            ptr::null_mut()
        }
        pub unsafe fn JS_GetPromiseResult<O, R>(_obj: O, _rval: R) {}
        pub unsafe fn JS_ParseJSON<C, S, V>(_cx: &C, _chars: S, _len: u32, _vp: V) -> bool {
            false
        }
        pub unsafe fn JS_ReadStructuredClone<C: ?Sized, D, Ver, S, V, CB, Cl, P>(
            _cx: &C,
            _data: D,
            _version: Ver,
            _scope: S,
            _vp: V,
            _callbacks: CB,
            _closure: Cl,
            _policy: P,
        ) -> bool {
            false
        }
        pub unsafe fn JS_Stringify<C: ?Sized, V, R, S, W, D>(
            _cx: &C,
            _value: V,
            _replacer: R,
            _space: S,
            _callback: W,
            _data: D,
        ) -> bool {
            false
        }
        pub unsafe fn JS_TransplantObject<C: ?Sized, O, T>(
            _cx: &C,
            _orig: O,
            _target: T,
        ) -> *mut jsapi::JSObject {
            ptr::null_mut()
        }
        pub unsafe fn JS_TypeOfValue<C, V>(_cx: &C, _value: V) -> jsapi::JSType {
            jsapi::JSType::JSTYPE_OBJECT
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_ValueToSource<C, V>(_cx: &C, _value: V) -> *mut jsapi::JSString {
            ptr::null_mut()
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_ValueToSource<C, V>(_cx: &C, value: V) -> *mut jsapi::JSString
        where
            V: Into<jsapi::JSVal>,
        {
            crate::v8_glue::value_to_source(value.into())
        }
        pub unsafe fn JS_WriteStructuredClone<C: ?Sized, V, D, S, P, CB, Cl, R>(
            _cx: &C,
            _value: V,
            _data: D,
            _scope: S,
            _policy: P,
            _callbacks: CB,
            _closure: Cl,
            _transfer: R,
        ) -> bool {
            false
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn NewPromiseObject<C, H>(_cx: &C, _executor: H) -> *mut jsapi::JSObject {
            ptr::null_mut()
        }
        #[cfg(feature = "v8")]
        pub unsafe fn NewPromiseObject<C, H>(_cx: &C, executor: H) -> *mut jsapi::JSObject
        where
            H: Into<*mut jsapi::JSObject>,
        {
            crate::v8_glue::new_promise_object(executor.into())
        }
        pub unsafe fn NewWindowProxy<C: ?Sized, H, P>(
            cx: &C,
            window: H,
            handler: P,
        ) -> *mut jsapi::JSObject
        where
            C: ToJsContextPtr,
            H: Into<*mut jsapi::JSObject>,
            P: crate::glue::ToWrapperProxyHandlerPtr,
        {
            crate::v8_glue::new_proxy_object(
                cx.to_js_context_ptr(),
                handler.to_wrapper_proxy_handler_ptr(),
                window.into(),
                false,
            )
        }
        pub unsafe fn ObjectIsRegExp<C, O>(_cx: &C, _obj: O, out: *mut bool) -> bool {
            if !out.is_null() {
                *out = false;
            }
            true
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn RejectPromise<C, P, V>(_cx: &C, _promise: P, _value: V) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn RejectPromise<C, P, V>(_cx: &C, promise: P, _value: V) -> bool
        where
            P: Into<*mut jsapi::JSObject>,
        {
            crate::v8_glue::set_promise_state(promise.into(), jsapi::PromiseState::Rejected)
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn ResolvePromise<C, P, V>(_cx: &C, _promise: P, _value: V) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn ResolvePromise<C, P, V>(_cx: &C, promise: P, _value: V) -> bool
        where
            P: Into<*mut jsapi::JSObject>,
        {
            crate::v8_glue::set_promise_state(promise.into(), jsapi::PromiseState::Fulfilled)
        }
        pub unsafe fn SameValue<C, A, B>(_cx: &C, _a: A, _b: B, _same: *mut bool) -> bool {
            false
        }
        pub unsafe fn SetAnyPromiseIsHandled<C, O>(_cx: &C, _obj: O) -> bool {
            true
        }
        pub unsafe fn SetPromiseUserInputEventHandlingState<O, S>(_obj: O, _state: S) {}
        #[cfg(not(feature = "v8"))]
        pub unsafe fn SetWindowProxy<C, O, W>(_cx: &C, _obj: O, _window_proxy: W) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn SetWindowProxy<C, O, W>(_cx: &C, obj: O, window_proxy: W) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
            W: Into<*mut jsapi::JSObject>,
        {
            crate::v8_glue::set_window_proxy(obj.into(), window_proxy.into())
        }
    }

    pub struct Runtime;
    impl Runtime {
        pub fn new(_engine: JSEngineHandle) -> Self {
            Self
        }
        pub unsafe fn create_with_parent(_parent: ParentRuntime) -> Self {
            Self
        }
        pub fn prepare_for_new_child(&self) -> ParentRuntime {
            ParentRuntime
        }
        pub fn get() -> Option<std::ptr::NonNull<super::jsapi::JSContext>> {
            Some(std::ptr::NonNull::dangling())
        }
        pub fn cx(&self) -> crate::context::JSContext {
            unsafe { crate::context::JSContext::from_ptr(std::ptr::NonNull::dangling()) }
        }
        pub fn cx_no_gc(&self) -> crate::context::JSContext {
            unsafe { crate::context::JSContext::from_ptr(std::ptr::NonNull::dangling()) }
        }
        pub fn rt(&self) -> *mut super::jsapi::JSRuntime {
            ptr::null_mut()
        }
        pub fn thread_safe_js_context(&self) -> ThreadSafeJSContext {
            ThreadSafeJSContext
        }
    }

    pub mod conversions {
        use super::super::conversions::{ConversionBehavior, ConversionResult};
        use super::super::jsapi;

        pub trait ToJSValConvertible {
            unsafe fn to_jsval(
                &self,
                _cx: *mut jsapi::JSContext,
                _rval: super::MutableHandleValue,
            ) {
            }

            fn safe_to_jsval(
                &self,
                cx: &mut crate::context::JSContext,
                rval: super::MutableHandleValue<'_>,
            ) {
                unsafe {
                    self.to_jsval(cx.raw_cx(), rval);
                }
            }
        }
        pub trait FromJSValConvertible: Sized {
            type Config;
            unsafe fn from_jsval(
                _cx: *mut jsapi::JSContext,
                _val: super::HandleValue,
                _option: Self::Config,
            ) -> Result<ConversionResult<Self>, ()>;
            fn safe_from_jsval(
                cx: &mut crate::context::JSContext,
                val: super::HandleValue,
                option: Self::Config,
            ) -> Result<ConversionResult<Self>, ()> {
                unsafe { Self::from_jsval(cx.raw_cx(), val, option) }
            }
        }
    }

    pub type Handle<'a, T> = super::jsapi::Handle<'a, T>;
    pub type HandleObject<'a> = super::jsapi::Handle<'a, *mut super::jsapi::JSObject>;
    pub type HandleValue<'a> = super::jsapi::Handle<'a, super::jsapi::JSVal>;
    pub type MutableHandle<'a, T> = super::jsapi::MutableHandle<'a, T>;
    pub type MutableHandleObject<'a> = super::jsapi::MutableHandle<'a, *mut super::jsapi::JSObject>;
    pub type MutableHandleValue<'a> = super::jsapi::MutableHandle<'a, super::jsapi::JSVal>;
    pub struct IdVector(Vec<super::jsapi::jsid>);
    impl IdVector {
        pub unsafe fn new<C: ?Sized>(_cx: &C) -> Self {
            Self(Vec::new())
        }
        pub fn handle_mut(&mut self) -> super::jsapi::MutableHandleIdVector {
            &mut self.0
        }
    }
    impl std::ops::Deref for IdVector {
        type Target = Vec<super::jsapi::jsid>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    pub type HandleId<'a> = super::jsapi::Handle<'a, super::jsapi::jsid>;
    pub fn is_dom_class(clasp: *const super::jsapi::JSClass) -> bool {
        !clasp.is_null()
            && unsafe { (*clasp).flags & super::jsapi::JSCLASS_IS_DOMJSCLASS != 0 }
    }
    pub fn is_dom_object(obj: *mut super::jsapi::JSObject) -> bool {
        is_dom_class(get_object_class(obj))
    }
    pub fn maybe_wrap_value<C, V>(_cx: &C, _vp: V) -> bool {
        false
    }
    pub fn maybe_wrap_object<C, O>(_cx: &C, _obj: O) -> bool {
        false
    }
    pub type RealmOptions = super::jsapi::RealmOptions;
    #[cfg(not(feature = "v8"))]
    pub fn define_methods<C: ?Sized, O>(
        _cx: &C,
        _obj: O,
        _methods: &[super::jsapi::JSFunctionSpec],
    ) -> Result<(), ()> {
        Ok(())
    }
    #[cfg(feature = "v8")]
    pub fn define_methods<C: ?Sized, O>(
        _cx: &C,
        obj: O,
        methods: &[super::jsapi::JSFunctionSpec],
    ) -> Result<(), ()>
    where
        O: Into<*mut super::jsapi::JSObject> + Copy,
    {
        for method in methods {
            // SAFETY: Servo's JSFunctionSpec method arrays use the string_ name variant.
            let name = unsafe { method.name.string_ } as *const u8;
            if name.is_null() {
                break;
            }
            let Some(name_string) = crate::v8_glue::property_name_from_raw(name) else {
                continue;
            };
            let fun = crate::v8_glue::js_new_function(
                ptr::null_mut(),
                method.call.op,
                method.nargs as u32,
                method.flags as u32,
                Some(name_string.clone()),
            );
            let fun_obj = crate::v8_glue::get_function_object(fun);
            if fun_obj.is_null()
                || !crate::v8_glue::set_property(
                    obj.into(),
                    &name_string,
                    super::jsapi::JSVal::from_object(fun_obj),
                )
            {
                return Err(());
            }
        }
        Ok(())
    }
    pub fn define_properties<C: ?Sized, O>(
        _cx: &C,
        _obj: O,
        _props: &[super::jsapi::JSPropertySpec],
    ) -> Result<(), ()> {
        Ok(())
    }

    pub unsafe trait CustomTrace {
        fn trace(&self, _trc: *mut super::jsapi::JSTracer) {}
    }
    unsafe impl<T: super::gc::Traceable> CustomTrace for T {
        fn trace(&self, trc: *mut super::jsapi::JSTracer) {
            unsafe {
                super::gc::Traceable::trace(self, trc);
            }
        }
    }

    pub fn typedarray_err_dummy() -> &'static mut () {
        static INIT: std::sync::Once = std::sync::Once::new();
        static mut PTR: *mut () = std::ptr::null_mut();
        INIT.call_once(|| {
            // SAFETY: runs exactly once during initialization.
            unsafe {
                PTR = Box::into_raw(Box::new(()));
            }
        });
        // SAFETY: initialized exactly once above; only used as an error sentinel.
        unsafe { &mut *PTR }
    }

    pub struct CustomAutoRooter<T> {
        data: T,
    }
    impl<T: CustomTrace> CustomAutoRooter<T> {
        pub fn new(data: T) -> Self {
            Self { data }
        }
        pub fn root<'a, C: ?Sized>(&'a mut self, cx: &C) -> CustomAutoRooterGuard<'a, T> {
            CustomAutoRooterGuard::new(cx, self)
        }
    }

    pub struct CustomAutoRooterGuard<'a, T: 'a + CustomTrace = ()> {
        rooter: Option<&'a mut CustomAutoRooter<T>>,
        owned: Option<T>,
    }
    impl<'a, T: CustomTrace> CustomAutoRooterGuard<'a, T> {
        pub fn new<C>(_cx: C, rooter: &'a mut CustomAutoRooter<T>) -> Self {
            Self {
                rooter: Some(rooter),
                owned: None,
            }
        }
        pub fn from_value(value: T) -> CustomAutoRooterGuard<'static, T> {
            CustomAutoRooterGuard {
                rooter: None,
                owned: Some(value),
            }
        }
        pub fn is_shared(&self) -> bool {
            false
        }
    }
    impl<'a, T> CustomAutoRooterGuard<'a, T>
    where
        T: super::typedarray::TypedArrayElement + CustomTrace,
    {
        pub fn underlying_object(&self) -> super::jsapi::Heap<*mut super::jsapi::JSObject> {
            super::jsapi::Heap::new(ptr::null_mut())
        }
        pub fn to_vec(&self) -> Vec<T::Element> {
            Vec::new()
        }
        pub fn as_slice(&self) -> &[T::Element] {
            &[]
        }
    }
    impl<'a, T> CustomAutoRooterGuard<'a, super::typedarray::TypedArray<T, *mut super::jsapi::JSObject>>
    where
        T: super::typedarray::TypedArrayElement + CustomTrace,
    {
        pub fn underlying_object(&self) -> &*mut super::jsapi::JSObject {
            std::ops::Deref::deref(self).object_ref()
        }
        pub fn to_vec(&self) -> Vec<T::Element> {
            std::ops::Deref::deref(self).to_vec()
        }
        pub fn as_slice(&self) -> &[T::Element] {
            std::ops::Deref::deref(self).as_slice()
        }
        pub fn as_mut_slice(&mut self) -> &mut [T::Element] {
            std::ops::DerefMut::deref_mut(self).as_mut_slice()
        }
    }
    impl<T: CustomTrace> From<T> for CustomAutoRooterGuard<'static, T> {
        fn from(value: T) -> Self {
            Self::from_value(value)
        }
    }
    impl<'a, T: CustomTrace> std::ops::Deref for CustomAutoRooterGuard<'a, T> {
        type Target = T;
        fn deref(&self) -> &T {
            match (&self.rooter, &self.owned) {
                (Some(rooter), None) => &rooter.data,
                (None, Some(value)) => value,
                _ => unreachable!("CustomAutoRooterGuard must be rooted or owned"),
            }
        }
    }
    impl<'a, T: CustomTrace> std::ops::DerefMut for CustomAutoRooterGuard<'a, T> {
        fn deref_mut(&mut self) -> &mut T {
            match (&mut self.rooter, &mut self.owned) {
                (Some(rooter), None) => &mut rooter.data,
                (None, Some(value)) => value,
                _ => unreachable!("CustomAutoRooterGuard must be rooted or owned"),
            }
        }
    }
    pub trait GCMethods {
        fn initial() -> Self
        where
            Self: Sized,
        {
            unimplemented!()
        }
        unsafe fn post_barrier(_v: *mut Self, _prev: Self, _next: Self)
        where
            Self: Sized,
        {
        }
    }
    impl<T> GCMethods for *mut T {
        fn initial() -> Self {
            ptr::null_mut()
        }
    }
    impl<T> GCMethods for *const T {
        fn initial() -> Self {
            ptr::null()
        }
    }
    impl GCMethods for super::jsapi::JSVal {
        fn initial() -> Self {
            super::jsapi::JSVal::default()
        }
    }
    pub fn get_context_realm<C: ?Sized>(_cx: &C) -> *mut super::jsapi::JSObject {
        ptr::null_mut()
    }
    #[cfg(not(feature = "v8"))]
    pub fn get_object_class(_obj: *mut super::jsapi::JSObject) -> *const super::jsapi::JSClass {
        ptr::null()
    }
    #[cfg(feature = "v8")]
    pub fn get_object_class(obj: *mut super::jsapi::JSObject) -> *const super::jsapi::JSClass {
        crate::v8_glue::get_object_class(obj)
    }
    pub fn get_object_realm(_obj: *mut super::jsapi::JSObject) -> *mut super::jsapi::JSObject {
        ptr::null_mut()
    }
    pub struct CapturedJSStack;
    impl CapturedJSStack {
        pub unsafe fn new<C, H>(_cx: &C, _stack: H, _max_frame_count: Option<u32>) -> Option<Self> {
            Some(Self)
        }
        pub fn for_each_stack_frame<F>(&self, _callback: F)
        where
            F: FnMut(super::jsapi::HandleObject<'_>),
        {
        }
    }
    pub struct CompileOptionsWrapper {
        pub ptr: *mut std::ffi::c_void,
    }
    impl CompileOptionsWrapper {
        pub fn new<C, F>(_cx: &C, _filename: F, _line: u32) -> Self {
            Self {
                ptr: std::ptr::null_mut(),
            }
        }
        pub unsafe fn new_raw<C, F>(_cx: &C, _filename: F, _line: u32) -> Self {
            Self {
                ptr: std::ptr::null_mut(),
            }
        }
        pub fn set_introduction_type<T>(&mut self, _introduction_type: T) {}
        pub fn set_muted_errors(&mut self, _muted_errors: bool) {}
        pub fn set_is_run_once(&mut self, _is_run_once: bool) {}
        pub fn set_no_script_rval(&mut self, _no_script_rval: bool) {}
    }

    pub struct EnvironmentChain;
    impl EnvironmentChain {
        pub fn new<C, S>(_cx: &C, _support_unscopables: S) -> Self {
            Self
        }
        pub fn append<O>(&mut self, _object: O) {}
        pub fn get(&self) -> *const std::ffi::c_void {
            std::ptr::null()
        }
    }
    pub struct StructuredCloneBuffer {
        pub data_: Vec<u8>,
    }
    pub struct JSAutoStructuredCloneBufferWrapper {
        buffer: StructuredCloneBuffer,
    }
    impl JSAutoStructuredCloneBufferWrapper {
        pub fn new<S, C>(_scope: S, _callbacks: C) -> Self {
            Self {
                buffer: StructuredCloneBuffer { data_: Vec::new() },
            }
        }
        pub fn as_raw_ptr(&self) -> *mut StructuredCloneBuffer {
            &self.buffer as *const StructuredCloneBuffer as *mut StructuredCloneBuffer
        }
    }
    pub struct JSEngine;
    pub struct JSEngineHandle;
    pub struct ParentRuntime;
    pub struct Stencil;
    #[derive(Clone)]
    pub struct ThreadSafeJSContext;
    impl ThreadSafeJSContext {
        pub fn request_interrupt_callback(&self) {}
    }
    impl JSEngine {
        pub fn init() -> Result<Self, ()> {
            #[cfg(feature = "v8")]
            super::ensure_v8();
            Ok(Self)
        }
        pub fn handle(&self) -> JSEngineHandle {
            JSEngineHandle
        }
        pub fn can_shutdown(&self) -> bool {
            true
        }
    }
    impl Clone for JSEngineHandle {
        fn clone(&self) -> Self {
            Self
        }
    }
    pub fn ToNumber<C, V>(_cx: &C, _value: V) -> Result<f64, ()> {
        Ok(0.0)
    }
    #[derive(Default)]
    pub struct ScriptedCaller {
        pub filename: String,
        pub line: u32,
        pub col: u32,
    }
    #[derive(Default)]
    pub struct ExceptionStackInfo {
        pub message: String,
        pub filename: String,
        pub line: u32,
        pub col: u32,
    }
    #[derive(Clone, Copy, Debug)]
    pub enum ForOfIterationFailure<OtherError> {
        ValueIsNotIterable,
        JSFailed,
        Other(OtherError),
    }
    impl<OtherError> From<OtherError> for ForOfIterationFailure<OtherError> {
        fn from(value: OtherError) -> Self {
            Self::Other(value)
        }
    }
    pub fn for_of<Callback, OtherError>(
        _cx: *mut super::jsapi::JSContext,
        _iterable: super::jsapi::HandleValue<'_>,
        _callback: Callback,
    ) -> Result<(), ForOfIterationFailure<OtherError>>
    where
        Callback: FnMut(super::jsapi::HandleValue<'_>) -> Result<std::ops::ControlFlow<()>, ForOfIterationFailure<OtherError>>,
    {
        Ok(())
    }
    pub fn describe_scripted_caller<C: ?Sized>(_cx: &C) -> Result<ScriptedCaller, ()> {
        Err(())
    }
    pub fn error_info_from_exception_stack<C: ?Sized>(
        _cx: &C,
        _value: super::jsapi::JSVal,
    ) -> Option<ExceptionStackInfo> {
        None
    }
    pub trait SourceTextPtr {
        fn as_source_ptr(self) -> *const std::ffi::c_void;
    }

    impl SourceTextPtr for *const std::ffi::c_void {
        fn as_source_ptr(self) -> *const std::ffi::c_void {
            self
        }
    }

    impl SourceTextPtr for *mut std::ffi::c_void {
        fn as_source_ptr(self) -> *const std::ffi::c_void {
            self as *const std::ffi::c_void
        }
    }

    impl SourceTextPtr for &*const std::ffi::c_void {
        fn as_source_ptr(self) -> *const std::ffi::c_void {
            *self
        }
    }

    impl SourceTextPtr for &mut *const std::ffi::c_void {
        fn as_source_ptr(self) -> *const std::ffi::c_void {
            *self
        }
    }

    impl SourceTextPtr for &*mut std::ffi::c_void {
        fn as_source_ptr(self) -> *const std::ffi::c_void {
            *self as *const std::ffi::c_void
        }
    }

    impl SourceTextPtr for &mut *mut std::ffi::c_void {
        fn as_source_ptr(self) -> *const std::ffi::c_void {
            *self as *const std::ffi::c_void
        }
    }

    pub fn transform_u16_to_source_text<S>(_source: S) -> *const std::ffi::c_void {
        // UTF-16 path not mirrored yet; empty source still compiles.
        #[cfg(feature = "v8")]
        {
            return crate::v8_glue::store_source_text("");
        }
        #[cfg(not(feature = "v8"))]
        {
            ptr::null()
        }
    }
    pub fn transform_str_to_source_text<S>(source: S) -> *const std::ffi::c_void
    where
        S: AsRef<str>,
    {
        #[cfg(feature = "v8")]
        {
            return crate::v8_glue::store_source_text(source.as_ref());
        }
        #[cfg(not(feature = "v8"))]
        {
            let _ = source;
            ptr::null()
        }
    }

    pub trait IntoJSScriptPtr {
        fn into_js_script_ptr(self) -> *mut super::jsapi::JSScript;
    }

    impl IntoJSScriptPtr for *mut super::jsapi::JSScript {
        fn into_js_script_ptr(self) -> *mut super::jsapi::JSScript {
            self
        }
    }

    impl IntoJSScriptPtr for *const super::jsapi::JSScript {
        fn into_js_script_ptr(self) -> *mut super::jsapi::JSScript {
            self as *mut super::jsapi::JSScript
        }
    }

    impl IntoJSScriptPtr for super::jsapi::Handle<'_, *mut super::jsapi::JSScript> {
        fn into_js_script_ptr(self) -> *mut super::jsapi::JSScript {
            self.get()
        }
    }
    pub mod wrappers2 {
        use super::super::jsapi;
        use std::ptr;

        pub use super::super::jsapi::{
            GetRealmObjectPrototype, JS_GetLatin1StringCharsAndLength,
            JS_GetTwoByteStringCharsAndLength, JS_NewPlainObject,
        };
        pub use super::wrappers::{
            AddPromiseReactions, AppendToIdVector, CallOriginalPromiseReject,
            CallOriginalPromiseResolve, CheckRegExpSyntax, DetachArrayBuffer, ExecuteRegExpNoStatics,
            GetBuiltinClass, GetPropertyKeys, GetPromiseIsHandled, GetPromiseState, IsPromiseObject,
            JS_AlreadyHasOwnPropertyById, JS_CallFunctionName, JS_CopyOwnPropertiesAndPrivateFields,
            JS_DefineProperty, JS_DefineProperty3, JS_DefineProperty4, JS_DefineProperty5,
            JS_DefinePropertyById2,
            JS_DefinePropertyById5, JS_DefineUCProperty2, JS_DeletePropertyById,
            JS_ErrorFromException, JS_FireOnNewGlobalObject, JS_ForwardGetPropertyTo,
            JS_GetPromiseResult, JS_GetPrototype, JS_HasOwnProperty, JS_HasProperty,
            JS_HasPropertyById, JS_IdToValue, JS_InitializePropertiesFromCompatibleNativeObject,
            JS_LinkConstructorAndPrototype, JS_NewFunction, JS_NewGlobalObject,
            JS_NewObjectWithoutMetadata, JS_ReadStructuredClone, JS_SetImmutablePrototype,
            JS_SetProperty, JS_Stringify, JS_TransplantObject, JS_TypeOfValue, JS_ValueToSource,
            JS_WriteStructuredClone, NewPromiseObject, NewProxyObject, NewWindowProxy, ObjectIsRegExp,
            RUST_SYMBOL_TO_JSID, RejectPromise, ResolvePromise, SetAnyPromiseIsHandled,
            SetDataPropertyDescriptor, SetPromiseUserInputEventHandlingState, SetWindowProxy, ToJSON,
            int_to_jsid,
        };
        pub use super::super::jsapi::{
            ArrayBufferClone, ArrayBufferCopyData, GetSavedFrameColumn,
            GetSavedFrameFunctionDisplayName, GetSavedFrameLine, GetSavedFrameSource,
            HasDefinedArrayBufferDetachKey, JS_FreezeObject, JS_GetArrayBufferViewBuffer,
            JS_GetFunctionDisplayId, JS_GetFunctionId, JS_NewBigInt64ArrayWithBuffer,
            JS_NewBigUint64ArrayWithBuffer, JS_NewDataView, JS_NewFloat16ArrayWithBuffer,
            JS_NewFloat32ArrayWithBuffer, JS_NewFloat64ArrayWithBuffer, JS_NewInt8ArrayWithBuffer,
            JS_NewInt16ArrayWithBuffer, JS_NewInt32ArrayWithBuffer, JS_NewUint8ArrayWithBuffer,
            JS_NewUint8ClampedArrayWithBuffer, JS_NewUint16ArrayWithBuffer,
            JS_NewUint32ArrayWithBuffer, JS_ValueToFunction, NewArrayBuffer,
            NewArrayBufferWithContents, NewFunctionWithReserved, NewUCRegExpObject,
            StealArrayBufferContents, ToPrimitive,
        };
        pub use crate::glue::{CollectServoSizes, DispatchableRun};
        pub use super::super::jsapi::{
            AddRawValueRoot, GetObjectProto, GetRealmErrorPrototype, GetRealmFunctionPrototype,
            GetRealmIteratorPrototype, JS_DefinePropertyById, JS_GetPropertyDescriptorById,
            JS_IterateCompartments, JS_SetTrustedPrincipals,
        };
        pub use crate::glue::{CallJitGetterOp, CallJitMethodOp, CallJitSetterOp, RUST_JSID_IS_VOID};

        pub unsafe fn JS_GetRuntime(_cx: *mut jsapi::JSContext) -> *mut std::ffi::c_void {
            ptr::null_mut()
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_IsExceptionPending<C: ?Sized>(_cx: &C) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_IsExceptionPending<C: ?Sized>(_cx: &C) -> bool {
            crate::v8_glue::is_exception_pending()
        }
        pub unsafe fn JS_WrapObject<C, O>(_cx: &C, _obj: O) -> bool {
            false
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_GetProperty<C, O, N, V>(_cx: &C, _obj: O, _name: N, _vp: V) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_GetProperty<C, O, N, V>(_cx: &C, obj: O, name: N, vp: V) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
            N: jsapi::ToBytePtr,
            V: super::wrappers::SetPropertyOut,
        {
            match crate::v8_glue::get_property_by_name(obj.into(), name.to_byte_ptr()) {
                Some(val) => vp.set_property_out(val),
                None => vp.set_property_out(jsapi::JSVal::undefined()),
            }
            true
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_ClearPendingException<C: ?Sized>(_cx: &C) {}
        #[cfg(feature = "v8")]
        pub unsafe fn JS_ClearPendingException<C: ?Sized>(_cx: &C) {
            crate::v8_glue::clear_pending_exception();
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_GetPendingException<C, V>(_cx: &C, _vp: V) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_GetPendingException<C, V>(_cx: &C, vp: V) -> bool
        where
            V: jsapi::SetJsapiValOut,
        {
            if let Some(val) = crate::v8_glue::get_pending_exception() {
                vp.set_jsapi_val_out(val);
                true
            } else {
                false
            }
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_SetPendingException<C, V, B>(_cx: &C, _val: V, _behavior: B)
        where
            V: Into<jsapi::JSVal>,
        {
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_SetPendingException<C, V, B>(_cx: &C, val: V, _behavior: B)
        where
            V: Into<jsapi::JSVal>,
        {
            crate::v8_glue::set_pending_exception(val.into());
        }
        pub unsafe fn JS_ParseJSON<C, S, L, V>(_cx: &C, _chars: S, _len: L, _vp: V) -> bool {
            false
        }
        pub unsafe fn GetFunctionRealm<C, F>(_cx: &C, _fun: F) -> *mut jsapi::JSObject {
            ptr::null_mut()
        }
        pub unsafe fn GetWellKnownSymbol<C, W>(_cx: &C, _which: W) -> jsapi::JSVal {
            jsapi::JSVal::default()
        }
        pub unsafe fn RUST_INTERNED_STRING_TO_JSID<C, S, O>(_cx: &C, _s: S, _out: O) -> jsapi::jsid {
            jsapi::jsid(0)
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_AtomizeAndPinString<C, S>(_cx: &C, _s: S) -> *mut jsapi::JSString {
            ptr::null_mut()
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_AtomizeAndPinString<C: ?Sized, S: jsapi::ToBytePtr>(
            _cx: &C,
            s: S,
        ) -> *mut jsapi::JSString {
            let bytes = s.to_byte_ptr();
            if bytes.is_null() {
                return ptr::null_mut();
            }
            // SAFETY: JSAPI string name inputs are null-terminated C strings.
            let len = unsafe { std::ffi::CStr::from_ptr(bytes as *const std::os::raw::c_char) }
                .to_bytes()
                .len();
            crate::v8_glue::atomize_string_n(ptr::null_mut(), bytes, len)
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_NewObjectWithGivenProto<C: ?Sized, P>(
            _cx: &C,
            _clasp: *const jsapi::JSClass,
            _proto: P,
        ) -> *mut jsapi::JSObject {
            ptr::null_mut()
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_NewObjectWithGivenProto<C: ?Sized, P>(
            _cx: &C,
            clasp: *const jsapi::JSClass,
            proto: P,
        ) -> *mut jsapi::JSObject
        where
            P: Into<*mut jsapi::JSObject>,
        {
            crate::v8_glue::js_new_object_with_proto(clasp, proto.into())
        }
        pub unsafe fn JS_DefineProperties<C: ?Sized, O>(
            _cx: &C,
            _obj: O,
            _props: *const jsapi::JSPropertySpec,
        ) -> bool {
            false
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_DefineFunctions<C: ?Sized, O>(
            _cx: &C,
            _obj: O,
            _funcs: *const jsapi::JSFunctionSpec,
        ) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_DefineFunctions<C: ?Sized, O>(
            cx: &C,
            obj: O,
            funcs: *const jsapi::JSFunctionSpec,
        ) -> bool
        where
            O: Into<*mut jsapi::JSObject> + Copy,
        {
            if funcs.is_null() {
                return true;
            }
            let mut len = 0usize;
            while !unsafe { (*funcs.add(len)).name.string_ }.is_null() {
                len += 1;
            }
            // SAFETY: JSFunctionSpec arrays are null-name terminated; len was found above.
            let specs = unsafe { std::slice::from_raw_parts(funcs, len) };
            super::define_methods(cx, obj, specs).is_ok()
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_SetPrototype<C, O, P>(_cx: &C, _obj: O, _proto: P) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_SetPrototype<C, O, P>(_cx: &C, obj: O, proto: P) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
            P: Into<*mut jsapi::JSObject>,
        {
            !crate::v8_glue::set_prototype(obj.into(), proto.into()).is_null()
        }
        pub unsafe fn SameValue<C, A, B>(_cx: &C, _a: A, _b: B, _same: *mut bool) -> bool {
            false
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn NewArrayObject<C, A>(_cx: &C, _args: A) -> *mut jsapi::JSObject {
            ptr::null_mut()
        }
        #[cfg(feature = "v8")]
        pub unsafe fn NewArrayObject<C, A>(_cx: &C, args: A) -> *mut jsapi::JSObject
        where
            A: jsapi::ToArrayObjectInit,
        {
            match args.to_array_values() {
                Some(values) => crate::v8_glue::new_array_object(values),
                None => ptr::null_mut(),
            }
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn GetArrayLength<C, O>(_cx: &C, _obj: O, _len: *mut u32) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn GetArrayLength<C, O>(_cx: &C, obj: O, len: *mut u32) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
        {
            if !len.is_null() {
                // SAFETY: non-null JSAPI out-param checked above.
                unsafe { *len = crate::v8_glue::array_length(obj.into()) };
            }
            true
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn IsArrayObject<C, O>(_cx: &C, _obj: O, _is_array: *mut bool) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn IsArrayObject<C, O>(_cx: &C, obj: O, is_array: *mut bool) -> bool
        where
            O: jsapi::ToFunctionObjectPtr,
        {
            if !is_array.is_null() {
                // SAFETY: non-null JSAPI out-param checked above.
                unsafe {
                    *is_array = crate::v8_glue::is_array_object(obj.to_function_object_ptr())
                };
            }
            true
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_HasOwnPropertyById<C: ?Sized, O, I>(
            _cx: &C,
            _obj: O,
            _id: I,
            _found: *mut bool,
        ) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_HasOwnPropertyById<C: ?Sized, O, I>(
            _cx: &C,
            obj: O,
            id: I,
            found: *mut bool,
        ) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
            I: Into<jsapi::jsid>,
        {
            if !found.is_null() {
                // SAFETY: non-null JSAPI out-param checked above.
                unsafe { *found = crate::v8_glue::has_property_by_jsid(obj.into(), id.into()) };
            }
            true
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_IndexToId<C, I, O>(_cx: &C, _index: I, _id: O) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_IndexToId<C, I, O>(_cx: &C, index: I, id: O) -> bool
        where
            I: Into<u32>,
            O: jsapi::SetJsapiIdOut,
        {
            id.set_jsapi_id_out(jsapi::jsid::from_int(index.into() as i32));
            true
        }
        pub unsafe fn JS_IsIdentifier<C, S>(_cx: &C, _chars: S, _is_valid: *mut bool) -> bool {
            false
        }
        pub unsafe fn JS_NewObject<C: ?Sized>(_cx: &C, _clasp: *const jsapi::JSClass) -> *mut jsapi::JSObject {
            ptr::null_mut()
        }
        pub unsafe fn CompileFunction<C: ?Sized, O, Opt, N, A, S>(
            _cx: &C,
            _obj: O,
            _options: Opt,
            _name: N,
            _nargs: u32,
            _args: A,
            _source: S,
        ) -> *mut jsapi::JSObject {
            ptr::null_mut()
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_GetOwnPropertyDescriptorById<C: ?Sized, O, I, D>(
            _cx: &C,
            _obj: O,
            _id: I,
            _desc: D,
            _found: *mut bool,
        ) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
            D: Into<*mut jsapi::PropertyDescriptor>,
        {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_GetOwnPropertyDescriptorById<C: ?Sized, O, I, D>(
            _cx: &C,
            obj: O,
            id: I,
            desc: D,
            is_none: *mut bool,
        ) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
            I: Into<jsapi::jsid>,
            D: Into<*mut jsapi::PropertyDescriptor>,
        {
            crate::v8_glue::get_property_descriptor_by_jsid(
                obj.into(),
                id.into(),
                desc.into(),
                is_none,
            )
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn InvokeGetOwnPropertyDescriptor<H, C, P, I, D>(
            _handler: H,
            _cx: &C,
            _proxy: P,
            _id: I,
            _desc: D,
            _found: *mut bool,
        ) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn InvokeGetOwnPropertyDescriptor<H, C, P, I, D>(
            _handler: H,
            _cx: &C,
            proxy: P,
            id: I,
            desc: D,
            is_none: *mut bool,
        ) -> bool
        where
            P: Into<*mut jsapi::JSObject>,
            I: Into<jsapi::jsid>,
            D: Into<*mut jsapi::PropertyDescriptor>,
        {
            crate::v8_glue::get_property_descriptor_by_jsid(
                proxy.into(),
                id.into(),
                desc.into(),
                is_none,
            )
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn SetPropertyIgnoringNamedGetter<C: ?Sized, O, I, V, R, D, S>(
            _cx: &C,
            _obj: O,
            _id: I,
            _v: V,
            _receiver: R,
            _desc: D,
            _result: S,
        ) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn SetPropertyIgnoringNamedGetter<C: ?Sized, O, I, V, R, D, S>(
            _cx: &C,
            obj: O,
            id: I,
            v: V,
            _receiver: R,
            _desc: D,
            result: S,
        ) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
            I: Into<jsapi::jsid>,
            V: jsapi::ToJsapiPropertyValue,
            S: jsapi::SetObjectOpResultOut,
        {
            let ok = crate::v8_glue::set_property_by_jsid(
                obj.into(),
                id.into(),
                v.to_jsapi_property_value(),
            );
            if ok {
                result.set_object_op_success();
            }
            ok
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn Call<C, T, F, A, R>(_cx: &C, _this: T, _fun: F, _args: A, _rval: R) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn Call<C, T, F, A, R>(_cx: &C, _this: T, fun: F, args: A, rval: R) -> bool
        where
            F: jsapi::ToFunctionObjectPtr,
            A: jsapi::ToCallArgs,
            R: Into<*mut jsapi::JSVal>,
        {
            crate::v8_glue::call_function(
                ptr::null_mut(),
                jsapi::JSVal::from_object(fun.to_function_object_ptr()),
                args.to_call_args(),
                rval.into(),
            )
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn EnterRealm<C, O>(_cx: &C, _realm: O) -> *mut std::ffi::c_void {
            ptr::null_mut()
        }
        #[cfg(feature = "v8")]
        pub unsafe fn EnterRealm<C, O>(_cx: &C, realm: O) -> *mut std::ffi::c_void
        where
            O: crate::realm::IntoJSObject,
        {
            crate::v8_glue::enter_realm(realm.into_js_object());
            ptr::null_mut()
        }
        pub unsafe fn LeaveRealm<C, R>(_cx: &C, _old_realm: R) {}
        #[cfg(not(feature = "v8"))]
        pub unsafe fn Compile1<C, O, S>(_cx: &C, _options: O, _source: S) -> *mut jsapi::JSScript {
            ptr::null_mut()
        }
        #[cfg(feature = "v8")]
        pub unsafe fn Compile1<C, O, S>(_cx: &C, _options: O, source: S) -> *mut jsapi::JSScript
        where
            S: super::SourceTextPtr,
        {
            crate::v8_glue::compile_script_from_source_ptr(source.as_source_ptr())
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn CompileJsonModule1<C: ?Sized, O, S>(
            _cx: &C,
            _options: O,
            _source: S,
        ) -> *mut jsapi::JSObject {
            ptr::null_mut()
        }
        #[cfg(feature = "v8")]
        pub unsafe fn CompileJsonModule1<C: ?Sized, O, S>(
            _cx: &C,
            _options: O,
            _source: S,
        ) -> *mut jsapi::JSObject {
            crate::v8_glue::js_new_object()
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn CompileModule1<C: ?Sized, O, S>(
            _cx: &C,
            _options: O,
            _source: S,
        ) -> *mut jsapi::JSObject {
            ptr::null_mut()
        }
        #[cfg(feature = "v8")]
        pub unsafe fn CompileModule1<C: ?Sized, O, S>(
            _cx: &C,
            _options: O,
            _source: S,
        ) -> *mut jsapi::JSObject {
            crate::v8_glue::js_new_object()
        }
        pub unsafe fn Construct1<C, F, A, R>(_cx: &C, _fun: F, _args: A, _rval: R) -> bool {
            false
        }
        pub unsafe fn ContextOptionsRef<C: ?Sized>(_cx: &C) -> *mut jsapi::ContextOptions {
            jsapi::context_options_ref()
        }
        pub unsafe fn DateGetMsecSinceEpoch<C, O>(_cx: &C, _obj: O, _ms: *mut f64) -> bool {
            false
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn DefineFunctionWithReserved<C: ?Sized, O, N>(
            _cx: &C,
            _obj: O,
            _name: N,
            _call: Option<jsapi::NativeCallback>,
            _nargs: u32,
            _attrs: u32,
        ) -> *mut jsapi::JSFunction {
            ptr::null_mut()
        }
        #[cfg(feature = "v8")]
        pub unsafe fn DefineFunctionWithReserved<C: ?Sized, O, N>(
            _cx: &C,
            obj: O,
            name: N,
            call: Option<jsapi::NativeCallback>,
            nargs: u32,
            attrs: u32,
        ) -> *mut jsapi::JSFunction
        where
            O: Into<*mut jsapi::JSObject>,
            N: jsapi::ToBytePtr,
        {
            let name_ptr = name.to_byte_ptr();
            let fun = crate::v8_glue::js_new_function(
                ptr::null_mut(),
                call,
                nargs,
                attrs,
                crate::v8_glue::property_name_from_raw(name_ptr),
            );
            let fun_obj = crate::v8_glue::get_function_object(fun);
            if fun_obj.is_null()
                || !crate::v8_glue::set_property_by_name(
                    obj.into(),
                    name_ptr,
                    jsapi::JSVal::from_object(fun_obj),
                )
            {
                ptr::null_mut()
            } else {
                fun
            }
        }
        pub unsafe fn GetModuleNamespace<C, M>(_cx: &C, _module: M) -> *mut jsapi::JSObject {
            ptr::null_mut()
        }
        pub unsafe fn GetModuleRequestSpecifier<M, I>(
            _module: M,
            _index: I,
        ) -> *mut jsapi::JSString {
            ptr::null_mut()
        }
        pub unsafe fn GetModuleRequestType<M, I>(_module: M, _index: I) -> jsapi::ModuleType {
            jsapi::ModuleType::JavaScript
        }
        pub unsafe fn GetRequestedModuleSpecifier<C: ?Sized, M, I>(
            _cx: &C,
            _module: M,
            _index: I,
        ) -> *mut jsapi::JSString {
            ptr::null_mut()
        }
        pub unsafe fn GetRequestedModuleType<C: ?Sized, M, I>(
            _cx: &C,
            _module: M,
            _index: I,
        ) -> jsapi::ModuleType {
            jsapi::ModuleType::JavaScript
        }
        pub unsafe fn GetRequestedModulesCount<C, M>(_cx: &C, _module: M) -> u32 {
            0
        }
        pub unsafe fn InitConsumeStreamCallback<C, F, E>(_cx: &C, _callback: F, _error: E) {}
        pub unsafe fn JobQueueIsEmpty<C: ?Sized>(_cx: &C) -> bool {
            true
        }
        pub unsafe fn JS_AddExtraGCRootsTracer<C, F, D>(_cx: &C, _tracer: F, _data: D) {}
        pub unsafe fn JS_AddInterruptCallback<C, F>(_cx: &C, _callback: F) -> bool {
            false
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_DefineDebuggerObject<C, O>(_cx: &C, _obj: O) -> bool {
            true
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_DefineDebuggerObject<C, O>(_cx: &C, obj: O) -> bool
        where
            O: crate::realms::IntoJSObject,
        {
            crate::v8_glue::define_debugger_object(obj.into_js_object())
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_ExecuteScript<C, S, R>(_cx: &C, _script: S, _rval: R) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_ExecuteScript<C, S, R>(_cx: &C, script: S, rval: R) -> bool
        where
            S: crate::rust::IntoJSScriptPtr,
            R: jsapi::SetJsapiValOut,
        {
            match crate::v8_glue::execute_script_handle(script.into_js_script_ptr()) {
                Some(val) => {
                    rval.set_jsapi_val_out(val);
                    true
                }
                None => {
                    rval.set_jsapi_val_out(jsapi::JSVal::undefined());
                    false
                }
            }
        }
        pub unsafe fn JS_GC<C, R>(_cx: &C, _reason: R) {}
        pub unsafe fn JS_GetGCParameter<C, K>(_cx: &C, _key: K) -> u32 {
            0
        }
        pub unsafe fn JS_GetModulePrivate<M>(_module: M) -> jsapi::JSVal {
            jsapi::JSVal::default()
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_GetPropertyById<C, O, I, V>(_cx: &C, _obj: O, _id: I, _vp: V) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_GetPropertyById<C, O, I, V>(_cx: &C, obj: O, id: I, vp: V) -> bool
        where
            O: Into<*mut jsapi::JSObject>,
            I: Into<jsapi::jsid>,
            V: super::wrappers::SetPropertyOut,
        {
            match crate::v8_glue::get_property_by_jsid(obj.into(), id.into()) {
                Some(val) => vp.set_property_out(val),
                None => vp.set_property_out(jsapi::JSVal::undefined()),
            }
            true
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_GetScriptPrivate<S, V>(_script: S, _value: V) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_GetScriptPrivate<S, V>(_script: S, value: V) -> bool
        where
            S: crate::rust::IntoJSScriptPtr,
            V: jsapi::SetJsapiValOut,
        {
            value.set_jsapi_val_out(crate::v8_glue::get_script_private(
                _script.into_js_script_ptr(),
            ));
            true
        }
        pub unsafe fn JS_InitDestroyPrincipalsCallback<C, F>(_cx: &C, _callback: F) {}
        pub unsafe fn JS_InitReadPrincipalsCallback<C, F>(_cx: &C, _callback: F) {}
        #[cfg(not(feature = "v8"))]
        pub unsafe fn JS_NewStringCopyN<C, S>(_cx: &C, _s: S, _len: usize) -> *mut jsapi::JSString {
            ptr::null_mut()
        }
        #[cfg(feature = "v8")]
        pub unsafe fn JS_NewStringCopyN<C: ?Sized, S: jsapi::ToBytePtr>(
            _cx: &C,
            s: S,
            len: usize,
        ) -> *mut jsapi::JSString {
            crate::v8_glue::atomize_string_n(ptr::null_mut(), s.to_byte_ptr(), len)
        }
        pub unsafe fn JS_SetGCCallback<C, F, D>(_cx: &C, _callback: F, _data: D) {}
        pub unsafe fn JS_SetGCParameter<C, K, V>(_cx: &C, _key: K, _value: V) {}
        pub unsafe fn JS_SetGlobalJitCompilerOption<C, O, V>(_cx: &C, _option: O, _value: V) {}
        pub unsafe fn JS_SetOffthreadIonCompilationEnabled<C>(_cx: &C, _enabled: bool) {}
        pub unsafe fn JS_SetSecurityCallbacks<C, S>(_cx: &C, _callbacks: S) {}
        #[cfg(not(feature = "v8"))]
        pub unsafe fn ModuleEvaluate<C, M, R>(_cx: &C, _module: M, _rval: R) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn ModuleEvaluate<C, M, R>(_cx: &C, _module: M, rval: R) -> bool
        where
            R: jsapi::SetJsapiValOut,
        {
            let promise = crate::v8_glue::new_promise_object(ptr::null_mut());
            rval.set_jsapi_val_out(jsapi::JSVal::from_object(promise));
            true
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn ModuleLink<C, M>(_cx: &C, _module: M) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn ModuleLink<C, M>(_cx: &C, _module: M) -> bool {
            true
        }
        #[cfg(not(feature = "v8"))]
        pub unsafe fn NewDateObject<C, T>(_cx: &C, _time: T) -> *mut jsapi::JSObject {
            ptr::null_mut()
        }
        #[cfg(feature = "v8")]
        pub unsafe fn NewDateObject<C, T>(_cx: &C, time: T) -> *mut jsapi::JSObject
        where
            T: IntoDateMs,
        {
            crate::v8_glue::new_date_object(time.into_date_ms())
        }

        pub trait IntoDateMs {
            fn into_date_ms(self) -> f64;
        }

        impl IntoDateMs for f64 {
            fn into_date_ms(self) -> f64 {
                self
            }
        }

        impl IntoDateMs for jsapi::ClippedTime {
            fn into_date_ms(self) -> f64 {
                self.t
            }
        }

        impl IntoDateMs for &jsapi::ClippedTime {
            fn into_date_ms(self) -> f64 {
                self.t
            }
        }
        pub unsafe fn ObjectIsDate<C, O>(_cx: &C, _obj: O, out: *mut bool) -> bool {
            if !out.is_null() {
                *out = false;
            }
            true
        }
        pub unsafe fn SetDOMCallbacks<C, D>(_cx: &C, _callbacks: D) {}
        pub unsafe fn SetGCSliceCallback<C, F>(_cx: &C, _callback: F) {}
        pub unsafe fn SetJobQueue<C, Q>(_cx: &C, _queue: Q) {}
        pub unsafe fn SetPreserveWrapperCallbacks<C, A, B>(_cx: &C, _preserve: A, _has_released: B) {
        }
        pub unsafe fn SetPromiseRejectionTrackerCallback<C, F, D>(_cx: &C, _callback: F, _data: D) {}
        pub unsafe fn SetUpEventLoopDispatch<C, F, D>(_cx: &C, _callback: F, _data: D) {}
        pub unsafe fn SetWindowProxyClass<C, O>(_cx: &C, _class: O) {}
        #[cfg(not(feature = "v8"))]
        pub unsafe fn ThrowOnModuleEvaluationFailure<C: ?Sized, P, B>(
            _cx: &C,
            _promise: P,
            _behavior: B,
        ) -> bool {
            false
        }
        #[cfg(feature = "v8")]
        pub unsafe fn ThrowOnModuleEvaluationFailure<C: ?Sized, P, B>(
            _cx: &C,
            _promise: P,
            _behavior: B,
        ) -> bool {
            true
        }
        pub unsafe fn JS_GetElement<C, O, I, V>(_cx: &C, _obj: O, _index: I, _vp: V) -> bool {
            false
        }
        pub unsafe fn JS_GetScriptedCallerPrivate<C, V>(_cx: &C, _vp: V) -> bool {
            false
        }
        pub unsafe fn MapEntries<C, O, V>(_cx: &C, _obj: O, _iterator: V) -> bool {
            false
        }
        pub unsafe fn MapSize<C, O>(_cx: &C, _obj: O) -> u32 {
            0
        }
    }

    pub unsafe fn ToString<C: ?Sized, V>(_cx: &C, _val: V) -> *mut super::jsapi::JSString {
        std::ptr::null_mut()
    }
    pub unsafe trait Trace {
        unsafe fn trace(&self, _tracer: *mut super::jsapi::JSTracer) {}
    }
    unsafe impl<T: super::gc::Traceable + ?Sized> Trace for std::rc::Rc<T> {
        unsafe fn trace(&self, tr: *mut super::jsapi::JSTracer) {
            <T as super::gc::Traceable>::trace(self, tr);
        }
    }

    pub trait IntoHandle {
        type Target;
        fn into_handle(self) -> Self::Target;
    }
} // end pub mod rust

// ── GC ──────────────────────────────────────────────────────────────────────

pub mod gc {
    use super::jsapi;
    use std::ptr;

    pub use crate::rust::CustomAutoRooterGuard;
    pub use super::jsapi::MutableHandle;

    pub unsafe trait Traceable {
        unsafe fn trace(&self, _tracer: *mut super::jsapi::JSTracer) {}
    }
    pub trait Rootable {}
    pub trait Initialize {
        unsafe fn initial() -> Option<Self>
        where
            Self: Sized,
        {
            None
        }
    }
    pub trait RootedTraceableSet {}

    pub struct RootedGuard<'a, T> {
        value: T,
        _phantom: std::marker::PhantomData<&'a T>,
    }
    impl<'a, T> RootedGuard<'a, T> {
        pub unsafe fn new<C>(_cx: C, _root: &'a mut std::mem::MaybeUninit<T>, val: T) -> Self {
            Self {
                value: val,
                _phantom: std::marker::PhantomData,
            }
        }
        pub fn handle(&self) -> jsapi::Handle<'_, T> {
            unsafe { jsapi::Handle::from_raw(&self.value as *const T) }
        }
        pub fn handle_mut(&mut self) -> jsapi::MutableHandle<'_, T> {
            unsafe { jsapi::MutableHandle::from_raw(&mut self.value as *mut T) }
        }
        pub fn get(&self) -> T
        where
            T: Copy,
        {
            self.value
        }
        pub fn set(&mut self, val: T) {
            self.value = val;
        }
        pub fn safe_to_jsval<C, G>(
            &self,
            _cx: C,
            mut rval: jsapi::MutableHandleValue<'_>,
            _can_gc: G,
        ) {
            rval.set(jsapi::JSVal::default());
        }
    }
    impl<'a> RootedGuard<'a, Vec<jsapi::JSVal>> {
        pub fn handle_mut_at(&mut self, index: usize) -> jsapi::MutableHandle<'_, jsapi::JSVal> {
            unsafe { jsapi::MutableHandle::from_raw(&mut self.value[index] as *mut jsapi::JSVal) }
        }
        pub fn set_index(&mut self, index: usize, value: jsapi::JSVal) {
            if index >= self.value.len() {
                self.value.resize(index + 1, jsapi::JSVal::default());
            }
            self.value[index] = value;
        }
    }
    impl<'a> RootedGuard<'a, jsapi::JSVal> {
        pub fn to_int32(&self) -> i32 {
            self.value.to_int32()
        }
    }
    impl<'a, T> std::ops::Deref for RootedGuard<'a, T> {
        type Target = T;
        fn deref(&self) -> &T {
            &self.value
        }
    }
    impl<'a, T> std::ops::DerefMut for RootedGuard<'a, T> {
        fn deref_mut(&mut self) -> &mut T {
            &mut self.value
        }
    }
    impl<'a, T> RootedGuard<'a, T> {
        pub fn take(self) -> T {
            self.value
        }
    }

    impl<'a> RootedGuard<'a, *mut std::ffi::c_void> {
        pub fn is_undefined(&self) -> bool {
            false
        }
        pub fn is_object(&self) -> bool {
            false
        }
        pub fn is_null_or_undefined(&self) -> bool {
            true
        }
        pub fn to_object(&self) -> *mut std::ffi::c_void {
            std::ptr::null_mut()
        }
    }
    impl<'a> RootedGuard<'a, jsapi::JSVal> {
        pub fn is_undefined(&self) -> bool {
            self.value.is_undefined()
        }
        pub fn is_object(&self) -> bool {
            self.value.is_object()
        }
        pub fn is_null_or_undefined(&self) -> bool {
            self.value.is_null_or_undefined()
        }
        pub fn to_object(&self) -> *mut jsapi::JSObject {
            self.value.to_object()
        }
        pub fn to_string(&self) -> *mut jsapi::JSString {
            self.value.to_string()
        }
    }
    impl<'a> std::fmt::Display for RootedGuard<'a, jsapi::JSVal> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "")
        }
    }
    impl<'a, T> RootedGuard<'a, *mut T> {
        pub fn is_null(&self) -> bool {
            self.value.is_null()
        }
    }

    pub unsafe fn add_associated_memory(_obj: *const jsapi::JSObject, _sz: usize) {}
    pub unsafe fn remove_associated_memory(_obj: *const jsapi::JSObject, _sz: usize) {}
    pub fn add_root(_obj: &dyn Traceable) {}
    pub fn remove_root(_obj: &dyn Traceable) {}

    unsafe impl Traceable for super::jsapi::JSVal {}
    unsafe impl Traceable for () {}
    unsafe impl Traceable for u8 {}
    unsafe impl Traceable for u16 {}
    unsafe impl Traceable for u32 {}
    unsafe impl Traceable for u64 {}
    unsafe impl Traceable for usize {}
    unsafe impl Traceable for i8 {}
    unsafe impl Traceable for i16 {}
    unsafe impl Traceable for i32 {}
    unsafe impl Traceable for i64 {}
    unsafe impl Traceable for f32 {}
    unsafe impl Traceable for f64 {}
    unsafe impl Traceable for bool {}
    unsafe impl<T> Traceable for super::jsapi::Heap<T> {}
    unsafe impl Traceable for crate::rust::Runtime {}
    unsafe impl<T> Traceable for *mut T {}
    unsafe impl<T> Traceable for *const T {}
    unsafe impl<T: Traceable> Traceable for &T {}
    unsafe impl<T: Traceable> Traceable for Option<T> {}
    unsafe impl<T: Traceable> Traceable for Vec<T> {}
    unsafe impl<T: Traceable> Traceable for [T] {}

    unsafe impl<T: Traceable> Traceable for std::collections::VecDeque<T> {}
    unsafe impl<T: Traceable, E: Traceable> Traceable for Result<T, E> {}
    unsafe impl<T: Traceable + ?Sized> Traceable for Box<T> {}
    unsafe impl<T: Traceable + ?Sized> Traceable for std::rc::Rc<T> {}
    unsafe impl<T: Traceable + ?Sized> Traceable for std::sync::Arc<T> {}
    unsafe impl Traceable for std::sync::atomic::AtomicBool {}
    unsafe impl<A: Traceable, B: Traceable> Traceable for (A, B) {}
    unsafe impl<A: Traceable, B: Traceable, C: Traceable, D: Traceable> Traceable for (A, B, C, D) {}
    unsafe impl<K: Traceable, V: Traceable, S> Traceable for std::collections::HashMap<K, V, S> {}
    unsafe impl<T: Traceable, S> Traceable for std::collections::HashSet<T, S> {}

    unsafe impl<T: Traceable, const N: usize> Traceable for [T; N] {}
    unsafe impl<T: Traceable> Traceable for std::thread::JoinHandle<T> {}
    unsafe impl<T: Traceable> Traceable for std::cell::OnceCell<T> {}
    unsafe impl<T> Traceable for std::ops::Range<T> {}
    unsafe impl Traceable for std::sync::atomic::AtomicUsize {}
    unsafe impl<T> Traceable for std::marker::PhantomData<T> {}
    unsafe impl<T: Traceable + Copy> Traceable for std::cell::Cell<T> {
        unsafe fn trace(&self, trc: *mut super::jsapi::JSTracer) {
            unsafe { Traceable::trace(&self.get(), trc) }
        }
    }
    unsafe impl<T: Traceable> Traceable for std::cell::RefCell<T> {
        unsafe fn trace(&self, trc: *mut super::jsapi::JSTracer) {
            unsafe { Traceable::trace(&*self.borrow(), trc) }
        }
    }
    unsafe impl<T> Traceable for std::cell::UnsafeCell<T> {}
    unsafe impl Traceable for String {}
    unsafe impl Traceable for std::time::Instant {}
    unsafe impl Traceable for std::time::Duration {}
    unsafe impl Traceable for std::time::SystemTime {}
    unsafe impl Traceable for std::num::NonZero<u16> {}
    unsafe impl<T: Traceable, S: std::hash::BuildHasher> Traceable for indexmap::IndexSet<T, S> {}
    unsafe impl<T> Traceable for crossbeam_channel::Sender<T> {}
    unsafe impl Traceable for crate::rust::Stencil {}
    unsafe impl<T: super::typedarray::TypedArrayElement, O> Traceable
        for super::typedarray::TypedArray<T, O>
    {
    }
    unsafe impl<T: super::typedarray::TypedArrayElement, O> crate::rust::Trace
        for super::typedarray::TypedArray<T, O>
    {
    }

    pub type HandleValue<'a> = super::jsapi::Handle<'a, super::jsapi::JSVal>;
    pub type MutableHandleValue<'a> = super::jsapi::MutableHandle<'a, super::jsapi::JSVal>;
    pub type HandleObject<'a> = super::jsapi::Handle<'a, *mut super::jsapi::JSObject>;
    pub type MutableHandleObject<'a> =
        super::jsapi::MutableHandle<'a, *mut super::jsapi::JSObject>;
    pub type Handle<'a, T> = super::jsapi::Handle<'a, T>;
    pub struct RootableVec<T: Traceable> {
        v: Vec<T>,
    }
    impl<T: Traceable> RootableVec<T> {
        pub fn new_unrooted() -> Self {
            Self { v: Vec::new() }
        }
    }
    unsafe impl<T: Traceable> Traceable for RootableVec<T> {}

    pub struct RootedVec<'a, T: Traceable + 'static> {
        root: &'a mut RootableVec<T>,
    }
    impl<'a, T: Traceable + 'static> RootedVec<'a, T> {
        pub fn new(root: &'a mut RootableVec<T>) -> Self {
            Self { root }
        }
        pub fn from_iter<I>(root: &'a mut RootableVec<T>, iter: I) -> Self
        where
            I: Iterator<Item = T>,
        {
            root.v.extend(iter);
            Self { root }
        }
    }
    impl<'a, T: Traceable> std::ops::Deref for RootedVec<'a, T> {
        type Target = Vec<T>;
        fn deref(&self) -> &Vec<T> {
            &self.root.v
        }
    }
    impl<'a, T: Traceable> std::ops::DerefMut for RootedVec<'a, T> {
        fn deref_mut(&mut self) -> &mut Vec<T> {
            &mut self.root.v
        }
    }

    pub type StackGCVector<T> = Vec<T>;

    pub struct RootedTraceableBox<T>(*mut T);
    impl<T> RootedTraceableBox<T> {
        pub fn new(val: T) -> Self {
            Self(Box::into_raw(Box::new(val)))
        }
        pub fn from_box(val: Box<T>) -> Self {
            Self(Box::into_raw(val))
        }
        pub fn handle(&self) -> super::jsapi::Handle<'_, T> {
            unsafe { super::jsapi::Handle::from_raw(self.ptr() as *const T) }
        }
        pub fn ptr(&self) -> *mut T {
            self.0
        }
        pub fn into_box(self) -> Box<T> {
            let ptr = self.0;
            std::mem::forget(self);
            unsafe { Box::from_raw(ptr) }
        }
        pub unsafe fn trace(&self, _tracer: *mut super::jsapi::JSTracer) {}
    }
    impl<T> Drop for RootedTraceableBox<T> {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    drop(Box::from_raw(self.0));
                }
            }
        }
    }
    impl<T> std::ops::Deref for RootedTraceableBox<T> {
        type Target = T;
        fn deref(&self) -> &T {
            unsafe { &*self.ptr() }
        }
    }
    impl<T> std::ops::DerefMut for RootedTraceableBox<T> {
        fn deref_mut(&mut self) -> &mut T {
            unsafe { &mut *self.ptr() }
        }
    }

    pub struct CoreGcTypes;

    pub trait GCMethods {
        fn initial() -> Self
        where
            Self: Sized;
        unsafe fn post_barrier(_v: *mut Self, _prev: Self, _next: Self)
        where
            Self: Sized,
        {
        }
    }
    impl<T> GCMethods for *mut T {
        fn initial() -> Self {
            std::ptr::null_mut()
        }
    }
    impl<T> GCMethods for *const T {
        fn initial() -> Self {
            std::ptr::null()
        }
    }
    impl GCMethods for u64 {
        fn initial() -> Self {
            0
        }
    }
    impl GCMethods for u32 {
        fn initial() -> Self {
            0
        }
    }
    impl GCMethods for bool {
        fn initial() -> Self {
            false
        }
    }
    impl GCMethods for jsapi::JSVal {
        fn initial() -> Self {
            jsapi::JSVal::default()
        }
    }
}

// ── Context / Rooting ───────────────────────────────────────────────────────

pub mod context {
    use super::jsapi;
    use super::*;

    pub use super::jsapi::JSContext as RawJSContext;

    #[derive(Debug, Copy, Clone)]
    #[repr(transparent)]
    pub struct JSContext {
        ptr: std::ptr::NonNull<RawJSContext>,
    }
    pub type SafeJSContext = JSContext;
    pub struct NoGC(());

    pub type CallArgs = jsapi::CallArgs;

    pub struct AutoCheckRequest;
    impl AutoCheckRequest {
        pub unsafe fn new(_cx: *mut jsapi::JSContext) -> Self {
            Self
        }
        pub unsafe fn new_unchecked(_cx: *mut jsapi::JSContext) -> Self {
            Self
        }
    }

    impl JSContext {
        pub unsafe fn from_ptr(ptr: std::ptr::NonNull<RawJSContext>) -> Self {
            Self { ptr }
        }
        pub unsafe fn from_raw_ptr(ptr: *mut RawJSContext) -> Self {
            Self::from_ptr(std::ptr::NonNull::new(ptr).expect("null JSContext"))
        }
        pub unsafe fn get_from_thread() -> Option<Self> {
            #[cfg(feature = "v8")]
            {
                crate::v8_glue::thread_js_context()
            }
            #[cfg(not(feature = "v8"))]
            {
                Some(unsafe { Self::from_ptr(std::ptr::NonNull::dangling()) })
            }
        }
        pub unsafe fn raw_cx(&mut self) -> *mut RawJSContext {
            self.ptr.as_ptr()
        }
        pub unsafe fn raw_cx_no_gc(&self) -> *mut RawJSContext {
            self.ptr.as_ptr()
        }
        pub fn no_gc(&self) -> &NoGC {
            static NO_GC: NoGC = NoGC(());
            &NO_GC
        }
    }

    impl std::ops::Deref for JSContext {
        type Target = NoGC;
        fn deref(&self) -> &NoGC {
            self.no_gc()
        }
    }
    impl std::ops::DerefMut for JSContext {
        fn deref_mut(&mut self) -> &mut NoGC {
            Box::leak(Box::new(NoGC(())))
        }
    }

    impl std::convert::AsMut<JSContext> for JSContext {
        fn as_mut(&mut self) -> &mut JSContext {
            self
        }
    }

    impl From<&mut JSContext> for JSContext {
        fn from(cx: &mut JSContext) -> Self {
            unsafe { JSContext::from_raw_ptr(cx.raw_cx()) }
        }
    }

    impl From<&JSContext> for JSContext {
        fn from(cx: &JSContext) -> Self {
            unsafe { JSContext::from_raw_ptr(cx.raw_cx_no_gc()) }
        }
    }

    pub unsafe fn from_ptr(p: std::ptr::NonNull<RawJSContext>) -> JSContext {
        JSContext { ptr: p }
    }
}

// ── Realm management ────────────────────────────────────────────────────────

pub mod realms {
    use super::jsapi;
    use std::ptr;

    pub trait IntoJSObject {
        fn into_js_object(self) -> *mut jsapi::JSObject;
    }
    impl IntoJSObject for *mut jsapi::JSObject {
        fn into_js_object(self) -> *mut jsapi::JSObject {
            self
        }
    }
    impl IntoJSObject for ptr::NonNull<jsapi::JSObject> {
        fn into_js_object(self) -> *mut jsapi::JSObject {
            self.as_ptr()
        }
    }
    impl<'a> IntoJSObject for jsapi::Handle<'a, *mut jsapi::JSObject> {
        fn into_js_object(self) -> *mut jsapi::JSObject {
            self.get()
        }
    }

    pub fn AlreadyInRealm(_cx: *mut jsapi::JSContext) -> bool {
        true
    }
    #[cfg(not(feature = "v8"))]
    pub fn EnterRealm(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject) {}
    #[cfg(feature = "v8")]
    pub fn EnterRealm(_cx: *mut jsapi::JSContext, obj: *mut jsapi::JSObject) {
        crate::v8_glue::enter_realm(obj);
    }
    #[cfg(not(feature = "v8"))]
    pub fn LeaveRealm(_cx: *mut jsapi::JSContext) {}
    #[cfg(feature = "v8")]
    pub fn LeaveRealm(_cx: *mut jsapi::JSContext) {}

    pub struct AutoRealm<'a> {
        _marker: std::marker::PhantomData<&'a ()>,
        #[cfg(feature = "v8")]
        previous_global: Option<usize>,
    }
    impl<'a> AutoRealm<'a> {
        pub fn new<C, O>(_cx: C, obj: O) -> Self
        where
            O: IntoJSObject,
        {
            #[cfg(feature = "v8")]
            let previous_global = crate::v8_glue::enter_realm(obj.into_js_object());
            #[cfg(not(feature = "v8"))]
            let previous_global = None;
            Self {
                _marker: std::marker::PhantomData,
                #[cfg(feature = "v8")]
                previous_global,
            }
        }
        pub unsafe fn new_from_handle<T, O>(_cx: T, obj: O) -> Self
        where
            O: IntoJSObject,
        {
            Self::new(_cx, obj)
        }
        pub fn current_realm(&mut self) -> CurrentRealm<'_> {
            CurrentRealm(std::marker::PhantomData)
        }
        pub unsafe fn raw_cx(&mut self) -> *mut jsapi::JSContext {
            ptr::null_mut()
        }
        pub fn global_and_reborrow(
            &mut self,
        ) -> (
            jsapi::HandleObject<'static>,
            &'static mut crate::context::JSContext,
        ) {
            let cx = unsafe { crate::context::JSContext::from_ptr(std::ptr::NonNull::dangling()) };
            (jsapi::HandleObject::null(), Box::leak(Box::new(cx)))
        }
    }
    impl<'a> std::ops::Deref for AutoRealm<'a> {
        type Target = crate::context::JSContext;
        fn deref(&self) -> &crate::context::JSContext {
            Box::leak(Box::new(unsafe {
                crate::context::JSContext::from_ptr(std::ptr::NonNull::dangling())
            }))
        }
    }
    impl<'a> std::ops::DerefMut for AutoRealm<'a> {
        fn deref_mut(&mut self) -> &mut crate::context::JSContext {
            Box::leak(Box::new(unsafe {
                crate::context::JSContext::from_ptr(std::ptr::NonNull::dangling())
            }))
        }
    }

    pub struct CurrentRealm<'a>(std::marker::PhantomData<&'a ()>);
    impl<'a> CurrentRealm<'a> {
        pub unsafe fn new(_cx: *mut jsapi::JSContext) -> Self {
            Self(std::marker::PhantomData)
        }
        pub fn assert<T>(_: T) -> Self {
            Self(std::marker::PhantomData)
        }
        pub unsafe fn raw_cx(&mut self) -> *mut jsapi::JSContext {
            ptr::null_mut()
        }
        pub unsafe fn raw_cx_no_gc(&self) -> *mut jsapi::JSContext {
            ptr::null_mut()
        }
        pub fn realm(&self) -> std::ptr::NonNull<std::ffi::c_void> {
            std::ptr::NonNull::dangling()
        }
        pub fn global(&self) -> jsapi::HandleObject<'static> {
            #[cfg(feature = "v8")]
            {
                crate::v8_glue::current_global_handle()
            }
            #[cfg(not(feature = "v8"))]
            {
                jsapi::HandleObject::null()
            }
        }
    }
    impl<'a> Drop for AutoRealm<'a> {
        fn drop(&mut self) {
            #[cfg(feature = "v8")]
            crate::v8_glue::leave_realm(self.previous_global);
        }
    }
    impl<'a> std::ops::Deref for CurrentRealm<'a> {
        type Target = crate::context::JSContext;
        fn deref(&self) -> &crate::context::JSContext {
            Box::leak(Box::new(unsafe {
                crate::context::JSContext::from_ptr(std::ptr::NonNull::dangling())
            }))
        }
    }
    impl<'a> std::ops::DerefMut for CurrentRealm<'a> {
        fn deref_mut(&mut self) -> &mut crate::context::JSContext {
            Box::leak(Box::new(unsafe {
                crate::context::JSContext::from_ptr(std::ptr::NonNull::dangling())
            }))
        }
    }
    impl<'a> std::convert::AsMut<crate::context::JSContext> for CurrentRealm<'a> {
        fn as_mut(&mut self) -> &mut crate::context::JSContext {
            std::ops::DerefMut::deref_mut(self)
        }
    }
    impl<'a> std::convert::AsMut<crate::context::JSContext> for AutoRealm<'a> {
        fn as_mut(&mut self) -> &mut crate::context::JSContext {
            std::ops::DerefMut::deref_mut(self)
        }
    }

    impl<'a> From<&mut CurrentRealm<'a>> for crate::context::JSContext {
        fn from(realm: &mut CurrentRealm<'a>) -> Self {
            unsafe { crate::context::JSContext::from_raw_ptr(realm.raw_cx()) }
        }
    }

    impl<'a> From<&mut AutoRealm<'a>> for crate::context::JSContext {
        fn from(realm: &mut AutoRealm<'a>) -> Self {
            unsafe { crate::context::JSContext::from_raw_ptr(realm.raw_cx()) }
        }
    }
}

// ── Error handling ──────────────────────────────────────────────────────────

pub mod error {
    use super::jsapi;

    pub fn throw_range_error<C, M>(_cx: C, _msg: M) {}
    pub fn throw_type_error<C, M>(_cx: C, _msg: M) {}
}

// ── Panic ───────────────────────────────────────────────────────────────────

pub mod panic {
    pub fn maybe_resume_unwind() {}
    pub fn wrap_panic(f: &mut dyn FnMut()) {
        f();
    }
}

// ── Conversions ─────────────────────────────────────────────────────────────

pub mod conversions {
    use super::jsapi;

    pub use super::rust::conversions::{FromJSValConvertible, ToJSValConvertible};

    #[derive(Clone, Copy)]
    pub enum ConversionBehavior {
        Default,
        EnforceRange,
        Clamp,
    }
    impl Default for ConversionBehavior {
        fn default() -> Self {
            Self::Default
        }
    }
    pub enum ConversionResult<T> {
        Success(T),
        Failure(std::borrow::Cow<'static, std::ffi::CStr>),
    }
    impl<T> ConversionResult<T> {
        pub fn get_success_value(self) -> Option<T> {
            match self {
                Self::Success(value) => Some(value),
                Self::Failure(_) => None,
            }
        }
    }

    pub fn jsstr_to_string<C, S>(_cx: C, _s: S) -> String {
        String::new()
    }
    pub fn latin1_to_string<C, S>(_cx: C, _s: S) -> String {
        String::new()
    }
    pub struct Utf8Chars {
        ptr: *const u8,
        len: usize,
    }
    impl Utf8Chars {
        pub fn from<S>(_s: S) -> Self {
            Self {
                ptr: std::ptr::null(),
                len: 0,
            }
        }
        pub fn as_ptr(&self) -> *const u8 {
            self.ptr
        }
        pub fn len(&self) -> usize {
            self.len
        }
    }
    pub unsafe fn ToString(_cx: *mut jsapi::JSContext, _val: jsapi::JSVal) -> String {
        String::new()
    }

    pub trait FromJSValConvertibleRc: Sized {
        unsafe fn from_jsval(
            _cx: *mut jsapi::JSContext,
            _value: jsapi::HandleValue,
        ) -> Result<ConversionResult<std::rc::Rc<Self>>, ()>;

        fn safe_from_jsval(
            cx: &mut crate::context::JSContext,
            value: jsapi::HandleValue,
        ) -> Result<ConversionResult<std::rc::Rc<Self>>, ()> {
            unsafe { Self::from_jsval(cx.raw_cx(), value) }
        }
    }

    macro_rules! primitive_to_jsval {
        ($($ty:ty),* $(,)?) => {
            $(
                impl ToJSValConvertible for $ty {
                    unsafe fn to_jsval(&self, _cx: *mut jsapi::JSContext, mut rval: jsapi::MutableHandleValue) {
                        rval.set(jsapi::JSVal::default());
                    }
                }
            )*
        };
    }
    primitive_to_jsval!(
        (),
        bool,
        f32,
        f64,
        i8,
        i16,
        i32,
        i64,
        isize,
        u8,
        u16,
        u32,
        u64,
        usize,
        String
    );

    macro_rules! primitive_from_jsval {
        ($($ty:ty => $config:ty),* $(,)?) => {
            $(
                impl FromJSValConvertible for $ty {
                    type Config = $config;
                    unsafe fn from_jsval(_cx: *mut jsapi::JSContext, _val: jsapi::HandleValue, _option: Self::Config) -> Result<ConversionResult<Self>, ()> {
                        Ok(ConversionResult::Success(Default::default()))
                    }
                }
            )*
        };
    }
    primitive_from_jsval!(
        bool => (),
        f32 => (),
        f64 => (),
        i8 => ConversionBehavior,
        i16 => ConversionBehavior,
        i32 => ConversionBehavior,
        i64 => ConversionBehavior,
        isize => ConversionBehavior,
        u8 => ConversionBehavior,
        u16 => ConversionBehavior,
        u32 => ConversionBehavior,
        u64 => ConversionBehavior,
        usize => ConversionBehavior,
        String => (),
    );

    impl ToJSValConvertible for &str {
        unsafe fn to_jsval(&self, _cx: *mut jsapi::JSContext, mut rval: jsapi::MutableHandleValue) {
            rval.set(jsapi::JSVal::default());
        }
    }

    impl ToJSValConvertible for *mut jsapi::JSObject {
        unsafe fn to_jsval(&self, _cx: *mut jsapi::JSContext, mut rval: jsapi::MutableHandleValue) {
            rval.set(jsapi::JSVal::default());
        }
    }

    impl ToJSValConvertible for jsapi::JSVal {
        unsafe fn to_jsval(&self, _cx: *mut jsapi::JSContext, mut rval: jsapi::MutableHandleValue) {
            rval.set(*self);
        }
    }

    impl<T> ToJSValConvertible for std::ptr::NonNull<T> {
        unsafe fn to_jsval(&self, _cx: *mut jsapi::JSContext, mut rval: jsapi::MutableHandleValue) {
            rval.set(jsapi::JSVal::default());
        }
    }

    impl ToJSValConvertible for jsapi::Handle<'_, *mut jsapi::JSObject> {
        unsafe fn to_jsval(&self, _cx: *mut jsapi::JSContext, mut rval: jsapi::MutableHandleValue) {
            rval.set(jsapi::JSVal::default());
        }
    }
    impl ToJSValConvertible for jsapi::Handle<'_, jsapi::JSVal> {
        unsafe fn to_jsval(&self, _cx: *mut jsapi::JSContext, mut rval: jsapi::MutableHandleValue) {
            rval.set(self.get());
        }
    }
    impl<T: ToJSValConvertible> ToJSValConvertible for [T] {
        unsafe fn to_jsval(&self, _cx: *mut jsapi::JSContext, mut rval: jsapi::MutableHandleValue) {
            rval.set(jsapi::JSVal::default());
        }
    }
    impl<T: ToJSValConvertible> ToJSValConvertible for Vec<T> {
        unsafe fn to_jsval(&self, cx: *mut jsapi::JSContext, rval: jsapi::MutableHandleValue) {
            unsafe { <[T]>::to_jsval(self, cx, rval) }
        }
    }
    impl<T: ToJSValConvertible> ToJSValConvertible for Box<T> {
        unsafe fn to_jsval(&self, cx: *mut jsapi::JSContext, rval: jsapi::MutableHandleValue) {
            unsafe { (**self).to_jsval(cx, rval) }
        }
    }

    impl<T: ToJSValConvertible> ToJSValConvertible for Option<T> {
        unsafe fn to_jsval(&self, cx: *mut jsapi::JSContext, rval: jsapi::MutableHandleValue) {
            if let Some(value) = self {
                unsafe { value.to_jsval(cx, rval) }
            }
        }
    }
    impl<T: ToJSValConvertible> ToJSValConvertible for &T {
        unsafe fn to_jsval(&self, cx: *mut jsapi::JSContext, rval: jsapi::MutableHandleValue) {
            unsafe { (*self).to_jsval(cx, rval) }
        }
    }
    impl<T: ToJSValConvertible> ToJSValConvertible for std::rc::Rc<T> {
        unsafe fn to_jsval(&self, cx: *mut jsapi::JSContext, rval: jsapi::MutableHandleValue) {
            unsafe { self.as_ref().to_jsval(cx, rval) }
        }
    }
    impl<T: ToJSValConvertible> ToJSValConvertible for crate::gc::RootedGuard<'_, T> {
        unsafe fn to_jsval(&self, cx: *mut jsapi::JSContext, rval: jsapi::MutableHandleValue) {
            unsafe { std::ops::Deref::deref(self).to_jsval(cx, rval) }
        }
    }
    impl<T> FromJSValConvertible for Vec<T>
    where
        T: FromJSValConvertible,
    {
        type Config = T::Config;
        unsafe fn from_jsval(
            _cx: *mut jsapi::JSContext,
            _val: jsapi::HandleValue,
            _option: Self::Config,
        ) -> Result<ConversionResult<Self>, ()> {
            Ok(ConversionResult::Success(Vec::new()))
        }
    }
    impl<T> FromJSValConvertible for Option<T>
    where
        T: FromJSValConvertible,
    {
        type Config = T::Config;
        unsafe fn from_jsval(
            cx: *mut jsapi::JSContext,
            val: jsapi::HandleValue,
            option: Self::Config,
        ) -> Result<ConversionResult<Self>, ()> {
            match T::from_jsval(cx, val, option)? {
                ConversionResult::Success(value) => Ok(ConversionResult::Success(Some(value))),
                ConversionResult::Failure(message) => Ok(ConversionResult::Failure(message)),
            }
        }
    }
    impl<T> FromJSValConvertible for std::rc::Rc<T> {
        type Config = ();
        unsafe fn from_jsval(
            _cx: *mut jsapi::JSContext,
            _val: jsapi::HandleValue,
            _option: Self::Config,
        ) -> Result<ConversionResult<Self>, ()> {
            Ok(ConversionResult::Failure(c"Failed to convert Rc".into()))
        }
    }
    impl FromJSValConvertible for jsapi::JSVal {
        type Config = ();
        unsafe fn from_jsval(
            _cx: *mut jsapi::JSContext,
            val: jsapi::HandleValue,
            _option: Self::Config,
        ) -> Result<ConversionResult<Self>, ()> {
            Ok(ConversionResult::Success(val.get()))
        }
    }
    impl ToJSValConvertible for jsapi::Heap<jsapi::JSVal> {
        unsafe fn to_jsval(&self, _cx: *mut jsapi::JSContext, mut rval: jsapi::MutableHandleValue) {
            rval.set(self.get());
        }
    }
    impl ToJSValConvertible for jsapi::Heap<*mut jsapi::JSObject> {
        unsafe fn to_jsval(&self, _cx: *mut jsapi::JSContext, mut rval: jsapi::MutableHandleValue) {
            rval.set(jsapi::JSVal::default());
        }
    }
    impl FromJSValConvertible for *mut jsapi::JSObject {
        type Config = ();
        unsafe fn from_jsval(
            _cx: *mut jsapi::JSContext,
            _val: jsapi::HandleValue,
            _option: (),
        ) -> Result<ConversionResult<Self>, ()> {
            Ok(ConversionResult::Success(std::ptr::null_mut()))
        }
    }
    impl<T, O> FromJSValConvertible for super::typedarray::TypedArray<T, O>
    where
        T: super::typedarray::TypedArrayElement,
    {
        type Config = ();
        unsafe fn from_jsval(
            _cx: *mut jsapi::JSContext,
            _val: jsapi::HandleValue,
            _option: (),
        ) -> Result<ConversionResult<Self>, ()> {
            Ok(ConversionResult::Success(
                super::typedarray::TypedArray::from(std::ptr::null_mut()).unwrap(),
            ))
        }
    }
}

// ── JSVal ───────────────────────────────────────────────────────────────────

pub mod jsval {
    use std::ptr;

    pub use super::conversions;

    pub type JSVal = super::jsapi::JSVal;

    pub const UndefinedValue: fn() -> JSVal = || JSVal::undefined();
    pub const NullValue: fn() -> JSVal = || JSVal::null();
    pub const TrueValue: fn() -> JSVal = || JSVal::from_bool(true);
    pub const FalseValue: fn() -> JSVal = || JSVal::from_bool(false);
    pub fn ObjectValue(obj: *const super::jsapi::JSObject) -> JSVal {
        JSVal::from_object(obj)
    }
    pub fn ObjectOrNullValue(obj: *const super::jsapi::JSObject) -> JSVal {
        if obj.is_null() {
            JSVal::null()
        } else {
            JSVal::from_object(obj)
        }
    }
    pub fn PrivateValue(p: *const std::ffi::c_void) -> JSVal {
        JSVal::from_private(p)
    }
    pub fn BooleanValue(b: bool) -> JSVal {
        JSVal::from_bool(b)
    }
    pub fn DoubleValue(d: f64) -> JSVal {
        // ponytail: no NaN-box double tag; use int when exact, else private bits of f64.
        if d.is_finite() && d == (d as i32 as f64) && (i32::MIN as f64..=i32::MAX as f64).contains(&d)
        {
            JSVal::from_int32(d as i32)
        } else {
            JSVal::from_private(d.to_bits() as *const std::ffi::c_void)
        }
    }
    pub fn Int32Value(i: i32) -> JSVal {
        JSVal::from_int32(i)
    }
    pub fn UInt32Value(u: u32) -> JSVal {
        JSVal::from_uint32(u)
    }
    pub trait ToStringPtr {
        fn to_string_ptr(self) -> *mut super::jsapi::JSString;
    }

    impl ToStringPtr for *mut super::jsapi::JSString {
        fn to_string_ptr(self) -> *mut super::jsapi::JSString {
            self
        }
    }

    impl ToStringPtr for *const super::jsapi::JSString {
        fn to_string_ptr(self) -> *mut super::jsapi::JSString {
            self as *mut super::jsapi::JSString
        }
    }

    impl ToStringPtr for &super::jsapi::JSString {
        fn to_string_ptr(self) -> *mut super::jsapi::JSString {
            self as *const super::jsapi::JSString as *mut super::jsapi::JSString
        }
    }

    pub fn StringValue<S: ToStringPtr>(s: S) -> JSVal {
        JSVal::from_string(s.to_string_ptr())
    }
    pub fn NumberValue(n: f64) -> JSVal {
        DoubleValue(n)
    }

    pub mod glue {
        use super::super::jsapi;
        use super::*;
        pub fn IsWrapper(_obj: *mut super::super::jsapi::JSObject) -> bool {
            false
        }
    }
}

// ── Typed arrays ────────────────────────────────────────────────────────────

pub mod typedarray {
    use super::conversions::ToJSValConvertible;
    use super::jsapi;
    use std::marker::PhantomData;

    fn elements_to_bytes<T: Copy>(values: &[T]) -> &[u8] {
        let len = std::mem::size_of_val(values);
        // SAFETY: a slice of Copy primitives may be viewed as its initialized byte representation.
        unsafe { std::slice::from_raw_parts(values.as_ptr() as *const u8, len) }
    }

    fn bytes_to_elements<T: Copy>(bytes: &[u8]) -> Vec<T> {
        let size = std::mem::size_of::<T>();
        if size == 0 || bytes.len() % size != 0 {
            return Vec::new();
        }
        bytes
            .chunks_exact(size)
            .map(|chunk| {
                // SAFETY: chunk has exactly size_of::<T>() bytes; read_unaligned accepts unaligned input.
                unsafe { std::ptr::read_unaligned(chunk.as_ptr() as *const T) }
            })
            .collect()
    }

    macro_rules! typed_array_element {
        ($t:ident, $element:ty, $array_type:expr) => {
            pub struct $t;
            impl TypedArrayElement for $t {
                type Element = $element;
                fn array_type() -> jsapi::Type {
                    $array_type
                }
            }
            impl TypedArrayElementCreator for $t {}
            unsafe impl super::gc::Traceable for $t {}
        };
    }

    typed_array_element!(Uint8, u8, jsapi::Type::Uint8);
    typed_array_element!(Uint16, u16, jsapi::Type::Uint16);
    typed_array_element!(Uint32, u32, jsapi::Type::Uint32);
    typed_array_element!(Int8, i8, jsapi::Type::Int8);
    typed_array_element!(Int16, i16, jsapi::Type::Int16);
    typed_array_element!(Int32, i32, jsapi::Type::Int32);
    typed_array_element!(Float32, f32, jsapi::Type::Float32);
    typed_array_element!(Float64, f64, jsapi::Type::Float64);
    typed_array_element!(ClampedU8, u8, jsapi::Type::Uint8Clamped);
    typed_array_element!(ArrayBufferU8, u8, jsapi::Type::Uint8);
    typed_array_element!(ArrayBufferViewU8, u8, jsapi::Type::Uint8);

    macro_rules! array_alias {
        ($arr:ident, $heap_arr:ident, $elem:ty) => {
            pub type $arr = TypedArray<$elem, *mut jsapi::JSObject>;
            pub type $heap_arr = TypedArray<$elem, Box<jsapi::Heap<*mut jsapi::JSObject>>>;
        };
    }

    array_alias!(Uint8ClampedArray, HeapUint8ClampedArray, ClampedU8);
    array_alias!(Uint8Array, HeapUint8Array, Uint8);
    array_alias!(Int8Array, HeapInt8Array, Int8);
    array_alias!(Uint16Array, HeapUint16Array, Uint16);
    array_alias!(Int16Array, HeapInt16Array, Int16);
    array_alias!(Uint32Array, HeapUint32Array, Uint32);
    array_alias!(Int32Array, HeapInt32Array, Int32);
    array_alias!(Float32Array, HeapFloat32Array, Float32);
    array_alias!(Float64Array, HeapFloat64Array, Float64);
    array_alias!(ArrayBuffer, HeapArrayBuffer, ArrayBufferU8);
    array_alias!(ArrayBufferView, HeapArrayBufferView, ArrayBufferViewU8);
    impl jsapi::Type {
        pub fn byte_size(&self) -> Option<usize> {
            match self {
                jsapi::Type::Int8 | jsapi::Type::Uint8 | jsapi::Type::Uint8Clamped => Some(1),
                jsapi::Type::Int16 | jsapi::Type::Uint16 | jsapi::Type::Float16 => Some(2),
                jsapi::Type::Int32 | jsapi::Type::Uint32 | jsapi::Type::Float32 => Some(4),
                jsapi::Type::Float64 | jsapi::Type::BigInt64 | jsapi::Type::BigUint64 => Some(8),
                jsapi::Type::Int64 | jsapi::Type::Simd128 | jsapi::Type::MaxTypedArrayViewType => {
                    None
                },
            }
        }
    }
    pub struct TypedArray<T: TypedArrayElement, O = *mut jsapi::JSObject> {
        object: *mut jsapi::JSObject,
        data: Vec<T::Element>,
        _phantom: PhantomData<O>,
    }
    impl<T: TypedArrayElement, O> TypedArray<T, O> {
        pub fn from(obj: *mut jsapi::JSObject) -> Result<Self, ()> {
            #[cfg(feature = "v8")]
            let data = bytes_to_elements(&crate::v8_glue::array_view_bytes(obj));
            #[cfg(not(feature = "v8"))]
            let data = Vec::new();
            Ok(Self {
                object: obj,
                data,
                _phantom: PhantomData,
            })
        }
        pub fn create<C, R>(_cx: C, with: CreateWith<'_, T::Element>, mut rval: R) -> Result<(), ()>
        where
            R: jsapi::SetJsapiObjectOut,
        {
            #[cfg(feature = "v8")]
            {
                let obj = match with {
                    CreateWith::Slice(values) => crate::v8_glue::new_typed_array_from_bytes(
                        elements_to_bytes(values),
                        T::array_type(),
                    ),
                    CreateWith::Length(len) => {
                        let byte_len = len.saturating_mul(std::mem::size_of::<T::Element>());
                        crate::v8_glue::new_typed_array_from_bytes(
                            &vec![0; byte_len],
                            T::array_type(),
                        )
                    },
                };
                if obj.is_null() {
                    Err(())
                } else {
                    rval.set_jsapi_object_out(obj);
                    Ok(())
                }
            }
            #[cfg(not(feature = "v8"))]
            {
                let _ = (with, &mut rval);
                Ok(())
            }
        }
        pub fn underlying_object(&self) -> *const *mut jsapi::JSObject {
            &self.object
        }
        pub fn object_ref(&self) -> &*mut jsapi::JSObject {
            &self.object
        }
        pub fn object(&self) -> *mut jsapi::JSObject {
            self.object
        }
        pub fn len(&self) -> usize {
            self.data.len()
        }
        pub fn as_slice(&self) -> &[T::Element] {
            &self.data
        }
        pub fn as_mut_slice(&mut self) -> &mut [T::Element] {
            &mut self.data
        }
        pub fn update(&mut self, value: &[T::Element]) {
            self.data.clear();
            self.data.extend_from_slice(value);
        }
        pub fn to_vec(&self) -> Vec<T::Element>
        where
            T::Element: Clone,
        {
            self.data.clone()
        }
        pub fn get_array_type(&self) -> jsapi::Type {
            T::array_type()
        }
    }
    impl<T: TypedArrayElement, O> ToJSValConvertible for TypedArray<T, O> {
        unsafe fn to_jsval(&self, _cx: *mut jsapi::JSContext, mut rval: jsapi::MutableHandleValue) {
            rval.set(jsapi::JSVal::default());
        }
    }
    pub trait TypedArrayElement {
        type Element: Copy;
        fn array_type() -> jsapi::Type;
    }
    pub trait TypedArrayElementCreator: TypedArrayElement {}
    pub enum CreateWith<'a, T> {
        Slice(&'a [T]),
        Length(usize),
    }
}

// ── Misc modules ────────────────────────────────────────────────────────────

pub mod jsid {
    #[derive(Copy, Clone)]
    pub struct SymbolId {
        pub asBits_: u64,
    }
    #[derive(Copy, Clone)]
    pub struct StringId {
        pub ptr: *mut super::jsapi::JSString,
    }

    pub trait ToStringIdPtr {
        fn to_string_id_ptr(self) -> *mut super::jsapi::JSString;
    }

    impl ToStringIdPtr for *mut super::jsapi::JSString {
        fn to_string_id_ptr(self) -> *mut super::jsapi::JSString {
            self
        }
    }

    impl ToStringIdPtr for *const super::jsapi::JSString {
        fn to_string_id_ptr(self) -> *mut super::jsapi::JSString {
            self as *mut super::jsapi::JSString
        }
    }

    pub fn SymbolId(_value: super::jsapi::JSVal) -> super::jsapi::jsid {
        super::jsapi::jsid(0)
    }
    pub fn StringId<S: ToStringIdPtr>(value: S) -> StringId {
        StringId {
            ptr: value.to_string_id_ptr(),
        }
    }
}
impl<'a> From<jsapi::Handle<'a, jsid::StringId>> for jsapi::jsid {
    fn from(handle: jsapi::Handle<'a, jsid::StringId>) -> Self {
        jsapi::jsid::from_string(handle.get().ptr)
    }
}

pub mod realm {
    pub use super::realms::*;
}

pub mod record {
    pub struct JsRecord;
}
pub mod guard {
    pub struct CustomAutoRooter;
}
pub mod finalize {
    use super::jsapi;
    pub type FinalizationRegistryObject = *mut jsapi::JSObject;
}
pub mod weakref {
    use super::jsapi;
    pub type WeakRefObject = *mut jsapi::JSObject;
}
pub mod interface {
    pub struct JsInterface;
}
pub mod constructor {
    pub struct JsConstructor;
}

pub const JSCLASS_GLOBAL_SLOT_COUNT: u32 = 4;
pub const JSCLASS_IS_DOMJSCLASS: u32 = 1 << 4;
pub const JSCLASS_IS_GLOBAL: u32 = 1 << 5;
pub const JSCLASS_IS_PROXY: u32 = 1 << 3;
pub const JSCLASS_RESERVED_SLOTS_MASK: u32 = 0xff << 8;
pub const JSCLASS_USERBIT1: u32 = 1 << 14;
pub fn JS_CALLEE<C>(_cx: C, _vp: *mut jsapi::JSVal) -> jsapi::JSVal {
    jsapi::JSVal::default()
}

pub mod js {
    pub use crate::{
        JS_CALLEE, JSCLASS_IS_DOMJSCLASS, JSCLASS_IS_GLOBAL, JSCLASS_RESERVED_SLOTS_MASK,
    };
}
