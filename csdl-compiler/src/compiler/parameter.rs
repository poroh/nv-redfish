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

use crate::compiler::MapType;
use crate::compiler::NavPropertyType;
use crate::compiler::OData;
use crate::compiler::PropertyType;
use crate::compiler::QualifiedName;
use crate::edmx::ParameterName;
use crate::IsNullable;
use crate::IsRequired;

/// Compiled parameter of the action.
#[derive(Debug, Clone, Copy)]
pub struct Parameter<'a> {
    /// Name of the parameter.
    pub name: &'a ParameterName,
    /// Type of the parameter. Can be either entity reference or some
    /// specific type.
    pub ptype: ParameterType<'a>,
    /// Flag that parameter is nullable.
    pub nullable: IsNullable,
    /// Flag that parameter is required.
    pub required: IsRequired,
    /// Odata for parameter
    pub odata: OData<'a>,
}

/// Type of the parameter. Note we reuse `CompiledPropertyType`, it
/// maybe not exact and may be change in future.
#[derive(Debug, Clone, Copy)]
pub enum ParameterType<'a> {
    Entity(NavPropertyType<'a>),
    Type(PropertyType<'a>),
}

impl<'a> ParameterType<'a> {
    fn map<F>(self, f: F) -> Self
    where
        F: Fn(QualifiedName<'a>) -> QualifiedName<'a>,
    {
        match self {
            Self::Entity(v) => Self::Entity(v.map(f)),
            Self::Type(v) => Self::Type(v.map(|(typeclass, ptype)| (typeclass, f(ptype)))),
        }
    }
}

impl<'a> MapType<'a> for Parameter<'a> {
    fn map_type<F>(self, f: F) -> Self
    where
        F: Fn(QualifiedName<'a>) -> QualifiedName<'a>,
    {
        Self {
            name: self.name,
            ptype: self.ptype.map(f),
            nullable: self.nullable,
            required: self.required,
            odata: self.odata,
        }
    }
}
