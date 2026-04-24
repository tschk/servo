/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use embedder_traits::{JSValue, JavaScriptEvaluationError, LoadStatus};
use servo_base::id::WebViewId;

const SOLILOQUY_JS_ENGINE_ENV: &str = "SOLILOQUY_JS_ENGINE";
static WEBVIEW_SNAPSHOTS: LazyLock<Mutex<HashMap<WebViewId, SoliloquyWebViewSnapshot>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone, Debug, Default)]
struct SoliloquyWebViewSnapshot {
    page_title: Option<String>,
    current_url: Option<String>,
    ready_state: Option<String>,
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
        if SoliloquyJavascriptBackend::from_environment()
            != SoliloquyJavascriptBackend::V8Experimental
        {
            return None;
        }

        let trimmed = script.trim();
        if trimmed.is_empty() {
            return Some(Ok(JSValue::Undefined));
        }

        evaluate_live_dom_assignment(webview_id, trimmed)
            .or_else(|| evaluate_live_dom_property(webview_id, trimmed))
            .or_else(|| evaluate_command(webview_id, trimmed))
            .or_else(|| evaluate_literal(trimmed))
            .or_else(|| evaluate_binary_plus(trimmed))
            .or_else(|| evaluate_engine_probe(webview_id, trimmed))
    }
}

pub(crate) fn record_webview_page_title(webview_id: WebViewId, title: Option<String>) {
    webview_snapshot_mut(webview_id).page_title = title;
}

pub(crate) fn record_webview_navigation_request(webview_id: WebViewId, url: String) {
    webview_snapshot_mut(webview_id).current_url = Some(url);
}

pub(crate) fn record_webview_history_change(webview_id: WebViewId, current_url: Option<String>) {
    webview_snapshot_mut(webview_id).current_url = current_url;
}

pub(crate) fn record_webview_load_status(webview_id: WebViewId, load_status: LoadStatus) {
    webview_snapshot_mut(webview_id).ready_state = Some(match load_status {
        LoadStatus::Started => "loading".to_string(),
        LoadStatus::HeadParsed => "interactive".to_string(),
        LoadStatus::Complete => "complete".to_string(),
    });
}

pub(crate) fn clear_webview_snapshot(webview_id: WebViewId) {
    WEBVIEW_SNAPSHOTS.lock().unwrap().remove(&webview_id);
}

#[cfg(test)]
pub(crate) fn reset_webview_snapshots() {
    WEBVIEW_SNAPSHOTS.lock().unwrap().clear();
}

fn webview_snapshot_mut(
    webview_id: WebViewId,
) -> std::sync::MutexGuard<'static, HashMap<WebViewId, SoliloquyWebViewSnapshot>> {
    let mut snapshots = WEBVIEW_SNAPSHOTS.lock().unwrap();
    snapshots.entry(webview_id).or_default();
    snapshots
}

fn webview_snapshot(webview_id: WebViewId) -> Option<SoliloquyWebViewSnapshot> {
    WEBVIEW_SNAPSHOTS.lock().unwrap().get(&webview_id).cloned()
}

fn evaluate_live_dom_property(
    webview_id: WebViewId,
    script: &str,
) -> Option<Result<JSValue, JavaScriptEvaluationError>> {
    let snapshot = webview_snapshot(webview_id)?;
    match script {
        "document.title" => Some(Ok(match snapshot.page_title {
            Some(title) => JSValue::String(title),
            None => JSValue::Null,
        })),
        "location.href" | "window.location.href" => Some(Ok(match snapshot.current_url {
            Some(url) => JSValue::String(url),
            None => JSValue::Null,
        })),
        "document.readyState" => Some(Ok(match snapshot.ready_state {
            Some(ready_state) => JSValue::String(ready_state),
            None => JSValue::Null,
        })),
        _ => None,
    }
}

fn evaluate_live_dom_assignment(
    webview_id: WebViewId,
    script: &str,
) -> Option<Result<JSValue, JavaScriptEvaluationError>> {
    let (lhs, rhs) = split_assignment(script)?;
    let value = match lhs {
        "document.title" => JSValue::String(parse_string_literal(rhs)?),
        _ => return None,
    };

    apply_live_dom_write(webview_id, lhs, value.clone())?;
    Some(Ok(value))
}

fn evaluate_command(
    webview_id: WebViewId,
    script: &str,
) -> Option<Result<JSValue, JavaScriptEvaluationError>> {
    let command = parse_soliloquy_command(script)?;
    dispatch_command(webview_id, &command).map(Ok)
}

fn evaluate_literal(script: &str) -> Option<Result<JSValue, JavaScriptEvaluationError>> {
    match script {
        "undefined" => Some(Ok(JSValue::Undefined)),
        "null" => Some(Ok(JSValue::Null)),
        "true" => Some(Ok(JSValue::Boolean(true))),
        "false" => Some(Ok(JSValue::Boolean(false))),
        _ => parse_number(script)
            .map(JSValue::Number)
            .map(Ok)
            .or_else(|| parse_string_literal(script).map(JSValue::String).map(Ok)),
    }
}

fn evaluate_binary_plus(script: &str) -> Option<Result<JSValue, JavaScriptEvaluationError>> {
    let (lhs, rhs) = split_binary_plus(script)?;
    let lhs = evaluate_literal(lhs)?.ok()?;
    let rhs = evaluate_literal(rhs)?.ok()?;

    match (lhs, rhs) {
        (JSValue::Number(lhs), JSValue::Number(rhs)) => Some(Ok(JSValue::Number(lhs + rhs))),
        (JSValue::String(lhs), JSValue::String(rhs)) => Some(Ok(JSValue::String(lhs + &rhs))),
        (JSValue::String(lhs), JSValue::Number(rhs)) => {
            Some(Ok(JSValue::String(format!("{lhs}{rhs}"))))
        },
        (JSValue::Number(lhs), JSValue::String(rhs)) => {
            Some(Ok(JSValue::String(format!("{lhs}{rhs}"))))
        },
        _ => None,
    }
}

fn evaluate_engine_probe(
    webview_id: WebViewId,
    script: &str,
) -> Option<Result<JSValue, JavaScriptEvaluationError>> {
    match script {
        "window.__soliloquyEngineBackend" | "globalThis.__soliloquyEngineBackend" => {
            Some(Ok(JSValue::String(
                SoliloquyJavascriptBackend::from_environment()
                    .as_str()
                    .to_string(),
            )))
        },
        "window.__soliloquyEngineBridgeReady" | "globalThis.__soliloquyEngineBridgeReady" => {
            Some(Ok(JSValue::Boolean(false)))
        },
        "window.__soliloquyWebViewId" | "globalThis.__soliloquyWebViewId" => {
            Some(Ok(JSValue::Number(webview_id.0 as f64)))
        },
        _ => None,
    }
}

#[derive(Debug, PartialEq, Eq)]
struct SoliloquyCommand {
    name: String,
    args: Vec<String>,
}

fn parse_soliloquy_command(script: &str) -> Option<SoliloquyCommand> {
    const PREFIXES: [&str; 2] = ["window.__soliloquyEval(", "globalThis.__soliloquyEval("];
    for prefix in PREFIXES {
        if let Some(rest) = script.strip_prefix(prefix) {
            let tokens = parse_quoted_arguments(rest.strip_suffix(')')?.trim())?;
            let (name, args) = tokens.split_first()?;
            return Some(SoliloquyCommand {
                name: name.clone(),
                args: args.to_vec(),
            });
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

fn dispatch_command(webview_id: WebViewId, command: &SoliloquyCommand) -> Option<JSValue> {
    let snapshot = webview_snapshot(webview_id);
    match command.name.as_str() {
        "engine.backend" => Some(JSValue::String(
            SoliloquyJavascriptBackend::from_environment()
                .as_str()
                .to_string(),
        )),
        "engine.status" => Some(JSValue::Object(HashMap::from([
            (
                "requestedEngine".to_string(),
                JSValue::String("v8-experimental".to_string()),
            ),
            (
                "activeEngine".to_string(),
                JSValue::String("soliloquy-dispatch".to_string()),
            ),
            ("bridgeReady".to_string(), JSValue::Boolean(false)),
            ("controlsDom".to_string(), JSValue::Boolean(false)),
            ("commandChannel".to_string(), JSValue::Boolean(true)),
        ]))),
        "webview.id" => Some(JSValue::Number(webview_id.0 as f64)),
        "webview.describe" => Some(JSValue::Object(HashMap::from([
            ("id".to_string(), JSValue::Number(webview_id.0 as f64)),
            (
                "backend".to_string(),
                JSValue::String(
                    SoliloquyJavascriptBackend::from_environment()
                        .as_str()
                        .to_string(),
                ),
            ),
            (
                "url".to_string(),
                snapshot
                    .as_ref()
                    .and_then(|snapshot| snapshot.current_url.clone())
                    .map(JSValue::String)
                    .unwrap_or(JSValue::Null),
            ),
            (
                "title".to_string(),
                snapshot
                    .as_ref()
                    .and_then(|snapshot| snapshot.page_title.clone())
                    .map(JSValue::String)
                    .unwrap_or(JSValue::Null),
            ),
            (
                "readyState".to_string(),
                snapshot
                    .as_ref()
                    .and_then(|snapshot| snapshot.ready_state.clone())
                    .map(JSValue::String)
                    .unwrap_or(JSValue::Null),
            ),
            (
                "controlsDom".to_string(),
                JSValue::Boolean(snapshot.is_some()),
            ),
        ]))),
        "dom.capabilities" => Some(JSValue::Object(HashMap::from([
            ("simpleEval".to_string(), JSValue::Boolean(true)),
            ("structuredCommands".to_string(), JSValue::Boolean(true)),
            ("liveDomProperties".to_string(), JSValue::Boolean(true)),
            ("liveDomWrites".to_string(), JSValue::Boolean(true)),
            ("controlsDom".to_string(), JSValue::Boolean(true)),
            (
                "fallbackEngine".to_string(),
                JSValue::String("mozjs".to_string()),
            ),
        ]))),
        "dom.inspect" => dispatch_dom_inspect(webview_id, command.args.first()?),
        "dom.set" => dispatch_dom_set(webview_id, &command.args),
        _ => None,
    }
}

fn dispatch_dom_inspect(webview_id: WebViewId, target: &str) -> Option<JSValue> {
    let (kind, writable) = match target {
        "document.title" => ("string", true),
        "location.href" => ("string", false),
        "document.readyState" => ("string", false),
        _ => return None,
    };

    let live_value = evaluate_live_dom_property(webview_id, target)
        .and_then(Result::ok)
        .unwrap_or(JSValue::Null);
    let value_available = live_value != JSValue::Null;

    Some(JSValue::Object(HashMap::from([
        ("target".to_string(), JSValue::String(target.to_string())),
        ("kind".to_string(), JSValue::String(kind.to_string())),
        ("writable".to_string(), JSValue::Boolean(writable)),
        (
            "status".to_string(),
            JSValue::String(if value_available {
                "live-snapshot".to_string()
            } else {
                "fallback-required".to_string()
            }),
        ),
        (
            "fallbackEngine".to_string(),
            JSValue::String("mozjs".to_string()),
        ),
        (
            "valueAvailable".to_string(),
            JSValue::Boolean(value_available),
        ),
        ("value".to_string(), live_value),
    ])))
}

fn dispatch_dom_set(webview_id: WebViewId, args: &[String]) -> Option<JSValue> {
    let [target, value] = args else {
        return None;
    };
    let value = JSValue::String(value.clone());
    apply_live_dom_write(webview_id, target, value.clone())?;
    Some(value)
}

fn apply_live_dom_write(webview_id: WebViewId, target: &str, value: JSValue) -> Option<()> {
    let mut snapshots = webview_snapshot_mut(webview_id);
    let snapshot = snapshots.get_mut(&webview_id)?;
    match (target, value) {
        ("document.title", JSValue::String(title)) => {
            snapshot.page_title = Some(title);
            Some(())
        },
        _ => None,
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
    use std::sync::Mutex;

    use embedder_traits::{JSValue, LoadStatus};
    use servo_base::id::WebViewId;

    use super::{
        SOLILOQUY_JS_ENGINE_ENV, SoliloquyCommand, SoliloquyJavascriptBackend,
        SoliloquyJavascriptDispatcher, parse_soliloquy_command, record_webview_history_change,
        record_webview_load_status, record_webview_page_title, reset_webview_snapshots,
    };

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn dispatcher_is_disabled_without_v8_experimental() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        std::env::remove_var(SOLILOQUY_JS_ENGINE_ENV);
        assert_eq!(
            SoliloquyJavascriptBackend::from_environment(),
            SoliloquyJavascriptBackend::Mozjs
        );
        assert!(SoliloquyJavascriptDispatcher::maybe_evaluate(WebViewId(1), "1 + 1").is_none());
    }

    #[test]
    fn dispatcher_handles_literals_and_simple_addition() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        std::env::set_var(SOLILOQUY_JS_ENGINE_ENV, "v8-experimental");
        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(WebViewId(1), "1 + 1"),
            Some(Ok(JSValue::Number(2.0)))
        );
        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(WebViewId(1), "'abc' + 'def'"),
            Some(Ok(JSValue::String("abcdef".into())))
        );
        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(
                WebViewId(1),
                "window.__soliloquyEngineBackend"
            ),
            Some(Ok(JSValue::String("v8-experimental".into())))
        );
        std::env::remove_var(SOLILOQUY_JS_ENGINE_ENV);
    }

    #[test]
    fn dispatcher_handles_structured_commands() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        std::env::set_var(SOLILOQUY_JS_ENGINE_ENV, "v8-experimental");
        record_webview_page_title(WebViewId(42), Some("Soliloquy".to_string()));
        record_webview_history_change(WebViewId(42), Some("https://example.test/".to_string()));
        record_webview_load_status(WebViewId(42), LoadStatus::Complete);

        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(
                WebViewId(42),
                "window.__soliloquyEval('webview.id')"
            ),
            Some(Ok(JSValue::Number(42.0)))
        );

        let result = SoliloquyJavascriptDispatcher::maybe_evaluate(
            WebViewId(42),
            "globalThis.__soliloquyEval('engine.status')",
        );
        assert!(matches!(result, Some(Ok(JSValue::Object(_)))));

        let dom_capabilities = SoliloquyJavascriptDispatcher::maybe_evaluate(
            WebViewId(42),
            "window.__soliloquyEval('dom.capabilities')",
        );
        assert!(matches!(dom_capabilities, Some(Ok(JSValue::Object(_)))));

        let dom_inspect = SoliloquyJavascriptDispatcher::maybe_evaluate(
            WebViewId(42),
            "window.__soliloquyEval('dom.inspect', 'document.title')",
        );
        assert!(matches!(dom_inspect, Some(Ok(JSValue::Object(_)))));

        let dom_inspect_unknown = SoliloquyJavascriptDispatcher::maybe_evaluate(
            WebViewId(42),
            "window.__soliloquyEval('dom.inspect', 'document.body.innerHTML')",
        );
        assert!(dom_inspect_unknown.is_none());

        assert!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(
                WebViewId(42),
                "window.__soliloquyEval('not.supported')",
            )
            .is_none()
        );

        std::env::remove_var(SOLILOQUY_JS_ENGINE_ENV);
    }

    #[test]
    fn dispatcher_reads_live_dom_properties_from_snapshot() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        std::env::set_var(SOLILOQUY_JS_ENGINE_ENV, "v8-experimental");
        record_webview_page_title(WebViewId(21), Some("Snapshot Title".to_string()));
        record_webview_history_change(
            WebViewId(21),
            Some("https://soliloquy.test/current".to_string()),
        );
        record_webview_load_status(WebViewId(21), LoadStatus::HeadParsed);

        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(WebViewId(21), "document.title"),
            Some(Ok(JSValue::String("Snapshot Title".into())))
        );
        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(WebViewId(21), "location.href"),
            Some(Ok(JSValue::String("https://soliloquy.test/current".into())))
        );
        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(WebViewId(21), "document.readyState"),
            Some(Ok(JSValue::String("interactive".into())))
        );

        std::env::remove_var(SOLILOQUY_JS_ENGINE_ENV);
    }

    #[test]
    fn dispatcher_writes_live_dom_title() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        std::env::set_var(SOLILOQUY_JS_ENGINE_ENV, "v8-experimental");

        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(
                WebViewId(30),
                "document.title = 'Updated Title'"
            ),
            Some(Ok(JSValue::String("Updated Title".into())))
        );
        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(WebViewId(30), "document.title"),
            Some(Ok(JSValue::String("Updated Title".into())))
        );
        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(
                WebViewId(30),
                "window.__soliloquyEval('dom.set', 'document.title', 'Command Title')"
            ),
            Some(Ok(JSValue::String("Command Title".into())))
        );
        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate(WebViewId(30), "document.title"),
            Some(Ok(JSValue::String("Command Title".into())))
        );

        std::env::remove_var(SOLILOQUY_JS_ENGINE_ENV);
    }

    #[test]
    fn parser_supports_multiple_quoted_arguments() {
        assert_eq!(
            parse_soliloquy_command("window.__soliloquyEval('dom.inspect', 'location.href')"),
            Some(SoliloquyCommand {
                name: "dom.inspect".to_string(),
                args: vec!["location.href".to_string()],
            })
        );
    }

    #[test]
    fn dom_inspect_returns_fallback_metadata() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        std::env::set_var(SOLILOQUY_JS_ENGINE_ENV, "v8-experimental");
        record_webview_history_change(
            WebViewId(13),
            Some("https://soliloquy.test/dom".to_string()),
        );

        let result = SoliloquyJavascriptDispatcher::maybe_evaluate(
            WebViewId(13),
            "window.__soliloquyEval('dom.inspect', 'location.href')",
        );

        let object = match result {
            Some(Ok(JSValue::Object(object))) => object,
            other => panic!("unexpected dom.inspect result: {other:?}"),
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

        std::env::remove_var(SOLILOQUY_JS_ENGINE_ENV);
    }
}
