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
use nv_redfish_scraper::ControlError;
use nv_redfish_scraper::Runtime;
use nv_redfish_scraper::RuntimeOutput;
use nv_redfish_scraper::TargetConfig;

#[test]
fn add_generator_requires_existing_target() {
    let (_runtime, handle) = Runtime::<String, String>::new();
    let removed_target = handle.add_target(TargetConfig {}).expect("add target");
    assert!(handle.remove_target(removed_target).expect("remove target"));
    let (generator, _) = QueueGenerator::new(true, [Ok(vec!["event".to_owned()])]);

    let error = handle
        .add_generator(removed_target, generator)
        .expect_err("missing target must fail");

    assert_eq!(
        error,
        ControlError::TargetNotFound {
            target_id: removed_target,
        }
    );
}

#[tokio::test]
async fn remove_generator_stops_future_queries_and_outputs() {
    let (mut runtime, handle) = Runtime::<String, String>::new();
    let target = handle.add_target(TargetConfig {}).expect("add target");
    let (first_generator, _) = RepeatingGenerator::new(true, "A");
    let (second_generator, second_probe) = RepeatingGenerator::new(true, "B");
    handle
        .add_generator(target, first_generator)
        .expect("add first generator");
    let second_id = handle
        .add_generator(target, second_generator)
        .expect("add second generator");

    assert!(handle
        .remove_generator(second_id)
        .expect("remove generator"));
    assert!(!handle
        .remove_generator(second_id)
        .expect("remove generator again"));
    assert_eq!(
        common::collect_string_events(&mut runtime, 2).await,
        vec!["A", "A"]
    );
    assert_eq!(second_probe.lock().expect("probe lock").readiness_checks, 0);
}

#[test]
fn remove_target_removes_generators() {
    let (_runtime, handle) = Runtime::<String, String>::new();
    let target = handle.add_target(TargetConfig {}).expect("add target");
    let (first_generator, first_probe) = RepeatingGenerator::new(true, "A");
    let (second_generator, second_probe) = RepeatingGenerator::new(true, "B");
    handle
        .add_generator(target, first_generator)
        .expect("add first generator");
    handle
        .add_generator(target, second_generator)
        .expect("add second generator");

    assert!(handle.remove_target(target).expect("remove target"));
    assert!(!handle.remove_target(target).expect("remove target again"));
    assert_eq!(first_probe.lock().expect("probe lock").readiness_checks, 0);
    assert_eq!(second_probe.lock().expect("probe lock").readiness_checks, 0);
}

#[tokio::test]
async fn produced_outputs_survive_removal() {
    let (mut runtime, handle) = Runtime::<String, String>::new();
    let target = handle.add_target(TargetConfig {}).expect("add target");
    let (generator, _) = QueueGenerator::new(true, [Ok(vec!["event".to_owned()])]);
    let generator_id = handle
        .add_generator(target, generator)
        .expect("add generator");

    loop {
        match runtime.next().await {
            RuntimeOutput::Work(Ok(success)) => {
                assert_eq!(success.generator_id, generator_id);
                assert_eq!(success.events, vec!["event"]);
                break;
            }
            RuntimeOutput::Work(Err(error)) => panic!("unexpected error: {}", error.error),
            RuntimeOutput::Shutdown => panic!("unexpected shutdown"),
            #[cfg(feature = "runtime-events")]
            RuntimeOutput::Runtime(_) => {}
        }
    }
    assert!(handle
        .remove_generator(generator_id)
        .expect("remove generator"));
}

#[tokio::test]
async fn graceful_shutdown_with_no_targets() {
    let (mut runtime, handle) = Runtime::<String, String>::new();
    handle.graceful_shutdown();
    assert!(matches!(runtime.next().await, RuntimeOutput::Shutdown));
    assert!(matches!(runtime.next().await, RuntimeOutput::Shutdown));
}
