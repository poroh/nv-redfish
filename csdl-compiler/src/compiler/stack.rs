// SPDX-FileCopyrightText: Copyright (c) 2025 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
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

use crate::compiler::Compiled;
use crate::compiler::QualifiedName;

/// Compilation stack. Creates every time when we go inside recursion
/// to compile inner type.
#[derive(Default)]
pub struct Stack<'a, 'stack> {
    parent: Option<&'stack Stack<'a, 'stack>>,
    // If entity type is currently being compiled we use this
    // field to prevent infinite recursion.
    entity_type: Option<QualifiedName<'a>>,
    current: Compiled<'a>,
}

impl<'a, 'stack> Stack<'a, 'stack> {
    #[must_use]
    pub fn new_frame(&'stack self) -> Self {
        Self {
            parent: Some(self),
            entity_type: None,
            current: Compiled::default(),
        }
    }

    #[must_use]
    pub const fn with_enitity_type(mut self, name: QualifiedName<'a>) -> Self {
        self.entity_type = Some(name);
        self
    }

    #[must_use]
    pub fn contains_entity(&self, qtype: QualifiedName<'a>) -> bool {
        self.current.entity_types.contains_key(&qtype)
            || self.entity_type.is_some_and(|v| v == qtype)
            || self.parent.is_some_and(|p| p.contains_entity(qtype))
    }

    #[must_use]
    pub fn merge(self, c: Compiled<'a>) -> Self {
        Self {
            parent: self.parent,
            entity_type: self.entity_type,
            current: self.current.merge(c),
        }
    }

    #[must_use]
    pub fn done(self) -> Compiled<'a> {
        self.current
    }
}
