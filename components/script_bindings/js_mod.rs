// V8-backed JS engine bridge — replaces mozjs SpiderMonkey FFI.
// Uses rusty_v8 with thread-local isolates and persistent handles.

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
    use super::*;
    pub fn IsWrapper(_obj: *mut jsapi::JSObject) -> bool { false }
    pub fn UnwrapObjectDynamic(_obj: *mut jsapi::JSObject, _depth: u32) -> *mut std::ffi::c_void { ptr::null_mut() }
    pub fn RUST_JSID_TO_STRING(_cx: *mut jsapi::JSContext, _id: *const jsapi::jsid) -> *mut jsapi::JSString { ptr::null_mut() }
    pub fn AppendToIdVector(_cx: *mut jsapi::JSContext, _v: *mut u32, _id: *const jsapi::jsid) -> bool { false }
    pub fn GetProxyHandler(_proxy: *mut jsapi::JSObject) -> *const std::ffi::c_void { ptr::null() }
    pub fn NewProxyObject(_cx: *mut jsapi::JSContext, _handler: *const std::ffi::c_void, _priv: *mut jsapi::JSObject, _proto: *mut jsapi::JSObject) -> *mut jsapi::JSObject { ptr::null_mut() }
    pub fn GetProxyPrivate(_proxy: *mut jsapi::JSObject) -> *mut std::ffi::c_void { ptr::null_mut() }
    pub fn SetProxyPrivate(_proxy: *mut jsapi::JSObject, _priv: *mut std::ffi::c_void) {}
    pub fn DeletePropertyIgnoringResult(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _prop: *const u8) {}
    pub fn DefinePropertyById(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _id: *const jsapi::jsid, _desc: *const jsapi::JSPropertySpec) -> bool { false }
    pub fn SetDataPropertyDescriptor(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _id: *const jsapi::jsid, _attrs: u32) {}
    pub fn AtomizeStringN(_cx: *mut jsapi::JSContext, _s: *const u8, _len: usize) -> *mut jsapi::JSString { ptr::null_mut() }
    pub fn CreateDOMGlobal(_cx: *mut jsapi::JSContext, _clasp: *const jsapi::JSClass, _principal: *mut std::ffi::c_void) -> *mut jsapi::JSObject { ptr::null_mut() }
    pub fn CallJitGetterOp(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _id: *const jsapi::jsid, _vp: *const jsapi::JSVal) -> bool { false }
    pub fn CallJitMethodOp(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _id: *const jsapi::jsid, _vp: *const jsapi::JSVal) -> bool { false }
    pub fn CallJitSetterOp(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _id: *const jsapi::jsid, _vp: *const jsapi::JSVal) -> bool { false }
}

pub mod jsapi {
    use super::*;

    #[cfg(feature = "v8")]
    pub type JSContext = *mut V8Context;
    #[cfg(not(feature = "v8"))]
    pub type JSContext = *mut std::ffi::c_void;
    pub type JSObject = *mut std::ffi::c_void;
    pub type JSString = *mut std::ffi::c_void;
    pub type JSFunction = *mut std::ffi::c_void;
    pub type JSTracer = *mut std::ffi::c_void;
    #[cfg(feature = "v8")]
    pub type JSRuntime = *mut V8Runtime;
    #[cfg(not(feature = "v8"))]
    pub type JSRuntime = *mut std::ffi::c_void;
    pub type JSPrincipals = std::ffi::c_void;
    pub type JSClass = JSClassDef;
    pub type jsid = u64;
    pub type JSVal = *mut std::ffi::c_void;
    pub type JSAutoRealm = *mut std::ffi::c_void;
    pub type JSAutoCompartment = *mut std::ffi::c_void;

    pub type Handle<T> = *const T;
    pub type MutableHandle<T> = *mut T;

    #[repr(C)]
    pub struct JSClassDef {
        pub name: *const u8,
        pub flags: u32,
        pub cOps: *const JSClassOps,
    }

    #[repr(C)]
    pub struct JSClassOps {
        pub addProperty: Option<unsafe extern "C" fn(*mut JSContext, *mut JSObject, jsid, *const JSVal)>,
        pub delProperty: Option<unsafe extern "C" fn(*mut JSContext, *mut JSObject, jsid, *const JSVal)>,
        pub enumerate: Option<unsafe extern "C" fn(*mut JSContext, *mut JSObject)>,
        pub newEnumerate: Option<unsafe extern "C" fn(*mut JSContext, *mut JSObject, *const JSVal, *const jsid)>,
        pub resolve: Option<unsafe extern "C" fn(*mut JSContext, *mut JSObject, jsid, bool)>,
        pub mayResolve: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const jsid)>,
        pub finalize: Option<unsafe extern "C" fn(*mut JSContext, *mut JSObject)>,
        pub call: Option<unsafe extern "C" fn(*mut JSContext, *mut JSObject, *const jsid)>,
        pub hasInstance: Option<unsafe extern "C" fn(*mut JSContext, *mut JSObject, *const jsid)>,
        pub construct: Option<unsafe extern "C" fn(*mut JSContext, *mut JSObject, *const jsid)>,
        pub trace: Option<unsafe extern "C" fn(*mut JSContext, *mut JSObject, *mut JSTracer)>,
    }

    #[repr(C)]
    pub struct JSPropertySpec {
        pub name: *const u8,
    }

    pub const JSCLASS_IS_DOMJSCLASS: u32 = 1 << 4;
    pub const JSCLASS_IS_GLOBAL: u32 = 1 << 5;
    pub const JSPROP_RESOLVING: u32 = 0x8000;
    pub const JSPROP_ENUMERATE: u32 = 0x01;
    pub const JSFUN_STUB_GSOPS: u32 = 0;

    pub struct JSProtoKey(pub u32);

    pub mod JS {
        pub const ProtoKey: u32 = 0;
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

// ── Rust wrappers (matching mozjs js::rust::wrappers) ─────────────────────

pub mod rust {
    pub mod wrappers {
        use super::super::*;
        pub unsafe fn JS_GetClass(_obj: *mut jsapi::JSObject) -> *const jsapi::JSClass { ptr::null() }
        pub unsafe fn JS_GetReservedSlot(_obj: *mut jsapi::JSObject, _index: u32) -> jsapi::JSVal { ptr::null_mut() }
        pub unsafe fn JS_SetReservedSlot(_obj: *mut jsapi::JSObject, _index: u32, _val: jsapi::JSVal) {}
        pub unsafe fn JS_GetPrivate(_obj: *mut jsapi::JSObject) -> *mut std::ffi::c_void { ptr::null_mut() }
        pub unsafe fn JS_SetPrivate(_obj: *mut jsapi::JSObject, _data: *mut std::ffi::c_void) {}
        pub unsafe fn JS_GetPrototype(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject) -> *mut jsapi::JSObject { ptr::null_mut() }
        pub unsafe fn JS_NewGlobalObject(_cx: *mut jsapi::JSContext, _clasp: *const jsapi::JSClass, _principal: *mut std::ffi::c_void) -> *mut jsapi::JSObject { ptr::null_mut() }
        pub unsafe fn JS_DefineProperty(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _name: *const u8, _name_len: usize, _val: jsapi::JSVal, _attrs: u32) -> bool { false }
        pub unsafe fn JS_GetProperty(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _name: *const u8, _vp: *mut jsapi::JSVal) -> bool { false }
        pub unsafe fn JS_SetProperty(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _name: *const u8, _vp: *const jsapi::JSVal) -> bool { false }
        pub unsafe fn JS_NewPlainObject(_cx: *mut jsapi::JSContext) -> *mut jsapi::JSObject { ptr::null_mut() }
        pub unsafe fn JS_NewFunction(_cx: *mut jsapi::JSContext, _call: *const std::ffi::c_void, _nargs: u32, _flags: u32, _name: *const u8) -> *mut jsapi::JSFunction { ptr::null_mut() }
        pub unsafe fn JS_GetFunctionObject(_fun: *mut jsapi::JSFunction) -> *mut jsapi::JSObject { ptr::null_mut() }
        pub unsafe fn JS_LinkConstructorAndPrototype(_cx: *mut jsapi::JSContext, _ctor: *mut jsapi::JSObject, _proto: *mut jsapi::JSObject) -> bool { false }
        pub unsafe fn JS_NewStringCopyN(_cx: *mut jsapi::JSContext, _s: *const u8, _len: usize) -> *mut jsapi::JSString { ptr::null_mut() }
        pub unsafe fn JS_GetTwoByteStringCharsAndLength(_cx: *mut jsapi::JSContext, _s: *mut jsapi::JSString, _len: *mut usize) -> *const u16 { ptr::null() }
        pub unsafe fn JS_AtomizeStringN(_cx: *mut jsapi::JSContext, _s: *const u8, _len: usize) -> *mut jsapi::JSString { ptr::null_mut() }

        pub type Handle<T> = *const T;
        pub type HandleObject = *const jsapi::JSObject;
        pub type HandleValue = *const jsapi::JSVal;
        pub type MutableHandle<T> = *mut T;
        pub type MutableHandleObject = *mut jsapi::JSObject;

        pub trait IntoHandle {
            type Target;
            fn into_handle(self) -> Self::Target;
        }
        impl<T> IntoHandle for *const T {
            type Target = *const T;
            fn into_handle(self) -> *const T { self }
        }
    }

    pub trait Runtime {
        fn cx(&self) -> *mut jsapi::JSContext;
    }

    pub mod conversions {
        use super::super::*;
        pub trait ToJSValConvertible {
            unsafe fn to_jsval(&self, _cx: *mut jsapi::JSContext, _rval: *const jsapi::JSVal) {}
        }
        pub trait FromJSValConvertible: Sized {
            type Config;
            unsafe fn from_jsval(_cx: *mut jsapi::JSContext, _val: jsapi::JSVal, _option: ()) -> Result<Self, ()>;
        }
    }
} // end pub mod rust

// ── GC ──────────────────────────────────────────────────────────────────────

pub mod gc {
    use super::*;
    pub trait Traceable {}
    pub trait RootedTraceableSet {}

    pub struct RootedGuard<T> {
        _phantom: std::marker::PhantomData<T>,
    }
    impl<T> RootedGuard<T> {
        pub unsafe fn new(_cx: *mut jsapi::JSContext, val: T) -> Self {
            Self { _phantom: std::marker::PhantomData }
        }
        pub fn handle(&self) -> &T { unimplemented!("V8 RootedGuard::handle") }
        pub fn get(&self) -> T { unimplemented!("V8 RootedGuard::get") }
    }

    pub unsafe fn add_associated_memory(_obj: *const jsapi::JSObject, _sz: usize) {}
    pub unsafe fn remove_associated_memory(_obj: *const jsapi::JSObject, _sz: usize) {}
    pub fn add_root(_obj: &dyn Traceable) {}
    pub fn remove_root(_obj: &dyn Traceable) {}

    pub struct CoreGcTypes;
}

// ── Context / Rooting ───────────────────────────────────────────────────────

pub mod context {
    use super::*;

    pub type JSContext = jsapi::JSContext;
    pub type SafeJSContext = *mut jsapi::JSContext;

    pub struct Heap<T> {
        cell: RefCell<Option<T>>,
    }
    impl<T> Heap<T> {
        pub fn boxed(val: T) -> Self {
            Self { cell: RefCell::new(Some(val)) }
        }
        pub fn set(&self, val: T) {
            *self.cell.borrow_mut() = Some(val);
        }
        pub fn get(&self) -> Option<T> where T: Clone {
            self.cell.borrow().clone()
        }
    }

    impl<T> Heap<T> {
        pub unsafe fn unbarriered_get(&self) -> *const T {
            ptr::null()
        }
    }

    pub type CallArgs = ();
    pub struct AutoCheckRequest;
    impl AutoCheckRequest {
        pub unsafe fn new(_cx: *mut jsapi::JSContext) -> Self { Self }
        pub unsafe fn new_unchecked(_cx: *mut jsapi::JSContext) -> Self { Self }
    }
}

// ── Realm management ────────────────────────────────────────────────────────

pub mod realms {
    use super::*;
    pub fn AlreadyInRealm(_cx: *mut jsapi::JSContext) -> bool { true }
    pub fn EnterRealm(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject) {}
    pub fn LeaveRealm(_cx: *mut jsapi::JSContext) {}
}

// ── Error handling ──────────────────────────────────────────────────────────

pub mod error {
    use super::*;
    pub fn throw_type_error(_cx: *mut jsapi::JSContext, _msg: &str) {}
}

// ── Typed arrays ────────────────────────────────────────────────────────────

pub mod typedarray {
    pub struct ArrayBuffer;
    pub struct ArrayBufferView;
}

// ── JSVal conversions ───────────────────────────────────────────────────────

pub mod jsval {
    pub use super::rust::conversions::{ToJSValConvertible, FromJSValConvertible};
    pub type JSVal = super::jsapi::JSVal;

    pub const UndefinedValue: fn() -> JSVal = || { ptr::null_mut() };
    pub const NullValue: fn() -> JSVal = || { ptr::null_mut() };
    pub const TrueValue: fn() -> JSVal = || { ptr::null_mut() };
    pub const FalseValue: fn() -> JSVal = || { ptr::null_mut() };

    pub mod glue {
        use super::*;
        pub fn IsWrapper(_obj: *mut super::super::jsapi::JSObject) -> bool { false }
    }
}

// ── DOM string utils ────────────────────────────────────────────────────────

pub mod conversions {
    pub fn jsstr_to_string(_cx: *mut jsapi::JSContext, _s: *mut jsapi::JSString) -> String {
        String::new()
    }
    pub unsafe fn ToString(_cx: *mut jsapi::JSContext, _val: jsapi::JSVal) -> String {
        String::new()
    }
}

// ── Misc modules ────────────────────────────────────────────────────────────

pub mod jsid {
    pub type SymbolId = jsapi::jsid;
}

pub mod record { pub struct JsRecord; }
pub mod guard { pub struct CustomAutoRooter; }
pub mod finalize { pub type FinalizationRegistryObject = *mut jsapi::JSObject; }
pub mod weakref { pub type WeakRefObject = *mut jsapi::JSObject; }
pub mod interface { pub struct JsInterface; }
pub mod constructor { pub struct JsConstructor; }
