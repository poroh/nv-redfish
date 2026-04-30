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

mod common;

use common::QueueGenerator;
use common::RepeatingGenerator;
use nv_redfish_scraper::AddGeneratorError;
use nv_redfish_scraper::RunOnce;
use nv_redfish_scraper::Runtime;
use nv_redfish_scraper::RuntimeOutput;
use nv_redfish_scraper::TargetConfig;

#[test]
fn add_generator_requires_existing_target() {
    let mut runtime = Runtime::<String, String>::new();
    let removed_target = runtime.add_target(TargetConfig {});
    assert!(runtime.remove_target(removed_target));
    let (generator, _) = QueueGenerator::new(true, [Ok(vec!["event".to_owned()])]);

    let error = runtime
        .add_generator(removed_target, generator)
        .expect_err("missing target must fail");

    assert_eq!(
        error,
        AddGeneratorError::TargetNotFound {
            target_id: removed_target,
        }
    );
}

#[tokio::test]
async fn remove_generator_stops_future_queries_and_outputs() {
    let mut runtime = Runtime::<String, String>::new();
    let target = runtime.add_target(TargetConfig {});
    let (first_generator, _) = RepeatingGenerator::new(true, "A");
    let (second_generator, second_probe) = RepeatingGenerator::new(true, "B");
    runtime
        .add_generator(target, first_generator)
        .expect("add first generator");
    let second_id = runtime
        .add_generator(target, second_generator)
        .expect("add second generator");

    assert!(runtime.remove_generator(second_id));
    assert!(!runtime.remove_generator(second_id));
    assert_eq!(runtime.run_once().await, RunOnce::Executed);
    assert_eq!(runtime.run_once().await, RunOnce::Executed);
    assert_eq!(
        common::only_string_events(runtime.drain_outputs()),
        vec!["A", "A"]
    );
    assert_eq!(second_probe.lock().expect("probe lock").readiness_checks, 0);
}

#[tokio::test]
async fn remove_target_removes_generators() {
    let mut runtime = Runtime::<String, String>::new();
    let target = runtime.add_target(TargetConfig {});
    let (first_generator, first_probe) = RepeatingGenerator::new(true, "A");
    let (second_generator, second_probe) = RepeatingGenerator::new(true, "B");
    runtime
        .add_generator(target, first_generator)
        .expect("add first generator");
    runtime
        .add_generator(target, second_generator)
        .expect("add second generator");

    assert!(runtime.remove_target(target));
    assert!(!runtime.remove_target(target));
    assert_eq!(runtime.run_once().await, RunOnce::Idle);
    assert_eq!(first_probe.lock().expect("probe lock").readiness_checks, 0);
    assert_eq!(second_probe.lock().expect("probe lock").readiness_checks, 0);
}

#[tokio::test]
async fn produced_outputs_survive_removal() {
    let mut runtime = Runtime::<String, String>::new();
    let target = runtime.add_target(TargetConfig {});
    let (generator, _) = QueueGenerator::new(true, [Ok(vec!["event".to_owned()])]);
    let generator_id = runtime
        .add_generator(target, generator)
        .expect("add generator");

    assert_eq!(runtime.run_once().await, RunOnce::Executed);
    assert!(runtime.remove_generator(generator_id));

    let outputs = runtime.drain_outputs();
    assert_eq!(outputs.len(), 1);
    match outputs.into_iter().next().expect("one output") {
        RuntimeOutput::Work(Ok(success)) => {
            assert_eq!(success.generator_id, generator_id);
            assert_eq!(success.events, vec!["event"]);
        }
        RuntimeOutput::Work(Err(error)) => panic!("unexpected error: {}", error.error),
    }
}
