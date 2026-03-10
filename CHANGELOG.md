# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- **Breaking:** Bumped the `reqwest` dependency to `0.13`.

- Added the `rewrite_uri` module with a `RewriteUri` trait and a
  `RewriteUriLayer` / `RewriteUriService` middleware pair for composable
  client-side URI rewriting (e.g. load balancing, environment switching).

- Added a `query` method to `ClientRequestBuilder` and `RequestBuilderExt` for
  setting URL query parameters from any `serde::Serialize` type. The method
  replaces any existing query string entirely. Enabled by default via the
  `query` feature flag; it can be disabled with `default-features = false`.

- **Breaking:** Unified body types across `ClientRequestBuilder` methods.
  `form()` now returns `Bytes` instead of `String`, and `send()` on the builder
  requires `From<Bytes>` instead of `Default`. All standard body-producing
  methods (`send`, `without_body`, `json`, `form`) now consistently produce
  `Bytes`, requiring only a single `From<Bytes>` bound on the service request
  body type.

- **Breaking:** Made the `url` and `serde` dependencies optional. Some
  functionality is now behind the `json` / `form` (enabling `serde`) and `url`
  feature flags.

- Added the `MakeHeaderValue` trait bound to the `SetRequestHeaderLayer` generic
  parameter in the `tower-request` crate so the resulting type implements
  `Service`.

- **Breaking:** Improved integration with `reqwest`: request body conversion now
  uses `reqwest::Body::wrap` instead of a custom `into_reqwest_body` method.
  `ClientRequestBuilder::without_body` now returns `Bytes` to improve
  compatibility with other HTTP clients.

- **Breaking:** Renamed the `build` method to `without_body` on `ClientRequest`
  to better reflect its purpose and improve API clarity.

- **Breaking:** Removed the direct `reqwest` dependency from `tower-http-client`
  to avoid pinning a specific `reqwest` version. The crate is now fully generic
  and not tied to any particular HTTP client implementation. Users who need
  `reqwest`-specific features should use `tower-reqwest` directly.

- **Breaking:** Updated the request adapter (`HttpClientService` /
  `ExecuteRequestFuture`) to propagate `S::Error` directly (now constrained to
  `request::Error`) instead of wrapping it in a crate-specific error type.

- Refactored `ExecuteRequestFuture` in `tower-reqwest` to remove unnecessary
  double-pinning and simplify the internal structure.

## [0.5.3] - 2025-09-23

- Improved `Debug` implementations for `ClientRequest` and
  `ClientRequestBuilder`.

- Added `From<ClientRequest>` for `http::Request` and
  `From<ClientRequestBuilder>` for `http::request::Builder` to help with
  debugging and testing.

- Bumped the minimum supported Rust version to `1.88.0`.

## [0.5.2] - 2025-04-04

- Added a `typed_header` method to `ClientRequest` and `RequestBuilderExt` for
  inserting typed headers.

- Made the `RequestBuilderExt` trait sealed.

## [0.5.1] - 2025-04-01

- **Breaking:** Replaced the `body_reader` implementation with the
  `http_body_reader` crate.

- Added a `form` method to the `BodyReader` to decode form data.

- **Breaking:** Split the old `ClientRequest` into separate builder and request
  structs with updated error handling.

- Introduced the `RequestBuilderExt` trait to extend `http::request::Builder`
  with additional methods to send form data and JSON objects.

- Bumped the minimum supported Rust version to `1.81.0`.

- **Breaking:** Made the `request_builder` module private.

- Added a `form` method to `ClientRequest` to send form data.

- **Breaking:** Removed the crate's own implementation of `BoxCloneSyncService`;
  please use `tower::util::BoxCloneSyncService` instead.

- Added an `auth` module to the `tower-reqwest` crate to add authorization
  headers to requests.

- Added a `set-header` module to the `tower-reqwest` crate to modify request
  headers.

## [0.4.1] - 2024-12-04

- Fixed typos in the documentation.
- Bumped the minimum supported Rust version to `1.78.0`.

## [0.4.0] - 2024-10-14

- **Breaking:** Extensions and utilities for Tower services that provide HTTP
  client implementations were moved to the `client` module.

- **Breaking:** `ClientRequest` and `ServiceBuilderExt` methods now accept the
  `IntoUri` trait instead of relying on `Uri: TryFrom` to improve
  interoperability with the `url` crate.

- Added `#[from]` and `#[source]` to `Error` and `ClientError` to expose
  underlying source errors.

- Added a `BoxCloneSyncService`, borrowed from this
  [PR](https://github.com/tower-rs/tower/pull/777).

- **Breaking:** Renamed the `request` module to `request_builder`.

- **Breaking:** Removed the `reqwest-middleware` feature from the
  `tower-http-client` and `tower-http` crates.

- Added a [retry](tower-http-client/examples/retry.rs) example.

- Added a [rate limiter](tower-http-client/examples/rate_limiter.rs) example.

- **Breaking:** Changed the `ServiceBuilder::execute` signature to be more
  compatible with the `Service::call` method.

## [0.3.2] - 2024-05-05

- Added more information about the crates.
- Set the minimum supported Rust version to `1.75.0`.

## [0.3.1] - 2024-05-03

- Added `reqwest` and `reqwest-middleware` features to the `tower-http-client`
  crate.

## [0.3.0] - 2024-04-30

- Added a `ResponseExt` extension trait.

- Added a `json` feature to enable reading and writing JSON bodies in requests
  and responses.

- Added a `request` module with utilities such as `ClientRequest` for
  constructing HTTP requests.

- Removed the separate `util` feature; the functionality is now always
  available.

- Added a `body_reader` module in `tower-http-client` to simplify reading
  response bodies in common cases.

- Replaced `tower_http_client::util::HttpClientExt` with
  `tower_http_client::ServiceExt`.

## [0.2.0] - 2024-04-21

- The `tower-reqwest` project was split into two parts: `tower-reqwest` itself,
  which contains adapters for `tower-http`, and `tower-http-client`, which
  contains utilities and extensions for creating clients.

[`tower-http-client`]: tower-http-client
[`tower-reqwest`]: tower-reqwest
