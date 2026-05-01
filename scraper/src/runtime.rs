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
#[cfg(feature = "runtime-events")]
use crate::RuntimeEvent;
use crate::RuntimeOutput;
use crate::ScheduledWork;
use crate::TargetId;
use crate::WorkCompletion;
use crate::WorkError;
use crate::WorkOutcome;
use crate::WorkSuccess;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::future::poll_fn;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::PoisonError;
use std::task::Poll;
use std::task::Waker;
use std::time::Instant;

/// Generic scraper runtime consumer and driver.
pub struct Runtime<'rt, Ev, Err> {
    shared: Arc<RuntimeShared<'rt, Ev, Err>>,
}

/// Cloneable synchronous runtime control handle.
pub struct RuntimeHandle<'rt, Ev, Err> {
    shared: Arc<RuntimeShared<'rt, Ev, Err>>,
}

/// Empty target configuration.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TargetConfig {}

/// Error returned by runtime control operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlError {
    /// Graceful shutdown has started, so mutations are rejected.
    RuntimeShutdown,
    /// The requested target does not exist or is removing.
    TargetNotFound {
        /// Missing or unavailable target id.
        target_id: TargetId,
    },
}

struct RuntimeShared<'rt, Ev, Err> {
    state: Mutex<RuntimeState<'rt, Ev, Err>>,
}

struct RuntimeState<'rt, Ev, Err> {
    next_target_id: u64,
    targets: HashMap<TargetId, TargetState>,
    generators: HashMap<GeneratorId, GeneratorSlot<'rt, Ev, Err>>,
    scheduler: FlatRoundRobin,
    outputs: VecDeque<RuntimeOutput<Ev, Err>>,
    shutting_down: bool,
    shutdown_returned: bool,
    wake: WakeState,
}

#[derive(Default)]
struct WakeState {
    generation: u64,
    waiter: Option<Waker>,
}

struct TargetState {
    next_generator_id: u64,
    generators: Vec<GeneratorId>,
    removing: bool,
}

struct GeneratorSlot<'rt, Ev, Err> {
    generator: Box<dyn Generator<'rt, Ev, Err> + 'rt>,
    in_flight: usize,
    removing: bool,
}

struct SelectedWork<'rt, Ev, Err> {
    generator_id: GeneratorId,
    work: ScheduledWork<'rt, Ev, Err>,
}

impl<'rt, Ev, Err> Runtime<'rt, Ev, Err> {
    /// Creates an empty runtime and its cloneable control handle.
    #[must_use]
    pub fn new() -> (Self, RuntimeHandle<'rt, Ev, Err>) {
        let shared = Arc::new(RuntimeShared {
            state: Mutex::new(RuntimeState {
                next_target_id: 1,
                targets: HashMap::new(),
                generators: HashMap::new(),
                scheduler: FlatRoundRobin::default(),
                outputs: VecDeque::new(),
                shutting_down: false,
                shutdown_returned: false,
                wake: WakeState::default(),
            }),
        });
        (
            Self {
                shared: Arc::clone(&shared),
            },
            RuntimeHandle { shared },
        )
    }

    /// Returns the next ordered runtime output, driving at most one work item.
    pub async fn next(&mut self) -> RuntimeOutput<Ev, Err> {
        loop {
            match self.next_action() {
                NextAction::Output(output) => return output,
                NextAction::Work(selected) => self.execute_selected_work(selected).await,
                NextAction::Wait(observed) => self.wait_for_wake(observed).await,
            }
        }
    }

    fn next_action(&self) -> NextAction<'rt, Ev, Err> {
        let mut state = self.shared.state.lock().expect("runtime state lock");
        if let Some(output) = state.outputs.pop_front() {
            return NextAction::Output(output);
        }
        if state.shutdown_returned {
            return NextAction::Output(RuntimeOutput::Shutdown);
        }
        if state.shutdown_complete() {
            state.shutdown_returned = true;
            return NextAction::Output(RuntimeOutput::Shutdown);
        }
        if let Some(selected) = state.select_work() {
            return NextAction::Work(selected);
        }
        NextAction::Wait(state.wake.observe())
    }

    async fn execute_selected_work(&self, selected: SelectedWork<'rt, Ev, Err>) {
        let generator_id = selected.generator_id;
        let result = selected.work.execute().await;
        let waker = {
            let mut state = self.shared.state.lock().expect("runtime state lock");
            state.complete_work(generator_id, result)
        };
        wake_after_unlock(waker);
    }

    async fn wait_for_wake(&self, observed: u64) {
        poll_fn(|context| {
            let mut state = self.shared.state.lock().expect("runtime state lock");
            if state.can_progress(observed) {
                Poll::Ready(())
            } else {
                state.wake.register(context.waker());
                Poll::Pending
            }
        })
        .await;
    }
}

enum NextAction<'rt, Ev, Err> {
    Output(RuntimeOutput<Ev, Err>),
    Work(SelectedWork<'rt, Ev, Err>),
    Wait(u64),
}

impl<Ev, Err> Clone for RuntimeHandle<'_, Ev, Err> {
    fn clone(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl<'rt, Ev, Err> RuntimeHandle<'rt, Ev, Err> {
    /// Adds a target to the runtime.
    ///
    /// # Errors
    ///
    /// Returns [`ControlError::RuntimeShutdown`] after graceful shutdown starts.
    pub fn add_target(&self, config: TargetConfig) -> Result<TargetId, ControlError> {
        let (result, waker) = {
            let mut state = self
                .shared
                .state
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            state.add_target(config)
        };
        wake_after_unlock(waker);
        result
    }

    /// Requests removal of a target and its generators.
    ///
    /// # Errors
    ///
    /// Returns [`ControlError::RuntimeShutdown`] after graceful shutdown starts.
    pub fn remove_target(&self, target_id: TargetId) -> Result<bool, ControlError> {
        let (result, waker) = {
            let mut state = self
                .shared
                .state
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            state.remove_target(target_id)
        };
        wake_after_unlock(waker);
        result
    }

    /// Adds a generator under an active target.
    ///
    /// # Errors
    ///
    /// Returns [`ControlError::RuntimeShutdown`] after graceful shutdown starts, or
    /// [`ControlError::TargetNotFound`] when `target_id` is missing or removing.
    pub fn add_generator<G>(
        &self,
        target_id: TargetId,
        generator: G,
    ) -> Result<GeneratorId, ControlError>
    where
        G: Generator<'rt, Ev, Err> + 'rt,
    {
        let (result, waker) = {
            let mut state = self
                .shared
                .state
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            state.add_generator(target_id, generator)
        };
        wake_after_unlock(waker);
        result
    }

    /// Requests removal of one generator.
    ///
    /// # Errors
    ///
    /// Returns [`ControlError::RuntimeShutdown`] after graceful shutdown starts.
    pub fn remove_generator(&self, generator_id: GeneratorId) -> Result<bool, ControlError> {
        let (result, waker) = {
            let mut state = self
                .shared
                .state
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            state.remove_generator(generator_id)
        };
        wake_after_unlock(waker);
        result
    }

    /// Starts graceful shutdown, removing all targets after in-flight work drains.
    pub fn graceful_shutdown(&self) {
        let waker = {
            let mut state = self
                .shared
                .state
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            state.graceful_shutdown()
        };
        wake_after_unlock(waker);
    }
}

impl WakeState {
    const fn observe(&self) -> u64 {
        self.generation
    }

    fn register(&mut self, waker: &Waker) {
        if self
            .waiter
            .as_ref()
            .is_some_and(|stored| stored.will_wake(waker))
        {
            return;
        }
        self.waiter = Some(waker.clone());
    }

    const fn mark_changed(&mut self) -> Option<Waker> {
        self.generation = self.generation.wrapping_add(1);
        self.waiter.take()
    }
}

impl<'rt, Ev, Err> RuntimeState<'rt, Ev, Err> {
    fn add_target(
        &mut self,
        _config: TargetConfig,
    ) -> (Result<TargetId, ControlError>, Option<Waker>) {
        if self.shutting_down {
            return (Err(ControlError::RuntimeShutdown), None);
        }
        let target_id = TargetId::new(self.next_target_id);
        self.next_target_id += 1;
        self.targets.insert(
            target_id,
            TargetState {
                next_generator_id: 1,
                generators: Vec::new(),
                removing: false,
            },
        );
        #[cfg(feature = "runtime-events")]
        self.enqueue_runtime_event(RuntimeEventKind::TargetAdded { target_id });
        (Ok(target_id), self.wake.mark_changed())
    }

    fn remove_target(
        &mut self,
        target_id: TargetId,
    ) -> (Result<bool, ControlError>, Option<Waker>) {
        if self.shutting_down {
            return (Err(ControlError::RuntimeShutdown), None);
        }
        if !self.mark_target_removing(target_id) {
            return (Ok(false), None);
        }
        self.finalize_target_removals(target_id);
        (Ok(true), self.wake.mark_changed())
    }

    fn add_generator<G>(
        &mut self,
        target_id: TargetId,
        generator: G,
    ) -> (Result<GeneratorId, ControlError>, Option<Waker>)
    where
        G: Generator<'rt, Ev, Err> + 'rt,
    {
        if self.shutting_down {
            return (Err(ControlError::RuntimeShutdown), None);
        }
        let Some(target) = self.targets.get_mut(&target_id) else {
            return (Err(ControlError::TargetNotFound { target_id }), None);
        };
        if target.removing {
            return (Err(ControlError::TargetNotFound { target_id }), None);
        }
        let generator_id = GeneratorId::new(target_id, target.next_generator_id);
        target.next_generator_id += 1;
        target.generators.push(generator_id);
        self.generators.insert(
            generator_id,
            GeneratorSlot {
                generator: Box::new(generator),
                in_flight: 0,
                removing: false,
            },
        );
        self.scheduler.insert(generator_id);
        #[cfg(feature = "runtime-events")]
        self.enqueue_runtime_event(RuntimeEventKind::GeneratorAdded { generator_id });
        (Ok(generator_id), self.wake.mark_changed())
    }

    fn remove_generator(
        &mut self,
        generator_id: GeneratorId,
    ) -> (Result<bool, ControlError>, Option<Waker>) {
        if self.shutting_down {
            return (Err(ControlError::RuntimeShutdown), None);
        }
        if !self.mark_generator_removing(generator_id) {
            return (Ok(false), None);
        }
        self.finalize_generator_after_remove(generator_id);
        (Ok(true), self.wake.mark_changed())
    }

    fn graceful_shutdown(&mut self) -> Option<Waker> {
        if self.shutting_down {
            return None;
        }
        self.shutting_down = true;
        let target_ids = self.target_ids_in_order();
        for target_id in target_ids {
            let _marked = self.mark_target_removing(target_id);
            self.finalize_target_removals(target_id);
        }
        self.wake.mark_changed()
    }

    fn select_work(&mut self) -> Option<SelectedWork<'rt, Ev, Err>> {
        if self.shutting_down {
            return None;
        }
        let mut scan = self.scheduler.start_scan();
        while let Some(generator_id) = self.scheduler.next_candidate(&mut scan) {
            let Some(slot) = self.generators.get_mut(&generator_id) else {
                continue;
            };
            if slot.removing || !slot.generator.update_ready(Instant::now()).ready {
                continue;
            }
            if let Some(work) = slot.generator.take_next() {
                slot.in_flight += 1;
                return Some(SelectedWork { generator_id, work });
            }
        }
        None
    }

    fn complete_work(
        &mut self,
        generator_id: GeneratorId,
        result: Result<Vec<Ev>, Err>,
    ) -> Option<Waker> {
        let outcome = self.enqueue_work_output(generator_id, result);
        if let Some(slot) = self.generators.get_mut(&generator_id) {
            slot.generator.on_complete(&WorkCompletion {
                generator_id,
                outcome,
            });
            slot.in_flight = slot.in_flight.saturating_sub(1);
        }
        self.finalize_generator_after_remove(generator_id);
        self.wake.mark_changed()
    }

    fn enqueue_work_output(
        &mut self,
        generator_id: GeneratorId,
        result: Result<Vec<Ev>, Err>,
    ) -> WorkOutcome {
        match result {
            Ok(events) => {
                self.outputs.push_back(RuntimeOutput::Work(Ok(WorkSuccess {
                    generator_id,
                    events,
                })));
                WorkOutcome::Succeeded
            }
            Err(error) => {
                self.outputs.push_back(RuntimeOutput::Work(Err(WorkError {
                    generator_id,
                    error,
                })));
                WorkOutcome::Failed
            }
        }
    }

    fn can_progress(&self, observed: u64) -> bool {
        !self.outputs.is_empty()
            || self.shutdown_returned
            || self.shutdown_complete()
            || self.wake.observe() != observed
    }

    fn shutdown_complete(&self) -> bool {
        self.shutting_down && self.targets.is_empty() && self.generators.is_empty()
    }

    fn mark_target_removing(&mut self, target_id: TargetId) -> bool {
        let Some(target) = self.targets.get_mut(&target_id) else {
            return false;
        };
        if target.removing {
            return false;
        }
        target.removing = true;
        let generator_ids = target.generators.clone();
        for generator_id in generator_ids {
            let _marked = self.mark_generator_removing(generator_id);
        }
        true
    }

    fn mark_generator_removing(&mut self, generator_id: GeneratorId) -> bool {
        let Some(slot) = self.generators.get_mut(&generator_id) else {
            return false;
        };
        if slot.removing {
            return false;
        }
        slot.removing = true;
        self.scheduler.remove(generator_id);
        true
    }

    fn finalize_generator_after_remove(&mut self, generator_id: GeneratorId) {
        let Some(slot) = self.generators.get(&generator_id) else {
            return;
        };
        if !slot.removing || slot.in_flight != 0 {
            return;
        }
        if self
            .targets
            .get(&generator_id.target_id())
            .is_some_and(|target| target.removing)
        {
            self.finalize_target_removals(generator_id.target_id());
        } else {
            self.finalize_one_generator(generator_id);
        }
    }

    fn finalize_target_removals(&mut self, target_id: TargetId) {
        loop {
            let Some(generator_id) = self
                .targets
                .get(&target_id)
                .and_then(|target| target.generators.first().copied())
            else {
                break;
            };
            let Some(slot) = self.generators.get(&generator_id) else {
                self.remove_generator_from_target(generator_id);
                continue;
            };
            if !slot.removing || slot.in_flight != 0 {
                break;
            }
            self.finalize_one_generator(generator_id);
        }
        if self
            .targets
            .get(&target_id)
            .is_some_and(|target| target.removing && target.generators.is_empty())
        {
            self.targets.remove(&target_id);
            #[cfg(feature = "runtime-events")]
            self.enqueue_runtime_event(RuntimeEventKind::TargetRemoved { target_id });
        }
    }

    fn finalize_one_generator(&mut self, generator_id: GeneratorId) {
        self.scheduler.remove(generator_id);
        self.remove_generator_from_target(generator_id);
        self.generators.remove(&generator_id);
        #[cfg(feature = "runtime-events")]
        self.enqueue_runtime_event(RuntimeEventKind::GeneratorRemoved { generator_id });
    }

    fn remove_generator_from_target(&mut self, generator_id: GeneratorId) {
        if let Some(target) = self.targets.get_mut(&generator_id.target_id()) {
            target.generators.retain(|id| *id != generator_id);
        }
    }

    fn target_ids_in_order(&self) -> Vec<TargetId> {
        let mut target_ids = self.targets.keys().copied().collect::<Vec<_>>();
        target_ids.sort_by_key(|target_id| target_id.raw());
        target_ids
    }

    #[cfg(feature = "runtime-events")]
    fn enqueue_runtime_event(&mut self, event: RuntimeEventKind) {
        let output = match event {
            RuntimeEventKind::TargetAdded { target_id } => {
                RuntimeOutput::Runtime(RuntimeEvent::TargetAdded { target_id })
            }
            RuntimeEventKind::GeneratorAdded { generator_id } => {
                RuntimeOutput::Runtime(RuntimeEvent::GeneratorAdded { generator_id })
            }
            RuntimeEventKind::GeneratorRemoved { generator_id } => {
                RuntimeOutput::Runtime(RuntimeEvent::GeneratorRemoved { generator_id })
            }
            RuntimeEventKind::TargetRemoved { target_id } => {
                RuntimeOutput::Runtime(RuntimeEvent::TargetRemoved { target_id })
            }
        };
        self.outputs.push_back(output);
    }
}

#[cfg(feature = "runtime-events")]
#[derive(Clone, Copy)]
pub enum RuntimeEventKind {
    TargetAdded { target_id: TargetId },
    GeneratorAdded { generator_id: GeneratorId },
    GeneratorRemoved { generator_id: GeneratorId },
    TargetRemoved { target_id: TargetId },
}

fn wake_after_unlock(waker: Option<Waker>) {
    if let Some(waker) = waker {
        waker.wake();
    }
}
