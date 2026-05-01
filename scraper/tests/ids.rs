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
use nv_redfish_scraper::Runtime;
use nv_redfish_scraper::TargetConfig;

#[test]
fn runtime_construction_returns_consumer_and_handle() {
    let (_runtime, handle) = Runtime::<String, String>::new();
    let _cloned = handle.clone();
}

#[test]
fn target_ids_are_generated() {
    let (_runtime, handle) = Runtime::<String, String>::new();

    let first = handle
        .add_target(TargetConfig {})
        .expect("add first target");
    let second = handle
        .add_target(TargetConfig {})
        .expect("add second target");

    assert_ne!(first, second);
    assert_eq!(first.to_string(), "target #1");
    assert_eq!(second.to_string(), "target #2");
}

#[test]
fn generator_ids_include_target_ids() {
    let (_runtime, handle) = Runtime::<String, String>::new();
    let first_target = handle
        .add_target(TargetConfig {})
        .expect("add first target");
    let second_target = handle
        .add_target(TargetConfig {})
        .expect("add second target");
    let (first_generator, _) = QueueGenerator::new(true, [Ok(vec!["a".to_owned()])]);
    let (second_generator, _) = QueueGenerator::new(true, [Ok(vec!["b".to_owned()])]);
    let (third_generator, _) = QueueGenerator::new(true, [Ok(vec!["c".to_owned()])]);

    let first = handle
        .add_generator(first_target, first_generator)
        .expect("add first generator");
    let second = handle
        .add_generator(first_target, second_generator)
        .expect("add second generator");
    let third = handle
        .add_generator(second_target, third_generator)
        .expect("add third generator");

    assert_eq!(first.target_id(), first_target);
    assert_eq!(second.target_id(), first_target);
    assert_eq!(third.target_id(), second_target);
    assert_eq!(first.to_string(), "generator #1.1");
    assert_eq!(second.to_string(), "generator #1.2");
    assert_eq!(third.to_string(), "generator #2.1");
}
