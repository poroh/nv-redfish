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

use crate::edmx::TypeName;
use serde::Deserialize;

pub type EnumMemberName = String;

/// 10.1 Element edm:EnumType
#[derive(Debug, Deserialize)]
pub struct EnumType {
    /// 10.1.1 Attribute `Name`
    #[serde(rename = "@Name")]
    pub name: TypeName,
    /// 10.1.2 Attribute `UnderlyingType`
    #[serde(rename = "@UnderlyingType")]
    pub underlying_type: Option<TypeName>,
    /// 10.1.3 Attribute `IsFlags`
    #[serde(rename = "@IsFlags")]
    pub is_flags: Option<bool>,
    /// Child elements of `EnumType`.
    #[serde(rename = "Member", default)]
    pub members: Vec<EnumMember>,
}

/// 10.2 Element edm:Member
#[derive(Debug, Deserialize)]
pub struct EnumMember {
    /// 10.2.1 Attribute Name
    #[serde(rename = "@Name")]
    pub name: EnumMemberName,
    /// 10.2.2 Attribute Value
    #[serde(rename = "@Value")]
    pub value: Option<String>,
}
