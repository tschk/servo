# js_stub Remaining Work — V8 Glue Surface

## Current state

| Metric | Count |
|--------|-------|
| `v8_glue.rs` public fns | ~109 |
| `v8_glue.rs` tests | 8 |
| `lib.rs` total lines | ~7,600 |
| `glue_stub!` fallbacks (non-v8 only) | 19 |
| `JSVal::default()` still-stubs | ~18 |
| `ptr::null_mut()` returns | ~138 |
| `v8`-feature `cfg` blocks | ~383 |

Page scripts now **compile and execute on rusty_v8**. `SourceText → Compile1 → JS_ExecuteScript`
is real V8. Debugger constructor, Date, EventTarget bootstrap all present.

## Implemented (done)

- [x] SourceText store + `Compile1` + `JS_ExecuteScript`
- [x] `SetScriptPrivate` / `JS_GetScriptPrivate` 
- [x] `DoubleValue` / `NumberValue` (int when exact, else string)
- [x] `Debugger` constructor bootstrap
- [x] `NewDateObject` via V8 `Date`
- [x] `EventTarget` / `addEventListener` / `dispatchEvent` bootstrap
- [x] Core object lifecycle: `create_dom_global`, `js_new_object`, `enter/leave_realm`
- [x] Property get/set by name and jsid
- [x] Reserved slots, prototype chain, window proxy
- [x] Typed arrays, ArrayBuffers, Promise tracking
- [x] Function creation + native callback dispatch
- [x] Proxy handler + proxy reserved slots
- [x] Exception pending/clear/set
- [x] `define_property_by_id`, `delete_property_ignoring_result`
- [x] `atomize_string_n`, string readback
- [x] `JS_GetReservedSlot`, `JS_GetPropertyById`
- [x] `JS_NewStringCopyN`
- [x] `CallJitGetterOp`, `CallJitMethodOp`, `CallJitSetterOp`
- [x] `JS_NewObject`, `JS_NewPlainObject` (via v8_glue)
- [x] `ModuleEvaluate`, `ModuleLink` (return pending promise)
- [x] `JS_ExecuteScript` real V8 compile+run

## Still stubbed — runtime critical (next)

| API | Current | Fix |
|-----|---------|-----|
| `JS_GetModulePrivate` | `JSVal::default()` | Store in thread-local map like script private |
| `JS_NewGlobalObject` | returns `null_mut()` | Wire to `create_dom_global` |
| `JS_GetPrototype` (wrappers2) | returns `bool` but no real read | Already wired via v8_glue `get_prototype` |
| `IsArrayObject` (wrappers2) | writes `false` | Wire to `ARRAYS` map check |
| `JS_TypeOfValue` | returns `JSType::Void` | Convert V8 type to JSType enum |
| `JS_GetClass` | returns `&ProxyClass` | Already wired via `get_object_class` |
| `Heap<T>` boxing/unboxing | `RefCell<Option<T>>` | May be fine for now; trace needs care |
| `JS_FireOnNewGlobalObject` | no-op | Need: define Debugger + dispatch DOMContentLoaded |
| `JS_AlreadyHasOwnPropertyById` | returns `false` | Wire to property map check |
| `JS_HasProperty` / `JS_HasOwnProperty` | returns `false` | Wire to property lookup |
| `JS_HasPropertyById` | returns `false` | Wire to property lookup |

## Still stubbed — less critical

| API | Current | Notes |
|-----|---------|-------|
| GC tracers (`Call*Tracer`) | no-op | V8 has own GC; these only needed for mozjs trace protocol |
| `GetProxyHandler` / `IsProxyHandlerFamily` | no-op | Proxy traps dispatch already works via `call_jit_*` |
| `JS_GetTwoByteStringCharsAndLength` | null | String readback works via `v8_glue::string_text` |
| `GetLinearStringCharAt` / `GetLinearStringLength` | 0 | Linear string path not used under V8 |
| `AddAssociatedMemory` / `RemoveAssociatedMemory` | no-op | Memory pressure reporting, not correctness |
| `JS_DropPrincipals` / `JS_HoldPrincipals` | no-op | Security principals not used in Servo |
| `HideScriptedCaller` / `UnhideScriptedCaller` | no-op | Devtools stack trace only |
| `JS_MayResolveStandardClass` | returns `false` | Standard class resolution not needed under V8 |
| `SetDOMProxyInformation` | no-op | DOM proxy metadata for devtools |
| `JS_IterateCompartments` | no-op | Multi-compartment diagnostic |
| `JS_GlobalObjectTraceHook` | no-op | GC trace hook for global |
| `GCTraceKindToAscii` | `b'\0'` | GC diagnostics |

## Remaining issues (from TODO.md)

| Issue | Status | Impact |
|-------|--------|--------|
| A. SafeJSContext reborrow generation | Open | `cx` moved into wrapper before later `cx.into()` |
| B. Rooting and trace boxes | Partial | `RootedGuard` may leak under V8 |
| C. JSJitInfo field initialization | Partial | Getter/setter info structs need correct bitfields |
| D. to_jsval / from_jsval traits | Partial | Some residual conversion mismatches |
| E. ProxyTraps field signatures | Done | All 30+ trap fields mapped |
| F. Heap\<T\> semantics mismatch | Open | V8 handles vs mozjs rooted heap |
| G. Intl/crown/jit feature stubs | No-op | Not needed for V8 path |
| H. Real V8 implementations needed | Partial | See "Still stubbed — runtime critical" |
| I. ConversionResult plumbing | Partial | Some paths still return `Failure` |

## What page scripts need next

1. **`document.createElement`** → DOM binding codegen path (servo-script)
2. **`window.location`** → real location object or proxy
3. **`setTimeout` / `setInterval`** → timer bootstrap via embedder
4. **`XMLHttpRequest` / `fetch`** → network path
5. **`console.log`** → polyfill or bridge to Rust logging
6. **`navigator.userAgent`** → string constant
7. **`MutationObserver`** → stub or real impl

These are Servo DOM binding gaps, not js_stub gaps — they live in `components/script/dom/`.

## Build commands

```bash
# js_stub tests (V8)
cargo test -p js --features v8 v8_glue

# rv8 servo render smoke
cargo test -p rv8 --features servo-render servo_v8_linking_smoke

# full servo-script build check
cargo check -p servo-script --features js_jit,soliloquy_v8
```

## File map

```
third_party/servo/support/js_stub/
├── src/
│   ├── lib.rs          # 7,600 lines — jsapi types + glue + wrappers
│   ├── v8_glue.rs      # 1,800 lines — real rusty_v8 implementations
│   └── macros.rs       # 130 lines — glue_stub! + helpers
└── TODO.md             # This document
```
