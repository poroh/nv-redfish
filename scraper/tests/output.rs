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
use nv_redfish_scraper::Generator;
use nv_redfish_scraper::Readiness;
use nv_redfish_scraper::Runtime;
use nv_redfish_scraper::RuntimeOutput;
use nv_redfish_scraper::ScheduledWork;
use nv_redfish_scraper::TargetConfig;
use nv_redfish_scraper::WorkCompletion;
use std::time::Instant;

#[tokio::test]
async fn output_event_ordering_inside_one_work_item() {
    let (mut runtime, handle) = Runtime::<String, String>::new();
    let target = handle.add_target(TargetConfig {}).expect("add target");
    let (generator, _) = QueueGenerator::new(
        true,
        [Ok(vec![
            "first".to_owned(),
            "second".to_owned(),
            "third".to_owned(),
        ])],
    );
    handle
        .add_generator(target, generator)
        .expect("add generator");

    assert_eq!(
        common::next_work(&mut runtime).await,
        vec!["first", "second", "third"]
    );
}

#[tokio::test]
async fn work_error_output() {
    let (mut runtime, handle) = Runtime::<String, String>::new();
    let target = handle.add_target(TargetConfig {}).expect("add target");
    let (generator, _) = QueueGenerator::new(true, [Err("failed".to_owned())]);
    let generator_id = handle
        .add_generator(target, generator)
        .expect("add generator");

    loop {
        match runtime.next().await {
            RuntimeOutput::Work(Err(error)) => {
                assert_eq!(error.generator_id, generator_id);
                assert_eq!(error.error, "failed");
                break;
            }
            RuntimeOutput::Work(Ok(_)) => panic!("unexpected success"),
            RuntimeOutput::Shutdown => panic!("unexpected shutdown"),
            #[cfg(feature = "runtime-events")]
            RuntimeOutput::Runtime(_) => {}
        }
    }
}

#[tokio::test]
async fn next_preserves_fifo_order() {
    let (mut runtime, handle) = Runtime::<String, String>::new();
    let target = handle.add_target(TargetConfig {}).expect("add target");
    let (first, _) = QueueGenerator::new(true, [Ok(vec!["first".to_owned()])]);
    let (second, _) = QueueGenerator::new(true, [Ok(vec!["second".to_owned()])]);
    handle.add_generator(target, first).expect("add first");
    handle.add_generator(target, second).expect("add second");

    assert_eq!(common::next_work(&mut runtime).await, vec!["first"]);
    assert_eq!(common::next_work(&mut runtime).await, vec!["second"]);
}

#[tokio::test]
async fn payloads_do_not_need_common_traits() {
    struct EventWithoutCommonTraits {
        value: u8,
    }

    struct ErrorWithoutCommonTraits;

    struct PayloadGenerator;

    impl<'rt> Generator<'rt, EventWithoutCommonTraits, ErrorWithoutCommonTraits> for PayloadGenerator {
        fn update_ready(&mut self, _now: Instant) -> Readiness {
            Readiness {
                ready: true,
                next_ready_at: None,
            }
        }

        fn take_next(
            &mut self,
        ) -> Option<ScheduledWork<'rt, EventWithoutCommonTraits, ErrorWithoutCommonTraits>>
        {
            Some(ScheduledWork::new(async {
                Ok(vec![EventWithoutCommonTraits { value: 7 }])
            }))
        }

        fn on_complete(&mut self, _completion: &WorkCompletion) {}
    }

    let (mut runtime, handle) =
        Runtime::<EventWithoutCommonTraits, ErrorWithoutCommonTraits>::new();
    let target = handle.add_target(TargetConfig {}).expect("add target");
    handle
        .add_generator(target, PayloadGenerator)
        .expect("add generator");

    loop {
        match runtime.next().await {
            RuntimeOutput::Work(Ok(success)) => {
                assert_eq!(success.events.len(), 1);
                assert_eq!(success.events[0].value, 7);
                break;
            }
            RuntimeOutput::Work(Err(_)) => panic!("unexpected error"),
            RuntimeOutput::Shutdown => panic!("unexpected shutdown"),
            #[cfg(feature = "runtime-events")]
            RuntimeOutput::Runtime(_) => {}
        }
    }
}
