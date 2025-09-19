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

use crate::edmx::PropertyName as EdmxPropertyName;
use heck::AsSnakeCase;
use proc_macro2::Ident;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::TokenStreamExt as _;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

/// Property name built from edmx `PropertyName`.
///
/// Example of representation: `protocol_features_supported`
#[derive(PartialEq, Eq, Hash, Copy, Clone, Ord, PartialOrd)]
pub struct PropertyName<'a>(&'a EdmxPropertyName);

impl<'a> PropertyName<'a> {
    /// Create new property name.
    #[must_use]
    pub const fn new(v: &'a EdmxPropertyName) -> Self {
        Self(v)
    }
}

impl ToTokens for PropertyName<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self.to_string().as_str() {
            "type" => tokens.append(Ident::new_raw("type", Span::call_site())),
            _ => tokens.append(Ident::new(&self.to_string(), Span::call_site())),
        }
    }
}

impl Display for PropertyName<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        AsSnakeCase(self.0.inner()).fmt(f)
    }
}

impl Debug for PropertyName<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(self, f)
    }
}
