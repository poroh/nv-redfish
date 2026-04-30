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

//! Generator and scheduled work types.

use crate::GeneratorId;
use std::future::Future;
use std::pin::Pin;
use std::time::Instant;

/// Readiness reported by a generator during a runtime scan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Readiness {
    /// Whether the generator can produce work now.
    pub ready: bool,
    /// Optional future time when the generator expects to become ready.
    pub next_ready_at: Option<Instant>,
}

/// Application-provided source of scheduled work.
pub trait Generator<'rt, Ev, Err>: Send {
    /// Updates and reports whether the generator is ready at `now`.
    fn update_ready(&mut self, now: Instant) -> Readiness;

    /// Creates the next executable work item when this generator is selected.
    fn take_next(&mut self) -> Option<ScheduledWork<'rt, Ev, Err>>;

    /// Reports completion of work produced by this generator.
    fn on_complete(&mut self, completion: &WorkCompletion);
}

/// One async work item selected by the runtime.
pub struct ScheduledWork<'rt, Ev, Err> {
    future: Pin<Box<dyn Future<Output = Result<Vec<Ev>, Err>> + Send + 'rt>>,
}

impl<'rt, Ev, Err> ScheduledWork<'rt, Ev, Err> {
    /// Wraps a fallible future as scheduled work.
    pub fn new<F>(future: F) -> Self
    where
        F: Future<Output = Result<Vec<Ev>, Err>> + Send + 'rt,
    {
        Self {
            future: Box::pin(future),
        }
    }

    pub(crate) async fn execute(self) -> Result<Vec<Ev>, Err> {
        self.future.await
    }
}

/// Completion details reported to a generator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorkCompletion {
    /// The generator whose work completed.
    pub generator_id: GeneratorId,
    /// Whether the work succeeded or failed.
    pub outcome: WorkOutcome,
}

/// Outcome of one completed work item.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkOutcome {
    /// The work future resolved successfully.
    Succeeded,
    /// The work future resolved with an error.
    Failed,
}
