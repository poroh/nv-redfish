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

//! Flat round-robin scheduler.

use crate::GeneratorId;

#[derive(Default)]
pub struct FlatRoundRobin {
    order: Vec<GeneratorId>,
    cursor: usize,
}

impl FlatRoundRobin {
    pub(crate) fn insert(&mut self, generator_id: GeneratorId) {
        self.order.push(generator_id);
    }

    pub(crate) fn remove(&mut self, generator_id: GeneratorId) {
        if let Some(index) = self.order.iter().position(|id| *id == generator_id) {
            self.order.remove(index);
            self.cursor = adjusted_cursor(self.cursor, index, self.order.len());
        }
    }

    pub(crate) fn next_candidate(&mut self, scan: &mut FlatScan) -> Option<GeneratorId> {
        if scan.remaining == 0 || self.order.is_empty() {
            None
        } else {
            let candidate = self.order[self.cursor];
            self.cursor = (self.cursor + 1) % self.order.len();
            scan.remaining -= 1;
            Some(candidate)
        }
    }

    pub(crate) const fn start_scan(&self) -> FlatScan {
        FlatScan {
            remaining: self.order.len(),
        }
    }
}

pub struct FlatScan {
    remaining: usize,
}

const fn adjusted_cursor(cursor: usize, removed_index: usize, new_len: usize) -> usize {
    match new_len {
        0 => 0,
        _ if removed_index < cursor => (cursor - 1) % new_len,
        _ => cursor % new_len,
    }
}
