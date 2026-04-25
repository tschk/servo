/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

#[cfg(feature = "soliloquy_v8")]
use std::cell::RefCell;
use std::collections::HashMap;
#[cfg(feature = "soliloquy_v8")]
use std::sync::Once;

use embedder_traits::{JSValue, JavaScriptEvaluationError};
#[cfg(feature = "soliloquy_v8")]
use rusty_v8 as v8;
use servo_base::id::WebViewId;

use crate::soliloquy_bridge::{
    SoliloquyBridgeCommand, SoliloquyBridgeMutation, SoliloquyBridgeReadTarget,
    SoliloquyBridgeResult, SoliloquyBridgeWrite, capabilities, describe_webview, inspect_property,
    read_property, resolve_write, webview_bridge_id, write_property,
};

const SOLILOQUY_JS_ENGINE_ENV: &str = "SOLILOQUY_JS_ENGINE";
#[cfg(feature = "soliloquy_v8")]
static SOLILOQUY_V8_INIT: Once = Once::new();
#[cfg(feature = "soliloquy_v8")]
thread_local! {
    static SOLILOQUY_V8_ISOLATE: RefCell<Option<v8::OwnedIsolate>> = RefCell::new(None);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SoliloquyJavascriptBackend {
    Mozjs,
    V8Experimental,
}

impl SoliloquyJavascriptBackend {
    pub(crate) fn from_environment() -> Self {
        match std::env::var(SOLILOQUY_JS_ENGINE_ENV) {
            Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
                "v8" | "v8-experimental" | "v8_experimental" => Self::V8Experimental,
                _ => Self::Mozjs,
            },
            Err(_) => Self::Mozjs,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Mozjs => "mozjs",
            Self::V8Experimental => "v8-experimental",
        }
    }
}

pub(crate) struct SoliloquyJavascriptDispatcher;

impl SoliloquyJavascriptDispatcher {
    pub(crate) fn maybe_evaluate(
        webview_id: WebViewId,
        script: &str,
    ) -> Option<Result<JSValue, JavaScriptEvaluationError>> {
        Self::maybe_evaluate_with_mutations(webview_id, script).map(|evaluation| evaluation.result)
    }

    pub(crate) fn maybe_evaluate_with_mutations(
        webview_id: WebViewId,
        script: &str,
    ) -> Option<SoliloquyJavascriptEvaluation> {
        match SoliloquyJavascriptBackend::from_environment() {
            SoliloquyJavascriptBackend::Mozjs => {
                MozjsScriptBackend.maybe_evaluate(webview_id, script)
            },
            SoliloquyJavascriptBackend::V8Experimental => {
                V8DispatchScriptBackend::default().maybe_evaluate(webview_id, script)
            },
        }
    }
}

trait SoliloquyScriptBackend {
    fn maybe_evaluate(
        &self,
        webview_id: WebViewId,
        script: &str,
    ) -> Option<SoliloquyJavascriptEvaluation>;
}

struct MozjsScriptBackend;

impl SoliloquyScriptBackend for MozjsScriptBackend {
    fn maybe_evaluate(
        &self,
        _webview_id: WebViewId,
        _script: &str,
    ) -> Option<SoliloquyJavascriptEvaluation> {
        None
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SoliloquyV8IsolateState {
    #[cfg(not(feature = "soliloquy_v8"))]
    DispatchOnly,
    #[cfg(feature = "soliloquy_v8")]
    RustyV8Ready,
}

impl SoliloquyV8IsolateState {
    fn as_str(self) -> &'static str {
        match self {
            #[cfg(not(feature = "soliloquy_v8"))]
            Self::DispatchOnly => "dispatch-only",
            #[cfg(feature = "soliloquy_v8")]
            Self::RustyV8Ready => "rusty_v8-ready",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SoliloquyV8OwnerLifetime {
    #[cfg(not(feature = "soliloquy_v8"))]
    None,
    #[cfg(feature = "soliloquy_v8")]
    ThreadLocal,
}

impl SoliloquyV8OwnerLifetime {
    fn as_str(self) -> &'static str {
        match self {
            #[cfg(not(feature = "soliloquy_v8"))]
            Self::None => "none",
            #[cfg(feature = "soliloquy_v8")]
            Self::ThreadLocal => "thread-local",
        }
    }
}

struct SoliloquyV8IsolateOwner {
    state: SoliloquyV8IsolateState,
    lifetime: SoliloquyV8OwnerLifetime,
}

impl SoliloquyV8IsolateOwner {
    fn new() -> Self {
        #[cfg(feature = "soliloquy_v8")]
        {
            return Self::rusty_v8();
        }

        #[cfg(not(feature = "soliloquy_v8"))]
        {
            Self::dispatch_only()
        }
    }

    #[cfg(not(feature = "soliloquy_v8"))]
    fn dispatch_only() -> Self {
        Self {
            state: SoliloquyV8IsolateState::DispatchOnly,
            lifetime: SoliloquyV8OwnerLifetime::None,
        }
    }

    #[cfg(feature = "soliloquy_v8")]
    fn rusty_v8() -> Self {
        SOLILOQUY_V8_INIT.call_once(|| {
            let platform = v8::new_default_platform(0, false).make_shared();
            v8::V8::initialize_platform(platform);
            v8::V8::initialize();
        });

        SOLILOQUY_V8_ISOLATE.with(|isolate| {
            let mut isolate = isolate.borrow_mut();
            if isolate.is_none() {
                *isolate = Some(v8::Isolate::new(v8::CreateParams::default()));
            }
        });

        Self {
            state: SoliloquyV8IsolateState::RustyV8Ready,
            lifetime: SoliloquyV8OwnerLifetime::ThreadLocal,
        }
    }

    fn state_label(&self) -> &'static str {
        self.state.as_str()
    }

    fn lifetime_label(&self) -> &'static str {
        self.lifetime.as_str()
    }

    fn real_isolate_ready(&self) -> bool {
        #[cfg(feature = "soliloquy_v8")]
        {
            matches!(self.state, SoliloquyV8IsolateState::RustyV8Ready)
        }

        #[cfg(not(feature = "soliloquy_v8"))]
        {
            false
        }
    }
}

struct V8DispatchScriptBackend {
    isolate_owner: SoliloquyV8IsolateOwner,
}

impl Default for V8DispatchScriptBackend {
    fn default() -> Self {
        Self {
            isolate_owner: SoliloquyV8IsolateOwner::new(),
        }
    }
}

impl SoliloquyScriptBackend for V8DispatchScriptBackend {
    fn maybe_evaluate(
        &self,
        webview_id: WebViewId,
        script: &str,
    ) -> Option<SoliloquyJavascriptEvaluation> {
        let trimmed = script.trim();
        if trimmed.is_empty() {
            return Some(SoliloquyJavascriptEvaluation::ok(JSValue::Undefined));
        }

        evaluate_live_dom_assignment(webview_id, trimmed)
            .or_else(|| evaluate_live_dom_property(webview_id, trimmed))
            .or_else(|| evaluate_command(webview_id, trimmed, &self.isolate_owner))
            .or_else(|| evaluate_literal(trimmed))
            .or_else(|| evaluate_binary_plus(trimmed))
            .or_else(|| evaluate_engine_probe(webview_id, trimmed))
    }
}

pub(crate) struct SoliloquyJavascriptEvaluation {
    pub(crate) result: Result<JSValue, JavaScriptEvaluationError>,
    pub(crate) mutations: Vec<SoliloquyBridgeMutation>,
}

impl SoliloquyJavascriptEvaluation {
    fn ok(value: JSValue) -> Self {
        Self {
            result: Ok(value),
            mutations: Vec::new(),
        }
    }

    fn bridge_result(result: SoliloquyBridgeResult) -> Self {
        Self::ok(result.into_js_value())
    }

    fn with_mutation(value: JSValue, mutation: SoliloquyBridgeMutation) -> Self {
        Self {
            result: Ok(value),
            mutations: vec![mutation],
        }
    }
}

fn evaluate_live_dom_property(
    webview_id: WebViewId,
    script: &str,
) -> Option<SoliloquyJavascriptEvaluation> {
    let target = SoliloquyBridgeReadTarget::parse(script)?;
    Some(SoliloquyJavascriptEvaluation::ok(
        read_property(webview_id, target).unwrap_or(JSValue::Null),
    ))
}

fn evaluate_live_dom_assignment(
    webview_id: WebViewId,
    script: &str,
) -> Option<SoliloquyJavascriptEvaluation> {
    let (lhs, rhs) = split_assignment(script)?;
    let write = SoliloquyBridgeWrite::parse(lhs, &parse_string_literal(rhs)?)?;
    let write = match resolve_write(webview_id, write) {
        Ok(write) => write,
        Err(error) => return Some(SoliloquyJavascriptEvaluation::bridge_result(error)),
    };

    let (value, mutation) = write_property(webview_id, write).into_parts();
    Some(match mutation {
        Some(mutation) => SoliloquyJavascriptEvaluation::with_mutation(value, mutation),
        None => SoliloquyJavascriptEvaluation::ok(value),
    })
}

fn evaluate_command(
    webview_id: WebViewId,
    script: &str,
    isolate_owner: &SoliloquyV8IsolateOwner,
) -> Option<SoliloquyJavascriptEvaluation> {
    let command = parse_soliloquy_command(script)?;
    let (result, mutation) = dispatch_command(webview_id, command, isolate_owner);
    Some(match mutation {
        Some(mutation) => SoliloquyJavascriptEvaluation {
            result: Ok(result.into_js_value()),
            mutations: vec![mutation],
        },
        None => SoliloquyJavascriptEvaluation::bridge_result(result),
    })
}

fn evaluate_literal(script: &str) -> Option<SoliloquyJavascriptEvaluation> {
    match script {
        "undefined" => Some(SoliloquyJavascriptEvaluation::ok(JSValue::Undefined)),
        "null" => Some(SoliloquyJavascriptEvaluation::ok(JSValue::Null)),
        "true" => Some(SoliloquyJavascriptEvaluation::ok(JSValue::Boolean(true))),
        "false" => Some(SoliloquyJavascriptEvaluation::ok(JSValue::Boolean(false))),
        _ => parse_number(script)
            .map(JSValue::Number)
            .map(SoliloquyJavascriptEvaluation::ok)
            .or_else(|| {
                parse_string_literal(script)
                    .map(JSValue::String)
                    .map(SoliloquyJavascriptEvaluation::ok)
            }),
    }
}

fn evaluate_binary_plus(script: &str) -> Option<SoliloquyJavascriptEvaluation> {
    let (lhs, rhs) = split_binary_plus(script)?;
    let lhs = evaluate_literal(lhs)?.result.ok()?;
    let rhs = evaluate_literal(rhs)?.result.ok()?;

    match (lhs, rhs) {
        (JSValue::Number(lhs), JSValue::Number(rhs)) => Some(SoliloquyJavascriptEvaluation::ok(
            JSValue::Number(lhs + rhs),
        )),
        (JSValue::String(lhs), JSValue::String(rhs)) => Some(SoliloquyJavascriptEvaluation::ok(
            JSValue::String(lhs + &rhs),
        )),
        (JSValue::String(lhs), JSValue::Number(rhs)) => Some(SoliloquyJavascriptEvaluation::ok(
            JSValue::String(format!("{lhs}{rhs}")),
        )),
        (JSValue::Number(lhs), JSValue::String(rhs)) => Some(SoliloquyJavascriptEvaluation::ok(
            JSValue::String(format!("{lhs}{rhs}")),
        )),
        _ => None,
    }
}

fn evaluate_engine_probe(
    webview_id: WebViewId,
    script: &str,
) -> Option<SoliloquyJavascriptEvaluation> {
    match script {
        "window.__soliloquyEngineBackend" | "globalThis.__soliloquyEngineBackend" => {
            Some(SoliloquyJavascriptEvaluation::ok(JSValue::String(
                SoliloquyJavascriptBackend::from_environment()
                    .as_str()
                    .to_string(),
            )))
        },
        "window.__soliloquyEngineBridgeReady" | "globalThis.__soliloquyEngineBridgeReady" => {
            Some(SoliloquyJavascriptEvaluation::ok(JSValue::Boolean(false)))
        },
        "window.__soliloquyWebViewId" | "globalThis.__soliloquyWebViewId" => Some(
            SoliloquyJavascriptEvaluation::ok(JSValue::Number(webview_bridge_id(webview_id))),
        ),
        _ => None,
    }
}

fn parse_soliloquy_command(script: &str) -> Option<SoliloquyBridgeCommand> {
    const PREFIXES: [&str; 2] = ["window.__soliloquyEval(", "globalThis.__soliloquyEval("];
    for prefix in PREFIXES {
        if let Some(rest) = script.strip_prefix(prefix) {
            let tokens = parse_quoted_arguments(rest.strip_suffix(')')?.trim())?;
            let (name, args) = tokens.split_first()?;
            return Some(SoliloquyBridgeCommand::parse(name, args));
        }
    }
    None
}

fn parse_quoted_arguments(payload: &str) -> Option<Vec<String>> {
    let mut values = Vec::new();
    let mut cursor = payload.trim();

    while !cursor.is_empty() {
        let quote = cursor.chars().next()?;
        if quote != '\'' && quote != '"' {
            return None;
        }

        let rest = &cursor[quote.len_utf8()..];
        let end = rest.find(quote)?;
        values.push(rest[..end].to_string());
        cursor = rest[end + quote.len_utf8()..].trim_start();

        if cursor.is_empty() {
            break;
        }

        if !cursor.starts_with(',') {
            return None;
        }
        cursor = cursor[1..].trim_start();
    }

    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}

fn dispatch_command(
    webview_id: WebViewId,
    command: SoliloquyBridgeCommand,
    isolate_owner: &SoliloquyV8IsolateOwner,
) -> (SoliloquyBridgeResult, Option<SoliloquyBridgeMutation>) {
    match command {
        SoliloquyBridgeCommand::EngineBackend => (
            SoliloquyBridgeResult::value(JSValue::String(
                SoliloquyJavascriptBackend::from_environment()
                    .as_str()
                    .to_string(),
            )),
            None,
        ),
        SoliloquyBridgeCommand::EngineStatus => (
            SoliloquyBridgeResult::value(JSValue::Object(HashMap::from([
                (
                    "requestedEngine".to_string(),
                    JSValue::String("v8-experimental".to_string()),
                ),
                (
                    "activeEngine".to_string(),
                    JSValue::String("soliloquy-dispatch".to_string()),
                ),
                (
                    "isolateOwner".to_string(),
                    JSValue::String(isolate_owner.state_label().to_string()),
                ),
                (
                    "realIsolate".to_string(),
                    JSValue::Boolean(isolate_owner.real_isolate_ready()),
                ),
                (
                    "ownerLifetime".to_string(),
                    JSValue::String(isolate_owner.lifetime_label().to_string()),
                ),
                ("bridgeReady".to_string(), JSValue::Boolean(false)),
                ("controlsDom".to_string(), JSValue::Boolean(false)),
                ("commandChannel".to_string(), JSValue::Boolean(true)),
            ]))),
            None,
        ),
        SoliloquyBridgeCommand::WebViewId => (
            SoliloquyBridgeResult::value(JSValue::Number(webview_bridge_id(webview_id))),
            None,
        ),
        SoliloquyBridgeCommand::WebViewDescribe => (
            SoliloquyBridgeResult::value(describe_webview(
                webview_id,
                SoliloquyJavascriptBackend::from_environment().as_str(),
            )),
            None,
        ),
        SoliloquyBridgeCommand::DomCapabilities => {
            (SoliloquyBridgeResult::value(capabilities()), None)
        },
        SoliloquyBridgeCommand::DomInspect(target) => (
            SoliloquyBridgeResult::value(inspect_property(webview_id, target)),
            None,
        ),
        SoliloquyBridgeCommand::DomSet(write) => {
            let write = match resolve_write(webview_id, write) {
                Ok(write) => write,
                Err(error) => return (error, None),
            };
            let (value, mutation) = write_property(webview_id, write).into_parts();
            (SoliloquyBridgeResult::value(value), mutation)
        },
        SoliloquyBridgeCommand::Unsupported(operation) => {
            (SoliloquyBridgeResult::unsupported(operation), None)
        },
    }
}

fn parse_number(script: &str) -> Option<f64> {
    if script.starts_with('\'') || script.starts_with('"') {
        return None;
    }
    script.parse::<f64>().ok()
}

fn parse_string_literal(script: &str) -> Option<String> {
    let quote = script.chars().next()?;
    if (quote != '\'' && quote != '"') || !script.ends_with(quote) || script.len() < 2 {
        return None;
    }

    let inner = &script[1..script.len() - 1];
    if inner.contains(quote) {
        return None;
    }
    Some(inner.to_string())
}

fn split_binary_plus(script: &str) -> Option<(&str, &str)> {
    let mut in_quote: Option<char> = None;
    for (index, ch) in script.char_indices() {
        match in_quote {
            Some(quote) if ch == quote => in_quote = None,
            Some(_) => {},
            None if ch == '\'' || ch == '"' => in_quote = Some(ch),
            None if ch == '+' => {
                let lhs = script[..index].trim();
                let rhs = script[index + 1..].trim();
                if !lhs.is_empty() && !rhs.is_empty() {
                    return Some((lhs, rhs));
                }
            },
            None => {},
        }
    }

    None
}

fn split_assignment(script: &str) -> Option<(&str, &str)> {
    let mut in_quote: Option<char> = None;
    for (index, ch) in script.char_indices() {
        match in_quote {
            Some(quote) if ch == quote => in_quote = None,
            Some(_) => {},
            None if ch == '\'' || ch == '"' => in_quote = Some(ch),
            None if ch == '=' => {
                let lhs = script[..index].trim();
                let rhs = script[index + 1..].trim();
                if !lhs.is_empty() && !rhs.is_empty() {
                    return Some((lhs, rhs));
                }
            },
            None => {},
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use embedder_traits::{JSValue, LoadStatus};
    use servo_base::id::{BrowsingContextId, WebViewId};

    use super::{
        SOLILOQUY_JS_ENGINE_ENV, SoliloquyJavascriptBackend, SoliloquyJavascriptDispatcher,
        parse_soliloquy_command,
    };
    use crate::soliloquy_bridge::{
        SoliloquyBridgeCommand, SoliloquyBridgeMutation, record_webview_history_change,
        record_webview_load_status, record_webview_page_title, reset_webview_snapshots,
    };

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn set_engine_env(_guard: &std::sync::MutexGuard<'_, ()>, value: &str) {
        // SAFETY: Tests in this module hold ENV_LOCK while mutating and reading this
        // process-wide variable, so the dispatcher sees a serialized test value.
        unsafe {
            std::env::set_var(SOLILOQUY_JS_ENGINE_ENV, value);
        }
    }

    fn clear_engine_env(_guard: &std::sync::MutexGuard<'_, ()>) {
        // SAFETY: Tests in this module hold ENV_LOCK while mutating and reading this
        // process-wide variable, so the dispatcher sees a serialized test value.
        unsafe {
            std::env::remove_var(SOLILOQUY_JS_ENGINE_ENV);
        }
    }

    fn test_webview_id(index: u32) -> WebViewId {
        let browsing_context_id =
            BrowsingContextId::from_string(&format!("BrowsingContext(1234,{index})"))
                .expect("test browsing context id should parse");
        WebViewId::mock_for_testing(browsing_context_id)
    }

    #[test]
    fn dispatcher_is_disabled_without_v8_experimental() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        clear_engine_env(&_guard);
        assert_eq!(
            SoliloquyJavascriptBackend::from_environment(),
            SoliloquyJavascriptBackend::Mozjs
        );
        assert!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(test_webview_id(1), "1 + 1").is_none()
        );
    }

    #[test]
    fn dispatcher_handles_literals_and_simple_addition() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        set_engine_env(&_guard, "v8-experimental");
        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(test_webview_id(1), "1 + 1"),
            Some(Ok(JSValue::Number(2.0)))
        );
        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(test_webview_id(1), "'abc' + 'def'"),
            Some(Ok(JSValue::String("abcdef".into())))
        );
        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(
                test_webview_id(1),
                "window.__soliloquyEngineBackend"
            ),
            Some(Ok(JSValue::String("v8-experimental".into())))
        );
        clear_engine_env(&_guard);
    }

    #[test]
    fn dispatcher_handles_structured_commands() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        set_engine_env(&_guard, "v8-experimental");
        record_webview_page_title(test_webview_id(42), Some("Soliloquy".to_string()));
        record_webview_history_change(
            test_webview_id(42),
            Some("https://example.test/".to_string()),
        );
        record_webview_load_status(test_webview_id(42), LoadStatus::Complete);

        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(
                test_webview_id(42),
                "window.__soliloquyEval('webview.id')"
            ),
            Some(Ok(bridge_envelope(JSValue::Number(42.0))))
        );

        let result = SoliloquyJavascriptDispatcher::maybe_evaluate(
            test_webview_id(42),
            "globalThis.__soliloquyEval('engine.status')",
        );
        assert!(matches!(result, Some(Ok(JSValue::Object(_)))));

        let dom_capabilities = SoliloquyJavascriptDispatcher::maybe_evaluate(
            test_webview_id(42),
            "window.__soliloquyEval('dom.capabilities')",
        );
        assert!(matches!(dom_capabilities, Some(Ok(JSValue::Object(_)))));

        let dom_inspect = SoliloquyJavascriptDispatcher::maybe_evaluate(
            test_webview_id(42),
            "window.__soliloquyEval('dom.inspect', 'document.title')",
        );
        assert!(matches!(dom_inspect, Some(Ok(JSValue::Object(_)))));

        let dom_inspect_unknown = SoliloquyJavascriptDispatcher::maybe_evaluate(
            test_webview_id(42),
            "window.__soliloquyEval('dom.inspect', 'document.body.innerHTML')",
        );
        assert_eq!(
            dom_inspect_unknown,
            Some(Ok(bridge_detail_envelope(
                "unsupported",
                JSValue::String("dom.inspect".to_string())
            )))
        );

        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(
                test_webview_id(42),
                "window.__soliloquyEval('not.supported')",
            ),
            Some(Ok(bridge_detail_envelope(
                "unsupported",
                JSValue::String("not.supported".to_string())
            )))
        );

        clear_engine_env(&_guard);
    }

    #[test]
    fn v8_dispatch_backend_reports_isolate_owner_status() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        set_engine_env(&_guard, "v8-experimental");

        let result = SoliloquyJavascriptDispatcher::maybe_evaluate(
            test_webview_id(7),
            "window.__soliloquyEval('engine.status')",
        );

        let envelope = match result {
            Some(Ok(JSValue::Object(object))) => object,
            other => panic!("unexpected engine.status envelope: {other:?}"),
        };
        let status = match envelope.get("value") {
            Some(JSValue::Object(object)) => object,
            other => panic!("unexpected engine.status value: {other:?}"),
        };

        assert_eq!(
            status.get("activeEngine"),
            Some(&JSValue::String("soliloquy-dispatch".to_string()))
        );
        #[cfg(feature = "soliloquy_v8")]
        {
            assert_eq!(
                status.get("isolateOwner"),
                Some(&JSValue::String("rusty_v8-ready".to_string()))
            );
            assert_eq!(status.get("realIsolate"), Some(&JSValue::Boolean(true)));
            assert_eq!(
                status.get("ownerLifetime"),
                Some(&JSValue::String("thread-local".to_string()))
            );
        }

        #[cfg(not(feature = "soliloquy_v8"))]
        {
            assert_eq!(
                status.get("isolateOwner"),
                Some(&JSValue::String("dispatch-only".to_string()))
            );
            assert_eq!(status.get("realIsolate"), Some(&JSValue::Boolean(false)));
            assert_eq!(
                status.get("ownerLifetime"),
                Some(&JSValue::String("none".to_string()))
            );
        }

        clear_engine_env(&_guard);
    }

    #[test]
    fn dispatcher_reads_live_dom_properties_from_snapshot() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        set_engine_env(&_guard, "v8-experimental");
        record_webview_page_title(test_webview_id(21), Some("Snapshot Title".to_string()));
        record_webview_history_change(
            test_webview_id(21),
            Some("https://soliloquy.test/current".to_string()),
        );
        record_webview_load_status(test_webview_id(21), LoadStatus::HeadParsed);

        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(test_webview_id(21), "document.title"),
            Some(Ok(JSValue::String("Snapshot Title".into())))
        );
        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(test_webview_id(21), "location.href"),
            Some(Ok(JSValue::String("https://soliloquy.test/current".into())))
        );
        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(
                test_webview_id(21),
                "document.readyState"
            ),
            Some(Ok(JSValue::String("interactive".into())))
        );

        clear_engine_env(&_guard);
    }

    #[test]
    fn dispatcher_writes_live_dom_title() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        set_engine_env(&_guard, "v8-experimental");

        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(
                test_webview_id(30),
                "document.title = 'Updated Title'"
            ),
            Some(Ok(JSValue::String("Updated Title".into())))
        );
        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(test_webview_id(30), "document.title"),
            Some(Ok(JSValue::String("Updated Title".into())))
        );
        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(
                test_webview_id(30),
                "window.__soliloquyEval('dom.set', 'document.title', 'Command Title')"
            ),
            Some(Ok(bridge_envelope(JSValue::String("Command Title".into()))))
        );
        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(test_webview_id(30), "document.title"),
            Some(Ok(JSValue::String("Command Title".into())))
        );

        clear_engine_env(&_guard);
    }

    #[test]
    fn parser_supports_multiple_quoted_arguments() {
        assert_eq!(
            parse_soliloquy_command("window.__soliloquyEval('dom.inspect', 'location.href')"),
            Some(SoliloquyBridgeCommand::DomInspect(
                crate::soliloquy_bridge::SoliloquyBridgeReadTarget::LocationHref
            ))
        );
    }

    #[test]
    fn dom_inspect_returns_fallback_metadata() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        set_engine_env(&_guard, "v8-experimental");
        record_webview_history_change(
            test_webview_id(13),
            Some("https://soliloquy.test/dom".to_string()),
        );

        let result = SoliloquyJavascriptDispatcher::maybe_evaluate(
            test_webview_id(13),
            "window.__soliloquyEval('dom.inspect', 'location.href')",
        );

        let envelope = match result {
            Some(Ok(JSValue::Object(object))) => object,
            other => panic!("unexpected dom.inspect result: {other:?}"),
        };
        assert_eq!(envelope.get("ok"), Some(&JSValue::Boolean(true)));
        assert_eq!(
            envelope.get("status"),
            Some(&JSValue::String("ok".to_string()))
        );

        let object = match envelope.get("value") {
            Some(JSValue::Object(object)) => object,
            other => panic!("unexpected dom.inspect envelope value: {other:?}"),
        };

        assert_eq!(
            object.get("target"),
            Some(&JSValue::String("location.href".to_string()))
        );
        assert_eq!(
            object.get("status"),
            Some(&JSValue::String("live-snapshot".to_string()))
        );
        assert_eq!(
            object.get("fallbackEngine"),
            Some(&JSValue::String("mozjs".to_string()))
        );
        assert_eq!(object.get("valueAvailable"), Some(&JSValue::Boolean(true)));
        assert_eq!(
            object.get("value"),
            Some(&JSValue::String("https://soliloquy.test/dom".to_string()))
        );

        clear_engine_env(&_guard);
    }

    #[test]
    fn dispatcher_writes_live_location_href() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        set_engine_env(&_guard, "v8-experimental");

        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(
                test_webview_id(31),
                "location.href = 'https://soliloquy.test/next'"
            ),
            Some(Ok(JSValue::String(
                "https://soliloquy.test/next".to_string()
            )))
        );
        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(test_webview_id(31), "location.href"),
            Some(Ok(JSValue::String(
                "https://soliloquy.test/next".to_string()
            )))
        );
        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(
                test_webview_id(31),
                "document.readyState"
            ),
            Some(Ok(JSValue::String("loading".to_string())))
        );

        let evaluation = SoliloquyJavascriptDispatcher::maybe_evaluate_with_mutations(
            test_webview_id(31),
            "location.href = 'https://soliloquy.test/queued'",
        )
        .expect("location.href write should be handled");
        assert_eq!(
            evaluation.result,
            Ok(JSValue::String("https://soliloquy.test/queued".to_string()))
        );
        assert_eq!(
            evaluation.mutations,
            vec![SoliloquyBridgeMutation::Navigate {
                url: "https://soliloquy.test/queued".to_string()
            }]
        );

        let relative_evaluation = SoliloquyJavascriptDispatcher::maybe_evaluate_with_mutations(
            test_webview_id(31),
            "location.href = 'relative/page'",
        )
        .expect("relative location.href write should be handled");
        assert_eq!(
            relative_evaluation.result,
            Ok(JSValue::String(
                "https://soliloquy.test/relative/page".to_string()
            ))
        );
        assert_eq!(
            relative_evaluation.mutations,
            vec![SoliloquyBridgeMutation::Navigate {
                url: "https://soliloquy.test/relative/page".to_string()
            }]
        );

        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(
                test_webview_id(32),
                "location.href = 'not a url'"
            ),
            Some(Ok(bridge_detail_envelope(
                "error",
                JSValue::String("invalid location.href: relative URL without a base".to_string())
            )))
        );

        let command_result = SoliloquyJavascriptDispatcher::maybe_evaluate(
            test_webview_id(31),
            "window.__soliloquyEval('dom.set', 'location.href', 'https://soliloquy.test/final')",
        );
        assert_eq!(
            command_result,
            Some(Ok(bridge_envelope(JSValue::String(
                "https://soliloquy.test/final".to_string()
            ))))
        );
        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(test_webview_id(31), "location.href"),
            Some(Ok(JSValue::String(
                "https://soliloquy.test/final".to_string()
            )))
        );

        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(
                test_webview_id(31),
                "window.__soliloquyEval('dom.set', 'location.href', '')"
            ),
            Some(Ok(bridge_detail_envelope(
                "error",
                JSValue::String("location.href cannot be empty".to_string())
            )))
        );

        clear_engine_env(&_guard);
    }

    fn bridge_envelope(value: JSValue) -> JSValue {
        JSValue::Object(HashMap::from([
            ("ok".to_string(), JSValue::Boolean(true)),
            ("status".to_string(), JSValue::String("ok".to_string())),
            ("value".to_string(), value),
            ("detail".to_string(), JSValue::Null),
        ]))
    }

    fn bridge_detail_envelope(status: &str, detail: JSValue) -> JSValue {
        JSValue::Object(HashMap::from([
            ("ok".to_string(), JSValue::Boolean(false)),
            ("status".to_string(), JSValue::String(status.to_string())),
            ("value".to_string(), JSValue::Null),
            ("detail".to_string(), detail),
        ]))
    }
}
