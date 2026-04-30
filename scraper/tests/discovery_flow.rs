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

use nv_redfish_scraper::Generator;
use nv_redfish_scraper::Readiness;
use nv_redfish_scraper::RunOnce;
use nv_redfish_scraper::Runtime;
use nv_redfish_scraper::RuntimeOutput;
use nv_redfish_scraper::ScheduledWork;
use nv_redfish_scraper::TargetConfig;
use nv_redfish_scraper::TargetId;
use nv_redfish_scraper::WorkCompletion;
use std::time::Instant;

#[derive(Clone, Debug, Eq, PartialEq)]
enum FakeExplorerEvent {
    ServiceRootDiscovered {
        systems: Vec<String>,
        chassis: Vec<String>,
    },
    SystemDiscovered {
        system_id: String,
    },
    ChassisDiscovered {
        chassis_id: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FakeExplorerError;

#[derive(Debug, Eq, PartialEq)]
struct FakeExplorationReport {
    target_id: TargetId,
    systems: Vec<String>,
    chassis: Vec<String>,
}

struct OneShotDiscoveryGenerator {
    event: Option<FakeExplorerEvent>,
}

impl OneShotDiscoveryGenerator {
    fn service_root(systems: Vec<String>, chassis: Vec<String>) -> Self {
        Self {
            event: Some(FakeExplorerEvent::ServiceRootDiscovered { systems, chassis }),
        }
    }

    fn system(system_id: String) -> Self {
        Self {
            event: Some(FakeExplorerEvent::SystemDiscovered { system_id }),
        }
    }

    fn chassis(chassis_id: String) -> Self {
        Self {
            event: Some(FakeExplorerEvent::ChassisDiscovered { chassis_id }),
        }
    }
}

impl<'rt> Generator<'rt, FakeExplorerEvent, FakeExplorerError> for OneShotDiscoveryGenerator {
    fn update_ready(&mut self, _now: Instant) -> Readiness {
        Readiness {
            ready: self.event.is_some(),
            next_ready_at: None,
        }
    }

    fn take_next(&mut self) -> Option<ScheduledWork<'rt, FakeExplorerEvent, FakeExplorerError>> {
        self.event
            .take()
            .map(|event| ScheduledWork::new(async move { Ok(vec![event]) }))
    }

    fn on_complete(&mut self, _completion: &WorkCompletion) {}
}

#[tokio::test]
async fn bmc_explorer_like_discovery_flow() {
    let mut runtime = Runtime::<FakeExplorerEvent, FakeExplorerError>::new();
    let target = runtime.add_target(TargetConfig {});
    runtime
        .add_generator(
            target,
            OneShotDiscoveryGenerator::service_root(
                vec!["system-a".to_owned(), "system-b".to_owned()],
                vec!["chassis-a".to_owned()],
            ),
        )
        .expect("add service-root generator");

    assert_eq!(runtime.run_once().await, RunOnce::Executed);
    let service_root_outputs = runtime.drain_outputs();
    add_follow_up_generators(&mut runtime, service_root_outputs, target);

    assert_eq!(runtime.run_once().await, RunOnce::Executed);
    assert_eq!(runtime.run_once().await, RunOnce::Executed);
    assert_eq!(runtime.run_once().await, RunOnce::Executed);
    assert_eq!(runtime.run_once().await, RunOnce::Idle);

    let report = build_report(target, runtime.drain_outputs());

    assert_eq!(
        report,
        FakeExplorationReport {
            target_id: target,
            systems: vec!["system-a".to_owned(), "system-b".to_owned()],
            chassis: vec!["chassis-a".to_owned()],
        }
    );
}

fn add_follow_up_generators(
    runtime: &mut Runtime<'_, FakeExplorerEvent, FakeExplorerError>,
    outputs: Vec<RuntimeOutput<FakeExplorerEvent, FakeExplorerError>>,
    expected_target: TargetId,
) {
    let requests = outputs
        .into_iter()
        .flat_map(|output| follow_up_requests(output, expected_target))
        .collect::<Vec<_>>();

    requests.into_iter().for_each(|request| match request {
        FollowUpRequest::System(system_id) => {
            runtime
                .add_generator(
                    expected_target,
                    OneShotDiscoveryGenerator::system(system_id),
                )
                .expect("add system generator");
        }
        FollowUpRequest::Chassis(chassis_id) => {
            runtime
                .add_generator(
                    expected_target,
                    OneShotDiscoveryGenerator::chassis(chassis_id),
                )
                .expect("add chassis generator");
        }
    });
}

fn follow_up_requests(
    output: RuntimeOutput<FakeExplorerEvent, FakeExplorerError>,
    expected_target: TargetId,
) -> Vec<FollowUpRequest> {
    match output {
        RuntimeOutput::Work(Ok(success)) => {
            assert_eq!(success.generator_id.target_id(), expected_target);
            success
                .events
                .into_iter()
                .flat_map(|event| match event {
                    FakeExplorerEvent::ServiceRootDiscovered { systems, chassis } => systems
                        .into_iter()
                        .map(FollowUpRequest::System)
                        .chain(chassis.into_iter().map(FollowUpRequest::Chassis))
                        .collect::<Vec<_>>(),
                    FakeExplorerEvent::SystemDiscovered { .. }
                    | FakeExplorerEvent::ChassisDiscovered { .. } => Vec::new(),
                })
                .collect()
        }
        RuntimeOutput::Work(Err(_)) => panic!("unexpected discovery error"),
    }
}

enum FollowUpRequest {
    System(String),
    Chassis(String),
}

fn build_report(
    target_id: TargetId,
    outputs: Vec<RuntimeOutput<FakeExplorerEvent, FakeExplorerError>>,
) -> FakeExplorationReport {
    let events = outputs
        .into_iter()
        .flat_map(|output| match output {
            RuntimeOutput::Work(Ok(success)) => {
                assert_eq!(success.generator_id.target_id(), target_id);
                success.events
            }
            RuntimeOutput::Work(Err(_)) => panic!("unexpected discovery error"),
        })
        .collect::<Vec<_>>();
    let systems = events
        .iter()
        .filter_map(|event| match event {
            FakeExplorerEvent::SystemDiscovered { system_id } => Some(system_id.clone()),
            FakeExplorerEvent::ServiceRootDiscovered { .. }
            | FakeExplorerEvent::ChassisDiscovered { .. } => None,
        })
        .collect();
    let chassis = events
        .iter()
        .filter_map(|event| match event {
            FakeExplorerEvent::ChassisDiscovered { chassis_id } => Some(chassis_id.clone()),
            FakeExplorerEvent::ServiceRootDiscovered { .. }
            | FakeExplorerEvent::SystemDiscovered { .. } => None,
        })
        .collect();

    FakeExplorationReport {
        target_id,
        systems,
        chassis,
    }
}
