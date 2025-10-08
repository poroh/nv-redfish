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

//! BMC trait definition

use serde::Deserialize;
use serde::Serialize;

use crate::http::ExpandQuery;
use crate::Action;
use crate::Empty;
use crate::EntityTypeRef;
use crate::Expandable;
use crate::ODataId;
use std::error::Error as StdError;
use std::fmt;
use std::future::Future;
use std::sync::Arc;

/// BMC trait defined access to Board Management Controller using
/// Redfish protocol.
pub trait Bmc: Send + Sync {
    /// BMC Error
    type Error: StdError + Send;

    /// Expand any expandable object (navigation property or entity).
    ///
    /// `T` is structure that is used for return type.
    fn expand<T: Expandable + Send + Sync>(
        &self,
        id: &ODataId,
        query: ExpandQuery,
    ) -> impl Future<Output = Result<Arc<T>, Self::Error>> + Send;

    /// Get data of the object (navigation property or entity).
    ///
    /// `T` is structure that is used for return type.
    fn get<T: EntityTypeRef + Sized + for<'a> Deserialize<'a> + 'static + Send + Sync>(
        &self,
        id: &ODataId,
    ) -> impl Future<Output = Result<Arc<T>, Self::Error>> + Send;

    /// Creates element of the collection.
    ///
    /// `V` is structure that is used for create.
    /// `R` is structure that is used for return type.
    fn create<V: Sync + Send + Serialize, R: Send + Sync + Sized + for<'a> Deserialize<'a>>(
        &self,
        id: &ODataId,
        query: &V,
    ) -> impl Future<Output = Result<R, Self::Error>> + Send;

    /// Update entity.
    ///
    /// `V` is structure that is used for update.
    /// `R` is structure that is used for return type (updated entity).
    fn update<V: Sync + Send + Serialize, R: Send + Sync + Sized + for<'a> Deserialize<'a>>(
        &self,
        id: &ODataId,
        query: &V,
    ) -> impl Future<Output = Result<R, Self::Error>> + Send;

    /// Delete entity.
    fn delete(&self, id: &ODataId) -> impl Future<Output = Result<Empty, Self::Error>> + Send;

    /// Run action.
    ///
    /// `T` is structure that contains action parameters.
    /// `R` is structure with return type.
    fn action<T: Send + Sync + Serialize, R: Send + Sync + Sized + for<'a> Deserialize<'a>>(
        &self,
        action: &Action<T, R>,
        params: &T,
    ) -> impl Future<Output = Result<R, Self::Error>> + Send;
}

/// Credentials used to access to the BMC.
#[derive(Clone)]
pub struct BmcCredentials {
    /// Username to access BMC.
    pub username: String,
    password: String,
}

impl BmcCredentials {
    /// Create new credentials.
    #[must_use]
    pub const fn new(username: String, password: String) -> Self {
        Self { username, password }
    }

    /// Get password.
    #[must_use]
    pub fn password(&self) -> &str {
        &self.password
    }
}

impl fmt::Debug for BmcCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BmcCredentials")
            .field("username", &self.username)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

impl fmt::Display for BmcCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BmcCredentials(username: {}, password: [REDACTED])",
            self.username
        )
    }
}
