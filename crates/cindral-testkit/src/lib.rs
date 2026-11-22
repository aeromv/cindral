// SPDX-License-Identifier: MIT OR Apache-2.0

//! # cindral-testkit
//!
//! Testing support for Cindral:
//!
//! - [`fake_server`] — a scriptable loopback 1.8 server (trustworthy local
//!   answer to spec M1's "local Paper" acceptance when no real server is
//!   available in CI).
//! - [`fake_online`] / [`fake_session`] — the encrypting login mode of the
//!   fake server and a recording session server (NET-4 tests).
//! - [`capture`] — an in-scoped packet capture format with file round-trip,
//!   recording either complete wire frames or decoded logical packets, plus
//!   offline frame description ( [`capture::describe_frame`] ).
//! - [`replay`] — streaming a recorded direction of a capture back over
//!   TCP, optionally pacing by the recorded gaps (spec M1 capture/replay).
//! - `cindral-packet-logger` — a MITM TCP proxy that records raw frames of
//!   a real client/server session to a `.umcapt` file for clean-room
//!   reference and later replay work.
//! - `cindral-capture-inspect` / `cindral-capture-replay` — CLI wrappers
//!   over the two modules above.
//! - [`screenshot`] — PNG screenshot comparison against a vanilla reference
//!   (`cindral-screenshot-diff`); references stay outside the repository.
//! - [`trace`] — serverbound play packets of a capture, grouped by client
//!   tick (parity harness, ROADMAP.md P0).
//! - [`tick_diff`] — per-tick comparison of a vanilla and a Cindral trace
//!   (`cindral-trace-diff`).
//! - [`state_log`] — internal player-state logs (vanilla agent and Cindral)
//!   and their phase-aligned, strict-by-default comparison (phase E.2).
//! - [`trajectory`] — per-tick reported positions and their comparison
//!   (`cindral-trajectory-diff`, `cindral-movement-probe --trajectory`).

pub mod capture;
pub mod fake_online;
pub mod fake_server;
pub mod fake_session;
pub mod replay;
pub mod screenshot;
pub mod state_log;
pub mod tick_diff;
pub mod trace;
pub mod trajectory;
