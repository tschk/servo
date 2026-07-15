# js_stub Remaining Work — V8 Glue Surface

## Status

| Check | Result |
|-------|--------|
| `cargo check -p js` | 0 errors, ~565 warnings (naming) |
| `cargo check -p servo-script --features js_jit` | **0 errors**, 15 E0133 warnings |
| `cargo test -p js --features v8 v8_glue` | **8/8 pass** |
| `cargo test -p rv8 --features servo-render servo_v8_linking_smoke` | **pass** |
| Page scripts on rusty_v8 | compile + execute |

## Implemented (done)

- [x] SourceText store + `Compile1` + `JS_ExecuteScript`
- [x] `SetScriptPrivate` / `JS_GetScriptPrivate`
- [x] `SetModulePrivate` / `JS_GetModulePrivate`
- [x] `DoubleValue` / `NumberValue` (int when exact, else string)
- [x] `JS_TypeOfValue` real V8 type detection
- [x] `Debugger` constructor bootstrap
- [x] `NewDateObject` via V8 Date
- [x] `EventTarget` / `addEventListener` / `dispatchEvent` bootstrap
- [x] Core object lifecycle: `create_dom_global`, `js_new_object`, `enter/leave_realm`
- [x] Property get/set by name and jsid
- [x] `JS_HasProperty` / `JS_HasOwnProperty` / `JS_HasPropertyById`
- [x] `JS_DeletePropertyById`
- [x] Reserved slots, prototype chain, window proxy
- [x] Typed arrays, ArrayBuffers, Promise tracking
- [x] Function creation + native callback dispatch
- [x] `define_property_by_id`, `delete_property_ignoring_result`
- [x] `atomize_string_n`, string readback
- [x] `JS_GetReservedSlot`, `JS_GetPropertyById`
- [x] `JS_NewStringCopyN`
- [x] `CallJitGetterOp`, `CallJitMethodOp`, `CallJitSetterOp`
- [x] `JS_NewObject`, `JS_NewPlainObject`, `JS_NewObjectWithGivenProto`
- [x] `ModuleEvaluate`, `ModuleLink` (return pending promise)
- [x] `IsArrayObject`, `GetBuiltinClass` (array detection)
- [x] `JS_FireOnNewGlobalObject`
- [x] `JS_Stringify` / `JS_ParseJSON` (mozjs call signatures)
- [x] `JS_ReadStructuredClone` / `JS_WriteStructuredClone`
- [x] `JS_TransplantObject`

## Remaining (refinements, not blockers)

| Category | Items | Impact |
|----------|-------|--------|
| `E0133` unsafe warnings | 15 | Cosmetic — safe fn calling unsafe fn |
| GC tracers (`Call*Tracer`) | 5 | V8 has own GC; only needed for trace protocol |
| ProxyHandler queries | 4 | Proxy traps dispatch works via `call_jit_*` |
| Linear string path | 3 | String readback works via `v8_glue::string_text` |
| Memory pressure | 2 | `AddAssociatedMemory` / `RemoveAssociatedMemory` |
| Devtools integration | 4 | `HideScriptedCaller`, `SetDOMProxyInformation` |
| Compartment iteration | 1 | `JS_IterateCompartments` diagnostic |
| Standard class resolution | 1 | `JS_MayResolveStandardClass` not needed under V8 |

## What page scripts need next (Servo DOM binding gaps, not js_stub)

1. **`document.createElement`** → DOM binding codegen path
2. **`window.location`** → real location object or proxy
3. **`setTimeout` / `setInterval`** → timer bootstrap via embedder
4. **`XMLHttpRequest` / `fetch`** → network path
5. **`console.log`** → bridge to Rust logging
6. **`navigator.userAgent`** → string constant

## Build commands

```bash
cargo test -p js --features v8 v8_glue
cargo test -p rv8 --features servo-render servo_v8_linking_smoke
cargo check -p servo-script --features js_jit
```

## File map

```
third_party/servo/support/js_stub/
├── src/
│   ├── lib.rs          # ~7,800 lines — jsapi types + glue + wrappers
│   ├── v8_glue.rs      # ~1,850 lines — real rusty_v8 implementations
│   └── macros.rs       # 130 lines — glue_stub! + helpers
└── TODO.md             # This document
```
