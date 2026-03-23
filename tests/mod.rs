//! Integration tests for the server router and middleware.
//!
//! These tests cover:
//! - public health endpoints
//! - route wiring and HTTP method guards
//! - auth middleware behavior
//! - Abuja boundary validation through `/stations/closest`
//! - `/stations/closest` rate limiting

pub mod common;
