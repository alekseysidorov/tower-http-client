# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- **breaking:** `ClientRequest` has been renamed the to the
  `ClientRequestBuilder`, added a new `ClientRequest` struct to send built
  requests. Both structs are strongly redesigned in order to improve the
  usability and composeability.

- Added a `request_ext` module with the `RequestBuilderExt` trait to extend the
  `http::request::Builder` with additional methods.

- Bump minimum supported Rust version to `1.81`.

- **breaking:** Made `request_builder` module private.

- Added a `form` method to the `ClientRequest` to send a form data.

- **breaking:** Removed the own implementation of the `BoxCloneSyncService`,
  please use the `tower::util::BoxCloneSyncService` instead.

- Added an `auth` module to the `tower-reqwest` crate, which provides a way to
  add an authorization header to the requests.

- Added a `set-header` module to the `tower-reqwest` crate, which provides a way
  to modify the request headers.

## [0.4.1] - 2024.12.04

- Fix typos in the documentation.

- Bump minimum supported Rust version to `1.78`.

## [0.4.0] - 2024.10.14

- **breaking:** Extensions and utilities for Tower services that provides HTTP
  client implementations have been moved to the `client` module.

- **breaking:** `ClientRequest` and `ServiceBuilderExt` methods now use the
  `IntoUri` trait instead of `Uri: TryFrom` conversion in order to improve
  interopability with the `url` crate.

- Added `#[from]` and `#[source]` to `Error` and `ClientError` to expose the
  underlying source error.

- Added a `BoxCloneSyncService`, borrowed from this
  [PR](https://github.com/tower-rs/tower/pull/777).

- **breaking:** `request` module has been renamed to the `request_builder`.

- **breaking:** Removed `reqwest-middleware` feature from the
  `tower-http-client` and `tower-http` crates.

- Added a [retry](tower-http-client/examples/retry.rs) example.

- Added a [rate-limiter](tower-http-client/examples/rate_limiter.rs) example.

- **breaking:** Changed `ServiceBuilder::execute` signature to be more
  compatible with the `Service::call` method.

## [0.3.2] - 2024.05.05

- Added more information about crates.

- The minimum supported Rust version is set to 1.75.

## [0.3.1] - 2024.05.03

- Added a `reqwest` and `reqwest-middleware` features to the `tower-http-client`
  crate.

## [0.3.0] - 2024.04.30

- Added an `ResponseExt` extension trait.

- Added a `json` feature to enable reading and writing JSON bodies in requests
  and responses.

- Added a `request` module with the useful utilities like `ClientRequest` for
  constructing HTTP requests.

- A separate feature `util` has been removed, now this functionality is always
  available.

- Added a new module `body_reader` in the [`tower-http-client`] to simplify the
  reading the response body in the most common cases.

- `tower_http_client::util::HttpClientExt` has been replaced by the
  `tower_http_client::ServiceExt`.

## [0.2.0] - 2024.04.21

The `tower-reqwest` has been split into two parts: [`tower-reqwest`] itself with
adapters for `tower-http` and [`tower-http-client`] with the useful utils and
extensions for creating an clients.

[`tower-http-client`]: tower-reqwest
[`tower-reqwest`]: tower-http-client
