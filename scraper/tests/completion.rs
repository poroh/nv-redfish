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
use nv_redfish_scraper::RunOnce;
use nv_redfish_scraper::Runtime;
use nv_redfish_scraper::TargetConfig;
use nv_redfish_scraper::WorkOutcome;

#[tokio::test]
async fn completion_callback_on_success() {
    let mut runtime = Runtime::<String, String>::new();
    let target = runtime.add_target(TargetConfig {});
    let (generator, probe) = QueueGenerator::new(true, [Ok(vec!["event".to_owned()])]);
    runtime
        .add_generator(target, generator)
        .expect("add generator");

    assert_eq!(runtime.run_once().await, RunOnce::Executed);

    assert_eq!(
        probe.lock().expect("probe lock").completions,
        vec![WorkOutcome::Succeeded]
    );
}

#[tokio::test]
async fn completion_callback_on_failure() {
    let mut runtime = Runtime::<String, String>::new();
    let target = runtime.add_target(TargetConfig {});
    let (generator, probe) = QueueGenerator::new(true, [Err("failed".to_owned())]);
    runtime
        .add_generator(target, generator)
        .expect("add generator");

    assert_eq!(runtime.run_once().await, RunOnce::Executed);

    assert_eq!(
        probe.lock().expect("probe lock").completions,
        vec![WorkOutcome::Failed]
    );
}
