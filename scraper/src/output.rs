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

//! Ordered runtime output types.

use crate::GeneratorId;
#[cfg(feature = "runtime-events")]
use crate::TargetId;

/// Result emitted by one completed work item.
pub type WorkResult<Ev, Err> = Result<WorkSuccess<Ev>, WorkError<Err>>;

/// Successful work output enriched with runtime identity.
pub struct WorkSuccess<Ev> {
    /// The generator that produced this work.
    pub generator_id: GeneratorId,
    /// Events returned by the work future, in order.
    pub events: Vec<Ev>,
}

/// Failed work output enriched with runtime identity.
pub struct WorkError<Err> {
    /// The generator whose work failed.
    pub generator_id: GeneratorId,
    /// Error returned by the work future.
    pub error: Err,
}

/// Output item emitted by the runtime.
pub enum RuntimeOutput<Ev, Err> {
    /// Completed work output.
    Work(WorkResult<Ev, Err>),
    /// Graceful runtime termination.
    Shutdown,
    /// Runtime-owned control-plane event.
    #[cfg(feature = "runtime-events")]
    Runtime(RuntimeEvent),
}

/// Runtime-owned control-plane event.
#[cfg(feature = "runtime-events")]
pub enum RuntimeEvent {
    /// A target was added.
    TargetAdded {
        /// Added target id.
        target_id: TargetId,
    },
    /// A generator was added.
    GeneratorAdded {
        /// Added generator id.
        generator_id: GeneratorId,
    },
    /// A generator removal finalized.
    GeneratorRemoved {
        /// Removed generator id.
        generator_id: GeneratorId,
    },
    /// A target removal finalized.
    TargetRemoved {
        /// Removed target id.
        target_id: TargetId,
    },
}
