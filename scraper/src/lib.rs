// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Generic runtime for scraper workflows.
//!
//! The crate owns target and generator control, flat round-robin scheduling,
//! single-work execution steps, completion callbacks, and ordered output queues.
//! It intentionally contains no Redfish, BMC, transport, or application policy.

#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::suspicious,
    clippy::complexity,
    clippy::perf
)]
#![deny(
    clippy::absolute_paths,
    clippy::todo,
    clippy::unimplemented,
    clippy::tests_outside_test_module,
    clippy::panic,
    clippy::unwrap_used,
    clippy::unwrap_in_result,
    clippy::unused_trait_names,
    clippy::print_stdout,
    clippy::print_stderr
)]
#![deny(missing_docs)]
#![allow(clippy::doc_markdown)]

mod generator;
mod ids;
mod output;
mod runtime;
mod scheduler;

#[doc(inline)]
pub use generator::Generator;
#[doc(inline)]
pub use generator::Readiness;
#[doc(inline)]
pub use generator::ScheduledWork;
#[doc(inline)]
pub use generator::WorkCompletion;
#[doc(inline)]
pub use generator::WorkOutcome;
#[doc(inline)]
pub use ids::GeneratorId;
#[doc(inline)]
pub use ids::TargetId;
#[cfg(feature = "runtime-events")]
#[doc(inline)]
pub use output::RuntimeEvent;
#[doc(inline)]
#[doc(inline)]
pub use output::RuntimeOutput;
#[doc(inline)]
pub use output::WorkError;
#[doc(inline)]
pub use output::WorkResult;
#[doc(inline)]
pub use output::WorkSuccess;
#[doc(inline)]
pub use runtime::ControlError;
#[doc(inline)]
pub use runtime::Runtime;
#[doc(inline)]
pub use runtime::RuntimeHandle;
#[doc(inline)]
pub use runtime::TargetConfig;
