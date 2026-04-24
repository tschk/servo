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
    use servo_constellation_traits::EmbedderToConstellationMessage;

    use super::JavaScriptEvaluator;
    use crate::proxies::ConstellationProxy;
    use crate::soliloquy_bridge::{
        record_webview_history_change, record_webview_page_title, reset_webview_snapshots,
    };

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn experimental_dispatcher_short_circuits_simple_scripts() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        std::env::set_var("SOLILOQUY_JS_ENGINE", "v8-experimental");
        let (proxy, receiver) = ConstellationProxy::new();
        let mut evaluator = JavaScriptEvaluator::new(proxy);
        let result = Rc::new(RefCell::new(None));
        let callback_result = result.clone();

        evaluator.evaluate(
            servo_base::id::WebViewId(7),
            "1 + 1".to_string(),
            Box::new(move |value| *callback_result.borrow_mut() = Some(value)),
        );

        assert_eq!(*result.borrow(), Some(Ok(JSValue::Number(2.0))));
        assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
        std::env::remove_var("SOLILOQUY_JS_ENGINE");
    }

    #[test]
    fn structured_command_dispatch_stays_local() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        std::env::set_var("SOLILOQUY_JS_ENGINE", "v8-experimental");
        let (proxy, receiver) = ConstellationProxy::new();
        let mut evaluator = JavaScriptEvaluator::new(proxy);
        let result = Rc::new(RefCell::new(None));
        let callback_result = result.clone();

        evaluator.evaluate(
            servo_base::id::WebViewId(11),
            "window.__soliloquyEval('webview.describe')".to_string(),
            Box::new(move |value| *callback_result.borrow_mut() = Some(value)),
        );

        assert!(matches!(*result.borrow(), Some(Ok(JSValue::Object(_)))));
        assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
        std::env::remove_var("SOLILOQUY_JS_ENGINE");
    }

    #[test]
    fn dom_inspection_dispatch_stays_local() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        std::env::set_var("SOLILOQUY_JS_ENGINE", "v8-experimental");
        record_webview_page_title(
            servo_base::id::WebViewId(17),
            Some("Servo Title".to_string()),
        );
        let (proxy, receiver) = ConstellationProxy::new();
        let mut evaluator = JavaScriptEvaluator::new(proxy);
        let result = Rc::new(RefCell::new(None));
        let callback_result = result.clone();

        evaluator.evaluate(
            servo_base::id::WebViewId(17),
            "window.__soliloquyEval('dom.inspect', 'document.title')".to_string(),
            Box::new(move |value| *callback_result.borrow_mut() = Some(value)),
        );

        assert!(matches!(*result.borrow(), Some(Ok(JSValue::Object(_)))));
        assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
        std::env::remove_var("SOLILOQUY_JS_ENGINE");
    }

    #[test]
    fn live_dom_property_dispatch_stays_local() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        std::env::set_var("SOLILOQUY_JS_ENGINE", "v8-experimental");
        record_webview_history_change(
            servo_base::id::WebViewId(23),
            Some("https://soliloquy.test/live".to_string()),
        );
        let (proxy, receiver) = ConstellationProxy::new();
        let mut evaluator = JavaScriptEvaluator::new(proxy);
        let result = Rc::new(RefCell::new(None));
        let callback_result = result.clone();

        evaluator.evaluate(
            servo_base::id::WebViewId(23),
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
        std::env::remove_var("SOLILOQUY_JS_ENGINE");
    }

    #[test]
    fn live_dom_write_dispatch_stays_local() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        std::env::set_var("SOLILOQUY_JS_ENGINE", "v8-experimental");
        let (proxy, receiver) = ConstellationProxy::new();
        let mut evaluator = JavaScriptEvaluator::new(proxy);
        let result = Rc::new(RefCell::new(None));
        let callback_result = result.clone();

        evaluator.evaluate(
            servo_base::id::WebViewId(29),
            "document.title = 'Bridge Title'".to_string(),
            Box::new(move |value| *callback_result.borrow_mut() = Some(value)),
        );

        assert_eq!(
            *result.borrow(),
            Some(Ok(JSValue::String("Bridge Title".to_string())))
        );
        assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
        std::env::remove_var("SOLILOQUY_JS_ENGINE");
    }

    #[test]
    fn live_location_write_dispatches_navigation_request() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        std::env::set_var("SOLILOQUY_JS_ENGINE", "v8-experimental");
        let (proxy, receiver) = ConstellationProxy::new();
        let mut evaluator = JavaScriptEvaluator::new(proxy);
        let result = Rc::new(RefCell::new(None));
        let callback_result = result.clone();

        evaluator.evaluate(
            servo_base::id::WebViewId(31),
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
                assert_eq!(webview_id, servo_base::id::WebViewId(31));
                assert_eq!(request.url.to_string(), "https://soliloquy.test/navigate");
            },
            _ => panic!("expected LoadUrl message"),
        }
        std::env::remove_var("SOLILOQUY_JS_ENGINE");
    }

    #[test]
    fn relative_location_write_dispatches_navigation_request() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        std::env::set_var("SOLILOQUY_JS_ENGINE", "v8-experimental");
        let webview_id = servo_base::id::WebViewId(33);
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
                assert_eq!(webview_id, servo_base::id::WebViewId(33));
                assert_eq!(request.url.to_string(), "https://soliloquy.test/next?x=1");
            },
            _ => panic!("expected LoadUrl message"),
        }
        std::env::remove_var("SOLILOQUY_JS_ENGINE");
    }

    #[test]
    fn unsupported_structured_commands_stay_local_with_envelope() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        std::env::set_var("SOLILOQUY_JS_ENGINE", "v8-experimental");
        let (proxy, receiver) = ConstellationProxy::new();
        let mut evaluator = JavaScriptEvaluator::new(proxy);
        let result = Rc::new(RefCell::new(None));
        let callback_result = result.clone();

        evaluator.evaluate(
            servo_base::id::WebViewId(19),
            "window.__soliloquyEval('dom.inspect', 'document.body.innerHTML')".to_string(),
            Box::new(move |value| *callback_result.borrow_mut() = Some(value)),
        );

        assert!(matches!(*result.borrow(), Some(Ok(JSValue::Object(_)))));
        assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
        std::env::remove_var("SOLILOQUY_JS_ENGINE");
    }

    #[test]
    fn unsupported_scripts_fall_back_to_constellation() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_webview_snapshots();
        std::env::set_var("SOLILOQUY_JS_ENGINE", "v8-experimental");
        let (proxy, receiver) = ConstellationProxy::new();
        let mut evaluator = JavaScriptEvaluator::new(proxy);

        evaluator.evaluate(
            servo_base::id::WebViewId(9),
            "document.body".to_string(),
            Box::new(|_| {}),
        );

        assert!(matches!(
            receiver.try_recv(),
            Ok(EmbedderToConstellationMessage::EvaluateJavaScript(_, _, _))
        ));
        std::env::remove_var("SOLILOQUY_JS_ENGINE");
    }
}
