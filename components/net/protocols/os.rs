/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! `os://` protocol handler for Soliloquy.
//!
//! Maps `os://<page>[/path]` → `http://127.0.0.1:8080/<page>[/path]` (sold local service).
//!
//! # Supported pages
//! - `os://terminal` — libghostty WASM terminal
//! - `os://files`    — local file browser
//! - `os://settings` — system settings UI
//! - `os://about`    — version/diagnostic page
//!
//! The handler rewrites the URL and delegates to Servo's standard HTTP fetch,
//! so all normal Servo resource-loading logic (caching, CSP, etc.) applies to
//! the rewritten `http://` URL.

use std::future;
use std::future::Future;
use std::pin::Pin;

use net_traits::blob_url_store::UrlWithBlobClaim;
use net_traits::request::Request;
use net_traits::response::Response;
use net_traits::{DiscardFetch, NetworkError};
use servo_url::ServoUrl;

use crate::fetch::methods::{DoneChannel, FetchContext, fetch};
use crate::protocols::ProtocolHandler;

/// Base URL for the sold local service.
const SOLD_BASE: &str = "http://127.0.0.1:8080";

/// `os://` protocol handler — proxies to `sold` running on localhost.
#[derive(Default)]
pub struct OsProtocolHandler;

impl ProtocolHandler for OsProtocolHandler {
    /// `os://` resources are fetchable from JS (same-origin-equivalent to sold).
    fn is_fetchable(&self) -> bool {
        true
    }

    /// Treat `os://` as a secure context so mixed-content checks against
    /// the local sold endpoint are suppressed.
    fn is_secure(&self) -> bool {
        true
    }

    fn load<'a>(
        &'a self,
        request: &'a mut Request,
        _done_chan: &mut DoneChannel,
        context: &FetchContext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>> {
        let url = request.current_url();

        // `os://terminal/path/to/thing`
        //        ^^^^^^^^ host  ^^^^^^^^^^^^^^ path
        // → `http://127.0.0.1:8080/terminal/path/to/thing`
        let host = url.host_str().unwrap_or("terminal");
        let path = url.path();
        // `url.path()` is always "/" for bare `os://terminal` (no trailing component).
        // Avoid emitting `http://127.0.0.1:8080/terminal/` when we want `.../terminal`.
        let rewritten = if path == "/" {
            format!("{}/{}", SOLD_BASE, host)
        } else {
            format!("{}/{}{}", SOLD_BASE, host, path)
        };

        let result_url = match ServoUrl::parse(&rewritten) {
            Ok(u) => u,
            Err(_) => {
                return Box::pin(future::ready(Response::network_error(
                    NetworkError::ResourceLoadError(format!(
                        "os:// URL rewrite failed: {rewritten}"
                    )),
                )));
            },
        };

        request
            .url_list
            .push(UrlWithBlobClaim::new(result_url, None));
        let request2 = request.clone();
        let context2 = context.clone();
        Box::pin(async move { fetch(request2, &mut DiscardFetch, &context2).await })
    }
}
