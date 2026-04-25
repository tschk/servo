/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use embedder_traits::{JSValue, JavaScriptEvaluationError, JavaScriptEvaluationId, UrlRequest};
use log::info;
use rustc_hash::FxHashMap;
use servo_base::id::WebViewId;
use servo_constellation_traits::EmbedderToConstellationMessage;
use url::Url;

use crate::proxies::ConstellationProxy;
use crate::soliloquy_bridge::{SoliloquyBridgeMutation, SoliloquyBridgeResult};
use crate::soliloquy_javascript::SoliloquyJavascriptDispatcher;

struct PendingEvaluation {
    callback: Box<dyn FnOnce(Result<JSValue, JavaScriptEvaluationError>)>,
}

pub(crate) struct JavaScriptEvaluator {
    current_id: JavaScriptEvaluationId,
    constellation_proxy: ConstellationProxy,
    pending_evaluations: FxHashMap<JavaScriptEvaluationId, PendingEvaluation>,
}

impl JavaScriptEvaluator {
    pub(crate) fn new(constellation_proxy: ConstellationProxy) -> Self {
        Self {
            current_id: JavaScriptEvaluationId(0),
            constellation_proxy,
            pending_evaluations: Default::default(),
        }
    }

    fn generate_id(&mut self) -> JavaScriptEvaluationId {
        let next_id = JavaScriptEvaluationId(self.current_id.0 + 1);
        std::mem::replace(&mut self.current_id, next_id)
    }

    pub(crate) fn evaluate(
        &mut self,
        webview_id: WebViewId,
        script: String,
        callback: Box<dyn FnOnce(Result<JSValue, JavaScriptEvaluationError>)>,
    ) {
        if let Some(evaluation) =
            SoliloquyJavascriptDispatcher::maybe_evaluate_with_mutations(webview_id, &script)
        {
            info!(
                "Soliloquy experimental dispatcher handled JavaScript evaluation locally for webview {:?}",
                webview_id
            );
            if let Some(error) = self.apply_soliloquy_mutations(webview_id, evaluation.mutations) {
                callback(Ok(error.into_js_value()));
                return;
            }
            callback(evaluation.result);
            return;
        }

        let evaluation_id = self.generate_id();
        self.constellation_proxy
            .send(EmbedderToConstellationMessage::EvaluateJavaScript(
                webview_id,
                evaluation_id,
                script,
            ));
        self.pending_evaluations
            .insert(evaluation_id, PendingEvaluation { callback });
    }

    pub(crate) fn finish_evaluation(
        &mut self,
        evaluation_id: JavaScriptEvaluationId,
        result: Result<JSValue, JavaScriptEvaluationError>,
    ) {
        (self
            .pending_evaluations
            .remove(&evaluation_id)
            .expect("Received request to finish unknown JavaScript evaluation.")
            .callback)(result)
    }

    fn apply_soliloquy_mutations(
        &self,
        webview_id: WebViewId,
        mutations: Vec<SoliloquyBridgeMutation>,
    ) -> Option<SoliloquyBridgeResult> {
        for mutation in mutations {
            match mutation {
                SoliloquyBridgeMutation::Navigate { url } => {
                    let Ok(url) = Url::parse(&url) else {
                        return Some(SoliloquyBridgeResult::error("invalid location.href"));
                    };
                    self.constellation_proxy
                        .send(EmbedderToConstellationMessage::LoadUrl(
                            webview_id,
                            UrlRequest::new(url),
                        ));
                },
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Mutex;

    use crossbeam_channel::TryRecvError;
    use embedder_traits::JSValue;
    use servo_base::id::{BrowsingContextId, WebViewId};
    use servo_constellation_traits::EmbedderToConstellationMessage;

    use super::JavaScriptEvaluator;
    use crate::proxies::ConstellationProxy;
    use crate::soliloquy_bridge::{
        record_webview_history_change, record_webview_page_title, reset_webview_snapshots,
    };

    static ENV_LOCK: Mutex<()> = Mutex::new(());
    const SOLILOQUY_JS_ENGINE_ENV: &str = "SOLILOQUY_JS_ENGINE";

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
    fn experimental_dispatcher_short_circuits_simple_scripts() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        set_engine_env(&_guard, "v8-experimental");
        let (proxy, receiver) = ConstellationProxy::new();
        let mut evaluator = JavaScriptEvaluator::new(proxy);
        let result = Rc::new(RefCell::new(None));
        let callback_result = result.clone();

        evaluator.evaluate(
            test_webview_id(7),
            "1 + 1".to_string(),
            Box::new(move |value| *callback_result.borrow_mut() = Some(value)),
        );

        assert_eq!(*result.borrow(), Some(Ok(JSValue::Number(2.0))));
        assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
        clear_engine_env(&_guard);
    }

    #[test]
    fn structured_command_dispatch_stays_local() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        set_engine_env(&_guard, "v8-experimental");
        let (proxy, receiver) = ConstellationProxy::new();
        let mut evaluator = JavaScriptEvaluator::new(proxy);
        let result = Rc::new(RefCell::new(None));
        let callback_result = result.clone();

        evaluator.evaluate(
            test_webview_id(11),
            "window.__soliloquyEval('webview.describe')".to_string(),
            Box::new(move |value| *callback_result.borrow_mut() = Some(value)),
        );

        assert!(matches!(*result.borrow(), Some(Ok(JSValue::Object(_)))));
        assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
        clear_engine_env(&_guard);
    }

    #[test]
    fn dom_inspection_dispatch_stays_local() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        set_engine_env(&_guard, "v8-experimental");
        record_webview_page_title(test_webview_id(17), Some("Servo Title".to_string()));
        let (proxy, receiver) = ConstellationProxy::new();
        let mut evaluator = JavaScriptEvaluator::new(proxy);
        let result = Rc::new(RefCell::new(None));
        let callback_result = result.clone();

        evaluator.evaluate(
            test_webview_id(17),
            "window.__soliloquyEval('dom.inspect', 'document.title')".to_string(),
            Box::new(move |value| *callback_result.borrow_mut() = Some(value)),
        );

        assert!(matches!(*result.borrow(), Some(Ok(JSValue::Object(_)))));
        assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
        clear_engine_env(&_guard);
    }

    #[test]
    fn live_dom_property_dispatch_stays_local() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        set_engine_env(&_guard, "v8-experimental");
        record_webview_history_change(
            test_webview_id(23),
            Some("https://soliloquy.test/live".to_string()),
        );
        let (proxy, receiver) = ConstellationProxy::new();
        let mut evaluator = JavaScriptEvaluator::new(proxy);
        let result = Rc::new(RefCell::new(None));
        let callback_result = result.clone();

        evaluator.evaluate(
            test_webview_id(23),
            "location.href".to_string(),
            Box::new(move |value| *callback_result.borrow_mut() = Some(value)),
        );

        assert_eq!(
            *result.borrow(),
            Some(Ok(JSValue::String(
                "https://soliloquy.test/live".to_string()
            )))
        );
        assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
        clear_engine_env(&_guard);
    }

    #[test]
    fn live_dom_write_dispatch_stays_local() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        set_engine_env(&_guard, "v8-experimental");
        let (proxy, receiver) = ConstellationProxy::new();
        let mut evaluator = JavaScriptEvaluator::new(proxy);
        let result = Rc::new(RefCell::new(None));
        let callback_result = result.clone();

        evaluator.evaluate(
            test_webview_id(29),
            "document.title = 'Bridge Title'".to_string(),
            Box::new(move |value| *callback_result.borrow_mut() = Some(value)),
        );

        assert_eq!(
            *result.borrow(),
            Some(Ok(JSValue::String("Bridge Title".to_string())))
        );
        assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
        clear_engine_env(&_guard);
    }

    #[test]
    fn live_location_write_dispatches_navigation_request() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        set_engine_env(&_guard, "v8-experimental");
        let (proxy, receiver) = ConstellationProxy::new();
        let mut evaluator = JavaScriptEvaluator::new(proxy);
        let result = Rc::new(RefCell::new(None));
        let callback_result = result.clone();

        evaluator.evaluate(
            test_webview_id(31),
            "location.href = 'https://soliloquy.test/navigate'".to_string(),
            Box::new(move |value| *callback_result.borrow_mut() = Some(value)),
        );

        assert_eq!(
            *result.borrow(),
            Some(Ok(JSValue::String(
                "https://soliloquy.test/navigate".to_string()
            )))
        );

        match receiver.try_recv() {
            Ok(EmbedderToConstellationMessage::LoadUrl(webview_id, request)) => {
                assert_eq!(webview_id, test_webview_id(31));
                assert_eq!(request.url.to_string(), "https://soliloquy.test/navigate");
            },
            _ => panic!("expected LoadUrl message"),
        }
        clear_engine_env(&_guard);
    }

    #[test]
    fn relative_location_write_dispatches_navigation_request() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        set_engine_env(&_guard, "v8-experimental");
        let webview_id = test_webview_id(33);
        record_webview_history_change(
            webview_id,
            Some("https://soliloquy.test/path/page".to_string()),
        );
        let (proxy, receiver) = ConstellationProxy::new();
        let mut evaluator = JavaScriptEvaluator::new(proxy);
        let result = Rc::new(RefCell::new(None));
        let callback_result = result.clone();

        evaluator.evaluate(
            webview_id,
            "location.href = '../next?x=1'".to_string(),
            Box::new(move |value| *callback_result.borrow_mut() = Some(value)),
        );

        assert_eq!(
            *result.borrow(),
            Some(Ok(JSValue::String(
                "https://soliloquy.test/next?x=1".to_string()
            )))
        );

        match receiver.try_recv() {
            Ok(EmbedderToConstellationMessage::LoadUrl(webview_id, request)) => {
                assert_eq!(webview_id, test_webview_id(33));
                assert_eq!(request.url.to_string(), "https://soliloquy.test/next?x=1");
            },
            _ => panic!("expected LoadUrl message"),
        }
        clear_engine_env(&_guard);
    }

    #[test]
    fn unsupported_structured_commands_stay_local_with_envelope() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        set_engine_env(&_guard, "v8-experimental");
        let (proxy, receiver) = ConstellationProxy::new();
        let mut evaluator = JavaScriptEvaluator::new(proxy);
        let result = Rc::new(RefCell::new(None));
        let callback_result = result.clone();

        evaluator.evaluate(
            test_webview_id(19),
            "window.__soliloquyEval('dom.inspect', 'document.body.innerHTML')".to_string(),
            Box::new(move |value| *callback_result.borrow_mut() = Some(value)),
        );

        assert!(matches!(*result.borrow(), Some(Ok(JSValue::Object(_)))));
        assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
        clear_engine_env(&_guard);
    }

    #[test]
    fn unsupported_scripts_fall_back_to_constellation() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        set_engine_env(&_guard, "v8-experimental");
        let (proxy, receiver) = ConstellationProxy::new();
        let mut evaluator = JavaScriptEvaluator::new(proxy);

        evaluator.evaluate(
            test_webview_id(9),
            "document.body".to_string(),
            Box::new(|_| {}),
        );

        assert!(matches!(
            receiver.try_recv(),
            Ok(EmbedderToConstellationMessage::EvaluateJavaScript(_, _, _))
        ));
        clear_engine_env(&_guard);
    }
}
