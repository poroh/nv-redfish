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

//! Runtime control plane, scheduling, execution, and output queue.

use crate::scheduler::flat_rr::FlatRoundRobin;
use crate::Generator;
use crate::GeneratorId;
use crate::RuntimeOutput;
use crate::ScheduledWork;
use crate::TargetId;
use crate::WorkCompletion;
use crate::WorkError;
use crate::WorkOutcome;
use crate::WorkSuccess;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::convert::Infallible;
use std::time::Instant;

/// Generic scraper runtime.
pub struct Runtime<'rt, Ev, Err, R = Infallible> {
    inner: RuntimeInner<'rt, Ev, Err, R>,
}

/// Empty target configuration for Phase 0.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TargetConfig {}

/// Error returned when adding a generator fails.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AddGeneratorError {
    /// The requested target does not exist.
    TargetNotFound {
        /// Missing target id.
        target_id: TargetId,
    },
}

/// Result of one runtime step.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunOnce {
    /// One work item was selected, awaited, output, and completed.
    Executed,
    /// No work item was available.
    Idle,
}

struct RuntimeInner<'rt, Ev, Err, R> {
    next_target_id: u64,
    targets: HashMap<TargetId, TargetState>,
    generators: HashMap<GeneratorId, GeneratorSlot<'rt, Ev, Err>>,
    scheduler: FlatRoundRobin,
    outputs: VecDeque<RuntimeOutput<Ev, Err, R>>,
}

struct TargetState {
    next_generator_id: u64,
    generators: Vec<GeneratorId>,
}

struct GeneratorSlot<'rt, Ev, Err> {
    generator: Box<dyn Generator<'rt, Ev, Err> + 'rt>,
}

impl<'rt, Ev, Err, R> Runtime<'rt, Ev, Err, R> {
    /// Creates an empty runtime.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: RuntimeInner {
                next_target_id: 1,
                targets: HashMap::new(),
                generators: HashMap::new(),
                scheduler: FlatRoundRobin::default(),
                outputs: VecDeque::new(),
            },
        }
    }

    /// Adds a target to the runtime.
    pub fn add_target(&mut self, _config: TargetConfig) -> TargetId {
        let target_id = TargetId::new(self.inner.next_target_id);
        self.inner.next_target_id += 1;
        self.inner.targets.insert(
            target_id,
            TargetState {
                next_generator_id: 1,
                generators: Vec::new(),
            },
        );
        target_id
    }

    /// Removes a target and all generators attached to it.
    pub fn remove_target(&mut self, target_id: TargetId) -> bool {
        if let Some(target) = self.inner.targets.remove(&target_id) {
            target.generators.into_iter().for_each(|generator_id| {
                self.inner.generators.remove(&generator_id);
                self.inner.scheduler.remove(generator_id);
            });
            true
        } else {
            false
        }
    }

    /// Adds a generator under an existing target.
    ///
    /// # Errors
    ///
    /// Returns [`AddGeneratorError::TargetNotFound`] when `target_id` is not in
    /// the runtime.
    pub fn add_generator<G>(
        &mut self,
        target_id: TargetId,
        generator: G,
    ) -> Result<GeneratorId, AddGeneratorError>
    where
        G: Generator<'rt, Ev, Err> + 'rt,
    {
        let target = self
            .inner
            .targets
            .get_mut(&target_id)
            .ok_or(AddGeneratorError::TargetNotFound { target_id })?;
        let generator_id = GeneratorId::new(target_id, target.next_generator_id);
        target.next_generator_id += 1;
        target.generators.push(generator_id);
        self.inner.generators.insert(
            generator_id,
            GeneratorSlot {
                generator: Box::new(generator),
            },
        );
        self.inner.scheduler.insert(generator_id);
        Ok(generator_id)
    }

    /// Removes a generator from the runtime.
    pub fn remove_generator(&mut self, generator_id: GeneratorId) -> bool {
        if self.inner.generators.remove(&generator_id).is_some() {
            if let Some(target) = self.inner.targets.get_mut(&generator_id.target_id()) {
                target.generators.retain(|id| *id != generator_id);
            }
            self.inner.scheduler.remove(generator_id);
            true
        } else {
            false
        }
    }

    /// Performs one complete runtime step.
    pub async fn run_once(&mut self) -> RunOnce {
        let mut scan = self.inner.scheduler.start_scan();
        while let Some(generator_id) = self.inner.scheduler.next_candidate(&mut scan) {
            let now = Instant::now();
            if !self.generator_ready(generator_id, now) {
                continue;
            }
            if let Some(work) = self.take_next(generator_id) {
                self.execute_work(generator_id, work).await;
                return RunOnce::Executed;
            }
        }
        RunOnce::Idle
    }

    /// Pops the oldest queued output.
    pub fn next_output(&mut self) -> Option<RuntimeOutput<Ev, Err, R>> {
        self.inner.outputs.pop_front()
    }

    /// Drains all queued outputs in FIFO order.
    pub fn drain_outputs(&mut self) -> Vec<RuntimeOutput<Ev, Err, R>> {
        self.inner.outputs.drain(..).collect()
    }

    fn generator_ready(&mut self, generator_id: GeneratorId, now: Instant) -> bool {
        self.inner
            .generators
            .get_mut(&generator_id)
            .is_some_and(|slot| slot.generator.update_ready(now).ready)
    }

    fn take_next(&mut self, generator_id: GeneratorId) -> Option<ScheduledWork<'rt, Ev, Err>> {
        self.inner
            .generators
            .get_mut(&generator_id)
            .and_then(|slot| slot.generator.take_next())
    }

    async fn execute_work(&mut self, generator_id: GeneratorId, work: ScheduledWork<'rt, Ev, Err>) {
        let outcome = match work.execute().await {
            Ok(events) => {
                self.inner
                    .outputs
                    .push_back(RuntimeOutput::Work(Ok(WorkSuccess {
                        generator_id,
                        events,
                    })));
                WorkOutcome::Succeeded
            }
            Err(error) => {
                self.inner
                    .outputs
                    .push_back(RuntimeOutput::Work(Err(WorkError {
                        generator_id,
                        error,
                    })));
                WorkOutcome::Failed
            }
        };
        self.complete_work(generator_id, outcome);
    }

    fn complete_work(&mut self, generator_id: GeneratorId, outcome: WorkOutcome) {
        if let Some(slot) = self.inner.generators.get_mut(&generator_id) {
            slot.generator.on_complete(&WorkCompletion {
                generator_id,
                outcome,
            });
        }
    }
}

impl<Ev, Err, R> Default for Runtime<'_, Ev, Err, R> {
    fn default() -> Self {
        Self::new()
    }
}
