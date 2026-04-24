/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use embedder_traits::{JSValue, JavaScriptEvaluationError};

const SOLILOQUY_JS_ENGINE_ENV: &str = "SOLILOQUY_JS_ENGINE";

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

        evaluate_literal(trimmed)
            .or_else(|| evaluate_binary_plus(trimmed))
            .or_else(|| evaluate_engine_probe(trimmed))
    }
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

fn evaluate_engine_probe(script: &str) -> Option<Result<JSValue, JavaScriptEvaluationError>> {
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

#[cfg(test)]
mod tests {
    use embedder_traits::JSValue;

    use super::{
        SOLILOQUY_JS_ENGINE_ENV, SoliloquyJavascriptBackend, SoliloquyJavascriptDispatcher,
    };

    #[test]
    fn dispatcher_is_disabled_without_v8_experimental() {
        std::env::remove_var(SOLILOQUY_JS_ENGINE_ENV);
        assert_eq!(
            SoliloquyJavascriptBackend::from_environment(),
            SoliloquyJavascriptBackend::Mozjs
        );
        assert!(SoliloquyJavascriptDispatcher::maybe_evaluate("1 + 1").is_none());
    }

    #[test]
    fn dispatcher_handles_literals_and_simple_addition() {
        std::env::set_var(SOLILOQUY_JS_ENGINE_ENV, "v8-experimental");
        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate("1 + 1"),
            Some(Ok(JSValue::Number(2.0)))
        );
        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate("'abc' + 'def'"),
            Some(Ok(JSValue::String("abcdef".into())))
        );
        assert_eq!(
            SoliloquyJavascriptDispatcher::maybe_evaluate("window.__soliloquyEngineBackend"),
            Some(Ok(JSValue::String("v8-experimental".into())))
        );
        std::env::remove_var(SOLILOQUY_JS_ENGINE_ENV);
    }
}
