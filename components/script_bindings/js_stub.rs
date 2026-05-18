// Stub mozjs types for builds without the `mozjs` feature.
// Provides minimal type definitions so the rest of the code compiles.
// All stub methods panic at runtime — V8 equivalents will be added under the `v8` feature.

#![allow(unused_imports, dead_code, non_snake_case, non_camel_case_types)]

pub mod glue {
    pub fn IsWrapper(_obj: *mut JSObject) -> bool { false }
    pub fn UnwrapObjectDynamic(_obj: *mut JSObject, _depth: u32) -> *mut std::ffi::c_void { std::ptr::null_mut() }
    pub fn RUST_JSID_TO_STRING(_cx: *mut JSContext, _id: *const jsid) -> *mut JSString { std::ptr::null_mut() }
    pub fn AppendToIdVector(_cx: *mut JSContext, _v: *mut u32, _id: *const jsid) -> bool { false }
    pub fn GetProxyHandler(_proxy: *mut JSObject) -> *const std::ffi::c_void { std::ptr::null() }
    pub fn NewProxyObject(_cx: *mut JSContext, _handler: *const std::ffi::c_void, _priv: *mut JSObject, _proto: *mut JSObject) -> *mut JSObject { std::ptr::null_mut() }
    pub fn GetProxyPrivate(_proxy: *mut JSObject) -> *mut std::ffi::c_void { std::ptr::null_mut() }
    pub fn SetProxyPrivate(_proxy: *mut JSObject, _priv: *mut std::ffi::c_void) {}
    pub fn DeletePropertyIgnoringResult(_cx: *mut JSContext, _obj: *mut JSObject, _prop: *const u8) {}
    pub fn DefinePropertyById(_cx: *mut JSContext, _obj: *mut JSObject, _id: *const jsid, _desc: *const JSPropertySpec) -> bool { false }
    pub fn SetDataPropertyDescriptor(_cx: *mut JSContext, _obj: *mut JSObject, _id: *const jsid, _attrs: u32) {}
    pub fn AtomizeStringN(_cx: *mut JSContext, _s: *const u8, _len: usize) -> *mut JSString { std::ptr::null_mut() }
    pub fn CreateDOMGlobal(_cx: *mut JSContext, _clasp: *const JSClass, _principal: *mut std::ffi::c_void) -> *mut JSObject { std::ptr::null_mut() }
    pub fn CallJitGetterOp(_cx: *mut JSContext, _obj: *mut JSObject, _id: *const jsid, _vp: *const JSVal) -> bool { false }
    pub fn CallJitMethodOp(_cx: *mut JSContext, _obj: *mut JSObject, _id: *const jsid, _vp: *const JSVal) -> bool { false }
    pub fn CallJitSetterOp(_cx: *mut JSContext, _obj: *mut JSObject, _id: *const jsid, _vp: *const JSVal) -> bool { false }
}

pub mod jsapi {
    pub type JSContext = std::ffi::c_void;
    pub type JSObject = std::ffi::c_void;
    pub type JSString = std::ffi::c_void;
    pub type JSFunction = std::ffi::c_void;
    pub type JSTracer = std::ffi::c_void;
    pub type JSRuntime = std::ffi::c_void;
    pub type JSPrincipals = std::ffi::c_void;
    pub type JSClass = JSClassDef;
    pub type jsid = u32;
    pub type JSVal = u64;
    pub type JSAutoRealm = std::ffi::c_void;
    pub type JSAutoCompartment = std::ffi::c_void;
    pub type Handle<*const JSObject> = *const JSObject;

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
        pub mayResolve: Option<unsafe extern "C" fn(*const JSAtom, *const jsid)>,
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

    pub const JSCLASS_IS_DOMJSCLASS: u32 = 0;
    pub const JSCLASS_IS_GLOBAL: u32 = 0;
    pub const JSPROP_RESOLVING: u32 = 0;
    pub const JSPROP_ENUMERATE: u32 = 0;
    pub const JSFUN_STUB_GSOPS: u32 = 0;

    pub struct JSProtoKey(pub u32);

    pub mod JS {
        pub const ProtoKey: u32 = 0;
    }
}

pub mod rust {
    pub mod wrappers {
        pub unsafe fn JS_GetClass(_obj: *mut crate::jsapi::JSObject) -> *const crate::jsapi::JSClass { std::ptr::null() }
        pub unsafe fn JS_GetReservedSlot(_obj: *mut crate::jsapi::JSObject, _index: u32) -> crate::jsapi::JSVal { 0 }
        pub unsafe fn JS_SetReservedSlot(_obj: *mut crate::jsapi::JSObject, _index: u32, _val: crate::jsapi::JSVal) {}
        pub unsafe fn JS_GetPrivate(_obj: *mut crate::jsapi::JSObject) -> *mut std::ffi::c_void { std::ptr::null_mut() }
        pub unsafe fn JS_SetPrivate(_obj: *mut crate::jsapi::JSObject, _data: *mut std::ffi::c_void) {}
        pub unsafe fn JS_GetPrototype(_obj: *mut crate::jsapi::JSObject) -> *mut crate::jsapi::JSObject { std::ptr::null_mut() }
        pub unsafe fn JS_NewGlobalObject(_cx: *mut crate::jsapi::JSContext, _clasp: *const crate::jsapi::JSClass, _principal: *mut std::ffi::c_void) -> *mut crate::jsapi::JSObject { std::ptr::null_mut() }
        pub unsafe fn JS_DefineProperty(_cx: *mut crate::jsapi::JSContext, _obj: *mut crate::jsapi::JSObject, _name: *const u8, _name_len: usize, _val: crate::jsapi::JSVal, _attrs: u32) -> bool { false }
        pub unsafe fn JS_GetProperty(_cx: *mut crate::jsapi::JSContext, _obj: *mut crate::jsapi::JSObject, _name: *const u8, _vp: *mut crate::jsapi::JSVal) -> bool { false }
        pub unsafe fn JS_SetProperty(_cx: *mut crate::jsapi::JSContext, _obj: *mut crate::jsapi::JSObject, _name: *const u8, _vp: *const crate::jsapi::JSVal) -> bool { false }
        pub unsafe fn JS_NewPlainObject(_cx: *mut crate::jsapi::JSContext) -> *mut crate::jsapi::JSObject { std::ptr::null_mut() }
        pub unsafe fn JS_NewFunction(_cx: *mut crate::jsapi::JSContext, _call: *const std::ffi::c_void, _nargs: u32, _flags: u32, _name: *const u8) -> *mut crate::jsapi::JSFunction { std::ptr::null_mut() }
        pub unsafe fn JS_GetFunctionObject(_fun: *mut crate::jsapi::JSFunction) -> *mut crate::jsapi::JSObject { std::ptr::null_mut() }
        pub unsafe fn JS_LinkConstructorAndPrototype(_cx: *mut crate::jsapi::JSContext, _ctor: *mut crate::jsapi::JSObject, _proto: *mut crate::jsapi::JSObject) -> bool { false }
        pub unsafe fn JS_NewStringCopyN(_cx: *mut crate::jsapi::JSContext, _s: *const u8, _len: usize) -> *mut crate::jsapi::JSString { std::ptr::null_mut() }
        pub unsafe fn JS_GetTwoByteStringCharsAndLength(_cx: *mut crate::jsapi::JSContext, _s: *mut crate::jsapi::JSString, _len: *mut usize) -> *const u16 { std::ptr::null() }
        pub unsafe fn JS_AtomizeStringN(_cx: *mut crate::jsapi::JSContext, _s: *const u8, _len: usize) -> *mut crate::jsapi::JSString { std::ptr::null_mut() }
    }

    pub trait Runtime {}
    pub struct Runtime;

    pub mod conversions {
        pub trait ToJSValConvertible {
            unsafe fn to_jsval(&self, _cx: *mut crate::jsapi::JSContext, _rval: *const crate::jsapi::JSVal) {}
        }
        pub trait FromJSValConvertible {
            type Config;
            unsafe fn from_jsval(_cx: *mut crate::jsapi::JSContext, _val: crate::jsapi::JSVal, _option: ()) -> Result<std::convert::Infallible, ()> { Err(()) }
        }
    }
}

pub mod gc {
    pub trait Traceable {}
    pub trait RootedTraceableSet {}
    pub struct RootedTraceable {}
    pub struct RootedGuard<T> { _phantom: std::marker::PhantomData<T> }
    impl<T> RootedGuard<T> {
        pub unsafe fn new(_cx: *mut crate::jsapi::JSContext, _val: T) -> Self { Self { _phantom: std::marker::PhantomData } }
        pub fn handle(&self) -> &T { unimplemented!() }
        pub fn get(&self) -> T { unimplemented!() }
    }

    pub unsafe fn add_associated_memory(_obj: *const crate::jsapi::JSObject, _sz: usize) {}
    pub unsafe fn remove_associated_memory(_obj: *const crate::jsapi::JSObject, _sz: usize) {}
    pub fn add_root(_obj: &dyn Traceable) {}
    pub fn remove_root(_obj: &dyn Traceable) {}

    pub struct CoreGcTypes;
}

pub mod context {
    pub struct JSContext;
    pub type CallArgs = ();
    pub struct AutoCheckRequest;
    pub struct SafeJSContext;
    pub struct Heap<T> { _phantom: std::marker::PhantomData<T> }
    impl<T> Heap<T> {
        pub fn boxed(_val: T) -> Self { Self { _phantom: std::marker::PhantomData } }
    }
}

pub mod error {
    pub fn throw_type_error(_cx: *mut crate::jsapi::JSContext, _msg: &str) {}
}

pub mod typedarray {
    pub struct ArrayBuffer;
    pub struct ArrayBufferView;
}

pub mod jsval {
    pub use crate::rust::conversions::{ToJSValConvertible, FromJSValConvertible};
}
