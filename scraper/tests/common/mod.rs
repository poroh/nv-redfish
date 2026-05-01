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

#![allow(dead_code)]

use nv_redfish_scraper::Generator;
use nv_redfish_scraper::Readiness;
use nv_redfish_scraper::Runtime;
use nv_redfish_scraper::RuntimeOutput;
use nv_redfish_scraper::ScheduledWork;
use nv_redfish_scraper::WorkCompletion;
use nv_redfish_scraper::WorkOutcome;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;

#[derive(Default)]
pub struct Probe {
    pub readiness_checks: usize,
    pub take_next_calls: usize,
    pub completions: Vec<WorkOutcome>,
}

pub type SharedProbe = Arc<Mutex<Probe>>;

pub fn probe() -> SharedProbe {
    Arc::new(Mutex::new(Probe::default()))
}

pub struct QueueGenerator<Ev, Err> {
    ready: bool,
    jobs: VecDeque<Result<Vec<Ev>, Err>>,
    probe: SharedProbe,
}

impl<Ev, Err> QueueGenerator<Ev, Err> {
    pub fn new(
        ready: bool,
        jobs: impl IntoIterator<Item = Result<Vec<Ev>, Err>>,
    ) -> (Self, SharedProbe) {
        let probe = probe();
        (
            Self {
                ready,
                jobs: jobs.into_iter().collect(),
                probe: Arc::clone(&probe),
            },
            probe,
        )
    }
}

impl<'rt, Ev, Err> Generator<'rt, Ev, Err> for QueueGenerator<Ev, Err>
where
    Ev: Send + 'rt,
    Err: Send + 'rt,
{
    fn update_ready(&mut self, _now: Instant) -> Readiness {
        self.probe.lock().expect("probe lock").readiness_checks += 1;
        Readiness {
            ready: self.ready,
            next_ready_at: None,
        }
    }

    fn take_next(&mut self) -> Option<ScheduledWork<'rt, Ev, Err>> {
        self.probe.lock().expect("probe lock").take_next_calls += 1;
        self.jobs
            .pop_front()
            .map(|result| ScheduledWork::new(async move { result }))
    }

    fn on_complete(&mut self, completion: &WorkCompletion) {
        self.probe
            .lock()
            .expect("probe lock")
            .completions
            .push(completion.outcome);
    }
}

pub struct RepeatingGenerator {
    ready: bool,
    event: &'static str,
    probe: SharedProbe,
}

impl RepeatingGenerator {
    pub fn new(ready: bool, event: &'static str) -> (Self, SharedProbe) {
        let probe = probe();
        (
            Self {
                ready,
                event,
                probe: Arc::clone(&probe),
            },
            probe,
        )
    }
}

impl<'rt> Generator<'rt, String, String> for RepeatingGenerator {
    fn update_ready(&mut self, _now: Instant) -> Readiness {
        self.probe.lock().expect("probe lock").readiness_checks += 1;
        Readiness {
            ready: self.ready,
            next_ready_at: None,
        }
    }

    fn take_next(&mut self) -> Option<ScheduledWork<'rt, String, String>> {
        self.probe.lock().expect("probe lock").take_next_calls += 1;
        let event = self.event.to_owned();
        Some(ScheduledWork::new(async move { Ok(vec![event]) }))
    }

    fn on_complete(&mut self, completion: &WorkCompletion) {
        self.probe
            .lock()
            .expect("probe lock")
            .completions
            .push(completion.outcome);
    }
}

pub async fn next_work(runtime: &mut Runtime<'_, String, String>) -> Vec<String> {
    loop {
        match runtime.next().await {
            RuntimeOutput::Work(Ok(success)) => return success.events,
            RuntimeOutput::Work(Err(error)) => panic!("unexpected error: {}", error.error),
            RuntimeOutput::Shutdown => panic!("unexpected shutdown"),
            #[cfg(feature = "runtime-events")]
            RuntimeOutput::Runtime(_) => {}
        }
    }
}

pub async fn collect_string_events(
    runtime: &mut Runtime<'_, String, String>,
    count: usize,
) -> Vec<String> {
    let mut events = Vec::new();
    while events.len() < count {
        events.extend(next_work(runtime).await);
    }
    events
}
