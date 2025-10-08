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
use crate::ServiceRoot;
use crate::patch_support::CollectionWithPatch;
use crate::patch_support::CreateWithPatch;
use crate::patch_support::JsonValue;
use crate::patch_support::ReadPatchFn;
use crate::patch_support::UpdateWithPatch;
use crate::schema::redfish::account_service::AccountService as SchemaAccountService;
use crate::schema::redfish::manager_account::ManagerAccount;
use crate::schema::redfish::manager_account_collection::ManagerAccountCollection;
use crate::schema::redfish::resource::ResourceCollection;
use nv_redfish_core::Bmc;
use nv_redfish_core::Deletable;
use nv_redfish_core::EntityTypeRef;
use nv_redfish_core::NavProperty;
use nv_redfish_core::ODataId;
use nv_redfish_core::http::ExpandQuery;
use std::sync::Arc;

pub use crate::schema::redfish::manager_account::AccountTypes;
pub use crate::schema::redfish::manager_account::ManagerAccountCreate;
pub use crate::schema::redfish::manager_account::ManagerAccountUpdate;

pub struct AccountService<B: Bmc> {
    account_read_patch_fn: Option<ReadPatchFn>,
    service: Arc<SchemaAccountService>,
    bmc: Arc<B>,
}

impl<B: Bmc> AccountService<B> {
    pub(crate) fn new(
        root: ServiceRoot<B>,
        service: Arc<SchemaAccountService>,
        bmc: Arc<B>,
    ) -> Self {
        let mut patches = Vec::new();
        if root.bug_no_account_type_in_accounts() {
            patches.push(append_default_account_type);
        }
        let account_read_patch_fn = if patches.is_empty() {
            None
        } else {
            let account_read_patch_fn: ReadPatchFn =
                Arc::new(move |v| patches.iter().fold(v, |acc, f| f(acc)));
            Some(account_read_patch_fn)
        };

        Self {
            account_read_patch_fn,
            service,
            bmc,
        }
    }
    pub fn odata_id(&self) -> &ODataId {
        self.service.as_ref().id()
    }

    pub async fn accounts(&self) -> Result<AccountCollection<B>, Error<B>> {
        let collection_ref = self
            .service
            .accounts
            .as_ref()
            .ok_or(Error::AccountServiceNotSupported)?;

        let query = ExpandQuery::default().levels(1);
        Ok(AccountCollection {
            read_patch_fn: self.account_read_patch_fn.clone(),
            bmc: self.bmc.clone(),
            collection: AccountCollection::read_collection(
                self.bmc.as_ref(),
                collection_ref,
                self.account_read_patch_fn.as_ref(),
                query,
            )
            .await?,
        })
    }
}

pub struct AccountCollection<B: Bmc> {
    read_patch_fn: Option<ReadPatchFn>,
    bmc: Arc<B>,
    collection: Arc<ManagerAccountCollection>,
}

impl<B: Bmc> CollectionWithPatch<ManagerAccountCollection, ManagerAccount, B>
    for AccountCollection<B>
{
    fn convert_patched(
        base: ResourceCollection,
        members: Vec<NavProperty<ManagerAccount>>,
    ) -> ManagerAccountCollection {
        ManagerAccountCollection { base, members }
    }
}

impl<B: Bmc> CreateWithPatch<ManagerAccountCollection, ManagerAccount, ManagerAccountCreate, B>
    for AccountCollection<B>
{
    fn entity_ref(&self) -> &ManagerAccountCollection {
        self.collection.as_ref()
    }
    fn patch(&self) -> Option<&ReadPatchFn> {
        self.read_patch_fn.as_ref()
    }
    fn bmc(&self) -> &B {
        &self.bmc
    }
}

impl<B: Bmc> AccountCollection<B> {
    pub fn odata_id(&self) -> &ODataId {
        self.collection.as_ref().id()
    }

    pub async fn create_account(
        &self,
        create: &ManagerAccountCreate,
    ) -> Result<Account<B>, Error<B>> {
        let account = self.create_with_patch(create).await?;
        Ok(Account {
            read_patch_fn: self.read_patch_fn.clone(),
            bmc: self.bmc.clone(),
            account: Arc::new(account),
        })
    }

    pub async fn all_accounts(&self) -> Result<Vec<Account<B>>, Error<B>> {
        let mut result = Vec::with_capacity(self.collection.members.len());
        for m in &self.collection.members {
            result.push(Account {
                read_patch_fn: self.read_patch_fn.clone(),
                bmc: self.bmc.clone(),
                account: m.get(self.bmc.as_ref()).await.map_err(Error::Bmc)?,
            })
        }
        Ok(result)
    }
}

pub struct Account<B: Bmc> {
    read_patch_fn: Option<ReadPatchFn>,
    bmc: Arc<B>,
    account: Arc<ManagerAccount>,
}

impl<B: Bmc> UpdateWithPatch<ManagerAccount, ManagerAccountUpdate, B> for Account<B> {
    fn entity_ref(&self) -> &ManagerAccount {
        self.account.as_ref()
    }
    fn patch(&self) -> Option<&ReadPatchFn> {
        self.read_patch_fn.as_ref()
    }
    fn bmc(&self) -> &B {
        &self.bmc
    }
}

impl<B: Bmc> Account<B> {
    pub fn raw(&self) -> Arc<ManagerAccount> {
        self.account.clone()
    }

    pub async fn update(&self, update: &ManagerAccountUpdate) -> Result<Self, Error<B>> {
        let account = self.update_with_patch(update).await?;
        Ok(Account {
            read_patch_fn: self.read_patch_fn.clone(),
            bmc: self.bmc.clone(),
            account: Arc::new(account),
        })
    }

    pub async fn update_password(&self, password: String) -> Result<Self, Error<B>> {
        self.update(
            &ManagerAccountUpdate::builder()
                .with_password(password)
                .build(),
        )
        .await
    }

    pub async fn update_user_name(&self, user_name: String) -> Result<Self, Error<B>> {
        self.update(
            &ManagerAccountUpdate::builder()
                .with_user_name(user_name)
                .build(),
        )
        .await
    }

    pub async fn delete(&self) -> Result<(), Error<B>> {
        self.account
            .delete(self.bmc.as_ref())
            .await
            .map_err(Error::Bmc)
            .map(|_| ())
    }
}

// `AccountTypes` is marked as `Redfish.Required` but some systems
// ignores this requirement. Account service replace it's value with
// 'reasonable' default (see below).
//
// Note quote from schema: "if this property is not provided by the client, the default value
// shall be an array that contains the value `Redfish`".
fn append_default_account_type(v: JsonValue) -> JsonValue {
    if let JsonValue::Object(mut obj) = v {
        obj.entry("AccountTypes")
            .or_insert(JsonValue::Array(vec![JsonValue::String("Redfish".into())]));
        JsonValue::Object(obj)
    } else {
        v
    }
}
