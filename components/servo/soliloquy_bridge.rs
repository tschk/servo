/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use embedder_traits::{JSValue, LoadStatus};
use servo_base::id::{BrowsingContextId, WebViewId};
use url::{ParseError, Url};

static WEBVIEW_SNAPSHOTS: LazyLock<Mutex<HashMap<WebViewId, SoliloquyWebViewSnapshot>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) const SOLILOQUY_BRIDGE_SCHEMA: &str = include_str!("soliloquy_bridge_schema.json");
pub(crate) const SOLILOQUY_BRIDGE_SCHEMA_VERSION: &str = "rv8-bridge-v1";

#[derive(Clone, Debug, Default)]
struct SoliloquyWebViewSnapshot {
    page_title: Option<String>,
    current_url: Option<String>,
    ready_state: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SoliloquyBridgeReadTarget {
    DocumentTitle,
    LocationHref,
    DocumentReadyState,
}

impl SoliloquyBridgeReadTarget {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "document.title" => Some(Self::DocumentTitle),
            "location.href" | "window.location.href" => Some(Self::LocationHref),
            "document.readyState" => Some(Self::DocumentReadyState),
            _ => None,
        }
    }

    fn kind(self) -> &'static str {
        "string"
    }

    fn writable(self) -> bool {
        matches!(self, Self::DocumentTitle | Self::LocationHref)
    }

    fn label(self) -> &'static str {
        match self {
            Self::DocumentTitle => "document.title",
            Self::LocationHref => "location.href",
            Self::DocumentReadyState => "document.readyState",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SoliloquyBridgeWrite {
    SetDocumentTitle(String),
    SetLocationHref(String),
}

impl SoliloquyBridgeWrite {
    pub(crate) fn parse(target: &str, value: &str) -> Option<Self> {
        match target {
            "document.title" => Some(Self::SetDocumentTitle(value.to_string())),
            "location.href" | "window.location.href" => {
                Some(Self::SetLocationHref(value.to_string()))
            },
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SoliloquyBridgeCommand {
    EngineBackend,
    EngineStatus,
    WebViewId,
    WebViewDescribe,
    DomCapabilities,
    DomInspect(SoliloquyBridgeReadTarget),
    DomSet(SoliloquyBridgeWrite),
    Unsupported(String),
}

impl SoliloquyBridgeCommand {
    pub(crate) fn parse(name: &str, args: &[String]) -> Self {
        match name {
            "engine.backend" if args.is_empty() => Self::EngineBackend,
            "engine.status" if args.is_empty() => Self::EngineStatus,
            "webview.id" if args.is_empty() => Self::WebViewId,
            "webview.describe" if args.is_empty() => Self::WebViewDescribe,
            "dom.capabilities" if args.is_empty() => Self::DomCapabilities,
            "dom.inspect" => parse_dom_inspect(args)
                .map(Self::DomInspect)
                .unwrap_or_else(|| Self::Unsupported(name.to_string())),
            "dom.set" => parse_dom_set(args)
                .map(Self::DomSet)
                .unwrap_or_else(|| Self::Unsupported(name.to_string())),
            _ => Self::Unsupported(name.to_string()),
        }
    }
}

pub(crate) fn resolve_write(
    webview_id: WebViewId,
    write: SoliloquyBridgeWrite,
) -> Result<SoliloquyBridgeWrite, SoliloquyBridgeResult> {
    match write {
        SoliloquyBridgeWrite::SetDocumentTitle(title) => {
            Ok(SoliloquyBridgeWrite::SetDocumentTitle(title))
        },
        SoliloquyBridgeWrite::SetLocationHref(url) => {
            resolve_location_href(webview_id, &url).map(SoliloquyBridgeWrite::SetLocationHref)
        },
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SoliloquyBridgeMutation {
    Navigate { url: String },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SoliloquyBridgeWriteOutcome {
    value: JSValue,
    mutation: Option<SoliloquyBridgeMutation>,
}

impl SoliloquyBridgeWriteOutcome {
    fn value(value: JSValue) -> Self {
        Self {
            value,
            mutation: None,
        }
    }

    fn with_mutation(value: JSValue, mutation: SoliloquyBridgeMutation) -> Self {
        Self {
            value,
            mutation: Some(mutation),
        }
    }

    pub(crate) fn into_parts(self) -> (JSValue, Option<SoliloquyBridgeMutation>) {
        (self.value, self.mutation)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SoliloquyBridgeResult {
    Value(JSValue),
    Unsupported { operation: String },
    Error { message: String },
}

impl SoliloquyBridgeResult {
    pub(crate) fn value(value: JSValue) -> Self {
        Self::Value(value)
    }

    pub(crate) fn unsupported(operation: impl Into<String>) -> Self {
        Self::Unsupported {
            operation: operation.into(),
        }
    }

    pub(crate) fn error(message: impl Into<String>) -> Self {
        Self::Error {
            message: message.into(),
        }
    }

    pub(crate) fn into_js_value(self) -> JSValue {
        match self {
            Self::Value(value) => bridge_result_object("ok", value, JSValue::Null),
            Self::Unsupported { operation } => {
                bridge_result_object("unsupported", JSValue::Null, JSValue::String(operation))
            },
            Self::Error { message } => {
                bridge_result_object("error", JSValue::Null, JSValue::String(message))
            },
        }
    }
}

pub fn record_webview_page_title(webview_id: WebViewId, title: Option<String>) {
    update_webview_snapshot(webview_id, |snapshot| {
        snapshot.page_title = title;
    });
}

pub fn record_webview_navigation_request(webview_id: WebViewId, url: String) {
    update_webview_snapshot(webview_id, |snapshot| {
        snapshot.current_url = Some(url);
    });
}

pub fn record_webview_history_change(webview_id: WebViewId, current_url: Option<String>) {
    update_webview_snapshot(webview_id, |snapshot| {
        snapshot.current_url = current_url;
    });
}

pub fn record_webview_load_status(webview_id: WebViewId, load_status: LoadStatus) {
    update_webview_snapshot(webview_id, |snapshot| {
        snapshot.ready_state = Some(match load_status {
            LoadStatus::Started => "loading".to_string(),
            LoadStatus::HeadParsed => "interactive".to_string(),
            LoadStatus::Complete => "complete".to_string(),
        });
    });
}

pub(crate) fn clear_webview_snapshot(webview_id: WebViewId) {
    WEBVIEW_SNAPSHOTS.lock().unwrap().remove(&webview_id);
}

#[cfg(test)]
pub(crate) fn reset_webview_snapshots() {
    WEBVIEW_SNAPSHOTS.lock().unwrap().clear();
}

pub(crate) fn read_property(
    webview_id: WebViewId,
    target: SoliloquyBridgeReadTarget,
) -> Option<JSValue> {
    let snapshot = webview_snapshot(webview_id)?;
    Some(match target {
        SoliloquyBridgeReadTarget::DocumentTitle => snapshot
            .page_title
            .map(JSValue::String)
            .unwrap_or(JSValue::Null),
        SoliloquyBridgeReadTarget::LocationHref => snapshot
            .current_url
            .map(JSValue::String)
            .unwrap_or(JSValue::Null),
        SoliloquyBridgeReadTarget::DocumentReadyState => snapshot
            .ready_state
            .map(JSValue::String)
            .unwrap_or(JSValue::Null),
    })
}

pub(crate) fn write_property(
    webview_id: WebViewId,
    write: SoliloquyBridgeWrite,
) -> SoliloquyBridgeWriteOutcome {
    let mut snapshots = WEBVIEW_SNAPSHOTS.lock().unwrap();
    let snapshot = snapshots.entry(webview_id).or_default();
    match write {
        SoliloquyBridgeWrite::SetDocumentTitle(title) => {
            snapshot.page_title = Some(title.clone());
            SoliloquyBridgeWriteOutcome::value(JSValue::String(title))
        },
        SoliloquyBridgeWrite::SetLocationHref(url) => {
            snapshot.current_url = Some(url.clone());
            snapshot.ready_state = Some("loading".to_string());
            SoliloquyBridgeWriteOutcome::with_mutation(
                JSValue::String(url.clone()),
                SoliloquyBridgeMutation::Navigate { url },
            )
        },
    }
}

pub(crate) fn inspect_property(
    webview_id: WebViewId,
    target: SoliloquyBridgeReadTarget,
) -> JSValue {
    let value = read_property(webview_id, target).unwrap_or(JSValue::Null);
    let value_available = value != JSValue::Null;
    JSValue::Object(HashMap::from([
        (
            "target".to_string(),
            JSValue::String(target.label().to_string()),
        ),
        (
            "kind".to_string(),
            JSValue::String(target.kind().to_string()),
        ),
        ("writable".to_string(), JSValue::Boolean(target.writable())),
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
        ("value".to_string(), value),
    ]))
}

/// True when embedder/Servo has pushed at least one live DOM field for this webview.
pub(crate) fn webview_bridge_ready(webview_id: WebViewId) -> bool {
    webview_snapshot(webview_id).is_some_and(|snapshot| {
        snapshot.page_title.is_some()
            || snapshot.current_url.is_some()
            || snapshot.ready_state.is_some()
    })
}

pub(crate) fn describe_webview(webview_id: WebViewId, backend: &str) -> JSValue {
    let snapshot = webview_snapshot(webview_id);
    JSValue::Object(HashMap::from([
        (
            "id".to_string(),
            JSValue::Number(webview_bridge_id(webview_id)),
        ),
        ("backend".to_string(), JSValue::String(backend.to_string())),
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
    ]))
}

pub(crate) fn webview_bridge_id(webview_id: WebViewId) -> f64 {
    let browsing_context_id = BrowsingContextId::from(webview_id);
    browsing_context_id.index.0.get() as f64
}

pub(crate) fn capabilities() -> JSValue {
    JSValue::Object(HashMap::from([
        ("simpleEval".to_string(), JSValue::Boolean(true)),
        ("structuredCommands".to_string(), JSValue::Boolean(true)),
        ("liveDomProperties".to_string(), JSValue::Boolean(true)),
        ("liveDomWrites".to_string(), JSValue::Boolean(true)),
        ("navigationWrites".to_string(), JSValue::Boolean(true)),
        ("controlsDom".to_string(), JSValue::Boolean(true)),
        (
            "fallbackEngine".to_string(),
            JSValue::String("mozjs".to_string()),
        ),
        (
            "schemaVersion".to_string(),
            JSValue::String(SOLILOQUY_BRIDGE_SCHEMA_VERSION.to_string()),
        ),
        (
            "schema".to_string(),
            JSValue::String(SOLILOQUY_BRIDGE_SCHEMA.to_string()),
        ),
    ]))
}

fn bridge_result_object(status: &str, value: JSValue, detail: JSValue) -> JSValue {
    let ok = matches!(status, "ok");
    JSValue::Object(HashMap::from([
        ("ok".to_string(), JSValue::Boolean(ok)),
        ("status".to_string(), JSValue::String(status.to_string())),
        ("value".to_string(), value),
        ("detail".to_string(), detail),
    ]))
}

fn parse_dom_inspect(args: &[String]) -> Option<SoliloquyBridgeReadTarget> {
    let [target] = args else {
        return None;
    };
    SoliloquyBridgeReadTarget::parse(target)
}

fn parse_dom_set(args: &[String]) -> Option<SoliloquyBridgeWrite> {
    let [target, value] = args else {
        return None;
    };
    SoliloquyBridgeWrite::parse(target, value)
}

fn resolve_location_href(
    webview_id: WebViewId,
    href: &str,
) -> Result<String, SoliloquyBridgeResult> {
    if href.is_empty() {
        return Err(SoliloquyBridgeResult::error(
            "location.href cannot be empty",
        ));
    }

    match Url::parse(href) {
        Ok(url) => Ok(url.to_string()),
        Err(ParseError::RelativeUrlWithoutBase) => {
            let snapshot = webview_snapshot(webview_id);
            let Some(base_url) = snapshot.and_then(|snapshot| snapshot.current_url) else {
                return Err(SoliloquyBridgeResult::error(
                    "invalid location.href: relative URL without a base",
                ));
            };
            let base = Url::parse(&base_url).map_err(|error| {
                SoliloquyBridgeResult::error(format!("invalid base location.href: {error}"))
            })?;
            base.join(href).map(|url| url.to_string()).map_err(|error| {
                SoliloquyBridgeResult::error(format!("invalid location.href: {error}"))
            })
        },
        Err(error) => Err(SoliloquyBridgeResult::error(format!(
            "invalid location.href: {error}"
        ))),
    }
}

fn update_webview_snapshot(
    webview_id: WebViewId,
    update: impl FnOnce(&mut SoliloquyWebViewSnapshot),
) {
    let mut snapshots = WEBVIEW_SNAPSHOTS.lock().unwrap();
    update(snapshots.entry(webview_id).or_default());
}

fn webview_snapshot(webview_id: WebViewId) -> Option<SoliloquyWebViewSnapshot> {
    WEBVIEW_SNAPSHOTS.lock().unwrap().get(&webview_id).cloned()
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct BridgeSchema {
        commands: Vec<BridgeSchemaCommand>,
        targets: BridgeSchemaTargets,
    }

    #[derive(Deserialize)]
    struct BridgeSchemaCommand {
        name: String,
        args: Vec<String>,
    }

    #[derive(Deserialize)]
    struct BridgeSchemaTargets {
        read: Vec<String>,
        write: Vec<String>,
    }

    #[test]
    fn capabilities_expose_bridge_schema() {
        let JSValue::Object(capabilities) = capabilities() else {
            panic!("expected capabilities object");
        };

        assert_eq!(
            capabilities.get("schemaVersion"),
            Some(&JSValue::String(
                SOLILOQUY_BRIDGE_SCHEMA_VERSION.to_string()
            ))
        );
        assert_eq!(
            capabilities.get("schema"),
            Some(&JSValue::String(SOLILOQUY_BRIDGE_SCHEMA.to_string()))
        );
    }

    #[test]
    fn bridge_schema_lists_supported_commands_and_targets() {
        let schema = bridge_schema();

        for command in schema.commands {
            let args = example_command_args(&command);
            assert_eq!(args.len(), command.args.len());
            let parsed = SoliloquyBridgeCommand::parse(&command.name, &args);
            assert!(
                !matches!(parsed, SoliloquyBridgeCommand::Unsupported(_)),
                "schema command should parse: {}",
                command.name
            );
        }

        for target in schema.targets.read {
            assert!(
                SoliloquyBridgeReadTarget::parse(&target).is_some(),
                "schema read target should parse: {target}"
            );
        }

        for target in schema.targets.write {
            assert!(
                SoliloquyBridgeWrite::parse(&target, "value").is_some(),
                "schema write target should parse: {target}"
            );
        }
    }

    fn bridge_schema() -> BridgeSchema {
        serde_json::from_str(SOLILOQUY_BRIDGE_SCHEMA).expect("bridge schema should be valid JSON")
    }

    fn example_command_args(command: &BridgeSchemaCommand) -> Vec<String> {
        match command.name.as_str() {
            "dom.inspect" => vec!["document.title".to_string()],
            "dom.set" => vec!["document.title".to_string(), "value".to_string()],
            _ => Vec::new(),
        }
    }
}
