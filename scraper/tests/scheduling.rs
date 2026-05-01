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
use nv_redfish_scraper::Runtime;
use nv_redfish_scraper::TargetConfig;

#[tokio::test]
async fn flat_round_robin_order() {
    let (mut runtime, handle) = Runtime::<String, String>::new();
    let target = handle.add_target(TargetConfig {}).expect("add target");
    let generators = ["A", "B", "C"].iter().map(|event| {
        let (generator, _) = RepeatingGenerator::new(true, event);
        generator
    });
    for generator in generators {
        handle
            .add_generator(target, generator)
            .expect("add generator");
    }

    assert_eq!(
        common::collect_string_events(&mut runtime, 6).await,
        vec!["A", "B", "C", "A", "B", "C"]
    );
}

#[tokio::test]
async fn not_ready_generator_is_skipped() {
    let (mut runtime, handle) = Runtime::<String, String>::new();
    let target = handle.add_target(TargetConfig {}).expect("add target");
    let (not_ready, not_ready_probe) = RepeatingGenerator::new(false, "not-ready");
    let (ready, ready_probe) = RepeatingGenerator::new(true, "ready");
    handle
        .add_generator(target, not_ready)
        .expect("add not-ready generator");
    handle
        .add_generator(target, ready)
        .expect("add ready generator");

    assert_eq!(common::next_work(&mut runtime).await, vec!["ready"]);
    assert_eq!(
        not_ready_probe.lock().expect("probe lock").readiness_checks,
        1
    );
    assert_eq!(
        not_ready_probe.lock().expect("probe lock").take_next_calls,
        0
    );
    assert_eq!(ready_probe.lock().expect("probe lock").take_next_calls, 1);
}

#[tokio::test]
async fn work_is_created_only_when_selected() {
    let (mut runtime, handle) = Runtime::<String, String>::new();
    let target = handle.add_target(TargetConfig {}).expect("add target");
    let (first, first_probe) = QueueGenerator::new(true, [Ok(vec!["A".to_owned()])]);
    let (second, second_probe) = QueueGenerator::new(true, [Ok(vec!["B".to_owned()])]);
    handle
        .add_generator(target, first)
        .expect("add first generator");
    handle
        .add_generator(target, second)
        .expect("add second generator");

    assert_eq!(first_probe.lock().expect("probe lock").take_next_calls, 0);
    assert_eq!(second_probe.lock().expect("probe lock").take_next_calls, 0);
    assert_eq!(common::next_work(&mut runtime).await, vec!["A"]);
    assert_eq!(first_probe.lock().expect("probe lock").take_next_calls, 1);
    assert_eq!(second_probe.lock().expect("probe lock").take_next_calls, 0);
    assert_eq!(common::next_work(&mut runtime).await, vec!["B"]);
    assert_eq!(first_probe.lock().expect("probe lock").take_next_calls, 1);
    assert_eq!(second_probe.lock().expect("probe lock").take_next_calls, 1);
}
