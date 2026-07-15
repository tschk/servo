# js_stub — V8 Bridge for Servo (replaces mozjs)

## Goal
Remove mozjs SpiderMonkey entirely. Replace with rusty_v8-based stub crate
(`support/js_stub/`) that satisfies the Servo WebIDL binding code generator.
All mono-browser features must keep working.

## Status
`cargo check -p js` — **0 errors**, ~278 warnings (mozjs naming conventions).
`cargo check -p servo-script-bindings` — still failing. The old missing `realm`,
`D: Traceable`, raw `JSContext` pointer-depth, reserved-slot, JSJit call-arg,
property descriptor, object-op, and realm-options classes are reduced. Current
top failures are mostly repeated mozjs API-shape gaps plus generated safe-context
reborrow sites (`cx` moved into wrapper calls before later `cx.into()` use).

---

## Done

### Workspace & build
- `js = { package = "mozjs", version = "0.15.13" }` replaced with
  `js = { path = "support/js_stub" }` (package name `js`, version `0.15.13`)
- `support/js_stub/` added to workspace members
- `js` dep non-optional in `script_bindings` and `script`
- `jstraceable_derive` non-optional
- `extern crate js` unconditional in both `script_bindings/lib.rs` and `script/lib.rs`
- All mozjs features stubbed: `crown`, `debugmozjs`, `jit`, `jitspew`, `profilemozjs`,
  `libz-sys`, `intl`

### Core type replacements
- `Handle<'a, T>`, `MutableHandle<'a, T>` — type aliases (`*const T` / `*mut T`)
- `JSContext` — type alias (`*mut c_void`)
- `HandleObject`, `HandleValue`, `MutableHandleObject`, `MutableHandleValue` — aliases
- `HandleId`, `HandleValueArray`, `MutableHandleIdVector`
- `CallArgs` — proper struct with `argc_`, `argv_`, `rval_`, `get()`, `rval()`, `is_constructing()`
- `jsid` — struct `pub struct jsid(pub u64)` with `From<*const jsid>`, `Deref<Target=u64>`

### JSJitInfo / JSPropertySpec / JSClassDef
- `JSJitInfo__bindgen_ty_1` — `method`, `getter`, `setter` fields + `Default` impl (all `None`)
- `JSJitInfo__bindgen_ty_2` — `protoID: u16`
- `JSJitInfo__bindgen_ty_3` — `depth: u16`
- `JSJitInfo` — `__bindgen_anon_1/2/3`, `_bitfield_align_1`, `_bitfield_1`
- `JSTypedMethodJitInfo`, `JSJitInfo_OpType`, `JSJitInfo_AliasSet`, `JSJitInfo_ArgType`
- `JSPropertySpec_Name` — `string_`, `symbol_`
- `JSPropertySpec_Accessor` — `native: JSNativeWrapper`
- `JSPropertySpec_AccessorsOrValue` — `accessors`, `value`
- `JSPropertySpec_AccessorsOrValue_Accessors` — `getter`, `setter`
- `JSPropertySpec_ValueWrapper` — `type_`, `__bindgen_anon_1`
- `JSPropertySpec_ValueWrapper__bindgen_ty_1` — `string`
- `JSPropertySpec_ValueWrapper_Type`, `JSPropertySpec_Kind`
- `JSPropertySpec` — `name`, `attributes_`, `kind_`, `u`, `ZERO` constant
- `JSNativeWrapper` — `op`, `info`
- `JSClassDef` — `name`, `flags`, `cOps`, `spec`, `ext`, `oOps`
- `JSClassOps` — all fields including `hasInstance`
- `JSFunctionSpec` — `name`, `call`, `nargs`, `flags`, `selfHostedName`
- `CallArgs` — struct with `get()`/`rval()`/`argc_`/`is_constructing()`
- `JSJitGetterCallArgs`, `JSJitMethodCallArgs`, `JSJitSetterCallArgs` — `CallArgs` aliases

### Proxy / glue
- `ProxyTraps` — all 30+ proxy trap fields with `jsapi::` prefixes
- `CreateProxyHandler` — 2 args (traps + class pointer)
- `GetProxyReservedSlot` — 3 args (proxy, slot, out)
- `SetProxyReservedSlot` — 3 args
- `AutoRealm::new_from_handle` — generic wrapper
- `CurrentRealm` — struct with `assert<T>(_: T) -> Self`
- `AlreadyInRealm`, `EnterRealm`, `LeaveRealm`

### Conversion traits
- `ConversionResult<T>` — `Success(T)` / `Failure(Box<str>)`
- `ConversionBehavior` — `Default`, `EnforceRange`, `Clamp`
- `ToJSValConvertible` — `fn to_jsval(&self, cx, rval: *mut JSVal) -> Result<(), ()>`
- `FromJSValConvertible` — `fn from_jsval(cx, val, option: Config) -> Result<ConversionResult<Self>, ()>`
- `StringificationBehavior` — imported from `crate::conversions` (script_bindings)

### JS API functions (all return false/null_mut stubs)
- `JS_HasPropertyById`, `JS_ForwardGetPropertyTo`, `JS_GetPropertyDescriptorById`,
  `JS_DefinePropertyById2`, `JS_DeletePropertyById` — take `impl Into<jsid>`
- `JS_GetPropertyById`, `JS_SetProperty`, `JS_GetProperty`, `JS_DefineProperty`
- `JS_NewObject`, `JS_NewPlainObject`, `JS_NewObjectWithGivenProto`,
  `JS_NewObjectWithoutMetadata`
- `JS_NewGlobalObject`, `JS_NewFunction`, `JS_GetFunctionObject`
- `JS_LinkConstructorAndPrototype`, `JS_WrapObject`, `JS_SetPrototype`,
  `JS_SetImmutablePrototype`
- `JS_GetReservedSlot`, `JS_SetReservedSlot`, `JS_GetPrivate`, `JS_SetPrivate`
- `JS_GetPrototype`, `JS_GetClass`
- `AppendToIdVector`, `GetPropertyKeys`, `Call`, `NewProxyObject`
- `JS_AtomizeStringN`, `JS_AtomizeAndPinString`, `JS_NewStringCopyN`
- `JS_GetTwoByteStringCharsAndLength`, `JS_GetLatin1StringCharsAndLength`
- `JS_ValueToSource`, `JS_IdToValue`, `JS_IsExceptionPending`, `JS_ClearPendingException`
- `JS_IsGlobalObject`, `GetCurrentRealmOrNull`, `GetRealmObjectPrototype`
- `ToJSON`, `RUST_INTERNED_STRING_TO_JSID`, `RUST_SYMBOL_TO_JSID`,
  `int_to_jsid` (returns `jsapi::jsid`)
- `AddRawValueRoot`, `RemoveRawValueRoot`, `CheckedUnwrapStatic`
- `GetNonCCWObjectGlobal`, `GetRealmGlobalOrNull`, `GetObjectProto`
- `JS_FireOnNewGlobalObject`, `JS_AlreadyHasOwnPropertyById`,
  `SetDataPropertyDescriptor`, `JS_HasProperty`, `JS_HasOwnProperty`
- `JS_GetPendingException`, `JS_SetPendingException`
- `CallOriginalPromiseReject`, `JS_DefineUCProperty2`
- `JS_GetOwnPropertyDescriptorById`, `InvokeGetOwnPropertyDescriptor`,
  `SetPropertyIgnoringNamedGetter`
- `JS_DefineProperty3/4/5`, `JS_DefinePropertyById5`
- `JS_CopyOwnPropertiesAndPrivateFields`, `JS_InitializePropertiesFromCompatibleNativeObject`
- `JS_MayResolveStandardClass`, `JS_NewEnumerateStandardClasses`,
  `JS_ResolveStandardClass`
- `JS_DropPrincipals`, `JS_HoldPrincipals`, `JS_SetTrustedPrincipals`
- `GCTraceKindToAscii`, `StringIsArrayIndex`, `IsArrayObject`
- `IsWindowProxy`, `IsSharableCompartment`, `IsSystemCompartment`
- `JS_IterateCompartments`, `GetStaticPrototype`, `SetDOMProxyInformation`
- `HideScriptedCaller`, `UnhideScriptedCaller`
- `AtomToLinearString`, `GetLinearStringCharAt`, `GetLinearStringLength`
- `AddAssociatedMemory`, `RemoveAssociatedMemory`
- `JS_GlobalObjectTraceHook`, `JS_DeprecatedStringHasLatin1Chars`,
  `JS_GetTwoByteLatin1Chars`, `JS_GetTwoByteStringChars`

### Typed arrays / jsval / jsstring
- `typedarray` module — `Uint8Array`, `Float64Array`, `Create` wrappers
- `Heap<T>` — `RefCell<Option<T>>` with `boxed`, `set`, `get`, `unbarriered_get`
- `Heap*Array*` types
- `JSVal` constructors — `UndefinedValue`, `NullValue`, `BooleanValue`, `DoubleValue`,
  `Int32Value`, `UInt32Value`, `StringValue`, `NumberValue`, `ObjectValue`,
  `ObjectOrNullValue`, `PrivateValue`
- `JSValueType` enum
- `JSString`, `JSFunction`, `JSTracer`, `JSRuntime`, `JSPrincipals` — opaque `*mut c_void`
- `JSAutoRealm`, `JSAutoCompartment`, `GCContext` — opaque pointers
- `ObjectOpResult`, `PropertyDescriptor`, `__BindgenBitfieldUnit`
- `SymbolCode`, `SymbolId`

### Macros
- `rooted!` — creates `RootedGuard` via `MaybeUninit` roots (supports `&in(cx)` and `in(cx)`)
- `auto_root!` — simple let-binding
- `new_jsjitinfo_bitfield_1!` — packs 10 flags into a u32
- `rooted_vec!` — Vec creation

### GCMethods / Traceable / RootedGuard
- `GCMethods` trait — `fn initial() -> Self`
- `Traceable` / `Trace` — `unsafe trait`, `trace()` method
- Blanket `Traceable` impls for `*mut T`, `*const T`, `Heap<T>`, `Option<T>`,
  `Vec<T>`, `Box<T>`, `Rc<T>`, `&T`, `u32`, `u64`, `bool`, `f64`, `String`
- `RootedGuard` — stores `value: T`, methods: `handle()`, `handle_mut()`,
  `get()`, `set()`, `is_null()`, `is_undefined()`, `is_object()`, `to_object()`

### Codegen changes (codegen.py)
- Removed all `HandleValue::from_raw(X)` → `X` (pass-through)
- Removed all `MutableHandleValue::from_raw(X)` → `X`
- Removed all `HandleObject::from_raw(obj)` → `obj`
- `Handle::from_raw(id)` → `id` (pass-through, jsid stays `*const jsid`)
- `Handle::from_raw(proxy)` → `proxy`
- `Handle::from_raw(receiver)` → `receiver`
- `MutableHandle::from_raw(desc)` → `desc`
- `MutableHandle::from_raw(vp)` → `vp`
- `JSContext::from_ptr(NonNull::new(cx).unwrap())` → `let mut cx = *cx;`
- Stripped `cx.raw_cx()` → `cx`
- Stripped `SafeJSContext::from_ptr(X)` → `X`
- Added `..Default::default()` to `JSJitInfo__bindgen_ty_1` initializers
- Partially handled `let cx = &mut cx;` removal (needs more work — see below)

---

## Done recently (V8-only cutover)

- `JSEngine::init` calls `ensure_v8()`
- SourceText store + `Compile1` + `JS_ExecuteScript` wired to rusty_v8 compile/run
- `SetScriptPrivate` / `JS_GetScriptPrivate` store private values on compiled scripts
- `DoubleValue` / `NumberValue` no longer always undefined
- Unit tests: compile/execute + script private roundtrip

## TODO — Remaining issues

### A. SafeJSContext reborrow generation
**Root cause:** generated code passes `cx: &mut JSContext` by value into generic
wrapper functions, then later uses `cx.into()`. Several generator sites now emit
`&mut *cx`, but more patterns remain, especially `JS_NewObjectWithoutMetadata`,
`JS_NewObjectWithGivenProto`, `JS_NewPlainObject`, and property-definition calls.

**Next step:** normalize codegen so every generated call that is not the final
consumer receives a fresh reborrow (`&mut *cx`) or raw context (`cx.raw_cx()`),
instead of moving `cx`.

### B. Rooting and trace boxes
`RootedTraceableBox::from_box` now accepts rooted values used by Servo call
sites, and `js::gc::RootedTraceableBox` exposes `handle`, `ptr`, `trace`, and
`Deref` surfaces. Remaining errors are trait-bound mismatches in derived
`JSTraceable` and generic containers.

### C. JSJitInfo field initialization (residual)
`JSJitInfo__bindgen_ty_1` uses `Default::default()` spread in generated code.
`JSJitInfo__bindgen_ty_1` has `Default` impl with all-`None` fields.
**Needs rebuild to verify.**

### D. to_jsval / from_jsval trait signatures (residual)
Traits updated to return `Result<(), ()>` / `Result<ConversionResult<Self>, ()>`.
`StringificationBehavior` / `ConversionBehavior` as `Config` associated type.
**May have residual mismatches with Servo-side impls.**

### E. ProxyTraps field signatures
All 30+ fields added. Function pointer signatures use `jsapi::*` types
(`*mut JSContext`, `*mut JSObject`, `*const jsid`, `*mut JSVal`, etc.).
**The actual `proxyhandler::*` function signatures may differ from field types.**
Errors show const/mut pointer mismatches and argument count differences.

### F. Heap<T> semantics mismatch
`Heap<T>` uses `RefCell<Option<T>>`. Generated code accesses `.get()` returning
`Option<T>`, `.set(val)`, and `unsafe { .unbarriered_get() }`. Works for stubs.

### G. Intl/crown/jit feature stubs
All mozjs features are declared but have no real implementations. This is
acceptable for a stub — real V8 implementations can replace them later.

### H. Real V8 implementations needed
Currently every JS API function returns `false` / `null_mut()`. For the browser
to actually work:
- `JS_NewGlobalObject`, `JS_NewObject`, `JS_NewPlainObject` — create V8 contexts/objects
- `JS_GetProperty`, `JS_SetProperty`, `JS_HasPropertyById` — V8 property access
- `JS_AtomizeStringN`, `JS_NewStringCopyN` — string interning
- `Call`, `JS_ForwardGetPropertyTo` — function calls
- `EvaluateString` (not yet implemented) — script evaluation
- GC integration (`RootedGuard`, `Traceable`, `Heap`) — V8 persistent handles

### I. ConversionResult plumbing
`ConversionResult::Success` / `Failure` variants used by `from_jsval`. The
`FromJSValConvertible` trait returns `Result<ConversionResult<Self>, ()>`.
Servo-side impls may need matching return types.

---

## Key files
| File | Role |
|------|------|
| `support/js_stub/Cargo.toml` | Crate manifest (name `js`, version `0.15.13`) |
| `support/js_stub/src/lib.rs` | Main stub (~920 lines) |
| `support/js_stub/src/macros.rs` | `rooted!`, `new_jsjitinfo_bitfield_1!` |
| `components/script_bindings/codegen/codegen.py` | WebIDL Rust code generator (~9550 lines, modified) |
| `components/script_bindings/import.rs` | Symbol re-exports for generated code |
| `components/script_bindings/Cargo.toml` | `js` non-optional |
| `components/script/Cargo.toml` | `js` non-optional |
| `Cargo.toml` (workspace root) | `js` dep changed, `support/js_stub` in members |

## Build commands
```bash
# Fast type-check of just the stub
cargo check -p js

# Full bindings check (5+ minutes with full rebuild)
cargo check -p servo-script-bindings

# Full workspace
cargo check
```
