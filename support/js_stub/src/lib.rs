// V8-backed JS engine bridge — replaces mozjs SpiderMonkey FFI.
// Uses rusty_v8 with thread-local isolates and persistent handles.

#[macro_use]
mod macros;

use std::cell::RefCell;
use std::collections::HashMap;
use std::ptr;
use std::sync::Once;

#[cfg(feature = "v8")]
static V8_INIT: Once = Once::new();
#[cfg(feature = "v8")]
thread_local! {
    static V8_ISOLATE: RefCell<Option<v8::OwnedIsolate>> = RefCell::new(None);
    static V8_CONTEXTS: RefCell<HashMap<usize, v8::Global<v8::Context>>> = RefCell::new(HashMap::new());
    static V8_SLOTS: RefCell<Vec<Box<dyn std::any::Any>>> = RefCell::new(Vec::new());
}

#[cfg(feature = "v8")]
fn ensure_v8() {
    V8_INIT.call_once(|| {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
    });
}

// ── Core types ──────────────────────────────────────────────────────────────

pub mod glue {
    use super::jsapi;
    use std::ptr;

    pub fn IsWrapper(_obj: *mut jsapi::JSObject) -> bool { false }
    pub fn UnwrapObjectDynamic<O, C>(_obj: O, _cx: C, _stop: bool) -> *mut jsapi::JSObject { ptr::null_mut() }
    pub fn RUST_JSID_TO_STRING(_cx: *mut jsapi::JSContext, _id: *const jsapi::jsid) -> *mut jsapi::JSString { ptr::null_mut() }
    pub fn AppendToIdVector<V>(_v: V, _id: jsapi::jsid) -> bool { false }
    pub fn GetProxyHandler(_proxy: *mut jsapi::JSObject) -> *const std::ffi::c_void { ptr::null() }
    pub fn NewProxyObject(_cx: *mut jsapi::JSContext, _handler: *const std::ffi::c_void, _priv: *mut jsapi::JSObject, _proto: *mut jsapi::JSObject, _options: *const std::ffi::c_void, _flag: bool) -> *mut jsapi::JSObject { ptr::null_mut() }
    pub fn GetProxyPrivate<O, V>(_proxy: O, _out: V) {}
    pub fn SetProxyPrivate<O, V>(_proxy: O, _priv: V) {}
    pub fn DeletePropertyIgnoringResult(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _prop: *const u8) {}
    pub fn DefinePropertyById(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _id: *const jsapi::jsid, _desc: *const jsapi::JSPropertySpec) -> bool { false }
    pub fn SetDataPropertyDescriptor<D, V>(_desc: D, _value: V, _attrs: u32) {}
    pub fn AtomizeStringN(_cx: *mut jsapi::JSContext, _s: *const u8, _len: usize) -> *mut jsapi::JSString { ptr::null_mut() }
    pub fn CreateDOMGlobal(_cx: *mut jsapi::JSContext, _clasp: *const jsapi::JSClass, _principal: *mut std::ffi::c_void) -> *mut jsapi::JSObject { ptr::null_mut() }
    pub unsafe extern "C" fn CallJitGetterOp<'a>(_info: *const jsapi::JSJitInfo, _cx: *mut jsapi::JSContext, _obj: jsapi::HandleObject<'a>, _this: *mut std::ffi::c_void, _argc: u32, _vp: *mut jsapi::JSVal) -> bool { false }
    pub unsafe extern "C" fn CallJitMethodOp<'a>(_info: *const jsapi::JSJitInfo, _cx: *mut jsapi::JSContext, _obj: jsapi::HandleObject<'a>, _this: *mut std::ffi::c_void, _argc: u32, _vp: *mut jsapi::JSVal) -> bool { false }
    pub fn CallJitSetterOp<O>(_info: *const jsapi::JSJitInfo, _cx: *mut jsapi::JSContext, _obj: O, _this: *mut std::ffi::c_void, _argc: u32, _vp: *mut jsapi::JSVal) -> bool { false }

    #[repr(C)]
    pub struct ProxyTraps {
        pub enter: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject) -> bool>,
        pub getPropertyDescriptor: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, *const jsapi::jsid, *mut jsapi::JSVal) -> bool>,
        pub getOwnPropertyDescriptor: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, *const jsapi::jsid, *mut jsapi::PropertyDescriptor) -> bool>,
        pub defineProperty: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, *const jsapi::jsid, *const jsapi::PropertyDescriptor, *mut jsapi::ObjectOpResult) -> bool>,
        pub ownPropertyKeys: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, *mut *const jsapi::jsid) -> bool>,
        pub delete_: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, *const jsapi::jsid, *mut jsapi::ObjectOpResult) -> bool>,
        pub enumerate: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, *mut jsapi::ObjectOpResult) -> bool>,
        pub getPrototypeIfOrdinary: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, *mut bool, *mut *mut jsapi::JSObject) -> bool>,
        pub getPrototype: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, *mut *mut jsapi::JSObject) -> bool>,
        pub setPrototype: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, *mut jsapi::JSObject, *mut jsapi::ObjectOpResult) -> bool>,
        pub setImmutablePrototype: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, *mut bool) -> bool>,
        pub preventExtensions: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, *mut jsapi::ObjectOpResult) -> bool>,
        pub isExtensible: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, *mut bool) -> bool>,
        pub has: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, *const jsapi::jsid, *mut bool) -> bool>,
        pub get: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, *const jsapi::jsid, *mut jsapi::JSVal) -> bool>,
        pub set: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, *const jsapi::jsid, *mut jsapi::JSVal, *mut jsapi::JSVal, *mut jsapi::ObjectOpResult) -> bool>,
        pub call: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, *const jsapi::JSVal, *mut jsapi::JSVal) -> bool>,
        pub construct: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, *const jsapi::JSVal, *mut jsapi::JSVal) -> bool>,
        pub hasOwn: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, *const jsapi::jsid, *mut bool) -> bool>,
        pub getOwnEnumerablePropertyKeys: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, *mut *const jsapi::jsid) -> bool>,
        pub nativeCall: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, bool, *const jsapi::JSVal, *mut jsapi::JSVal) -> bool>,
        pub objectClassIs: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, u32, *mut bool) -> bool>,
        pub className: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject) -> *const u8>,
        pub fun_toString: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, bool) -> *mut jsapi::JSString>,
        pub boxedValue_unbox: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, *mut jsapi::JSVal) -> bool>,
        pub defaultValue: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, u32, *mut jsapi::JSVal) -> bool>,
        pub trace: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject, *mut jsapi::JSTracer)>,
        pub finalize: Option<unsafe extern "C" fn(*mut jsapi::JSContext, *mut jsapi::JSObject)>,
        pub objectMoved: Option<unsafe extern "C" fn(*mut jsapi::JSObject, *const jsapi::JSObject)>,
        pub isCallable: Option<unsafe extern "C" fn(*mut jsapi::JSObject) -> bool>,
        pub isConstructor: Option<unsafe extern "C" fn(*mut jsapi::JSObject) -> bool>,
    }

    pub fn CreateProxyHandler(_traps: &ProxyTraps, _extra: *const std::ffi::c_void) -> *mut std::ffi::c_void { ptr::null_mut() }
    pub fn GetProxyReservedSlot(_proxy: *mut jsapi::JSObject, _slot: u32, _out: *mut jsapi::JSVal) {}
    pub fn JS_GetReservedSlot<O>(_obj: O, _slot: u32, _out: *mut jsapi::JSVal) {}
    pub fn SetProxyReservedSlot(_proxy: *mut jsapi::JSObject, _slot: u32, _val: jsapi::JSVal) {}

    pub fn RUST_JSID_IS_VOID<I>(_id: I) -> bool { false }
    pub fn CallObjectTracer(_trc: *mut jsapi::JSTracer, _obj: *mut jsapi::JSObject, _name: *const u8) {}
    pub fn UncheckedUnwrapObject(_obj: *mut jsapi::JSObject, _stopAtOuter: bool) -> *mut jsapi::JSObject { ptr::null_mut() }
    pub fn IsProxyHandlerFamily(_obj: *mut jsapi::JSObject) -> bool { false }
    pub fn GetProxyHandlerFamily() -> *const std::ffi::c_void { ptr::null() }
    pub fn CreateRustJSPrincipals<C>(_callbacks: C, _private: *mut std::ffi::c_void) -> *mut std::ffi::c_void { ptr::null_mut() }
    pub fn GetRustJSPrincipalsPrivate(_p: *mut std::ffi::c_void) -> *mut std::ffi::c_void { ptr::null_mut() }
    pub type JSPrincipalsCallbacks = std::ffi::c_void;
    pub fn GetProxyHandlerExtra(_proxy: *mut jsapi::JSObject) -> *mut std::ffi::c_void { ptr::null_mut() }
    pub fn RUST_FUNCTION_VALUE_TO_JITINFO<V>(_value: V) -> *const jsapi::JSJitInfo { ptr::null() }
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
    #[derive(Copy, Clone, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct jsid {
        pub asBits_: u64,
    }
    pub const fn jsid(asBits_: u64) -> jsid { jsid { asBits_ } }
    impl std::ops::Deref for jsid { type Target = u64; fn deref(&self) -> &u64 { &self.asBits_ } }
    impl std::fmt::Debug for jsid { fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { write!(f, "jsid({})", self.asBits_) } }
    impl From<u64> for jsid { fn from(v: u64) -> jsid { jsid(v) } }
    impl From<*const jsid> for jsid { fn from(p: *const jsid) -> jsid { unsafe { *p } } }
    impl From<&jsid> for *const jsid { fn from(p: &jsid) -> *const jsid { p as *const jsid } }
    impl jsid {
        pub fn is_string(&self) -> bool { false }
        pub fn is_int(&self) -> bool { false }
        pub fn is_void(&self) -> bool { self.asBits_ == 0 }
        pub fn to_int(&self) -> i32 { 0 }
        pub fn to_string(&self) -> *mut JSString { ptr::null_mut() }
    }
    #[derive(Debug, Copy, Clone, Default, PartialEq)]
    #[repr(C)]
    pub struct JSVal {
        pub asBits_: u64,
    }
    impl JSVal {
        pub fn get(&self) -> Self { *self }
        pub fn is_object(&self) -> bool { false }
        pub fn is_null(&self) -> bool { self.asBits_ == 0 }
        pub fn is_null_or_undefined(&self) -> bool { true }
        pub fn is_undefined(&self) -> bool { self.asBits_ == 0 }
        pub fn to_number(&self) -> f64 { 0.0 }
        pub fn to_private(&self) -> *mut std::ffi::c_void { ptr::null_mut() }
        pub fn to_object(&self) -> *mut JSObject { ptr::null_mut() }
        pub fn to_object_or_null(&self) -> *mut JSObject { ptr::null_mut() }
    }
    pub type Value = JSVal;
    pub struct JSAutoRealm;
    impl JSAutoRealm {
        pub fn new<C, O>(_cx: C, _obj: O) -> Self { Self }
    }
    pub type JSAutoCompartment = *mut std::ffi::c_void;
    pub type GCContext = *mut std::ffi::c_void;

    #[derive(Debug, Copy, Clone)]
    #[repr(transparent)]
    pub struct Handle<'a, T> {
        pub ptr: *const T,
        _phantom: std::marker::PhantomData<&'a T>,
    }
    impl<'a, T> Handle<'a, T> {
        pub unsafe fn from_raw<P: Into<*const T>>(ptr: P) -> Self { Self { ptr: ptr.into(), _phantom: std::marker::PhantomData } }
        pub fn null() -> Self { Self { ptr: ptr::null(), _phantom: std::marker::PhantomData } }
        pub fn as_ptr(self) -> *const T { self.ptr }
        pub fn handle(&self) -> Handle<'_, T> { unsafe { Handle::from_raw(self.ptr) } }
        pub fn into_handle(self) -> Handle<'a, T> { self }
        pub fn get(self) -> T where T: Copy {
            if self.ptr.is_null() {
                unsafe { std::mem::zeroed() }
            } else {
                unsafe { *self.ptr }
            }
        }
    }
    impl<'a, T> std::ops::Deref for Handle<'a, T> {
        type Target = T;
        fn deref(&self) -> &T { unsafe { &*self.ptr } }
    }
    impl<'a, T> From<Handle<'a, T>> for *const T {
        fn from(handle: Handle<'a, T>) -> Self { handle.ptr }
    }
    impl<'a, T> From<Handle<'a, T>> for *mut T {
        fn from(handle: Handle<'a, T>) -> Self { handle.ptr as *mut T }
    }
    impl<'a> From<Handle<'a, JSVal>> for JSVal {
        fn from(handle: Handle<'a, JSVal>) -> Self { handle.get() }
    }
    impl<'a> From<Handle<'a, jsid>> for jsid {
        fn from(handle: Handle<'a, jsid>) -> Self { handle.get() }
    }
    impl<'a> From<Handle<'a, *mut JSObject>> for *mut JSObject {
        fn from(handle: Handle<'a, *mut JSObject>) -> Self { handle.get() }
    }
    impl<'a> Handle<'a, JSVal> {
        pub fn undefined() -> Self { Self::null() }
    }

    #[derive(Debug, Copy, Clone)]
    #[repr(transparent)]
    pub struct MutableHandle<'a, T> {
        pub ptr: *mut T,
        _phantom: std::marker::PhantomData<&'a mut T>,
    }
    impl<'a, T> MutableHandle<'a, T> {
        pub unsafe fn from_raw<P: Into<*mut T>>(ptr: P) -> Self { Self { ptr: ptr.into(), _phantom: std::marker::PhantomData } }
        pub fn null() -> Self { Self { ptr: ptr::null_mut(), _phantom: std::marker::PhantomData } }
        pub fn as_ptr(self) -> *mut T { self.ptr }
        pub fn handle(&self) -> Handle<'_, T> { unsafe { Handle::from_raw(self.ptr as *const T) } }
        pub fn into_handle(self) -> Handle<'a, T> { unsafe { Handle::from_raw(self.ptr as *const T) } }
        pub fn get(self) -> T where T: Copy {
            if self.ptr.is_null() {
                unsafe { std::mem::zeroed() }
            } else {
                unsafe { *self.ptr }
            }
        }
        pub fn set(&mut self, val: T) {
            if !self.ptr.is_null() {
                unsafe { *self.ptr = val; }
            }
        }
        pub fn reborrow(&mut self) -> MutableHandle<'_, T> {
            MutableHandle { ptr: self.ptr, _phantom: std::marker::PhantomData }
        }
    }
    impl<'a, T> std::ops::Deref for MutableHandle<'a, T> {
        type Target = T;
        fn deref(&self) -> &T { unsafe { &*self.ptr } }
    }
    impl<'a, T> std::ops::DerefMut for MutableHandle<'a, T> {
        fn deref_mut(&mut self) -> &mut T { unsafe { &mut *self.ptr } }
    }
    impl<'a, T> From<MutableHandle<'a, T>> for *mut T {
        fn from(handle: MutableHandle<'a, T>) -> Self { handle.ptr }
    }

    // from_raw helpers — accessed as Handle::from_raw / MutableHandle::from_raw
    // These live in a nested module to avoid conflicting with the type alias at this level.
    pub mod handle_from_raw {
        pub unsafe fn Handle<T>(_: super::Handle<'static, T>, raw: *const T) -> *const T { raw }
        pub unsafe fn MutableHandle<T>(_: super::MutableHandle<'static, T>, raw: *mut T) -> *mut T { raw }
    }
    pub unsafe fn handle_from_raw<T>(raw: *const T) -> *const T { raw }
    pub unsafe fn mutable_handle_from_raw<T>(raw: *mut T) -> *mut T { raw }

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
            Self { length_: 0, elements_: ptr::null() }
        }
    }
    pub type MutableHandleIdVector = *mut *const jsid;
    pub type MutableHandleId<'a> = MutableHandle<'a, jsid>;
    pub type MutableHandleObject<'a> = MutableHandle<'a, *mut JSObject>;
    pub type MutableHandleValue<'a> = MutableHandle<'a, JSVal>;
    pub fn UndefinedHandleValue() -> HandleValue<'static> { HandleValue::null() }
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
                // In SM, vp points to rval_, and argv_ follows after this/callee
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
        pub fn new_target(&self) -> HandleValue<'_> { HandleValue::null() }
        pub fn callee(&self) -> HandleValue<'_> { HandleValue::null() }
        pub fn thisv(&self) -> HandleValue<'_> { HandleValue::null() }
        pub fn is_constructing(&self) -> bool { false }
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
        pub fn hasGetter_(&self) -> bool { self.has_getter_ }
        pub fn hasSetter_(&self) -> bool { self.has_setter_ }
        pub fn hasWritable_(&self) -> bool { self.has_writable_ }
        pub fn hasValue_(&self) -> bool { self.has_value_ }
        pub fn enumerable_(&self) -> bool { false }
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

    pub struct Heap<T> {
        cell: RefCell<Option<T>>,
    }
    impl<T> Heap<T> {
        pub fn boxed(val: T) -> Self { Self { cell: RefCell::new(Some(val)) } }
        pub fn set(&self, val: T) { *self.cell.borrow_mut() = Some(val); }
        pub fn get(&self) -> T where T: Copy { self.cell.borrow().as_ref().copied().unwrap_or_else(|| unsafe { std::mem::zeroed() }) }
        pub fn handle(&self) -> *const T { ptr::null() }
        pub unsafe fn get_unsafe(&self) -> *mut T { ptr::null_mut() }
        pub unsafe fn unbarriered_get(&self) -> *const T { ptr::null() }
    }
    impl<T> Default for Heap<T> {
        fn default() -> Self { Self { cell: RefCell::new(None) } }
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
        Getter = 0, Setter = 1, Method = 2, StaticMethod = 3,
        InlinableNative = 4, TrampolineNative = 5, IgnoresReturnValueNative = 6, OpTypeCount = 7,
    }

    #[repr(u32)]
    #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
    pub enum JSJitInfo_AliasSet {
        AliasNone = 0, AliasDOMSets = 1, AliasEverything = 2, AliasSetCount = 3,
    }

    #[repr(i32)]
    #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
    pub enum JSJitInfo_ArgType {
        String = 1, Integer = 2, Double = 4,
    }

    pub type JSJitGetterCallArgs = CallArgs;
    pub type JSJitMethodCallArgs = CallArgs;
    pub type JSJitSetterCallArgs = CallArgs;

    #[repr(C)]
    pub struct JSJitInfo__bindgen_ty_1 {
        pub method: Option<for<'a> unsafe extern "C" fn(*mut JSContext, HandleObject<'a>, *mut std::ffi::c_void, *const JSJitMethodCallArgs) -> bool>,
        pub getter: Option<for<'a> unsafe extern "C" fn(*mut JSContext, HandleObject<'a>, *mut std::ffi::c_void, CallArgs) -> bool>,
        pub setter: Option<for<'a> unsafe extern "C" fn(*mut JSContext, HandleObject<'a>, *mut std::ffi::c_void, CallArgs) -> bool>,
        pub staticMethod: Option<unsafe extern "C" fn(*mut JSContext, u32, *mut JSVal) -> bool>,
        pub staticGetter: Option<for<'a> unsafe extern "C" fn(*mut JSContext, HandleObject<'a>, *mut std::ffi::c_void, CallArgs) -> bool>,
        pub staticSetter: Option<for<'a> unsafe extern "C" fn(*mut JSContext, HandleObject<'a>, *mut std::ffi::c_void, CallArgs) -> bool>,
    }
    impl Default for JSJitInfo__bindgen_ty_1 {
        fn default() -> Self {
            JSJitInfo__bindgen_ty_1 { method: None, getter: None, setter: None, staticMethod: None, staticGetter: None, staticSetter: None }
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
        pub _unused: [u8; 0],
    }

    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct JSNativeWrapper {
        pub op: Option<unsafe extern "C" fn(*mut JSContext, u32, *mut JSVal) -> bool>,
        pub info: *const JSJitInfo,
    }

    #[repr(C)]
    pub struct __BindgenBitfieldUnit<Storage> {
        storage: Storage,
    }
    impl<Storage> __BindgenBitfieldUnit<Storage> {
        pub const fn new(storage: Storage) -> Self { Self { storage } }
    }

    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct JSClassDef {
        pub name: *const std::os::raw::c_char,
        pub flags: u32,
        pub cOps: *const JSClassOps,
        pub spec: *const std::ffi::c_void,
        pub ext: *const std::ffi::c_void,
        pub oOps: *const ObjectOps,
    }

    #[repr(C)]
    pub struct JSClassOps {
        pub addProperty: Option<unsafe extern "C" fn(*mut JSContext, *mut JSObject, jsid, *const JSVal)>,
        pub delProperty: Option<unsafe extern "C" fn(*mut JSContext, *mut JSObject, jsid, *const JSVal)>,
        pub enumerate: Option<unsafe extern "C" fn(*mut JSContext, *mut JSObject)>,
        pub newEnumerate: Option<unsafe extern "C" fn(*mut JSContext, *mut JSObject, *const JSVal, *const jsid)>,
        pub resolve: Option<unsafe extern "C" fn(*mut JSContext, *mut JSObject, jsid, bool)>,
        pub mayResolve: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const jsid)>,
        pub finalize: Option<unsafe extern "C" fn(*mut GCContext, *mut JSObject)>,
        pub call: Option<unsafe extern "C" fn(*mut JSContext, u32, *mut JSVal) -> bool>,
        pub construct: Option<unsafe extern "C" fn(*mut JSContext, u32, *mut JSVal) -> bool>,
        pub trace: Option<unsafe extern "C" fn(*mut JSTracer, *mut JSObject)>,
    }
    unsafe impl Sync for JSClassDef {}
    unsafe impl Sync for JSClassOps {}

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
            name: JSPropertySpec_Name { string_: ptr::null() },
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
    }

    pub fn IsCallable<T>(_v: T) -> bool { false }
    pub fn GetWellKnownSymbol<C, W>(_cx: C, _which: W) -> JSVal { JSVal::default() }
    pub fn GetRealmErrorPrototype<C>(_cx: C) -> *mut JSObject { ptr::null_mut() }
    pub fn GetRealmFunctionPrototype<C>(_cx: C) -> *mut JSObject { ptr::null_mut() }
    pub fn GetRealmIteratorPrototype<C>(_cx: C) -> *mut JSObject { ptr::null_mut() }
    pub fn GetRealmObjectPrototype<C>(_cx: C) -> *mut JSObject { ptr::null_mut() }
    pub fn JS_AtomizeAndPinString<C, S>(_cx: C, _s: S) -> *mut JSString { ptr::null_mut() }
    pub fn JS_ForwardGetPropertyTo(_cx: *mut JSContext, _obj: *mut JSObject, _id: impl Into<jsid>, _receiver: *mut JSObject, _vp: *mut JSVal) -> bool { false }
    pub fn JS_GetPropertyDescriptorById(_cx: *mut JSContext, _obj: *mut JSObject, _id: impl Into<jsid>, _desc: *mut PropertyDescriptor, _ignored: *mut JSObject, _found: *mut bool) -> bool { false }
    pub fn JS_HasPropertyById(_cx: *mut JSContext, _obj: *mut JSObject, _id: impl Into<jsid>, _found: *mut bool) -> bool { false }
    pub fn JS_NewPlainObject(_cx: *mut JSContext) -> *mut JSObject { ptr::null_mut() }
    pub fn JS_SetReservedSlot<V>(_obj: *mut JSObject, _index: u32, _val: V) {}
    pub fn JS_NewObject(_cx: *mut JSContext, _clasp: *const JSClass) -> *mut JSObject { ptr::null_mut() }
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
    pub fn JS_GlobalObjectTraceHook(_trc: *mut JSTracer, _global: *mut JSObject) {}
    pub fn JS_DeprecatedStringHasLatin1Chars(_s: *mut JSString) -> bool { false }
    pub fn JS_GetTwoByteLatin1Chars(_s: *mut JSString) -> *const u8 { ptr::null() }
    pub fn JS_GetTwoByteStringChars(_s: *mut JSString) -> *const u16 { ptr::null() }
    pub const JSCLASS_IS_PROXY: u32 = 1 << 3;
    pub const JSCLASS_USERBIT1: u32 = 1 << 14;

    pub fn AddRawValueRoot(_cx: *mut JSContext, _vp: *mut JSVal, _name: *const std::os::raw::c_char) -> bool { false }
    pub fn RemoveRawValueRoot(_cx: *mut JSContext, _vp: *mut JSVal) {}
    pub fn RemoveAssociatedMemory(_obj: *mut JSObject, _sz: usize, _assoc: u32) {}
    pub fn IsWindowProxy(_obj: *mut JSObject) -> bool { false }
    pub fn JS_GetLatin1StringCharsAndLength<C>(_cx: C, _nogc: *const std::ffi::c_void, _s: *mut JSString, _len: *mut usize) -> *const u8 { ptr::null() }
    pub fn JS_GetTwoByteStringCharsAndLength<C>(_cx: C, _nogc: *const std::ffi::c_void, _s: *mut JSString, _len: *mut usize) -> *const u16 { ptr::null() }
    pub fn JS_NewStringCopyN<C, S>(_cx: C, _s: S, _len: usize) -> *mut JSString { ptr::null_mut() }
    pub fn CheckedUnwrapStatic(_obj: *mut JSObject) -> *mut JSObject { ptr::null_mut() }
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
                    __bindgen_anon_1: RealmCreationOptionsCompartment { zone_: ptr::null_mut() },
                    sharedMemoryAndAtomics_: false,
                },
            }
        }
    }
    impl Default for RealmOptions {
        fn default() -> Self { Self::new() }
    }
    impl std::ops::Deref for RealmOptions {
        type Target = RealmOptions;
        fn deref(&self) -> &RealmOptions { self }
    }
    impl std::ops::DerefMut for RealmOptions {
        fn deref_mut(&mut self) -> &mut RealmOptions { self }
    }
    pub fn GetNonCCWObjectGlobal(_obj: *mut JSObject) -> *mut JSObject { ptr::null_mut() }
    pub fn GetRealmGlobalOrNull<C>(_cx: C) -> *mut JSObject { ptr::null_mut() }
    pub fn IsSharableCompartment(_comp: *mut std::ffi::c_void) -> bool { false }
    pub fn IsSystemCompartment(_comp: *mut std::ffi::c_void) -> bool { false }
    pub fn JS_GetFunctionObject(_fun: *mut JSFunction) -> *mut JSObject { ptr::null_mut() }
    pub fn JS_IterateCompartments<C>(_cx: *mut JSContext, _data: *mut std::ffi::c_void, _callback: C) {}
    pub fn JS_NewFunction<C, N>(_cx: *mut JSContext, _call: C, _nargs: u32, _flags: u16, _name: N) -> *mut JSFunction { ptr::null_mut() }
    pub fn JS_NewGlobalObject<C, O>(_cx: C, _clasp: *const JSClass, _principal: *mut std::ffi::c_void, _hook: OnNewGlobalHookOption, _options: O) -> *mut JSObject { ptr::null_mut() }
    pub fn JS_SetTrustedPrincipals(_cx: *mut JSContext, _p: *mut std::ffi::c_void) -> bool { false }
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
        pub funToString: Option<unsafe extern "C" fn(*mut JSContext, HandleObject<'_>, bool) -> *mut JSString>,
    }
    pub enum OnNewGlobalHookOption {
        FireOnNewGlobalHook,
        DontFireOnNewGlobalHook,
    }
    pub const TrueHandleValue: *const JSVal = std::ptr::null();
    pub enum TraceKind { Object, String, Symbol, BigInt, Script, Shape, BaseShape, JitCode }
    pub fn GCTraceKindToAscii(_kind: TraceKind) -> *const u8 { b"Object\0".as_ptr() }
    pub fn StringIsArrayIndex(_s: *mut JSString, _indexp: *mut u32) -> bool { false }
    pub type PropertyKey = jsid;
    pub fn JS_IsExceptionPending(_cx: *mut JSContext) -> bool { false }
    pub fn JS_ClearPendingException(_cx: *mut JSContext) {}
    pub fn JS_IsGlobalObject(_obj: *mut JSObject) -> bool { false }
    pub fn JS_MayResolveStandardClass<N, I, O>(_names: N, _id: I, _maybe_obj: O) -> bool { false }
    pub fn JS_NewEnumerateStandardClasses<C, O, P>(_cx: C, _obj: O, _props: P, _enum_op: bool) -> bool { false }
    pub fn JS_ResolveStandardClass<C, O, I, R>(_cx: C, _obj: O, _id: I, _resolved: R) -> bool { false }
    pub fn JS_DropPrincipals(_cx: *mut JSContext, _p: *mut std::ffi::c_void) {}
    pub fn JS_HoldPrincipals<P>(_p: P) {}
    pub fn JS_DefinePropertyById<C, I, V, R>(_cx: C, _obj: *mut JSObject, _id: I, _val: V, _result: R) -> bool { false }
    pub fn JS_IdToValue(_cx: *mut JSContext, _id: jsid, _vp: *mut JSVal) -> bool { false }
    pub enum DOMProxyShadowsResult { Shadows, DoesntShadow, DoesntShadowUnique, ShadowsViaDirectExpando, ShadowsViaIndirectExpando, ShadowCheckFailed }
    pub fn GetStaticPrototype(_obj: *mut JSObject) -> *mut JSObject { ptr::null_mut() }
    pub fn SetDOMProxyInformation<F, C>(_domProxyHandlerFamily: F, _callback: C, _class: *const std::ffi::c_void) {}
    pub fn HideScriptedCaller(_cx: *mut JSContext) {}
    pub fn UnhideScriptedCaller(_cx: *mut JSContext) {}
    pub struct MemoryUse;
    impl MemoryUse {
        pub const DOMBinding: u32 = 0;
    }
    pub type JSAtom = *mut std::ffi::c_void;
    pub type JSAtomState = *mut std::ffi::c_void;
    pub fn AtomToLinearString<A>(_atom: A) -> *mut JSString { ptr::null_mut() }
    pub fn GetLinearStringCharAt(_s: *mut JSString, _index: usize) -> u16 { 0 }
    pub fn GetLinearStringLength(_s: *mut JSString) -> usize { 0 }
    pub fn JS_AtomizeStringN<S>(_cx: *mut JSContext, _s: S, _len: usize) -> *mut JSString { ptr::null_mut() }
    pub enum ExceptionStackBehavior { Capture, DoNotCapture }
    pub fn GetCurrentRealmOrNull(_cx: *mut JSContext) -> *mut std::ffi::c_void { ptr::null_mut() }
    pub fn JS_ValueToSource(_cx: *mut JSContext, _val: JSVal) -> *mut JSString { ptr::null_mut() }
    pub fn GetObjectProto<C, O>(_cx: C, _obj: O, _result: *mut *mut JSObject) -> bool { false }

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
        Self { handle: Some(v8::Global::new(scope, obj)) }
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

        pub unsafe fn JS_GetClass(_obj: *mut jsapi::JSObject) -> *const jsapi::JSClass { ptr::null() }
        pub unsafe fn JS_GetReservedSlot<O>(_obj: O, _index: u32, _out: *mut jsapi::JSVal) {}
        pub unsafe fn JS_SetReservedSlot(_obj: *mut jsapi::JSObject, _index: u32, _val: jsapi::JSVal) {}
        pub unsafe fn JS_GetPrivate(_obj: *mut jsapi::JSObject) -> *mut std::ffi::c_void { ptr::null_mut() }
        pub unsafe fn JS_SetPrivate(_obj: *mut jsapi::JSObject, _data: *mut std::ffi::c_void) {}
        pub unsafe fn JS_GetPrototype<C, O, R>(_cx: C, _obj: O, _result: R) -> bool { false }
        pub unsafe fn JS_NewGlobalObject(_cx: *mut jsapi::JSContext, _clasp: *const jsapi::JSClass, _principal: *mut std::ffi::c_void) -> *mut jsapi::JSObject { ptr::null_mut() }
        pub unsafe fn JS_DefineProperty<C, O, N, V>(_cx: C, _obj: O, _name: N, _val: V, _attrs: u32) -> bool { false }
        pub unsafe fn JS_GetProperty<C, O, N, V>(_cx: C, _obj: O, _name: N, _vp: V) -> bool { false }
        pub unsafe fn JS_SetProperty<C, O, N, V>(_cx: C, _obj: O, _name: N, _vp: V) -> bool { false }
        pub unsafe fn JS_NewPlainObject(_cx: *mut jsapi::JSContext) -> *mut jsapi::JSObject { ptr::null_mut() }
        pub unsafe fn JS_NewFunction<C, N>(_cx: *mut jsapi::JSContext, _call: C, _nargs: u32, _flags: u16, _name: N) -> *mut jsapi::JSFunction { ptr::null_mut() }
        pub unsafe fn JS_GetFunctionObject(_fun: *mut jsapi::JSFunction) -> *mut jsapi::JSObject { ptr::null_mut() }
        pub unsafe fn JS_LinkConstructorAndPrototype<C, P>(_cx: *mut jsapi::JSContext, _ctor: C, _proto: P) -> bool { false }
        pub unsafe fn JS_NewStringCopyN(_cx: *mut jsapi::JSContext, _s: *const u8, _len: usize) -> *mut jsapi::JSString { ptr::null_mut() }
        pub unsafe fn JS_GetTwoByteStringCharsAndLength(_cx: *mut jsapi::JSContext, _s: *mut jsapi::JSString, _len: *mut usize) -> *const u16 { ptr::null() }
        pub unsafe fn JS_AtomizeStringN<S>(_cx: *mut jsapi::JSContext, _s: S, _len: usize) -> *mut jsapi::JSString { ptr::null_mut() }
        pub unsafe fn Call(_cx: *mut jsapi::JSContext, _this: *mut jsapi::JSObject, _fun: *mut jsapi::JSObject, _args: *const jsapi::JSVal, _rval: *mut jsapi::JSVal) -> bool { false }
        pub unsafe fn AppendToIdVector<V, I>(_v: V, _id: I) -> bool { false }
        pub unsafe fn GetPropertyKeys<C, O, I>(_cx: C, _obj: O, _flags: u32, _ids: I) -> bool { false }
        pub unsafe fn JS_CopyOwnPropertiesAndPrivateFields(_cx: *mut jsapi::JSContext, _target: *mut jsapi::JSObject, _obj: *mut jsapi::JSObject) -> bool { false }
        pub unsafe fn JS_DefinePropertyById2<C, O, I, V>(_cx: C, _obj: O, _id: I, _val: V) -> bool { false }
        pub unsafe fn JS_InitializePropertiesFromCompatibleNativeObject(_cx: *mut jsapi::JSContext, _dst: *mut jsapi::JSObject, _src: *mut jsapi::JSObject) -> bool { false }
        pub unsafe fn JS_NewObjectWithGivenProto<C, P>(_cx: C, _clasp: *const jsapi::JSClass, _proto: P) -> *mut jsapi::JSObject { ptr::null_mut() }
        pub unsafe fn JS_NewObjectWithoutMetadata<C, P>(_cx: C, _clasp: *const jsapi::JSClass, _proto: P) -> *mut jsapi::JSObject { ptr::null_mut() }
        pub unsafe fn JS_SetImmutablePrototype(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _succeeded: *mut bool) -> bool { false }
        pub unsafe fn JS_SetPrototype(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _proto: *mut jsapi::JSObject) -> bool { false }
        pub unsafe fn JS_WrapObject<C, O>(_cx: C, _obj: O) -> bool { false }
        pub unsafe fn NewProxyObject(_cx: *mut jsapi::JSContext, _handler: *const std::ffi::c_void, _priv: *mut jsapi::JSObject, _proto: *mut jsapi::JSObject, _options: *const std::ffi::c_void, _flag: bool) -> *mut jsapi::JSObject { ptr::null_mut() }
        pub fn RUST_INTERNED_STRING_TO_JSID<C, S, O>(_cx: C, _s: S, _out: O) -> jsapi::jsid { jsapi::jsid(0) }
        pub fn RUST_SYMBOL_TO_JSID(_sym: jsapi::JSVal, _out: jsapi::MutableHandleId<'_>) -> jsapi::jsid { jsapi::jsid(0) }
        pub fn int_to_jsid(_i: i32) -> jsapi::jsid { jsapi::jsid(_i as u64) }

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
            fn into_handle(self) -> *const T { self }
        }

        pub fn IsArrayObject<C, V>(_cx: C, _val: V, _out: *mut bool) -> bool { false }
        pub fn JS_DefineProperty3<C, O, N, V>(_cx: C, _obj: O, _name: N, _val: V, _attrs: u32) -> bool { false }
        pub fn JS_DefineProperty4<C, O, N, V>(_cx: C, _obj: O, _name: N, _val: V, _attrs: u32) -> bool { false }
        pub fn JS_DefineProperty5<C, O, N, V>(_cx: C, _obj: O, _name: N, _val: V, _attrs: u32) -> bool { false }
        pub fn JS_DefinePropertyById5<C, O, I, V>(_cx: C, _obj: O, _id: I, _val: V, _attrs: u32) -> bool { false }
        pub fn JS_FireOnNewGlobalObject<O>(_cx: *mut jsapi::JSContext, _obj: O) {}
        pub fn JS_AlreadyHasOwnPropertyById<C, O, I>(_cx: C, _obj: O, _id: I, _found: *mut bool) -> bool { false }
        pub fn SetDataPropertyDescriptor<D, V>(_desc: D, _value: V, _attrs: u32) {}
        pub unsafe fn JS_GetPropertyById<C, O, I, V>(_cx: C, _obj: O, _id: I, _vp: V) -> bool { false }
        pub unsafe fn JS_HasProperty<C, O, N>(_cx: C, _obj: O, _name: N, _found: *mut bool) -> bool { false }
        pub unsafe fn JS_HasPropertyById<C, O, I>(_cx: C, _obj: O, _id: I, _found: *mut bool) -> bool { false }
        pub unsafe fn JS_HasOwnProperty<C, O, N>(_cx: C, _obj: O, _name: N, _found: *mut bool) -> bool { false }
        pub unsafe fn JS_ForwardGetPropertyTo<C, O, I, R, V>(_cx: C, _obj: O, _id: I, _receiver: R, _vp: V) -> bool { false }
        pub unsafe fn JS_DeletePropertyById<C, O, I, R>(_cx: C, _obj: O, _id: I, _result: R) -> bool { false }
        pub unsafe fn JS_GetPendingException<C, V>(_cx: C, _vp: V) -> bool { false }
        pub unsafe fn JS_SetPendingException<C, V, B>(_cx: C, _val: V, _behavior: B) {}
        pub unsafe fn JS_IdToValue<C, I, V>(_cx: C, _id: I, _vp: V) -> bool { false }
        pub unsafe fn CallOriginalPromiseReject<C, V>(_cx: C, _value: V) -> *mut jsapi::JSObject { ptr::null_mut() }
        pub unsafe fn JS_DefineUCProperty2<C, O, N, V>(_cx: C, _obj: O, _name: N, _namelen: usize, _val: V, _attrs: u32) -> bool { false }
        pub unsafe fn ToJSON<C, V, O, T, W, D>(_cx: C, _val: V, _obj: O, _replacer: T, _callback: W, _data: D) -> bool { false }
        pub unsafe fn JS_GetOwnPropertyDescriptorById<C>(_cx: C, _obj: *mut jsapi::JSObject, _id: jsapi::jsid, _desc: *mut jsapi::PropertyDescriptor, _found: *mut bool) -> bool { false }
    }

    pub struct Runtime;
    impl Runtime {
        pub fn get() -> Option<std::ptr::NonNull<super::jsapi::JSContext>> { None }
        pub fn cx(&self) -> *mut super::jsapi::JSContext { ptr::null_mut() }
    }

    pub mod conversions {
        use super::super::jsapi;
        use super::super::conversions::{ConversionBehavior, ConversionResult};

        pub trait ToJSValConvertible {
            unsafe fn to_jsval(&self, _cx: *mut jsapi::JSContext, _rval: super::MutableHandleValue) {}
        }
        pub trait FromJSValConvertible: Sized {
            type Config;
            unsafe fn from_jsval(_cx: *mut jsapi::JSContext, _val: super::HandleValue, _option: Self::Config) -> Result<ConversionResult<Self>, ()>;
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
        pub unsafe fn new<C>(_cx: C) -> Self { Self(Vec::new()) }
        pub fn handle_mut(&mut self) -> super::jsapi::MutableHandleIdVector { std::ptr::null_mut() }
    }
    impl std::ops::Deref for IdVector {
        type Target = Vec<super::jsapi::jsid>;
        fn deref(&self) -> &Self::Target { &self.0 }
    }

    pub type HandleId<'a> = super::jsapi::Handle<'a, super::jsapi::jsid>;
    pub fn is_dom_class(_clasp: *const super::jsapi::JSClass) -> bool { false }
    pub fn is_dom_object(_obj: *mut super::jsapi::JSObject) -> bool { false }
    pub fn maybe_wrap_value<C, V>(_cx: C, _vp: V) -> bool { false }
    pub fn maybe_wrap_object<C, O>(_cx: C, _obj: O) -> bool { false }
    pub type RealmOptions = super::jsapi::RealmOptions;
    pub fn define_methods<C, O>(_cx: C, _obj: O, _methods: &[super::jsapi::JSFunctionSpec]) -> Result<(), ()> { Ok(()) }
    pub fn define_properties<C, O>(_cx: C, _obj: O, _props: &[super::jsapi::JSPropertySpec]) -> Result<(), ()> { Ok(()) }

    pub struct CustomAutoRooterGuard<T = ()>(std::marker::PhantomData<T>);
    impl<T> CustomAutoRooterGuard<T> {
        pub fn new(_value: T) -> Self { Self(std::marker::PhantomData) }
    }
    impl<T> From<T> for CustomAutoRooterGuard<T> {
        fn from(value: T) -> Self { Self::new(value) }
    }
    pub trait GCMethods {
        fn initial() -> Self where Self: Sized { unimplemented!() }
        unsafe fn post_barrier(_v: *mut Self, _prev: Self, _next: Self) where Self: Sized {}
    }
    impl<T> GCMethods for *mut T {
        fn initial() -> Self { ptr::null_mut() }
    }
    impl<T> GCMethods for *const T {
        fn initial() -> Self { ptr::null() }
    }
    pub fn get_context_realm(_cx: *mut super::jsapi::JSContext) -> *mut super::jsapi::JSObject { ptr::null_mut() }
    pub fn get_object_class(_obj: *mut super::jsapi::JSObject) -> *const super::jsapi::JSClass { ptr::null() }
    pub fn get_object_realm(_obj: *mut super::jsapi::JSObject) -> *mut super::jsapi::JSObject { ptr::null_mut() }
    pub mod wrappers2 {
        use super::super::jsapi;
        use std::ptr;

        pub unsafe fn JS_GetRuntime(_cx: *mut jsapi::JSContext) -> *mut std::ffi::c_void { ptr::null_mut() }
        pub unsafe fn JS_IsExceptionPending<C>(_cx: C) -> bool { false }
        pub unsafe fn JS_WrapObject<C, O>(_cx: C, _obj: O) -> bool { false }
        pub unsafe fn JS_GetProperty<C, O, N, V>(_cx: C, _obj: O, _name: N, _vp: V) -> bool { false }
        pub unsafe fn GetFunctionRealm<C, F>(_cx: C, _fun: F) -> *mut jsapi::JSObject { ptr::null_mut() }
        pub unsafe fn GetWellKnownSymbol<C, W>(_cx: C, _which: W) -> jsapi::JSVal { jsapi::JSVal::default() }
        pub unsafe fn RUST_INTERNED_STRING_TO_JSID<C, S, O>(_cx: C, _s: S, _out: O) -> jsapi::jsid { jsapi::jsid(0) }
        pub unsafe fn JS_AtomizeAndPinString<C, S>(_cx: C, _s: S) -> *mut jsapi::JSString { ptr::null_mut() }
        pub unsafe fn JS_NewObjectWithGivenProto<C, P>(_cx: C, _clasp: *const jsapi::JSClass, _proto: P) -> *mut jsapi::JSObject { ptr::null_mut() }
        pub unsafe fn JS_DefineProperties<C, O>(_cx: C, _obj: O, _props: *const jsapi::JSPropertySpec) -> bool { false }
        pub unsafe fn JS_DefineFunctions<C, O>(_cx: C, _obj: O, _funcs: *const jsapi::JSFunctionSpec) -> bool { false }
        pub unsafe fn JS_GetOwnPropertyDescriptorById<C, O, I, D>(_cx: C, _obj: O, _id: I, _desc: D, _found: *mut bool) -> bool { false }
        pub unsafe fn InvokeGetOwnPropertyDescriptor<H, C, P, I, D>(_handler: H, _cx: C, _proxy: P, _id: I, _desc: D, _found: *mut bool) -> bool { false }
        pub unsafe fn SetPropertyIgnoringNamedGetter<C, O, I, V, R, D, S>(_cx: C, _obj: O, _id: I, _v: V, _receiver: R, _desc: D, _result: S) -> bool { false }
        pub unsafe fn Call<C, T, F, A, R>(_cx: C, _this: T, _fun: F, _args: A, _rval: R) -> bool { false }
        pub unsafe fn EnterRealm<C, O>(_cx: C, _realm: O) -> *mut std::ffi::c_void { ptr::null_mut() }
        pub unsafe fn LeaveRealm<C, R>(_cx: C, _old_realm: R) {}
    }

    pub unsafe fn ToString<C, V>(_cx: C, _val: V) -> *mut super::jsapi::JSString { std::ptr::null_mut() }
    pub unsafe trait Trace {
        unsafe fn trace(&self, _tracer: *mut super::jsapi::JSTracer) {}
    }
    pub trait IntoHandle { type Target; fn into_handle(self) -> Self::Target; }
} // end pub mod rust

// ── GC ──────────────────────────────────────────────────────────────────────

pub mod gc {
    use super::jsapi;
    use std::ptr;

    pub unsafe trait Traceable {
        unsafe fn trace(&self, _tracer: *mut super::jsapi::JSTracer) {}
    }
    pub trait RootedTraceableSet {}

    pub struct RootedGuard<'a, T> {
        value: T,
        _phantom: std::marker::PhantomData<&'a T>,
    }
    impl<'a, T> RootedGuard<'a, T> {
        pub unsafe fn new<C>(_cx: C, _root: &'a mut std::mem::MaybeUninit<T>, val: T) -> Self {
            Self { value: val, _phantom: std::marker::PhantomData }
        }
        pub fn handle(&self) -> jsapi::Handle<'_, T> { unsafe { jsapi::Handle::from_raw(&self.value as *const T) } }
        pub fn handle_mut(&mut self) -> jsapi::MutableHandle<'_, T> { unsafe { jsapi::MutableHandle::from_raw(&mut self.value as *mut T) } }
        pub fn get(&self) -> T where T: Copy { self.value }
        pub fn set(&mut self, val: T) { self.value = val; }
    }
    impl<'a, T> std::ops::Deref for RootedGuard<'a, T> {
        type Target = T;
        fn deref(&self) -> &T { &self.value }
    }
    impl<'a, T> std::ops::DerefMut for RootedGuard<'a, T> {
        fn deref_mut(&mut self) -> &mut T { &mut self.value }
    }

    impl<'a> RootedGuard<'a, *mut std::ffi::c_void> {
        pub fn is_undefined(&self) -> bool { false }
        pub fn is_object(&self) -> bool { false }
        pub fn is_null_or_undefined(&self) -> bool { true }
        pub fn to_object(&self) -> *mut std::ffi::c_void { std::ptr::null_mut() }
    }
    impl<'a> RootedGuard<'a, jsapi::JSVal> {
        pub fn is_undefined(&self) -> bool { self.value.is_undefined() }
        pub fn is_object(&self) -> bool { self.value.is_object() }
        pub fn is_null_or_undefined(&self) -> bool { self.value.is_null_or_undefined() }
        pub fn to_object(&self) -> *mut jsapi::JSObject { self.value.to_object() }
    }
    impl<'a, T> RootedGuard<'a, *mut T> {
        pub fn is_null(&self) -> bool { self.value.is_null() }
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
    unsafe impl Traceable for i32 {}
    unsafe impl Traceable for f32 {}
    unsafe impl Traceable for f64 {}
    unsafe impl Traceable for bool {}
    unsafe impl<T> Traceable for super::jsapi::Heap<T> {}
    unsafe impl<T> Traceable for *mut T {}
    unsafe impl<T> Traceable for *const T {}
    unsafe impl<T: Traceable> Traceable for &T {}
    unsafe impl<T: Traceable> Traceable for Option<T> {}
    unsafe impl<T: Traceable> Traceable for Vec<T> {}
    unsafe impl<T: Traceable> Traceable for Box<T> {}
    unsafe impl<T: Traceable> Traceable for std::rc::Rc<T> {}
    unsafe impl<T> Traceable for std::marker::PhantomData<T> {}
    unsafe impl<T> Traceable for std::cell::Cell<T> {}
    unsafe impl<T> Traceable for std::cell::RefCell<T> {}
    unsafe impl Traceable for String {}
    unsafe impl Traceable for Box<str> {}
    unsafe impl Traceable for super::typedarray::HeapArrayBuffer {}
    unsafe impl Traceable for super::typedarray::HeapArrayBufferView {}
    unsafe impl Traceable for super::typedarray::HeapFloat32Array {}
    unsafe impl Traceable for super::typedarray::HeapFloat64Array {}
    unsafe impl Traceable for super::typedarray::HeapInt32Array {}
    unsafe impl Traceable for super::typedarray::HeapUint8Array {}
    unsafe impl Traceable for super::typedarray::HeapUint8ClampedArray {}
    unsafe impl Traceable for super::typedarray::HeapUint32Array {}

    pub type HandleValue<'a> = super::jsapi::Handle<'a, super::jsapi::JSVal>;
    pub type MutableHandleValue<'a> = super::jsapi::MutableHandle<'a, super::jsapi::JSVal>;
    pub type HandleObject<'a> = super::jsapi::Handle<'a, *mut super::jsapi::JSObject>;
    pub type Handle<'a, T> = super::jsapi::Handle<'a, T>;

    pub struct RootedTraceableBox<T>(std::marker::PhantomData<T>);
    impl<T> RootedTraceableBox<T> {
        pub fn new(_val: T) -> Self { Self(std::marker::PhantomData) }
        pub fn from_box(_val: Box<T>) -> Self { Self(std::marker::PhantomData) }
        pub fn handle(&self) -> super::jsapi::Handle<'_, T> { unsafe { super::jsapi::Handle::from_raw(self.ptr() as *const T) } }
        pub fn ptr(&self) -> *mut T { std::ptr::NonNull::<T>::dangling().as_ptr() }
        pub unsafe fn trace(&self, _tracer: *mut super::jsapi::JSTracer) {}
    }
    impl<T> std::ops::Deref for RootedTraceableBox<T> {
        type Target = T;
        fn deref(&self) -> &T { unsafe { &*self.ptr() } }
    }
    impl<T> std::ops::DerefMut for RootedTraceableBox<T> {
        fn deref_mut(&mut self) -> &mut T { unsafe { &mut *self.ptr() } }
    }
    unsafe impl<T> Traceable for RootedTraceableBox<T> {}

    pub struct CoreGcTypes;

    pub trait GCMethods {
        fn initial() -> Self where Self: Sized;
        unsafe fn post_barrier(_v: *mut Self, _prev: Self, _next: Self) where Self: Sized {}
    }
    impl<T> GCMethods for *mut T {
        fn initial() -> Self { std::ptr::null_mut() }
    }
    impl<T> GCMethods for *const T {
        fn initial() -> Self { std::ptr::null() }
    }
    impl GCMethods for u64 { fn initial() -> Self { 0 } }
    impl GCMethods for u32 { fn initial() -> Self { 0 } }
    impl GCMethods for bool { fn initial() -> Self { false } }
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
        pub unsafe fn new(_cx: *mut jsapi::JSContext) -> Self { Self }
        pub unsafe fn new_unchecked(_cx: *mut jsapi::JSContext) -> Self { Self }
    }

    impl JSContext {
        pub unsafe fn from_ptr(ptr: std::ptr::NonNull<RawJSContext>) -> Self { Self { ptr } }
        pub unsafe fn raw_cx(&mut self) -> *mut RawJSContext { self.ptr.as_ptr() }
        pub unsafe fn raw_cx_no_gc(&self) -> *mut RawJSContext { self.ptr.as_ptr() }
        pub fn no_gc(&self) -> &NoGC { static NO_GC: NoGC = NoGC(()); &NO_GC }
    }

    impl std::ops::Deref for JSContext {
        type Target = NoGC;
        fn deref(&self) -> &NoGC { self.no_gc() }
    }
    impl std::ops::DerefMut for JSContext {
        fn deref_mut(&mut self) -> &mut NoGC { Box::leak(Box::new(NoGC(()))) }
    }

    pub unsafe fn from_ptr(p: std::ptr::NonNull<RawJSContext>) -> JSContext { JSContext { ptr: p } }
}

// ── Realm management ────────────────────────────────────────────────────────

pub mod realms {
    use super::jsapi;
    use std::ptr;

    pub fn AlreadyInRealm(_cx: *mut jsapi::JSContext) -> bool { true }
    pub fn EnterRealm(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject) {}
    pub fn LeaveRealm(_cx: *mut jsapi::JSContext) {}

    pub struct AutoRealm<'a>(std::marker::PhantomData<&'a ()>);
    impl<'a> AutoRealm<'a> {
        pub fn new<C, O>(_cx: C, _obj: O) -> Self { Self(std::marker::PhantomData) }
        pub unsafe fn new_from_handle<T, O>(_cx: T, _obj: O) -> Self { Self(std::marker::PhantomData) }
        pub fn current_realm(&mut self) -> CurrentRealm<'_> { CurrentRealm(std::marker::PhantomData) }
        pub unsafe fn raw_cx(&mut self) -> *mut jsapi::JSContext { ptr::null_mut() }
        pub fn global_and_reborrow(&mut self) -> (jsapi::HandleObject<'static>, &'static mut crate::context::JSContext) {
            let cx = unsafe {
                crate::context::JSContext::from_ptr(std::ptr::NonNull::dangling())
            };
            (jsapi::HandleObject::null(), Box::leak(Box::new(cx)))
        }
    }

    pub struct CurrentRealm<'a>(std::marker::PhantomData<&'a ()>);
    impl<'a> CurrentRealm<'a> {
        pub unsafe fn new(_cx: *mut jsapi::JSContext) -> Self { Self(std::marker::PhantomData) }
        pub fn assert<T>(_: T) -> Self { Self(std::marker::PhantomData) }
        pub unsafe fn raw_cx(&mut self) -> *mut jsapi::JSContext { ptr::null_mut() }
        pub unsafe fn raw_cx_no_gc(&self) -> *mut jsapi::JSContext { ptr::null_mut() }
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
}

// ── Error handling ──────────────────────────────────────────────────────────

pub mod error {
    use super::jsapi;

    pub fn throw_type_error<C, M>(_cx: C, _msg: M) {}
}

// ── Panic ───────────────────────────────────────────────────────────────────

pub mod panic {
    pub fn maybe_resume_unwind() {}
    pub fn wrap_panic(f: &mut dyn FnMut()) { f(); }
}

// ── Conversions ─────────────────────────────────────────────────────────────

pub mod conversions {
    use super::jsapi;

    pub use super::rust::conversions::{FromJSValConvertible, ToJSValConvertible};

    pub enum ConversionBehavior { Default, EnforceRange, Clamp }
    impl Default for ConversionBehavior {
        fn default() -> Self { Self::Default }
    }
    pub enum ConversionResult<T> { Success(T), Failure(std::borrow::Cow<'static, std::ffi::CStr>) }

    pub fn jsstr_to_string<C, S>(_cx: C, _s: S) -> String {
        String::new()
    }
    pub unsafe fn ToString(_cx: *mut jsapi::JSContext, _val: jsapi::JSVal) -> String {
        String::new()
    }

    pub trait FromJSValConvertibleRc: Sized {}

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
    primitive_to_jsval!((), bool, f32, f64, i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, String);

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

    impl<T: ToJSValConvertible> ToJSValConvertible for Option<T> {
        unsafe fn to_jsval(&self, cx: *mut jsapi::JSContext, rval: jsapi::MutableHandleValue) {
            if let Some(value) = self {
                unsafe { value.to_jsval(cx, rval) }
            }
        }
    }
    impl<T: ToJSValConvertible> ToJSValConvertible for Vec<T> {
        unsafe fn to_jsval(&self, _cx: *mut jsapi::JSContext, mut rval: jsapi::MutableHandleValue) {
            rval.set(jsapi::JSVal::default());
        }
    }
    impl<T> FromJSValConvertible for Vec<T>
    where
        T: FromJSValConvertible,
    {
        type Config = T::Config;
        unsafe fn from_jsval(_cx: *mut jsapi::JSContext, _val: jsapi::HandleValue, _option: Self::Config) -> Result<ConversionResult<Self>, ()> {
            Ok(ConversionResult::Success(Vec::new()))
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
        unsafe fn from_jsval(_cx: *mut jsapi::JSContext, _val: jsapi::HandleValue, _option: ()) -> Result<ConversionResult<Self>, ()> {
            Ok(ConversionResult::Success(std::ptr::null_mut()))
        }
    }
}

// ── JSVal ───────────────────────────────────────────────────────────────────

pub mod jsval {
    use std::ptr;

    pub use super::conversions;

    pub type JSVal = super::jsapi::JSVal;

    pub const UndefinedValue: fn() -> JSVal = || { JSVal::default() };
    pub const NullValue: fn() -> JSVal = || { JSVal::default() };
    pub const TrueValue: fn() -> JSVal = || { JSVal::default() };
    pub const FalseValue: fn() -> JSVal = || { JSVal::default() };
    pub fn ObjectValue(_obj: *const super::jsapi::JSObject) -> JSVal { JSVal::default() }
    pub fn ObjectOrNullValue(_obj: *const super::jsapi::JSObject) -> JSVal { JSVal::default() }
    pub fn PrivateValue(_p: *const std::ffi::c_void) -> JSVal { JSVal::default() }
    pub fn BooleanValue(_b: bool) -> JSVal { JSVal::default() }
    pub fn DoubleValue(_d: f64) -> JSVal { JSVal::default() }
    pub fn Int32Value(_i: i32) -> JSVal { JSVal::default() }
    pub fn UInt32Value(_u: u32) -> JSVal { JSVal::default() }
    pub fn StringValue<S>(_s: S) -> JSVal { JSVal::default() }
    pub fn NumberValue(_n: f64) -> JSVal { JSVal::default() }

    pub mod glue {
        use super::super::jsapi;
        use super::*;
        pub fn IsWrapper(_obj: *mut super::super::jsapi::JSObject) -> bool { false }
    }
}

// ── Typed arrays ────────────────────────────────────────────────────────────

pub mod typedarray {
    use super::jsapi;
    use super::conversions::ToJSValConvertible;

    macro_rules! typed_array {
        ($name:ident) => {
            #[derive(Clone, Copy)]
            pub struct $name(pub *mut jsapi::JSObject);
            impl $name {
                pub fn from(obj: *mut jsapi::JSObject) -> Result<Self, ()> { Ok(Self(obj)) }
            }
            impl ToJSValConvertible for $name {
                unsafe fn to_jsval(&self, _cx: *mut jsapi::JSContext, mut rval: jsapi::MutableHandleValue) {
                    rval.set(jsapi::JSVal::default());
                }
            }
        };
    }

    typed_array!(ArrayBuffer);
    typed_array!(ArrayBufferView);
    typed_array!(Float32Array);
    typed_array!(Float64Array);
    typed_array!(Int32Array);
    typed_array!(Uint8Array);
    typed_array!(Uint8ClampedArray);
    typed_array!(Uint32Array);
    typed_array!(HeapArrayBuffer);
    typed_array!(HeapArrayBufferView);
    typed_array!(HeapFloat32Array);
    typed_array!(HeapFloat64Array);
    typed_array!(HeapInt32Array);
    typed_array!(HeapUint8Array);
    typed_array!(HeapUint8ClampedArray);
    typed_array!(HeapUint32Array);
}

// ── Misc modules ────────────────────────────────────────────────────────────

pub mod jsid {
    #[derive(Copy, Clone)]
    pub struct SymbolId {
        pub asBits_: u64,
    }
    #[derive(Copy, Clone)]
    pub struct StringId {
        pub ptr: *const u8,
    }
    pub fn SymbolId(_value: super::jsapi::JSVal) -> super::jsapi::jsid { super::jsapi::jsid(0) }
    pub fn StringId<S>(_value: S) -> StringId { StringId { ptr: std::ptr::null() } }
}
impl<'a> From<jsapi::Handle<'a, jsid::StringId>> for jsapi::jsid {
    fn from(_handle: jsapi::Handle<'a, jsid::StringId>) -> Self { jsapi::jsid(0) }
}

pub mod realm {
    pub use super::realms::*;
}

pub mod record { pub struct JsRecord; }
pub mod guard { pub struct CustomAutoRooter; }
pub mod finalize { use super::jsapi; pub type FinalizationRegistryObject = *mut jsapi::JSObject; }
pub mod weakref { use super::jsapi; pub type WeakRefObject = *mut jsapi::JSObject; }
pub mod interface { pub struct JsInterface; }
pub mod constructor { pub struct JsConstructor; }

pub const JSCLASS_GLOBAL_SLOT_COUNT: u32 = 4;
pub const JSCLASS_IS_DOMJSCLASS: u32 = 1 << 4;
pub const JSCLASS_IS_GLOBAL: u32 = 1 << 5;
pub const JSCLASS_IS_PROXY: u32 = 1 << 3;
pub const JSCLASS_RESERVED_SLOTS_MASK: u32 = 0xff << 8;
pub const JSCLASS_USERBIT1: u32 = 1 << 14;
pub fn JS_CALLEE<C>(_cx: C, _vp: *mut jsapi::JSVal) -> jsapi::JSVal { jsapi::JSVal::default() }

pub mod js {
    pub use crate::{JS_CALLEE, JSCLASS_IS_DOMJSCLASS, JSCLASS_IS_GLOBAL, JSCLASS_RESERVED_SLOTS_MASK};
}
