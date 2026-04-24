/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use embedder_traits::{JSValue, JavaScriptEvaluationError, JavaScriptEvaluationId};
use log::info;
use rustc_hash::FxHashMap;
use servo_base::id::WebViewId;
use servo_constellation_traits::EmbedderToConstellationMessage;

use crate::proxies::ConstellationProxy;
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
        if let Some(result) = SoliloquyJavascriptDispatcher::maybe_evaluate(&script) {
            info!(
                "Soliloquy experimental dispatcher handled JavaScript evaluation locally for webview {:?}",
                webview_id
            );
            callback(result);
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
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use crossbeam_channel::TryRecvError;
    use embedder_traits::JSValue;
    use servo_constellation_traits::EmbedderToConstellationMessage;

    use super::JavaScriptEvaluator;
    use crate::proxies::ConstellationProxy;

    #[test]
    fn experimental_dispatcher_short_circuits_simple_scripts() {
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
    fn unsupported_scripts_fall_back_to_constellation() {
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
