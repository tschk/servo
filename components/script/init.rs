/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */
use js::jsapi::JSObject;
use script_bindings::proxyhandler;
use servo_config::pref;

use crate::dom::bindings::codegen::RegisterBindings;
use crate::dom::bindings::conversions::is_dom_proxy;
use crate::dom::bindings::utils::is_platform_object_static;
use crate::script_runtime::{JSEngineSetup, ScriptJsBackend};

const SOLILOQUY_JS_ENGINE_ENV: &str = "SOLILOQUY_JS_ENGINE";

#[cfg(target_os = "linux")]
#[expect(unsafe_code)]
fn perform_platform_specific_initialization() {
    // 4096 is default max on many linux systems
    const MAX_FILE_LIMIT: libc::rlim_t = 4096;

    // Bump up our number of file descriptors to save us from impending doom caused by an onslaught
    // of iframes.
    unsafe {
        let mut rlim = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        match libc::getrlimit(libc::RLIMIT_NOFILE, &mut rlim) {
            0 => {
                if rlim.rlim_cur >= MAX_FILE_LIMIT {
                    // we have more than enough
                    return;
                }

                rlim.rlim_cur = match rlim.rlim_max {
                    libc::RLIM_INFINITY => MAX_FILE_LIMIT,
                    _ => {
                        if rlim.rlim_max < MAX_FILE_LIMIT {
                            rlim.rlim_max
                        } else {
                            MAX_FILE_LIMIT
                        }
                    },
                };
                match libc::setrlimit(libc::RLIMIT_NOFILE, &rlim) {
                    0 => (),
                    _ => warn!("Failed to set file count limit"),
                };
            },
            _ => warn!("Failed to get file count limit"),
        };
    }
}

#[cfg(not(target_os = "linux"))]
fn perform_platform_specific_initialization() {}

#[expect(unsafe_code)]
unsafe extern "C" fn is_dom_object(obj: *mut JSObject) -> bool {
    !obj.is_null() && (is_platform_object_static(obj) || unsafe { is_dom_proxy(obj) })
}

/// Returns true if JIT is forbidden
///
/// Spidermonkey will crash if JIT is not allowed on a system, so we do a short detection
/// if jit is allowed or not.
///
/// Note: This implementation should work fine on all Linux systems, perhaps even Unix systems,
/// but for now we only enable it on OpenHarmony, since that is where it is most needed.
#[cfg(target_env = "ohos")]
#[expect(unsafe_code)]
fn jit_forbidden() -> bool {
    debug!("Testing if JIT is allowed.");

    fn mem_is_writable(ptr: *mut core::ffi::c_void) -> std::io::Result<bool> {
        debug!("Testing if ptr {ptr:?} is writable");
        // Safety: This is cursed, but we can use read to determine if ptr
        // can be written to. `read` is a syscall and will return an error code
        // if ptr can't be written (instead of a segfault as with a regular access).
        // We also take care to always close `fd`.
        #[expect(unsafe_code)]
        unsafe {
            let fd = libc::open(c"/dev/zero".as_ptr(), libc::O_RDONLY);
            if fd < 0 {
                return Err(std::io::Error::last_os_error());
            }
            let writable = libc::read(fd, ptr, 1) > 0;
            if !writable {
                debug!(
                    "addr is not writable. Error: {}",
                    std::io::Error::last_os_error()
                );
            }
            libc::close(fd);
            Ok(writable)
        }
    }

    // We need to allocate at least one page, so we query the page size on the system.
    let map_size: libc::size_t = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as libc::size_t };
    let flags = libc::MAP_NORESERVE | libc::MAP_PRIVATE | libc::MAP_ANON;
    // SAFETY: We mmap one anonymous page, with no special flags, so this has no safety
    // implications.
    let first_mmap = unsafe {
        libc::mmap(
            core::ptr::null_mut(),
            map_size,
            libc::PROT_NONE,
            flags,
            -1,
            0,
        )
    };
    assert_ne!(first_mmap, libc::MAP_FAILED, "mmap not allowed?");

    let remap_flags =
        libc::MAP_ANONYMOUS | libc::MAP_FIXED | libc::MAP_PRIVATE | libc::MAP_EXECUTABLE;
    // remap the page with PROT_EXEC. If this fails, JIT is not possible.
    let second_mmap = unsafe {
        libc::mmap(
            first_mmap,
            map_size,
            libc::PROT_READ | libc::PROT_EXEC,
            remap_flags,
            -1,
            0,
        )
    };
    let mut jit_forbidden = second_mmap == libc::MAP_FAILED;
    if !jit_forbidden {
        // Spidermonkey uses mprotect to make the memory writable.
        // SAFETY: We obtained the memory in question via `mmap` and are not using the memory
        // in any way.
        let res =
            unsafe { libc::mprotect(first_mmap, map_size, libc::PROT_READ | libc::PROT_WRITE) };
        if res != 0 {
            // `mprotect` failed (to add write permissions), so we presume it is because JIT is forbidden.
            jit_forbidden = true;
        } else {
            // Additionally check if `mprotect` actually succeeded in adding `PROT_WRITE`.
            // We observed before that `mprotect` silently ignores the write permission without
            // returning an error.
            let is_writable = mem_is_writable(first_mmap)
                .inspect_err(|_e| {
                    debug!("Failed to determine if JIT is allowed. Conservatively assuming it is forbidden.");
                })
                .unwrap_or(false); // writable == false -> JIT is forbidden.
            jit_forbidden = !is_writable;
        }
    }
    // Ignore the result, since there is nothing we could do if unmap failed for whatever reason.
    // SAFETY: We unmap the `mmap`ed region completely again. There is no other `munmap` call in
    // this function, and we do not have any early returns in this function.
    let _ = unsafe { libc::munmap(first_mmap, map_size) };

    jit_forbidden
}

#[cfg(not(target_env = "ohos"))]
fn jit_forbidden() -> bool {
    false
}

fn script_js_backend_from_env_value(value: &str) -> Option<ScriptJsBackend> {
    match value.trim().to_ascii_lowercase().as_str() {
        // Legacy aliases accepted; V8 is the only engine.
        "" | "v8" | "v8-experimental" | "v8_experimental" | "mozjs" | "spidermonkey" => {
            Some(ScriptJsBackend::V8)
        },
        _ => None,
    }
}

fn configured_js_backend() -> ScriptJsBackend {
    match std::env::var(SOLILOQUY_JS_ENGINE_ENV) {
        Ok(value) => script_js_backend_from_env_value(&value).unwrap_or_else(|| {
            warn!(
                "Unsupported value `{value}` for {SOLILOQUY_JS_ENGINE_ENV}; defaulting Servo script bootstrap to v8."
            );
            ScriptJsBackend::V8
        }),
        Err(_err) => ScriptJsBackend::V8,
    }
}

#[expect(unsafe_code)]
#[servo_tracing::instrument(name = "script::init")]
pub fn init() -> JSEngineSetup {
    if pref!(js_disable_jit) || jit_forbidden() {
        let reason = if pref!(js_disable_jit) {
            "preference `js_disable_jit` is set to true"
        } else {
            "runtime test determined JIT is forbidden on this system"
        };
        warn!("Disabling JIT for Javascript, since {reason}. This may cause subpar performance");
        // SAFETY: This function has no particular preconditions.
        unsafe {
            js::jsapi::DisableJitBackend();
        }
    }
    proxyhandler::init();

    // Create the global vtables used by the (generated) DOM
    // bindings to implement JS proxies.
    RegisterBindings::RegisterProxyHandlers::<crate::DomTypeHolder>();
    RegisterBindings::InitAllStatics::<crate::DomTypeHolder>();

    unsafe {
        js::glue::InitializeMemoryReporter(Some(is_dom_object));
    }

    perform_platform_specific_initialization();

    let backend = configured_js_backend();
    info!(
        "Initializing Servo script runtime with `{}` backend selection.",
        backend.as_str()
    );
    // Single engine: V8 via js_stub.
    let _ = backend;
    JSEngineSetup::v8()
}

#[cfg(test)]
mod tests {
    use super::script_js_backend_from_env_value;
    use crate::script_runtime::ScriptJsBackend;

    #[test]
    fn parses_v8_backend_aliases() {
        for value in ["v8", "v8-experimental", "v8_experimental", "mozjs", "spidermonkey", ""] {
            assert_eq!(
                script_js_backend_from_env_value(value),
                Some(ScriptJsBackend::V8)
            );
        }
    }

    #[test]
    fn rejects_unknown_backend_names() {
        assert_eq!(script_js_backend_from_env_value("not-a-runtime"), None);
    }
}
