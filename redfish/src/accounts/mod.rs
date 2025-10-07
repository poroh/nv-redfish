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

//! This module represents AccountService defined in Redfish
//! specification.

use crate::Error;
use crate::schema::redfish::account_service::AccountService as SchemaAccountService;
use crate::schema::redfish::manager_account::ManagerAccount;
use crate::schema::redfish::manager_account_collection::ManagerAccountCollection;
use nv_redfish_core::Bmc;
use nv_redfish_core::Creatable;
use nv_redfish_core::Deletable;
use nv_redfish_core::Expandable;
use nv_redfish_core::Updatable;
use nv_redfish_core::http::ExpandQuery;
use std::sync::Arc;

pub use crate::schema::redfish::manager_account::ManagerAccountCreate;
pub use crate::schema::redfish::manager_account::ManagerAccountUpdate;

pub struct AccountService<B: Bmc> {
    service: Arc<SchemaAccountService>,
    bmc: Arc<B>,
}

impl<B: Bmc> AccountService<B> {
    pub(crate) fn new(service: Arc<SchemaAccountService>, bmc: Arc<B>) -> Self {
        Self { service, bmc }
    }
    pub async fn accounts(&self) -> Result<AccountCollection<B>, Error<B>> {
        let collection = self
            .service
            .accounts
            .as_ref()
            .ok_or(Error::AccountServiceNotSupported)?
            .expand(self.bmc.as_ref(), ExpandQuery::default().levels(1))
            .await
            .map_err(Error::Bmc)?
            .get(self.bmc.as_ref()) // should do nothing...
            .await
            .map_err(Error::Bmc)?;
        Ok(AccountCollection {
            bmc: self.bmc.clone(),
            collection,
        })
    }
}

pub struct AccountCollection<B: Bmc> {
    bmc: Arc<B>,
    collection: Arc<ManagerAccountCollection>,
}

impl<B: Bmc> AccountCollection<B> {
    pub async fn create_account(
        &self,
        create: &ManagerAccountCreate,
    ) -> Result<Account<B>, Error<B>> {
        let account = self
            .collection
            .create(self.bmc.as_ref(), create)
            .await
            .map_err(Error::Bmc)?;
        Ok(Account {
            bmc: self.bmc.clone(),
            account: Arc::new(account),
        })
    }

    pub async fn all_accounts(&self) -> Result<Vec<Account<B>>, Error<B>> {
        let mut result = Vec::with_capacity(self.collection.members.len());
        for m in &self.collection.members {
            result.push(Account {
                bmc: self.bmc.clone(),
                account: m.get(self.bmc.as_ref()).await.map_err(Error::Bmc)?,
            })
        }
        Ok(result)
    }
}

pub struct Account<B: Bmc> {
    bmc: Arc<B>,
    account: Arc<ManagerAccount>,
}

impl<B: Bmc> Account<B> {
    pub fn raw(&self) -> Arc<ManagerAccount> {
        self.account.clone()
    }

    pub async fn update_password(&self, password: String) -> Result<Self, Error<B>> {
        let account = self
            .account
            .update(
                self.bmc.as_ref(),
                &ManagerAccountUpdate::builder()
                    .with_password(password)
                    .build(),
            )
            .await
            .map_err(Error::Bmc)?;
        Ok(Account {
            bmc: self.bmc.clone(),
            account: Arc::new(account),
        })
    }

    pub async fn update_user_name(&self, user_name: String) -> Result<Self, Error<B>> {
        let account = self
            .account
            .update(
                self.bmc.as_ref(),
                &ManagerAccountUpdate::builder()
                    .with_user_name(user_name)
                    .build(),
            )
            .await
            .map_err(Error::Bmc)?;
        Ok(Account {
            bmc: self.bmc.clone(),
            account: Arc::new(account),
        })
    }

    pub async fn delete(&self) -> Result<(), Error<B>> {
        self.account
            .delete(self.bmc.as_ref())
            .await
            .map_err(Error::Bmc)
            .map(|_| ())
    }
}
