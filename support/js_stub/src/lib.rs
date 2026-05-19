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
    pub fn UnwrapObjectDynamic(_obj: *mut jsapi::JSObject, _depth: u32) -> *mut std::ffi::c_void { ptr::null_mut() }
    pub fn RUST_JSID_TO_STRING(_cx: *mut jsapi::JSContext, _id: *const jsapi::jsid) -> *mut jsapi::JSString { ptr::null_mut() }
    pub fn AppendToIdVector(_cx: *mut jsapi::JSContext, _v: *mut u32, _id: *const jsapi::jsid) -> bool { false }
    pub fn GetProxyHandler(_proxy: *mut jsapi::JSObject) -> *const std::ffi::c_void { ptr::null() }
    pub fn NewProxyObject(_cx: *mut jsapi::JSContext, _handler: *const std::ffi::c_void, _priv: *mut jsapi::JSObject, _proto: *mut jsapi::JSObject, _options: *const std::ffi::c_void, _flag: bool) -> *mut jsapi::JSObject { ptr::null_mut() }
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
    pub fn JS_GetReservedSlot(_obj: *mut jsapi::JSObject, _slot: u32) -> jsapi::JSVal { ptr::null_mut() }
    pub fn SetProxyReservedSlot(_proxy: *mut jsapi::JSObject, _slot: u32, _val: jsapi::JSVal) {}

    pub fn RUST_JSID_IS_VOID(_id: *const jsapi::jsid) -> bool { false }
    pub fn CallObjectTracer(_trc: *mut jsapi::JSTracer, _obj: *mut jsapi::JSObject, _name: *const u8) {}
    pub fn UncheckedUnwrapObject(_obj: *mut jsapi::JSObject, _stopAtOuter: bool) -> *mut jsapi::JSObject { ptr::null_mut() }
    pub fn IsProxyHandlerFamily(_obj: *mut jsapi::JSObject) -> bool { false }
    pub fn GetProxyHandlerFamily(_proxy: *mut jsapi::JSObject) -> *const std::ffi::c_void { ptr::null() }
    pub fn CreateRustJSPrincipals(_cx: *mut jsapi::JSContext) -> *mut std::ffi::c_void { ptr::null_mut() }
    pub fn GetRustJSPrincipalsPrivate(_p: *mut std::ffi::c_void) -> *mut std::ffi::c_void { ptr::null_mut() }
    pub type JSPrincipalsCallbacks = std::ffi::c_void;
    pub fn GetProxyHandlerExtra(_proxy: *mut jsapi::JSObject) -> *mut std::ffi::c_void { ptr::null_mut() }
    pub fn RUST_FUNCTION_VALUE_TO_JITINFO(_cx: *mut jsapi::JSContext, _fun: *mut jsapi::JSObject) -> *const jsapi::JSJitInfo { ptr::null() }
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
    #[derive(Copy, Clone, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct jsid(pub u64);
    impl std::ops::Deref for jsid { type Target = u64; fn deref(&self) -> &u64 { &self.0 } }
    impl std::fmt::Debug for jsid { fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { write!(f, "jsid({})", self.0) } }
    impl From<u64> for jsid { fn from(v: u64) -> jsid { jsid(v) } }
    impl From<*const jsid> for jsid { fn from(p: *const jsid) -> jsid { unsafe { *p } } }
    impl From<&jsid> for *const jsid { fn from(p: &jsid) -> *const jsid { p as *const jsid } }
    pub type JSVal = *mut std::ffi::c_void;
    pub type JSAutoRealm = *mut std::ffi::c_void;
    pub type JSAutoCompartment = *mut std::ffi::c_void;
    pub type GCContext = *mut std::ffi::c_void;

    pub type Handle<'a, T> = *const T;
    pub type MutableHandle<'a, T> = *mut T;

    // from_raw helpers — accessed as Handle::from_raw / MutableHandle::from_raw
    // These live in a nested module to avoid conflicting with the type alias at this level.
    pub mod handle_from_raw {
        pub unsafe fn Handle<T>(_: super::Handle<'static, T>, raw: *const T) -> *const T { raw }
        pub unsafe fn MutableHandle<T>(_: super::MutableHandle<'static, T>, raw: *mut T) -> *mut T { raw }
    }
    pub unsafe fn handle_from_raw<T>(raw: *const T) -> *const T { raw }
    pub unsafe fn mutable_handle_from_raw<T>(raw: *mut T) -> *mut T { raw }

    pub type HandleId<'a> = *const jsid;
    pub type HandleObject<'a> = Handle<'a, JSObject>;
    pub type HandleValue<'a> = Handle<'a, JSVal>;
    pub type HandleValueArray = *const JSVal;
    pub type MutableHandleIdVector = *mut *const jsid;
    pub type MutableHandleObject<'a> = MutableHandle<'a, JSObject>;
    pub type MutableHandleValue<'a> = MutableHandle<'a, JSVal>;
    pub fn UndefinedHandleValue() -> *const JSVal { std::ptr::null() }
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
                argv_: vp.offset(2),
                rval_: vp,
            }
        }
        pub fn get(&self, index: u32) -> JSVal {
            if (index as usize) < self.argc_ as usize {
                unsafe { *self.argv_.offset(index as isize) }
            } else {
                std::ptr::null_mut()
            }
        }
        pub fn rval(&self) -> *mut JSVal {
            self.rval_
        }
        pub fn is_constructing(&self) -> bool { false }
    }
    pub type ObjectOpResult = *mut std::ffi::c_void;

    pub struct PropertyDescriptor {
        pub obj: JSObject,
        pub attrs: u32,
    }

    pub struct Heap<T> {
        cell: RefCell<Option<T>>,
    }
    impl<T> Heap<T> {
        pub fn boxed(val: T) -> Self { Self { cell: RefCell::new(Some(val)) } }
        pub fn set(&self, val: T) { *self.cell.borrow_mut() = Some(val); }
        pub fn get(&self) -> Option<T> where T: Clone { self.cell.borrow().clone() }
        pub unsafe fn unbarriered_get(&self) -> *const T { ptr::null() }
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

    pub type JSJitGetterCallArgs = *mut std::ffi::c_void;
    pub type JSJitMethodCallArgs = *mut std::ffi::c_void;
    pub type JSJitSetterCallArgs = *mut std::ffi::c_void;

    #[repr(C)]
    pub struct JSJitInfo__bindgen_ty_1 {
        pub method: Option<unsafe extern "C" fn(*mut JSContext, u32, *mut JSVal) -> bool>,
        pub getter: Option<unsafe extern "C" fn(*mut JSContext, u32, *mut JSVal) -> bool>,
        pub setter: Option<unsafe extern "C" fn(*mut JSContext, u32, *mut JSVal) -> bool>,
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

    #[repr(C)]
    pub struct JSNativeWrapper {
        pub op: Option<unsafe extern "C" fn(*mut JSContext, *mut JSObject, *const JSVal) -> bool>,
        pub info: *const std::ffi::c_void,
    }

    #[repr(C)]
    pub struct __BindgenBitfieldUnit<Storage> {
        storage: Storage,
    }
    impl<Storage> __BindgenBitfieldUnit<Storage> {
        pub const fn new(storage: Storage) -> Self { Self { storage } }
    }

    #[repr(C)]
    pub struct JSClassDef {
        pub name: *const u8,
        pub flags: u32,
        pub cOps: *const JSClassOps,
        pub spec: *const std::ffi::c_void,
        pub ext: *const std::ffi::c_void,
        pub oOps: *const std::ffi::c_void,
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
    pub struct JSPropertySpec_Name {
        pub string_: *const u8,
        pub symbol_: usize,
    }

    #[repr(C)]
    pub struct JSPropertySpec_Accessor {
        pub native: JSNativeWrapper,
    }

    #[repr(C)]
    pub struct JSPropertySpec_AccessorsOrValue_Accessors {
        pub getter: JSPropertySpec_Accessor,
        pub setter: JSPropertySpec_Accessor,
    }

    #[repr(u32)]
    pub enum JSPropertySpec_Kind {
        NativeAccessor = 0,
        Value = 1,
    }

    #[repr(u32)]
    pub enum JSPropertySpec_ValueWrapper_Type {
        String = 0,
    }

    #[repr(C)]
    pub struct JSPropertySpec_ValueWrapper__bindgen_ty_1 {
        pub string: *const u8,
    }

    #[repr(C)]
    pub struct JSPropertySpec_ValueWrapper {
        pub type_: JSPropertySpec_ValueWrapper_Type,
        pub __bindgen_anon_1: JSPropertySpec_ValueWrapper__bindgen_ty_1,
    }

    #[repr(C)]
    pub struct JSPropertySpec_AccessorsOrValue {
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
    impl JSPropertySpec {
        pub const ZERO: Self = JSPropertySpec {
            name: JSPropertySpec_Name { string_: ptr::null(), symbol_: 0 },
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
                value: JSPropertySpec_ValueWrapper {
                    type_: JSPropertySpec_ValueWrapper_Type::String,
                    __bindgen_anon_1: JSPropertySpec_ValueWrapper__bindgen_ty_1 {
                        string: ptr::null(),
                    },
                },
            },
        };
    }

    #[repr(C)]
    pub struct JSFunctionSpec {
        pub name: *const u8,
        pub call: *const std::ffi::c_void,
        pub nargs: u16,
        pub flags: u16,
        pub selfHostedName: *const u8,
    }

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
        pub type CompartmentIterResult = *mut std::ffi::c_void;
    }

    pub fn IsCallable(_v: JSVal) -> bool { false }
    pub fn GetWellKnownSymbol(_cx: *mut JSContext, _which: u32) -> JSVal { ptr::null_mut() }
    pub fn GetRealmErrorPrototype(_cx: *mut JSContext) -> JSObject { ptr::null_mut() }
    pub fn GetRealmFunctionPrototype(_cx: *mut JSContext) -> JSObject { ptr::null_mut() }
    pub fn GetRealmIteratorPrototype(_cx: *mut JSContext) -> JSObject { ptr::null_mut() }
    pub fn GetRealmObjectPrototype(_cx: *mut JSContext) -> JSObject { ptr::null_mut() }
    pub fn JS_AtomizeAndPinString(_cx: *mut JSContext, _s: *const u8) -> *mut JSString { ptr::null_mut() }
    pub fn JS_ForwardGetPropertyTo(_cx: *mut JSContext, _obj: *mut JSObject, _id: impl Into<jsid>, _receiver: *mut JSObject, _vp: *mut JSVal) -> bool { false }
    pub fn JS_GetPropertyDescriptorById(_cx: *mut JSContext, _obj: *mut JSObject, _id: impl Into<jsid>, _desc: *mut PropertyDescriptor, _ignored: *mut JSObject, _found: *mut bool) -> bool { false }
    pub fn JS_HasPropertyById(_cx: *mut JSContext, _obj: *mut JSObject, _id: impl Into<jsid>, _found: *mut bool) -> bool { false }
    pub fn JS_NewPlainObject(_cx: *mut JSContext) -> *mut JSObject { ptr::null_mut() }
    pub fn JS_SetReservedSlot(_obj: *mut JSObject, _index: u32, _val: JSVal) {}
    pub fn JS_NewObject(_cx: *mut JSContext, _clasp: *const JSClass) -> *mut JSObject { ptr::null_mut() }
    pub type SymbolCode = u32;

    pub fn AddAssociatedMemory(_obj: *mut JSObject, _sz: usize, _assoc: u32) {}
    pub fn JS_GlobalObjectTraceHook(_trc: *mut JSTracer, _global: *mut JSObject) {}
    pub fn JS_DeprecatedStringHasLatin1Chars(_s: *mut JSString) -> bool { false }
    pub fn JS_GetTwoByteLatin1Chars(_s: *mut JSString) -> *const u8 { ptr::null() }
    pub fn JS_GetTwoByteStringChars(_s: *mut JSString) -> *const u16 { ptr::null() }
    pub const JSCLASS_IS_PROXY: u32 = 1 << 3;
    pub const JSCLASS_USERBIT1: u32 = 1 << 14;

    pub fn AddRawValueRoot(_cx: *mut JSContext, _vp: *mut JSVal) -> bool { false }
    pub fn RemoveRawValueRoot(_cx: *mut JSContext, _vp: *mut JSVal) {}
    pub fn RemoveAssociatedMemory(_obj: *mut JSObject, _sz: usize, _assoc: u32) {}
    pub fn IsWindowProxy(_obj: *mut JSObject) -> bool { false }
    pub fn JS_GetLatin1StringCharsAndLength(_cx: *mut JSContext, _s: *mut JSString, _len: *mut usize) -> *const u8 { ptr::null() }
    pub fn JS_GetTwoByteStringCharsAndLength(_cx: *mut JSContext, _s: *mut JSString, _len: *mut usize) -> *const u16 { ptr::null() }
    pub fn JS_NewStringCopyN(_cx: *mut JSContext, _s: *const u8, _len: usize) -> *mut JSString { ptr::null_mut() }
    pub fn CheckedUnwrapStatic(_obj: *mut JSObject) -> *mut JSObject { ptr::null_mut() }
    pub type Compartment = *mut std::ffi::c_void;
    pub type CompartmentSpecifier = *mut std::ffi::c_void;
    pub fn GetNonCCWObjectGlobal(_obj: *mut JSObject) -> *mut JSObject { ptr::null_mut() }
    pub fn GetRealmGlobalOrNull(_cx: *mut JSContext) -> *mut JSObject { ptr::null_mut() }
    pub fn IsSharableCompartment(_comp: *mut std::ffi::c_void) -> bool { false }
    pub fn IsSystemCompartment(_comp: *mut std::ffi::c_void) -> bool { false }
    pub fn JS_GetFunctionObject(_fun: *mut JSFunction) -> *mut JSObject { ptr::null_mut() }
    pub fn JS_IterateCompartments(_cx: *mut JSContext, _callback: *const std::ffi::c_void, _data: *mut std::ffi::c_void) {}
    pub fn JS_NewFunction(_cx: *mut JSContext, _call: *const std::ffi::c_void, _nargs: u32, _flags: u32, _name: *const u8) -> *mut JSFunction { ptr::null_mut() }
    pub fn JS_NewGlobalObject(_cx: *mut JSContext, _clasp: *const JSClass, _principal: *mut std::ffi::c_void, _hook: u32) -> *mut JSObject { ptr::null_mut() }
    pub fn JS_SetTrustedPrincipals(_cx: *mut JSContext, _p: *mut std::ffi::c_void) -> bool { false }
    pub const JSFUN_CONSTRUCTOR: u16 = 0x01;
    pub type ObjectOps = std::ffi::c_void;
    pub type OnNewGlobalHookOption = u32;
    pub fn TrueHandleValue() -> *const JSVal { std::ptr::null() }
    pub type Value = *mut std::ffi::c_void;
    pub type TraceKind = u32;
    pub fn GCTraceKindToAscii(_kind: u32) -> *const u8 { b"Object\0".as_ptr() }
    pub fn StringIsArrayIndex(_s: *mut JSString, _indexp: *mut u32) -> bool { false }
    pub type PropertyKey = *mut std::ffi::c_void;
    pub fn JS_IsExceptionPending(_cx: *mut JSContext) -> bool { false }
    pub fn JS_ClearPendingException(_cx: *mut JSContext) {}
    pub fn JS_IsGlobalObject(_obj: *mut JSObject) -> bool { false }
    pub fn JS_MayResolveStandardClass(_cx: *mut JSContext, _obj: *mut JSObject, _id: jsid, _resolved: *mut bool) -> bool { false }
    pub fn JS_NewEnumerateStandardClasses(_cx: *mut JSContext, _obj: *mut JSObject, _props: *mut *const jsid, _enum_op: u32) -> bool { false }
    pub fn JS_ResolveStandardClass(_cx: *mut JSContext, _obj: *mut JSObject, _id: jsid, _resolved: *mut bool) -> bool { false }
    pub fn JS_DropPrincipals(_cx: *mut JSContext, _p: *mut std::ffi::c_void) {}
    pub fn JS_HoldPrincipals(_cx: *mut JSContext, _p: *mut std::ffi::c_void) {}
    pub fn JS_DefinePropertyById(_cx: *mut JSContext, _obj: *mut JSObject, _id: jsid, _val: JSVal, _attrs: u32) -> bool { false }
    pub enum DOMProxyShadowsResult { Shadows, DoesntShadow, DoesntShadowUnique, ShadowsViaDirectExpando, ShadowsViaIndirectExpando }
    pub fn GetStaticPrototype(_obj: *mut JSObject) -> *mut JSObject { ptr::null_mut() }
    pub type JSErrNum = u32;
    pub fn SetDOMProxyInformation(_domProxyHandlerFamily: *const std::ffi::c_void, _domProxyExpandoSlot: u32) {}
    pub fn HideScriptedCaller(_cx: *mut JSContext) {}
    pub fn UnhideScriptedCaller(_cx: *mut JSContext) {}
    pub type MemoryUse = u32;
    pub type JSAtom = *mut std::ffi::c_void;
    pub type JSAtomState = *mut std::ffi::c_void;
    pub fn AtomToLinearString(_atom: *mut std::ffi::c_void) -> *mut JSString { ptr::null_mut() }
    pub fn GetLinearStringCharAt(_s: *mut JSString, _index: usize) -> u16 { 0 }
    pub fn GetLinearStringLength(_s: *mut JSString) -> usize { 0 }
    pub fn JS_AtomizeStringN(_cx: *mut JSContext, _s: *const u8, _len: usize) -> *mut JSString { ptr::null_mut() }
    pub enum ExceptionStackBehavior { Capture, DoNotCapture }
    pub fn GetCurrentRealmOrNull(_cx: *mut JSContext) -> *mut std::ffi::c_void { ptr::null_mut() }
    pub fn JS_ValueToSource(_cx: *mut JSContext, _val: JSVal) -> *mut JSString { ptr::null_mut() }
    pub fn GetObjectProto(_cx: *mut JSContext, _obj: *mut JSObject) -> *mut JSObject { ptr::null_mut() }
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
        pub unsafe fn Call(_cx: *mut jsapi::JSContext, _this: *mut jsapi::JSObject, _fun: *mut jsapi::JSObject, _args: *const jsapi::JSVal, _rval: *mut jsapi::JSVal) -> bool { false }
        pub unsafe fn AppendToIdVector(_cx: *mut jsapi::JSContext, _v: *mut u32, _id: *const jsapi::jsid) -> bool { false }
        pub unsafe fn GetPropertyKeys(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _flags: u32, _ids: *mut *const jsapi::jsid) -> bool { false }
        pub unsafe fn JS_CopyOwnPropertiesAndPrivateFields(_cx: *mut jsapi::JSContext, _target: *mut jsapi::JSObject, _obj: *mut jsapi::JSObject) -> bool { false }
        pub unsafe fn JS_DefinePropertyById2(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _id: impl Into<jsapi::jsid>, _val: jsapi::JSVal) -> bool { false }
        pub unsafe fn JS_InitializePropertiesFromCompatibleNativeObject(_cx: *mut jsapi::JSContext, _dst: *mut jsapi::JSObject, _src: *mut jsapi::JSObject) -> bool { false }
        pub unsafe fn JS_NewObjectWithGivenProto(_cx: *mut jsapi::JSContext, _clasp: *const jsapi::JSClass, _proto: *mut jsapi::JSObject) -> *mut jsapi::JSObject { ptr::null_mut() }
        pub unsafe fn JS_NewObjectWithoutMetadata(_cx: *mut jsapi::JSContext, _clasp: *const jsapi::JSClass, _proto: *mut jsapi::JSObject) -> *mut jsapi::JSObject { ptr::null_mut() }
        pub unsafe fn JS_SetImmutablePrototype(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _succeeded: *mut bool) -> bool { false }
        pub unsafe fn JS_SetPrototype(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _proto: *mut jsapi::JSObject) -> bool { false }
        pub unsafe fn JS_WrapObject(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject) -> bool { false }
        pub unsafe fn NewProxyObject(_cx: *mut jsapi::JSContext, _handler: *const std::ffi::c_void, _priv: *mut jsapi::JSObject, _proto: *mut jsapi::JSObject, _options: *const std::ffi::c_void, _flag: bool) -> *mut jsapi::JSObject { ptr::null_mut() }
        pub fn RUST_INTERNED_STRING_TO_JSID(_cx: *mut jsapi::JSContext, _s: *const u8) -> jsapi::jsid { jsapi::jsid(0) }
        pub fn RUST_SYMBOL_TO_JSID(_cx: *mut jsapi::JSContext, _sym: jsapi::SymbolCode) -> jsapi::jsid { jsapi::jsid(0) }
        pub fn int_to_jsid(_i: i32) -> jsapi::jsid { jsapi::jsid(_i as u64) }

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

        pub fn IsArrayObject(_obj: *mut jsapi::JSObject) -> bool { false }
        pub fn JS_DefineProperty3(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _name: *const u8, _val: jsapi::JSVal) -> bool { false }
        pub fn JS_DefineProperty4(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _name: *const u8, _val: jsapi::JSVal, _attrs: u32) -> bool { false }
        pub fn JS_DefineProperty5(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _name: *const u8, _len: usize, _val: jsapi::JSVal, _attrs: u32) -> bool { false }
        pub fn JS_DefinePropertyById5(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _id: jsapi::jsid, _val: jsapi::JSVal, _attrs: u32) -> bool { false }
        pub fn JS_FireOnNewGlobalObject(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject) {}
        pub fn JS_AlreadyHasOwnPropertyById(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _id: jsapi::jsid, _found: *mut bool) -> bool { false }
        pub fn SetDataPropertyDescriptor(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _id: jsapi::jsid, _attrs: u32) {}
        pub unsafe fn JS_GetPropertyById(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _id: jsapi::jsid, _vp: *mut jsapi::JSVal) -> bool { false }
        pub unsafe fn JS_HasProperty(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _name: *const u8, _found: *mut bool) -> bool { false }
        pub unsafe fn JS_HasPropertyById(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _id: impl Into<jsapi::jsid>, _found: *mut bool) -> bool { false }
        pub unsafe fn JS_HasOwnProperty(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _name: *const u8, _found: *mut bool) -> bool { false }
        pub unsafe fn JS_ForwardGetPropertyTo(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _id: impl Into<jsapi::jsid>, _receiver: *mut jsapi::JSObject, _vp: *mut jsapi::JSVal) -> bool { false }
        pub unsafe fn JS_DeletePropertyById(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _id: impl Into<jsapi::jsid>, _result: *mut jsapi::ObjectOpResult) -> bool { false }
        pub unsafe fn JS_GetPendingException(_cx: *mut jsapi::JSContext, _vp: *mut jsapi::JSVal) -> bool { false }
        pub unsafe fn JS_SetPendingException(_cx: *mut jsapi::JSContext, _val: jsapi::JSVal) {}
        pub unsafe fn JS_IdToValue(_cx: *mut jsapi::JSContext, _id: jsapi::jsid, _vp: *mut jsapi::JSVal) -> bool { false }
        pub unsafe fn CallOriginalPromiseReject(_cx: *mut jsapi::JSContext, _args: *const jsapi::JSVal, _rval: *mut jsapi::JSVal) -> bool { false }
        pub unsafe fn JS_DefineUCProperty2(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _name: *const u16, _namelen: usize, _val: jsapi::JSVal) -> bool { false }
        pub unsafe fn ToJSON(_cx: *mut jsapi::JSContext, _val: jsapi::JSVal, _vp: *mut jsapi::JSVal) -> bool { false }
        pub unsafe fn JS_GetOwnPropertyDescriptorById(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _id: jsapi::jsid, _desc: *mut jsapi::PropertyDescriptor) -> bool { false }
    }

    pub trait Runtime {
        fn cx(&self) -> *mut super::jsapi::JSContext;
    }

    pub mod conversions {
        use super::super::jsapi;
        use super::super::conversions::{ConversionBehavior, ConversionResult};

        pub trait ToJSValConvertible {
            unsafe fn to_jsval(&self, _cx: *mut jsapi::JSContext, _rval: *mut jsapi::JSVal) -> Result<(), ()> { Ok(()) }
        }
        pub trait FromJSValConvertible: Sized {
            type Config;
            unsafe fn from_jsval(_cx: *mut jsapi::JSContext, _val: jsapi::JSVal, _option: Self::Config) -> Result<ConversionResult<Self>, ()>;
        }
    }

    pub type Handle<T> = *const T;
    pub type HandleObject = *const super::jsapi::JSObject;
    pub type HandleValue = *const super::jsapi::JSVal;
    pub type MutableHandle<T> = *mut T;
    pub type MutableHandleObject = *mut super::jsapi::JSObject;
    pub type MutableHandleValue = *mut super::jsapi::JSVal;
    pub type IdVector = *mut std::ffi::c_void;

    pub type HandleId = *const super::jsapi::jsid;
    pub fn is_dom_class(_clasp: *const super::jsapi::JSClass) -> bool { false }
    pub fn is_dom_object(_obj: *mut super::jsapi::JSObject) -> bool { false }
    pub fn maybe_wrap_value(_cx: *mut super::jsapi::JSContext, _vp: *mut super::jsapi::JSVal) -> bool { false }
    pub fn maybe_wrap_object(_cx: *mut super::jsapi::JSContext, _obj: *mut super::jsapi::JSObject) -> bool { false }
    pub type RealmOptions = *mut std::ffi::c_void;
    pub fn define_methods(_cx: *mut super::jsapi::JSContext, _obj: *mut super::jsapi::JSObject, _methods: *const super::jsapi::JSFunctionSpec) -> bool { false }
    pub fn define_properties(_cx: *mut super::jsapi::JSContext, _obj: *mut super::jsapi::JSObject, _props: *const super::jsapi::JSPropertySpec) -> bool { false }

    pub struct CustomAutoRooterGuard;
    pub trait GCMethods {
        fn initial() -> Self where Self: Sized { unimplemented!() }
    }
    pub fn get_context_realm(_cx: *mut super::jsapi::JSContext) -> *mut super::jsapi::JSObject { ptr::null_mut() }
    pub fn get_object_class(_obj: *mut super::jsapi::JSObject) -> *const super::jsapi::JSClass { ptr::null() }
    pub fn get_object_realm(_obj: *mut super::jsapi::JSObject) -> *mut super::jsapi::JSObject { ptr::null_mut() }
    pub mod wrappers2 {
        use super::super::jsapi;
        use std::ptr;

        pub unsafe fn JS_GetRuntime(_cx: *mut jsapi::JSContext) -> *mut std::ffi::c_void { ptr::null_mut() }
        pub unsafe fn JS_IsExceptionPending(_cx: *mut jsapi::JSContext) -> bool { false }
        pub unsafe fn JS_WrapObject(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject) -> bool { false }
        pub unsafe fn JS_GetProperty(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _name: *const u8, _vp: *mut jsapi::JSVal) -> bool { false }
        pub unsafe fn GetFunctionRealm(_cx: *mut jsapi::JSContext, _fun: *mut jsapi::JSObject) -> *mut jsapi::JSObject { ptr::null_mut() }
        pub unsafe fn GetWellKnownSymbol(_cx: *mut jsapi::JSContext, _which: u32) -> jsapi::JSVal { ptr::null_mut() }
        pub unsafe fn RUST_INTERNED_STRING_TO_JSID(_cx: *mut jsapi::JSContext, _s: *const u8) -> jsapi::jsid { jsapi::jsid(0) }
        pub unsafe fn JS_AtomizeAndPinString(_cx: *mut jsapi::JSContext, _s: *const u8) -> *mut jsapi::JSString { ptr::null_mut() }
        pub unsafe fn JS_NewObjectWithGivenProto(_cx: *mut jsapi::JSContext, _clasp: *const jsapi::JSClass, _proto: *mut jsapi::JSObject) -> *mut jsapi::JSObject { ptr::null_mut() }
        pub unsafe fn JS_DefineProperties(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _props: *const jsapi::JSPropertySpec) -> bool { false }
        pub unsafe fn JS_DefineFunctions(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _funcs: *const jsapi::JSFunctionSpec) -> bool { false }
        pub unsafe fn JS_GetOwnPropertyDescriptorById(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _id: jsapi::jsid, _desc: *mut jsapi::PropertyDescriptor, _found: *mut bool) -> bool { false }
        pub unsafe fn InvokeGetOwnPropertyDescriptor(_cx: *mut jsapi::JSContext, _proxy: *mut jsapi::JSObject, _id: jsapi::jsid, _desc: *mut jsapi::PropertyDescriptor, _found: *mut bool) -> bool { false }
        pub unsafe fn SetPropertyIgnoringNamedGetter(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject, _id: jsapi::jsid, _v: *const jsapi::JSVal, _strict: bool) -> bool { false }
        pub unsafe fn Call(_cx: *mut jsapi::JSContext, _this: *mut jsapi::JSObject, _fun: *mut jsapi::JSObject, _args: *const jsapi::JSVal, _rval: *mut jsapi::JSVal) -> bool { false }
        pub unsafe fn EnterRealm(_cx: *mut jsapi::JSContext, _realm: *mut jsapi::JSObject) {}
        pub unsafe fn LeaveRealm(_cx: *mut jsapi::JSContext) {}
    }

    pub unsafe fn ToString(_cx: *mut super::jsapi::JSContext, _val: super::jsapi::JSVal) -> String { String::new() }
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
        pub unsafe fn new(_cx: *mut jsapi::JSContext, _root: &'a mut std::mem::MaybeUninit<T>, val: T) -> Self {
            Self { value: val, _phantom: std::marker::PhantomData }
        }
        pub fn handle(&self) -> *const T { &self.value as *const T }
        pub fn handle_mut(&mut self) -> *mut T { &mut self.value as *mut T }
        pub fn get(&self) -> T where T: Copy { self.value }
        pub fn set(&mut self, val: T) { self.value = val; }
    }

    impl<'a> RootedGuard<'a, *mut std::ffi::c_void> {
        pub fn is_undefined(&self) -> bool { false }
        pub fn is_object(&self) -> bool { false }
        pub fn is_null_or_undefined(&self) -> bool { true }
        pub fn to_object(&self) -> *mut std::ffi::c_void { std::ptr::null_mut() }
    }
    impl<'a, T> RootedGuard<'a, *mut T> {
        pub fn is_null(&self) -> bool { self.value.is_null() }
    }

    pub unsafe fn add_associated_memory(_obj: *const jsapi::JSObject, _sz: usize) {}
    pub unsafe fn remove_associated_memory(_obj: *const jsapi::JSObject, _sz: usize) {}
    pub fn add_root(_obj: &dyn Traceable) {}
    pub fn remove_root(_obj: &dyn Traceable) {}

    unsafe impl<T> Traceable for *mut T {}
    unsafe impl<T> Traceable for *const T {}
    unsafe impl<T: Traceable> Traceable for super::jsapi::Heap<T> {}
    unsafe impl<T: Traceable> Traceable for &T {}
    unsafe impl<T: Traceable> Traceable for Option<T> {}
    unsafe impl<T: Traceable> Traceable for Vec<T> {}
    unsafe impl<T: Traceable> Traceable for Box<T> {}
    unsafe impl<T: Traceable> Traceable for std::rc::Rc<T> {}

    pub type HandleValue = *const super::jsapi::JSVal;
    pub type MutableHandleValue = *mut super::jsapi::JSVal;
    pub type HandleObject = *const super::jsapi::JSObject;
    pub type Handle<T> = *const T;

    pub struct RootedTraceableBox<T>(std::marker::PhantomData<T>);
    impl<T> RootedTraceableBox<T> {
        pub fn new(_val: T) -> Self { Self(std::marker::PhantomData) }
        pub fn from_box(_val: Box<T>) -> Self { Self(std::marker::PhantomData) }
    }

    pub struct CoreGcTypes;

    pub trait GCMethods {
        fn initial() -> Self where Self: Sized;
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

    pub type JSContext = jsapi::JSContext;
    pub type RawJSContext = jsapi::JSContext;
    pub type SafeJSContext = jsapi::JSContext;

    pub type CallArgs = jsapi::CallArgs;

    pub struct AutoCheckRequest;
    impl AutoCheckRequest {
        pub unsafe fn new(_cx: *mut jsapi::JSContext) -> Self { Self }
        pub unsafe fn new_unchecked(_cx: *mut jsapi::JSContext) -> Self { Self }
    }

    pub unsafe fn from_ptr(_p: std::ptr::NonNull<jsapi::JSContext>) -> JSContext { std::ptr::null_mut() }
}

// ── Realm management ────────────────────────────────────────────────────────

pub mod realms {
    use super::jsapi;
    use std::ptr;

    pub fn AlreadyInRealm(_cx: *mut jsapi::JSContext) -> bool { true }
    pub fn EnterRealm(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject) {}
    pub fn LeaveRealm(_cx: *mut jsapi::JSContext) {}

    pub struct AutoRealm;
    impl AutoRealm {
        pub unsafe fn new(_cx: *mut jsapi::JSContext, _obj: *mut jsapi::JSObject) -> Self { Self }
        pub unsafe fn new_from_handle<T>(_cx: T, _obj: *const jsapi::JSObject) -> Self { Self }
    }

    pub struct CurrentRealm;
    impl CurrentRealm {
        pub unsafe fn new(_cx: *mut jsapi::JSContext) -> Self { Self }
        pub fn assert<T>(_: T) -> Self { Self }
    }
}

// ── Error handling ──────────────────────────────────────────────────────────

pub mod error {
    use super::jsapi;

    pub fn throw_type_error(_cx: *mut jsapi::JSContext, _msg: &str) {}
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
    pub enum ConversionResult<T> { Success(T), Failure(Box<str>) }

    pub fn jsstr_to_string(_cx: *mut jsapi::JSContext, _s: *mut jsapi::JSString) -> String {
        String::new()
    }
    pub unsafe fn ToString(_cx: *mut jsapi::JSContext, _val: jsapi::JSVal) -> String {
        String::new()
    }

    pub trait FromJSValConvertibleRc: Sized {}
}

// ── JSVal ───────────────────────────────────────────────────────────────────

pub mod jsval {
    use std::ptr;

    pub use super::conversions;

    pub type JSVal = super::jsapi::JSVal;

    pub const UndefinedValue: fn() -> JSVal = || { ptr::null_mut() };
    pub const NullValue: fn() -> JSVal = || { ptr::null_mut() };
    pub const TrueValue: fn() -> JSVal = || { ptr::null_mut() };
    pub const FalseValue: fn() -> JSVal = || { ptr::null_mut() };
    pub fn ObjectValue(_obj: *const super::jsapi::JSObject) -> JSVal { ptr::null_mut() }
    pub fn ObjectOrNullValue(_obj: *const super::jsapi::JSObject) -> JSVal { ptr::null_mut() }
    pub fn PrivateValue(_p: *const std::ffi::c_void) -> JSVal { ptr::null_mut() }
    pub fn BooleanValue(_b: bool) -> JSVal { ptr::null_mut() }
    pub fn DoubleValue(_d: f64) -> JSVal { ptr::null_mut() }
    pub fn Int32Value(_i: i32) -> JSVal { ptr::null_mut() }
    pub fn UInt32Value(_u: u32) -> JSVal { ptr::null_mut() }
    pub fn StringValue(_s: *mut super::jsapi::JSString) -> JSVal { ptr::null_mut() }
    pub fn NumberValue(_n: f64) -> JSVal { ptr::null_mut() }

    pub mod glue {
        use super::super::jsapi;
        use super::*;
        pub fn IsWrapper(_obj: *mut super::super::jsapi::JSObject) -> bool { false }
    }
}

// ── Typed arrays ────────────────────────────────────────────────────────────

pub mod typedarray {
    use super::jsapi;

    pub struct ArrayBuffer;
    pub struct ArrayBufferView;
    pub type HeapArrayBuffer = *mut jsapi::JSObject;
    pub type HeapArrayBufferView = *mut jsapi::JSObject;
    pub type HeapFloat32Array = *mut jsapi::JSObject;
    pub type Float32Array = *mut jsapi::JSObject;
    pub type HeapFloat64Array = *mut jsapi::JSObject;
    pub type HeapUint8Array = *mut jsapi::JSObject;
    pub type HeapUint8ClampedArray = *mut jsapi::JSObject;
}

// ── Misc modules ────────────────────────────────────────────────────────────

pub mod jsid {
    pub struct SymbolId(pub u64);
    pub type StringId = *const u8;
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
pub const JSCLASS_IS_PROXY: u32 = 1 << 3;
pub const JSCLASS_USERBIT1: u32 = 1 << 14;

pub mod js {
    pub const JSCLASS_IS_DOMJSCLASS: u32 = 1 << 4;
    pub const JSCLASS_IS_GLOBAL: u32 = 1 << 5;
    pub const JSCLASS_RESERVED_SLOTS_MASK: u32 = 0xff << 8;
    pub const JS_CALLEE: u32 = 0;
}
