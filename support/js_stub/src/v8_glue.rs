//! Real V8-backed implementations for glue stubs and wrappers.
//!
//! Each SM-style opaque handle (`*mut JSObject`, `*mut JSString`) is actually a
//! numeric ID into a thread-local V8 `Global` map.  ID 0 (null pointer) = none.

#![cfg(feature = "v8")]

use crate::jsapi;
use rusty_v8 as v8;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ptr;
use std::sync::{Mutex, OnceLock};

fn proxy_handler_extras() -> &'static Mutex<HashMap<usize, usize>> {
    static EXTRAS: OnceLock<Mutex<HashMap<usize, usize>>> = OnceLock::new();
    EXTRAS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn last_proxy_handler_extra() -> &'static Mutex<Option<usize>> {
    static EXTRA: OnceLock<Mutex<Option<usize>>> = OnceLock::new();
    EXTRA.get_or_init(|| Mutex::new(None))
}

fn object_classes() -> &'static Mutex<HashMap<usize, usize>> {
    static CLASSES: OnceLock<Mutex<HashMap<usize, usize>>> = OnceLock::new();
    CLASSES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn promise_states() -> &'static Mutex<HashMap<usize, jsapi::PromiseState>> {
    static STATES: OnceLock<Mutex<HashMap<usize, jsapi::PromiseState>>> = OnceLock::new();
    STATES.get_or_init(|| Mutex::new(HashMap::new()))
}

// ── Thread-local handle stores ─────────────────────────────────────────────

thread_local! {
    /// Current realm's V8 context ID (index into super::V8_CONTEXTS).
    pub(crate) static GLOBAL_ID: RefCell<Option<usize>> = const { RefCell::new(None) };

    pub(crate) static CURRENT_GLOBAL_HANDLE: RefCell<Option<usize>> = const { RefCell::new(None) };

    /// Object handle pointer → V8 object.
    pub(crate) static OBJECT_MAP: RefCell<HashMap<usize, v8::Global<v8::Object>>> =
        RefCell::new(HashMap::new());

    /// String handle pointer → V8 string.
    pub(crate) static STRING_MAP: RefCell<HashMap<usize, v8::Global<v8::String>>> =
        RefCell::new(HashMap::new());

    /// Function handle pointer → backing function object handle.
    pub(crate) static FUNCTION_OBJECTS: RefCell<HashMap<usize, usize>> = RefCell::new(HashMap::new());

    /// Function object handle pointer → native JSAPI callback.
    pub(crate) static FUNCTION_NATIVES: RefCell<HashMap<usize, jsapi::NativeCallback>> =
        RefCell::new(HashMap::new());

    /// Function object handle pointer → creation JSContext for callback dispatch.
    pub(crate) static FUNCTION_CONTEXTS: RefCell<HashMap<usize, usize>> = RefCell::new(HashMap::new());

    /// String handle pointer → owned Rust text mirror for JS string readback.
    pub(crate) static STRING_TEXT: RefCell<HashMap<usize, String>> = RefCell::new(HashMap::new());

    /// String handle pointer → Latin1/UTF-8 byte scratch, stable until next update for same string.
    pub(crate) static STRING_LATIN1: RefCell<HashMap<usize, Vec<u8>>> = RefCell::new(HashMap::new());

    /// String handle pointer → UTF-16 scratch, stable until next update for same string.
    pub(crate) static STRING_UTF16: RefCell<HashMap<usize, Vec<u16>>> = RefCell::new(HashMap::new());

    /// Owned opaque object handle allocations. Pointers are valid, non-null, aligned.
    pub(crate) static OBJECT_HANDLES: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());

    /// Owned opaque string handle allocations. Pointers are valid, non-null, aligned.
    pub(crate) static STRING_HANDLES: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());

    /// Owned opaque function handle allocations. Pointers are valid, non-null, aligned.
    pub(crate) static FUNCTION_HANDLES: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());

    /// Object handle pointer → reserved JS values.
    pub(crate) static RESERVED_SLOTS: RefCell<HashMap<usize, Vec<jsapi::JSVal>>> =
        RefCell::new(HashMap::new());

    /// Object handle pointer → private native pointer.
    pub(crate) static PRIVATE_DATA: RefCell<HashMap<usize, *mut std::ffi::c_void>> =
        RefCell::new(HashMap::new());

    pub(crate) static PROXY_OBJECT_EXTRA: RefCell<HashMap<usize, *mut std::ffi::c_void>> =
        RefCell::new(HashMap::new());

    pub(crate) static REALM_OBJECT_PROTOTYPE: RefCell<Option<usize>> = const { RefCell::new(None) };

    pub(crate) static REALM_FUNCTION_PROTOTYPE: RefCell<Option<usize>> = const { RefCell::new(None) };

    pub(crate) static REALM_ERROR_PROTOTYPE: RefCell<Option<usize>> = const { RefCell::new(None) };

    pub(crate) static REALM_ITERATOR_PROTOTYPE: RefCell<Option<usize>> = const { RefCell::new(None) };

    /// (object handle pointer, property name) → JS value.
    pub(crate) static PROPERTIES: RefCell<HashMap<(usize, String), jsapi::JSVal>> =
        RefCell::new(HashMap::new());

    /// Object handle pointer → prototype object handle pointer.
    pub(crate) static PROTOTYPES: RefCell<HashMap<usize, usize>> = RefCell::new(HashMap::new());

    /// ArrayBuffer object handle pointer → byte contents.
    pub(crate) static ARRAY_BUFFERS: RefCell<HashMap<usize, Vec<u8>>> = RefCell::new(HashMap::new());

    /// TypedArray/DataView object handle pointer → (buffer, byte offset, byte length, element type).
    pub(crate) static ARRAY_VIEWS: RefCell<HashMap<usize, (usize, usize, usize, jsapi::Type)>> =
        RefCell::new(HashMap::new());

    /// Detached ArrayBuffer object handles.
    pub(crate) static DETACHED_ARRAY_BUFFERS: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());

    /// Array object handle pointer → elements.
    pub(crate) static ARRAYS: RefCell<HashMap<usize, Vec<jsapi::JSVal>>> = RefCell::new(HashMap::new());

    pub(crate) static PENDING_EXCEPTION: RefCell<Option<jsapi::JSVal>> = const { RefCell::new(None) };

    /// Monotonically increasing handle ID counter (1-based; 0 = null).
    pub(crate) static NEXT_HANDLE: RefCell<usize> = const { RefCell::new(1) };
}

// ── Handle helpers ─────────────────────────────────────────────────────────

fn next_handle() -> usize {
    NEXT_HANDLE.with(|n| {
        let mut n = n.borrow_mut();
        let id = *n;
        *n = n.checked_add(1).expect("V8 JS handle ID overflow");
        id
    })
}

fn allocate_object_handle() -> *mut jsapi::JSObject {
    let raw = Box::into_raw(Box::new(0_u8)) as *mut jsapi::JSObject;
    OBJECT_HANDLES.with(|handles| {
        handles.borrow_mut().insert(raw as usize);
    });
    raw
}

fn allocate_string_handle() -> *mut jsapi::JSString {
    let raw = Box::into_raw(Box::new(0_u8)) as *mut jsapi::JSString;
    STRING_HANDLES.with(|handles| {
        handles.borrow_mut().insert(raw as usize);
    });
    raw
}

fn allocate_function_handle() -> *mut jsapi::JSFunction {
    let raw = Box::into_raw(Box::new(0_u8)) as *mut jsapi::JSFunction;
    FUNCTION_HANDLES.with(|handles| {
        handles.borrow_mut().insert(raw as usize);
    });
    raw
}

pub(crate) fn store_object(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    obj: v8::Local<v8::Object>,
) -> *mut jsapi::JSObject {
    let handle = allocate_object_handle();
    OBJECT_MAP.with(|m| {
        m.borrow_mut()
            .insert(handle as usize, v8::Global::new(scope, obj));
    });
    handle
}

pub(crate) fn lookup_object(ptr: *mut jsapi::JSObject) -> Option<v8::Global<v8::Object>> {
    if ptr.is_null() {
        return None;
    }
    OBJECT_MAP.with(|m| m.borrow().get(&(ptr as usize)).cloned())
}

pub(crate) fn store_string(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    s: v8::Local<v8::String>,
    text: &str,
) -> *mut jsapi::JSString {
    let handle = allocate_string_handle();
    let key = handle as usize;
    STRING_MAP.with(|m| {
        m.borrow_mut().insert(key, v8::Global::new(scope, s));
    });
    STRING_TEXT.with(|m| {
        m.borrow_mut().insert(key, text.to_string());
    });
    handle
}

pub(crate) fn new_v8_string(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    s: &str,
) -> *mut jsapi::JSString {
    match v8::String::new(scope, s) {
        Some(vs) => store_string(scope, vs, s),
        None => ptr::null_mut(),
    }
}

// ── Global object / context ────────────────────────────────────────────────

/// Create a DOM global — the first V8 context for this thread.  Must be called
/// before any other V8-backed operation.
pub fn create_dom_global(
    _cx: *mut jsapi::JSContext,
    clasp: *const jsapi::JSClass,
    _principal: *mut std::ffi::c_void,
) -> *mut jsapi::JSObject {
    crate::ensure_v8();

    crate::V8_ISOLATE.with(|cell| {
        let mut borrow = cell.borrow_mut();
        if borrow.is_none() {
            *borrow = Some(v8::Isolate::new(v8::CreateParams::default()));
        }
        let isolate = borrow.as_mut().unwrap();
        let hs = &mut v8::HandleScope::new(isolate);
        let context = v8::Context::new(hs);
        let ctx_id = next_handle();
        let ctx_global = v8::Global::new(hs, context);
        crate::V8_CONTEXTS.with(|m| {
            m.borrow_mut().insert(ctx_id, ctx_global.clone());
        });
        GLOBAL_ID.with(|g| *g.borrow_mut() = Some(ctx_id));

        let local_ctx = v8::Local::new(hs, ctx_global);
        let cs = &mut v8::ContextScope::new(hs, local_ctx);
        let global_obj = local_ctx.global(cs);
        let global = store_object(cs, global_obj);
        CURRENT_GLOBAL_HANDLE.with(|current| {
            *current.borrow_mut() = Some(global as usize);
        });
        let object_proto = v8::Object::new(cs);
        let object_proto = store_object(cs, object_proto);
        let function_proto = v8::Object::new(cs);
        let function_proto = store_object(cs, function_proto);
        let error_proto = v8::Object::new(cs);
        let error_proto = store_object(cs, error_proto);
        let iterator_proto = v8::Object::new(cs);
        let iterator_proto = store_object(cs, iterator_proto);
        REALM_OBJECT_PROTOTYPE.with(|prototype| {
            *prototype.borrow_mut() = Some(object_proto as usize);
        });
        REALM_FUNCTION_PROTOTYPE.with(|prototype| {
            *prototype.borrow_mut() = Some(function_proto as usize);
        });
        REALM_ERROR_PROTOTYPE.with(|prototype| {
            *prototype.borrow_mut() = Some(error_proto as usize);
        });
        REALM_ITERATOR_PROTOTYPE.with(|prototype| {
            *prototype.borrow_mut() = Some(iterator_proto as usize);
        });
        if let Ok(mut classes) = object_classes().lock() {
            classes.insert(global as usize, clasp as usize);
        }
        global
    })
}

// ── Object construction ────────────────────────────────────────────────────

pub fn js_new_object() -> *mut jsapi::JSObject {
    with_scope(|hs| {
        let obj = v8::Object::new(hs);
        store_object(hs, obj)
    })
}

pub fn js_new_object_with_class(class: *const jsapi::JSClass) -> *mut jsapi::JSObject {
    let obj = js_new_object();
    if !class.is_null() {
        if let Ok(mut classes) = object_classes().lock() {
            classes.insert(obj as usize, class as usize);
        }
    }
    obj
}

pub fn js_new_object_with_proto(
    class: *const jsapi::JSClass,
    proto: *mut jsapi::JSObject,
) -> *mut jsapi::JSObject {
    let obj = js_new_object_with_class(class);
    set_prototype(obj, proto)
}

pub fn get_realm_object_prototype() -> *mut jsapi::JSObject {
    REALM_OBJECT_PROTOTYPE.with(|prototype| {
        prototype
            .borrow()
            .map(|prototype| prototype as *mut jsapi::JSObject)
            .unwrap_or(ptr::null_mut())
    })
}

pub fn get_realm_function_prototype() -> *mut jsapi::JSObject {
    REALM_FUNCTION_PROTOTYPE.with(|prototype| {
        prototype
            .borrow()
            .map(|prototype| prototype as *mut jsapi::JSObject)
            .unwrap_or(ptr::null_mut())
    })
}

pub fn get_realm_error_prototype() -> *mut jsapi::JSObject {
    REALM_ERROR_PROTOTYPE.with(|prototype| {
        prototype
            .borrow()
            .map(|prototype| prototype as *mut jsapi::JSObject)
            .unwrap_or(ptr::null_mut())
    })
}

pub fn get_realm_iterator_prototype() -> *mut jsapi::JSObject {
    REALM_ITERATOR_PROTOTYPE.with(|prototype| {
        prototype
            .borrow()
            .map(|prototype| prototype as *mut jsapi::JSObject)
            .unwrap_or(ptr::null_mut())
    })
}

pub fn current_global_object() -> *mut jsapi::JSObject {
    CURRENT_GLOBAL_HANDLE.with(|current| {
        current
            .borrow()
            .map(|global| global as *mut jsapi::JSObject)
            .unwrap_or(ptr::null_mut())
    })
}

pub fn new_array_object(values: Vec<jsapi::JSVal>) -> *mut jsapi::JSObject {
    let obj = js_new_object();
    set_property_by_name(
        obj,
        c"length".as_ptr() as *const u8,
        jsapi::JSVal::from_uint32(values.len().min(u32::MAX as usize) as u32),
    );
    ARRAYS.with(|arrays| {
        arrays.borrow_mut().insert(obj as usize, values);
    });
    obj
}

pub fn is_array_object(obj: *mut jsapi::JSObject) -> bool {
    !obj.is_null() && ARRAYS.with(|arrays| arrays.borrow().contains_key(&(obj as usize)))
}

pub fn array_length(obj: *mut jsapi::JSObject) -> u32 {
    ARRAYS.with(|arrays| {
        arrays
            .borrow()
            .get(&(obj as usize))
            .map(|values| values.len().min(u32::MAX as usize) as u32)
            .unwrap_or(0)
    })
}

pub fn get_array_element(obj: *mut jsapi::JSObject, index: u32) -> Option<jsapi::JSVal> {
    ARRAYS.with(|arrays| {
        arrays
            .borrow()
            .get(&(obj as usize))
            .and_then(|values| values.get(index as usize).copied())
    })
}

pub fn set_array_element(obj: *mut jsapi::JSObject, index: u32, val: jsapi::JSVal) -> bool {
    if obj.is_null() || !is_array_object(obj) {
        return false;
    }
    ARRAYS.with(|arrays| {
        let mut arrays = arrays.borrow_mut();
        let values = arrays.entry(obj as usize).or_default();
        let index = index as usize;
        if values.len() <= index {
            values.resize(index + 1, jsapi::JSVal::undefined());
        }
        values[index] = val;
        set_property_by_name(
            obj,
            c"length".as_ptr() as *const u8,
            jsapi::JSVal::from_uint32(values.len().min(u32::MAX as usize) as u32),
        );
    });
    true
}

pub fn new_array_buffer(len: usize) -> *mut jsapi::JSObject {
    let obj = js_new_object();
    ARRAY_BUFFERS.with(|buffers| {
        buffers.borrow_mut().insert(obj as usize, vec![0; len]);
    });
    obj
}

pub fn new_array_buffer_with_bytes(bytes: &[u8]) -> *mut jsapi::JSObject {
    let obj = js_new_object();
    ARRAY_BUFFERS.with(|buffers| {
        buffers.borrow_mut().insert(obj as usize, bytes.to_vec());
    });
    obj
}

pub fn new_array_buffer_with_contents(len: usize, data: *const u8) -> *mut jsapi::JSObject {
    if data.is_null() {
        return new_array_buffer(len);
    }
    // SAFETY: JSAPI caller promises `data` points to `len` readable bytes.
    let bytes = unsafe { std::slice::from_raw_parts(data, len) };
    new_array_buffer_with_bytes(bytes)
}

pub fn array_buffer_len(obj: *mut jsapi::JSObject) -> usize {
    if obj.is_null() {
        return 0;
    }
    ARRAY_BUFFERS.with(|buffers| {
        buffers
            .borrow()
            .get(&(obj as usize))
            .map(Vec::len)
            .unwrap_or(0)
    })
}

pub fn is_array_buffer(obj: *mut jsapi::JSObject) -> bool {
    !obj.is_null() && ARRAY_BUFFERS.with(|buffers| buffers.borrow().contains_key(&(obj as usize)))
}

pub fn is_detached_array_buffer(obj: *mut jsapi::JSObject) -> bool {
    !obj.is_null()
        && DETACHED_ARRAY_BUFFERS.with(|detached| detached.borrow().contains(&(obj as usize)))
}

pub fn detach_array_buffer(obj: *mut jsapi::JSObject) -> bool {
    if !is_array_buffer(obj) {
        return false;
    }
    DETACHED_ARRAY_BUFFERS.with(|detached| {
        detached.borrow_mut().insert(obj as usize);
    });
    true
}

pub fn clone_array_buffer(
    obj: *mut jsapi::JSObject,
    byte_offset: usize,
    byte_length: usize,
) -> *mut jsapi::JSObject {
    if !is_array_buffer(obj) || is_detached_array_buffer(obj) {
        return ptr::null_mut();
    }
    let bytes = ARRAY_BUFFERS.with(|buffers| {
        buffers.borrow().get(&(obj as usize)).and_then(|bytes| {
            byte_offset
                .checked_add(byte_length)
                .and_then(|end| bytes.get(byte_offset..end))
                .map(<[u8]>::to_vec)
        })
    });
    match bytes {
        Some(bytes) => new_array_buffer_with_bytes(&bytes),
        None => ptr::null_mut(),
    }
}

pub fn new_typed_array_with_buffer(
    buffer: *mut jsapi::JSObject,
    offset: usize,
    len: i64,
    ty: jsapi::Type,
) -> *mut jsapi::JSObject {
    if !is_array_buffer(buffer) || is_detached_array_buffer(buffer) {
        return ptr::null_mut();
    }
    let Some(element_size) = ty.byte_size() else {
        return ptr::null_mut();
    };
    let buffer_len = array_buffer_len(buffer);
    let byte_len = if len < 0 {
        buffer_len.saturating_sub(offset)
    } else {
        (len as usize).saturating_mul(element_size)
    };
    if offset
        .checked_add(byte_len)
        .is_none_or(|end| end > buffer_len)
    {
        return ptr::null_mut();
    }
    let view = js_new_object();
    ARRAY_VIEWS.with(|views| {
        views
            .borrow_mut()
            .insert(view as usize, (buffer as usize, offset, byte_len, ty));
    });
    view
}

pub fn new_typed_array_from_bytes(bytes: &[u8], ty: jsapi::Type) -> *mut jsapi::JSObject {
    let buffer = new_array_buffer_with_bytes(bytes);
    new_typed_array_with_buffer(buffer, 0, -1, ty)
}

pub fn is_array_buffer_view(obj: *mut jsapi::JSObject) -> bool {
    !obj.is_null() && ARRAY_VIEWS.with(|views| views.borrow().contains_key(&(obj as usize)))
}

pub fn array_view_buffer(obj: *mut jsapi::JSObject) -> *mut jsapi::JSObject {
    if obj.is_null() {
        return ptr::null_mut();
    }
    ARRAY_VIEWS.with(|views| {
        views
            .borrow()
            .get(&(obj as usize))
            .map(|(buffer, _, _, _)| *buffer as *mut jsapi::JSObject)
            .unwrap_or(ptr::null_mut())
    })
}

pub fn array_view_byte_length(obj: *mut jsapi::JSObject) -> usize {
    ARRAY_VIEWS.with(|views| {
        views
            .borrow()
            .get(&(obj as usize))
            .map(|(_, _, byte_len, _)| *byte_len)
            .unwrap_or(0)
    })
}

pub fn array_view_byte_offset(obj: *mut jsapi::JSObject) -> usize {
    ARRAY_VIEWS.with(|views| {
        views
            .borrow()
            .get(&(obj as usize))
            .map(|(_, offset, _, _)| *offset)
            .unwrap_or(0)
    })
}

pub fn array_view_type(obj: *mut jsapi::JSObject) -> jsapi::Type {
    ARRAY_VIEWS.with(|views| {
        views
            .borrow()
            .get(&(obj as usize))
            .map(|(_, _, _, ty)| *ty)
            .unwrap_or(jsapi::Type::MaxTypedArrayViewType)
    })
}

pub fn typed_array_length(obj: *mut jsapi::JSObject) -> usize {
    let ty = array_view_type(obj);
    match ty.byte_size() {
        Some(size) if size != 0 => array_view_byte_length(obj) / size,
        _ => 0,
    }
}

pub fn array_view_bytes(obj: *mut jsapi::JSObject) -> Vec<u8> {
    let view = ARRAY_VIEWS.with(|views| views.borrow().get(&(obj as usize)).copied());
    let Some((buffer, offset, byte_len, _)) = view else {
        return ARRAY_BUFFERS
            .with(|buffers| buffers.borrow().get(&(obj as usize)).cloned())
            .unwrap_or_default();
    };
    ARRAY_BUFFERS.with(|buffers| {
        buffers
            .borrow()
            .get(&buffer)
            .and_then(|bytes| {
                offset
                    .checked_add(byte_len)
                    .and_then(|end| bytes.get(offset..end))
            })
            .map(<[u8]>::to_vec)
            .unwrap_or_default()
    })
}

pub fn new_proxy_object(
    _cx: *mut jsapi::JSContext,
    handler: *const std::ffi::c_void,
    proto: *mut jsapi::JSObject,
    _flag: bool,
) -> *mut jsapi::JSObject {
    let obj = js_new_object();
    if let Ok(mut classes) = object_classes().lock() {
        classes.insert(obj as usize, &jsapi::ProxyClass as *const _ as usize);
    }
    let extra = proxy_handler_extras()
        .lock()
        .ok()
        .and_then(|handlers| handlers.get(&(handler as usize)).copied())
        .or_else(|| {
            last_proxy_handler_extra()
                .lock()
                .ok()
                .and_then(|last| *last)
        });
    if let Some(extra) = extra {
        PROXY_OBJECT_EXTRA.with(|objects| {
            objects
                .borrow_mut()
                .insert(obj as usize, extra as *mut std::ffi::c_void);
        });
    }
    set_prototype(obj, proto)
}

pub fn new_promise_object(_executor: *mut jsapi::JSObject) -> *mut jsapi::JSObject {
    let obj = js_new_object();
    if let Ok(mut states) = promise_states().lock() {
        states.insert(obj as usize, jsapi::PromiseState::Pending);
    }
    obj
}

pub fn is_promise_object(obj: *mut jsapi::JSObject) -> bool {
    promise_states()
        .lock()
        .ok()
        .is_some_and(|states| states.contains_key(&(obj as usize)))
}

pub fn get_promise_state(obj: *mut jsapi::JSObject) -> jsapi::PromiseState {
    promise_states()
        .lock()
        .ok()
        .and_then(|states| states.get(&(obj as usize)).copied())
        .unwrap_or(jsapi::PromiseState::Pending)
}

pub fn set_promise_state(obj: *mut jsapi::JSObject, state: jsapi::PromiseState) -> bool {
    if obj.is_null() {
        return false;
    }
    if let Ok(mut states) = promise_states().lock() {
        states.insert(obj as usize, state);
    }
    true
}

pub fn js_new_function(
    cx: *mut jsapi::JSContext,
    native: Option<jsapi::NativeCallback>,
    _nargs: u32,
    _flags: u32,
    _name: Option<String>,
) -> *mut jsapi::JSFunction {
    let obj = js_new_object();
    if let Some(native) = native {
        FUNCTION_NATIVES.with(|m| {
            m.borrow_mut().insert(obj as usize, native);
        });
    }
    if !cx.is_null() {
        FUNCTION_CONTEXTS.with(|m| {
            m.borrow_mut().insert(obj as usize, cx as usize);
        });
    }
    let fun = allocate_function_handle();
    FUNCTION_OBJECTS.with(|m| {
        m.borrow_mut().insert(fun as usize, obj as usize);
    });
    fun
}

pub fn get_function_object(fun: *mut jsapi::JSFunction) -> *mut jsapi::JSObject {
    if fun.is_null() {
        return ptr::null_mut();
    }
    FUNCTION_OBJECTS.with(|m| {
        m.borrow()
            .get(&(fun as usize))
            .copied()
            .map(|obj| obj as *mut jsapi::JSObject)
            .unwrap_or(ptr::null_mut())
    })
}

pub fn set_function_native_reserved(obj: *mut jsapi::JSObject, slot: usize, val: jsapi::JSVal) {
    set_reserved_slot(obj, slot as u32, val)
}

pub fn get_function_native_reserved(obj: *mut jsapi::JSObject, slot: usize) -> jsapi::JSVal {
    get_reserved_slot(obj, slot as u32)
}

pub fn set_prototype(
    obj: *mut jsapi::JSObject,
    proto: *mut jsapi::JSObject,
) -> *mut jsapi::JSObject {
    if obj.is_null() {
        return ptr::null_mut();
    }
    PROTOTYPES.with(|m| {
        let mut m = m.borrow_mut();
        if proto.is_null() {
            m.remove(&(obj as usize));
        } else {
            m.insert(obj as usize, proto as usize);
        }
    });
    obj
}

pub fn get_prototype(obj: *mut jsapi::JSObject) -> *mut jsapi::JSObject {
    if obj.is_null() {
        return ptr::null_mut();
    }
    PROTOTYPES.with(|m| {
        m.borrow()
            .get(&(obj as usize))
            .copied()
            .map(|proto| proto as *mut jsapi::JSObject)
            .unwrap_or(ptr::null_mut())
    })
}

pub fn link_constructor_and_prototype(
    ctor: *mut jsapi::JSObject,
    proto: *mut jsapi::JSObject,
) -> bool {
    if ctor.is_null() || proto.is_null() {
        return false;
    }
    set_prototype(ctor, proto);
    set_property_by_name(
        ctor,
        c"prototype".as_ptr() as *const u8,
        jsapi::JSVal::from_object(proto),
    ) && set_property_by_name(
        proto,
        c"constructor".as_ptr() as *const u8,
        jsapi::JSVal::from_object(ctor),
    )
}

pub fn call_function(
    cx: *mut jsapi::JSContext,
    fun: jsapi::JSVal,
    args: jsapi::HandleValueArray,
    rval: *mut jsapi::JSVal,
) -> bool {
    let obj = fun.to_object();
    if obj.is_null() {
        return false;
    }
    let native = FUNCTION_NATIVES.with(|m| m.borrow().get(&(obj as usize)).copied());
    let Some(native) = native else {
        return false;
    };

    let argc = args.length_.min(u32::MAX as usize) as u32;
    let mut vp = Vec::with_capacity(argc as usize + 2);
    vp.push(jsapi::JSVal::undefined());
    vp.push(jsapi::JSVal::from_object(obj));
    if !args.elements_.is_null() {
        for i in 0..argc as usize {
            // SAFETY: HandleValueArray promises `elements_` points to `length_` values.
            vp.push(unsafe { *args.elements_.add(i) });
        }
    }

    let dispatch_cx = if cx.is_null() {
        FUNCTION_CONTEXTS.with(|m| {
            m.borrow()
                .get(&(obj as usize))
                .copied()
                .map(|cx| cx as *mut jsapi::JSContext)
                .unwrap_or(ptr::null_mut())
        })
    } else {
        cx
    };

    // SAFETY: native JSAPI callback receives the temporary vp layout expected by CallArgs::from_vp.
    let ok = unsafe { native(dispatch_cx, argc, vp.as_mut_ptr()) };
    if ok && !rval.is_null() {
        // SAFETY: non-null JSAPI out-param checked above.
        unsafe { *rval = vp[0] };
    }
    ok
}

pub fn get_private(obj: *mut jsapi::JSObject) -> *mut std::ffi::c_void {
    if obj.is_null() {
        return ptr::null_mut();
    }
    PRIVATE_DATA.with(|m| {
        m.borrow()
            .get(&(obj as usize))
            .copied()
            .unwrap_or(ptr::null_mut())
    })
}

pub fn set_private(obj: *mut jsapi::JSObject, data: *mut std::ffi::c_void) {
    if obj.is_null() {
        return;
    }
    PRIVATE_DATA.with(|m| {
        m.borrow_mut().insert(obj as usize, data);
    });
}

pub fn is_exception_pending() -> bool {
    PENDING_EXCEPTION.with(|pending| pending.borrow().is_some())
}

pub fn clear_pending_exception() {
    PENDING_EXCEPTION.with(|pending| {
        *pending.borrow_mut() = None;
    });
}

pub fn get_pending_exception() -> Option<jsapi::JSVal> {
    PENDING_EXCEPTION.with(|pending| *pending.borrow())
}

pub fn set_pending_exception(val: jsapi::JSVal) {
    PENDING_EXCEPTION.with(|pending| {
        *pending.borrow_mut() = Some(val);
    });
}

pub fn get_object_class(obj: *mut jsapi::JSObject) -> *const jsapi::JSClass {
    object_classes()
        .lock()
        .ok()
        .and_then(|classes| classes.get(&(obj as usize)).copied())
        .map(|class| class as *const jsapi::JSClass)
        .unwrap_or(&jsapi::ProxyClass)
}

pub fn get_proxy_private() {}
pub fn set_proxy_private() {}

// ── Property operations ────────────────────────────────────────────────────

pub fn property_name_from_raw(name: *const u8) -> Option<String> {
    if name.is_null() {
        return None;
    }
    if (name as usize) < 4096 {
        return None;
    }
    // SAFETY: JSAPI property-name inputs are null-terminated C strings.
    let cstr = unsafe { std::ffi::CStr::from_ptr(name as *const std::os::raw::c_char) };
    Some(cstr.to_string_lossy().into_owned())
}

fn property_name_from_ptr(name: *const u8) -> Option<String> {
    property_name_from_raw(name)
}

pub fn set_property_by_name(obj: *mut jsapi::JSObject, name: *const u8, val: jsapi::JSVal) -> bool {
    if obj.is_null() {
        return false;
    }
    let Some(name) = property_name_from_ptr(name) else {
        return false;
    };
    PROPERTIES.with(|props| {
        props.borrow_mut().insert((obj as usize, name), val);
    });
    true
}

pub fn set_property(obj: *mut jsapi::JSObject, name: &str, val: jsapi::JSVal) -> bool {
    if obj.is_null() {
        return false;
    }
    PROPERTIES.with(|props| {
        props
            .borrow_mut()
            .insert((obj as usize, name.to_string()), val);
    });
    true
}

pub fn get_property_by_name(obj: *mut jsapi::JSObject, name: *const u8) -> Option<jsapi::JSVal> {
    if obj.is_null() {
        return None;
    }
    let name = property_name_from_ptr(name)?;
    PROPERTIES.with(|props| props.borrow().get(&(obj as usize, name)).copied())
}

fn property_name_from_jsid(id: jsapi::jsid) -> Option<String> {
    if id.is_string() {
        string_text(id.to_string())
    } else if id.is_int() {
        Some(id.to_int().to_string())
    } else {
        None
    }
}

pub fn set_property_by_jsid(obj: *mut jsapi::JSObject, id: jsapi::jsid, val: jsapi::JSVal) -> bool {
    if obj.is_null() {
        return false;
    }
    if id.is_int() {
        return set_array_element(obj, id.to_int() as u32, val);
    }
    let Some(name) = property_name_from_jsid(id) else {
        return false;
    };
    PROPERTIES.with(|props| {
        props.borrow_mut().insert((obj as usize, name), val);
    });
    true
}

pub fn get_property_by_jsid(obj: *mut jsapi::JSObject, id: jsapi::jsid) -> Option<jsapi::JSVal> {
    if obj.is_null() {
        return None;
    }
    if id.is_int() {
        return get_array_element(obj, id.to_int() as u32);
    }
    let name = property_name_from_jsid(id)?;
    PROPERTIES.with(|props| props.borrow().get(&(obj as usize, name)).copied())
}

pub fn has_property_by_jsid(obj: *mut jsapi::JSObject, id: jsapi::jsid) -> bool {
    get_property_by_jsid(obj, id).is_some()
}

pub fn get_property_descriptor_by_jsid<F>(
    obj: *mut jsapi::JSObject,
    id: jsapi::jsid,
    desc: *mut jsapi::PropertyDescriptor,
    is_none: F,
) -> bool
where
    F: jsapi::SetJsapiBoolOut,
{
    if obj.is_null() {
        is_none.set_jsapi_bool_out(true);
        if !desc.is_null() {
            // SAFETY: non-null descriptor out-param checked above.
            unsafe { (*desc).clear() };
        }
        return true;
    }

    match get_property_by_jsid(obj, id) {
        Some(value) => {
            is_none.set_jsapi_bool_out(false);
            if !desc.is_null() {
                // SAFETY: non-null descriptor out-param checked above.
                unsafe { (*desc).set_data_descriptor(obj, value, jsapi::JSPROP_ENUMERATE) };
            }
            true
        },
        None => {
            is_none.set_jsapi_bool_out(true);
            if !desc.is_null() {
                // SAFETY: non-null descriptor out-param checked above.
                unsafe { (*desc).clear() };
            }
            true
        },
    }
}

pub fn append_to_id_vector(ids: jsapi::MutableHandleIdVector, id: jsapi::jsid) -> bool {
    if ids.is_null() {
        return false;
    }
    // SAFETY: non-null id vector pointer checked above; IdVector::handle_mut provides this pointer.
    unsafe { (*ids).push(id) };
    true
}

pub fn get_property_keys(obj: *mut jsapi::JSObject, ids: jsapi::MutableHandleIdVector) -> bool {
    if obj.is_null() || ids.is_null() {
        return false;
    }

    let array_len = if is_array_object(obj) {
        Some(array_length(obj))
    } else {
        None
    };
    let names = PROPERTIES.with(|props| {
        props
            .borrow()
            .keys()
            .filter_map(|(key_obj, name)| (*key_obj == obj as usize).then_some(name.clone()))
            .collect::<Vec<_>>()
    });

    with_scope(|hs| {
        if let Some(len) = array_len {
            for index in 0..len {
                if !append_to_id_vector(ids, jsapi::jsid::from_int(index as i32)) {
                    return false;
                }
            }
        }
        for name in names {
            let s = new_v8_string(hs, &name);
            if s.is_null() || !append_to_id_vector(ids, jsapi::jsid::from_string(s)) {
                return false;
            }
        }
        true
    })
}

pub fn id_to_string(id: jsapi::jsid) -> *mut jsapi::JSString {
    if id.is_string() {
        id.to_string()
    } else if id.is_int() {
        with_scope(|hs| new_v8_string(hs, &id.to_int().to_string()))
    } else {
        ptr::null_mut()
    }
}

pub fn id_to_value<V>(id: jsapi::jsid, vp: V) -> bool
where
    V: jsapi::SetJsapiValOut,
{
    if id.is_string() {
        vp.set_jsapi_val_out(jsapi::JSVal::from_string(id.to_string()));
        true
    } else if id.is_int() {
        vp.set_jsapi_val_out(jsapi::JSVal::from_int32(id.to_int()));
        true
    } else {
        false
    }
}

pub fn value_to_source(val: jsapi::JSVal) -> *mut jsapi::JSString {
    let source = if val.is_undefined() {
        "undefined".to_string()
    } else if val.is_null() {
        "null".to_string()
    } else if val.is_boolean() {
        val.to_boolean().to_string()
    } else if val.is_number() {
        val.to_number().to_string()
    } else if val.is_string() {
        string_text(val.to_string()).unwrap_or_default()
    } else if val.is_object() {
        "[object Object]".to_string()
    } else {
        String::new()
    };
    with_scope(|hs| new_v8_string(hs, &source))
}

pub fn define_property_by_descriptor(
    obj: *mut jsapi::JSObject,
    id: jsapi::jsid,
    desc: *const jsapi::PropertyDescriptor,
) -> bool {
    if obj.is_null() || desc.is_null() {
        return false;
    }
    // SAFETY: non-null descriptor input checked above.
    let desc = unsafe { *desc };
    if desc.hasValue_() {
        set_property_by_jsid(obj, id, desc.value())
    } else {
        true
    }
}

pub fn define_property_by_id(
    _cx: *mut jsapi::JSContext,
    obj: *mut jsapi::JSObject,
    _id: *const jsapi::jsid,
    _desc: *const jsapi::JSPropertySpec,
) -> bool {
    with_scope(|hs| match lookup_object(obj) {
        Some(g) => {
            let _local = v8::Local::new(hs, g);
            true // property definition skipped for now; bindings survive
        },
        None => false,
    })
}

pub fn set_data_property_descriptor() {}

pub fn delete_property_ignoring_result(
    _cx: *mut jsapi::JSContext,
    _obj: *mut jsapi::JSObject,
    _prop: *const u8,
) {
}

// ── String / atom operations ───────────────────────────────────────────────

pub fn atomize_string_n(
    _cx: *mut jsapi::JSContext,
    s: *const u8,
    len: usize,
) -> *mut jsapi::JSString {
    if s.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: JSAPI caller promises `s` points to `len` readable bytes.
    let slice = unsafe { std::slice::from_raw_parts(s, len) };
    let text = match std::str::from_utf8(slice) {
        Ok(text) => text,
        Err(_) => return ptr::null_mut(),
    };
    with_scope(|hs| new_v8_string(hs, text))
}

pub fn string_text(s: *mut jsapi::JSString) -> Option<String> {
    if s.is_null() {
        return None;
    }
    STRING_TEXT.with(|m| m.borrow().get(&(s as usize)).cloned())
}

pub fn string_len(s: *mut jsapi::JSString) -> usize {
    if s.is_null() {
        return 0;
    }
    STRING_TEXT.with(|m| {
        m.borrow()
            .get(&(s as usize))
            .map(|text| text.encode_utf16().count())
            .unwrap_or(0)
    })
}

pub fn linear_string_char_at(s: *mut jsapi::JSString, index: usize) -> u16 {
    if s.is_null() {
        return 0;
    }
    STRING_TEXT.with(|m| {
        m.borrow()
            .get(&(s as usize))
            .and_then(|text| text.encode_utf16().nth(index))
            .unwrap_or(0)
    })
}

pub fn latin1_chars_and_len(s: *mut jsapi::JSString, len: *mut usize) -> *const u8 {
    if s.is_null() {
        if !len.is_null() {
            // SAFETY: non-null out pointer checked above.
            unsafe { *len = 0 };
        }
        return ptr::null();
    }
    let key = s as usize;
    let text = STRING_TEXT.with(|m| m.borrow().get(&key).cloned());
    let Some(text) = text else {
        if !len.is_null() {
            // SAFETY: non-null out pointer checked above.
            unsafe { *len = 0 };
        }
        return ptr::null();
    };
    STRING_LATIN1.with(|m| {
        let mut m = m.borrow_mut();
        let buf = m.entry(key).or_default();
        buf.clear();
        buf.extend(text.bytes());
        if !len.is_null() {
            // SAFETY: non-null out pointer checked above.
            unsafe { *len = buf.len() };
        }
        buf.as_ptr()
    })
}

pub fn two_byte_chars_and_len(s: *mut jsapi::JSString, len: *mut usize) -> *const u16 {
    if s.is_null() {
        if !len.is_null() {
            // SAFETY: non-null out pointer checked above.
            unsafe { *len = 0 };
        }
        return ptr::null();
    }
    let key = s as usize;
    let text = STRING_TEXT.with(|m| m.borrow().get(&key).cloned());
    let Some(text) = text else {
        if !len.is_null() {
            // SAFETY: non-null out pointer checked above.
            unsafe { *len = 0 };
        }
        return ptr::null();
    };
    STRING_UTF16.with(|m| {
        let mut m = m.borrow_mut();
        let buf = m.entry(key).or_default();
        buf.clear();
        buf.extend(text.encode_utf16());
        if !len.is_null() {
            // SAFETY: non-null out pointer checked above.
            unsafe { *len = buf.len() };
        }
        buf.as_ptr()
    })
}

pub fn has_latin1_chars(s: *mut jsapi::JSString) -> bool {
    if s.is_null() {
        return false;
    }
    STRING_TEXT.with(|m| {
        m.borrow()
            .get(&(s as usize))
            .map(|text| text.chars().all(|c| c as u32 <= 0xff))
            .unwrap_or(false)
    })
}

// ── JIT callbacks ──────────────────────────────────────────────────────────

/// Return false → the caller falls back to the interpreted binding path.
pub fn call_jit_getter_op(
    _info: *const jsapi::JSJitInfo,
    _cx: *mut jsapi::JSContext,
    _obj: jsapi::HandleObject<'_>,
    _this: *mut std::ffi::c_void,
    _argc: u32,
    _vp: *mut jsapi::JSVal,
) -> bool {
    false
}

pub fn call_jit_method_op(
    _info: *const jsapi::JSJitInfo,
    _cx: *mut jsapi::JSContext,
    _obj: jsapi::HandleObject<'_>,
    _this: *mut std::ffi::c_void,
    _argc: u32,
    _vp: *mut jsapi::JSVal,
) -> bool {
    false
}

pub fn call_jit_setter_op<O>(
    _info: *const jsapi::JSJitInfo,
    _cx: *mut jsapi::JSContext,
    _obj: O,
    _this: *mut std::ffi::c_void,
    _argc: u32,
    _vp: *mut jsapi::JSVal,
) -> bool {
    false
}

// ── Proxy handler management ──────────────────────────────────────────────

pub fn create_proxy_handler(
    _traps: &crate::glue::ProxyTraps,
    extra: *const std::ffi::c_void,
) -> *mut std::ffi::c_void {
    let handler = _traps as *const _ as *mut std::ffi::c_void;
    if let Ok(mut handlers) = proxy_handler_extras().lock() {
        handlers.insert(handler as usize, extra as usize);
    }
    if let Ok(mut last) = last_proxy_handler_extra().lock() {
        *last = Some(extra as usize);
    }
    handler
}

pub fn get_proxy_handler_extra(proxy: *mut jsapi::JSObject) -> *mut std::ffi::c_void {
    PROXY_OBJECT_EXTRA.with(|objects| {
        objects
            .borrow()
            .get(&(proxy as usize))
            .copied()
            .or_else(|| {
                last_proxy_handler_extra()
                    .lock()
                    .ok()
                    .and_then(|last| *last)
                    .map(|extra| extra as *mut std::ffi::c_void)
            })
            .unwrap_or(ptr::null_mut())
    })
}

pub fn get_reserved_slot(obj: *mut jsapi::JSObject, slot: u32) -> jsapi::JSVal {
    if obj.is_null() {
        return jsapi::JSVal::default();
    }
    RESERVED_SLOTS.with(|slots| {
        slots
            .borrow()
            .get(&(obj as usize))
            .and_then(|values| values.get(slot as usize).copied())
            .unwrap_or_default()
    })
}

pub fn set_reserved_slot(obj: *mut jsapi::JSObject, slot: u32, val: jsapi::JSVal) {
    if obj.is_null() {
        return;
    }
    RESERVED_SLOTS.with(|slots| {
        let mut slots = slots.borrow_mut();
        let values = slots.entry(obj as usize).or_default();
        let slot = slot as usize;
        if values.len() <= slot {
            values.resize(slot + 1, jsapi::JSVal::default());
        }
        values[slot] = val;
    });
}

pub fn get_proxy_reserved_slot(proxy: *mut jsapi::JSObject, slot: u32, out: *mut jsapi::JSVal) {
    if out.is_null() {
        return;
    }
    // SAFETY: caller supplied non-null out pointer for JSAPI out-param.
    unsafe {
        *out = get_reserved_slot(proxy, slot);
    }
}

pub fn js_get_reserved_slot() {}
pub fn set_proxy_reserved_slot() {}

// ── Job queue (promise microtask integration) ──────────────────────────────

pub fn create_job_queue() -> *mut jsapi::JobQueue {
    Box::into_raw(Box::new(0_u8)) as *mut jsapi::JobQueue
}

pub fn delete_job_queue(queue: *mut jsapi::JobQueue) {
    if queue.is_null() {
        return;
    }
    // SAFETY: create_job_queue returns Box<u8> cast to JobQueue pointer.
    unsafe {
        drop(Box::from_raw(queue as *mut u8));
    }
}

pub fn dispatchable_run(_cx: *mut jsapi::JSContext) {
    crate::V8_ISOLATE.with(|cell| {
        if let Some(ref mut isolate) = *cell.borrow_mut() {
            isolate.perform_microtask_checkpoint();
        }
    })
}

// ── Script execution ───────────────────────────────────────────────────────

pub fn evaluate_script(script: &str) -> bool {
    with_scope(|hs| {
        let source = match v8::String::new(hs, script) {
            Some(s) => s,
            None => return false,
        };
        let script_obj = match v8::Script::compile(hs, source, None) {
            Some(s) => s,
            None => return false,
        };
        script_obj.run(hs).is_some()
    })
}

pub fn evaluate_to_string(script: &str) -> Option<String> {
    with_scope(|hs| {
        let source = match v8::String::new(hs, script) {
            Some(s) => s,
            None => return None,
        };
        let script_obj = match v8::Script::compile(hs, source, None) {
            Some(s) => s,
            None => return None,
        };
        script_obj
            .run(hs)
            .and_then(|r| r.to_string(hs).map(|s| s.to_rust_string_lossy(hs)))
    })
}

// ── Internal: scope helper ─────────────────────────────────────────────────

/// Helper: creates a HandleScope + ContextScope for the current global realm,
/// calls `f` with the HandleScope (so V8 handle allocation works).
fn with_scope<R>(f: impl FnOnce(&mut v8::ContextScope<v8::HandleScope>) -> R) -> R {
    crate::V8_ISOLATE.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let isolate = borrow
            .as_mut()
            .expect("V8 isolate not initialised; call create_dom_global first");
        let hs = &mut v8::HandleScope::new(isolate);

        let ctx_id = GLOBAL_ID.with(|g| g.borrow().expect("no global context set"));
        let ctx_global = crate::V8_CONTEXTS.with(|m| {
            m.borrow()
                .get(&ctx_id)
                .cloned()
                .expect("global context ID not in context map")
        });

        let local_ctx = v8::Local::new(hs, ctx_global);
        let cs = &mut v8::ContextScope::new(hs, local_ctx);
        f(cs)
    })
}

/// Thread-local JSContext token for Servo bindings that cannot thread `&mut JSContext`.
pub(crate) fn thread_js_context() -> Option<crate::context::JSContext> {
    GLOBAL_ID.with(|g| {
        g.borrow().map(|_| unsafe {
            crate::context::JSContext::from_ptr(std::ptr::NonNull::dangling())
        })
    })
}
