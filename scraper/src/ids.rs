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

//! Opaque runtime identifier types.

use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

/// Runtime-assigned identifier for one target.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TargetId {
    value: u64,
}

impl TargetId {
    pub(crate) const fn new(value: u64) -> Self {
        Self { value }
    }

    pub(crate) const fn raw(self) -> u64 {
        self.value
    }
}

impl Display for TargetId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "target #{}", self.value)
    }
}

/// Runtime-assigned identifier for one generator under a target.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct GeneratorId {
    target_id: TargetId,
    value: u64,
}

impl GeneratorId {
    pub(crate) const fn new(target_id: TargetId, value: u64) -> Self {
        Self { target_id, value }
    }

    /// Returns the parent target that owns this generator.
    #[must_use]
    pub const fn target_id(self) -> TargetId {
        self.target_id
    }
}

impl Display for GeneratorId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "generator #{}.{}", self.target_id.raw(), self.value)
    }
}
