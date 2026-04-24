/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use embedder_traits::{JSValue, LoadStatus};
use servo_base::id::WebViewId;
use url::{ParseError, Url};

static WEBVIEW_SNAPSHOTS: LazyLock<Mutex<HashMap<WebViewId, SoliloquyWebViewSnapshot>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

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
    let mut snapshots = webview_snapshot_mut(webview_id);
    let snapshot = snapshots.get_mut(&webview_id).expect("snapshot must exist");
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

pub(crate) fn describe_webview(webview_id: WebViewId, backend: &str) -> JSValue {
    let snapshot = webview_snapshot(webview_id);
    JSValue::Object(HashMap::from([
        ("id".to_string(), JSValue::Number(webview_id.0 as f64)),
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
