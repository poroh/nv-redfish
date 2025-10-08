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

//! Integration tests of Account Service.

use nv_redfish::accounts::AccountCollection;
use nv_redfish::accounts::AccountService;
use nv_redfish::accounts::AccountTypes;
use nv_redfish::ServiceRoot;
use nv_redfish_core::ODataId;
use nv_redfish_tests::Bmc;
use nv_redfish_tests::Expect;
use nv_redfish_tests::ODATA_ID;
use nv_redfish_tests::ODATA_TYPE;
use serde_json::json;
use serde_json::Value as JsonValue;
use std::error::Error as StdError;
use std::sync::Arc;
use tokio::test;

const ACCOUNT_SERVICE_DATA_TYPE: &str = "#AccountService.v1_5_0.AccountService";
const ACCOUNTS_DATA_TYPE: &str = "#ManagerAccountCollection.ManagerAccountCollection";
const MANAGER_ACCOUNT_DATA_TYPE: &str = "#ManagerAccount.v1_3_0.ManagerAccount";

#[test]
async fn list_accounts() -> Result<(), Box<dyn StdError>> {
    let bmc = Arc::new(Bmc::default());
    let root_id = ODataId::service_root();
    let account_service = get_account_service(bmc.clone(), &root_id, "Contoso").await?;
    let maccount_id = format!("{}/Accounts/1", account_service.odata_id());
    let accounts = get_account_collection(
        bmc.clone(),
        &account_service,
        json! {[{
            ODATA_ID: maccount_id,
            ODATA_TYPE: MANAGER_ACCOUNT_DATA_TYPE,
            "Id": "1",
            "Name": "User Account",
            "UserName": "Administrator",
            "RoleId": "AdministratorRole",
            "AccountTypes": []
        }]},
    )
    .await?;
    let accounts = accounts.all_accounts().await?;
    assert_eq!(accounts.len(), 1);
    let account = accounts.first().unwrap().raw();
    assert_eq!(account.user_name, Some("Administrator".into()));
    assert_eq!(account.role_id, Some("AdministratorRole".into()));
    assert_eq!(account.base.name, "User Account");
    assert_eq!(account.base.id, "1");
    Ok(())
}

#[test]
async fn list_hpe_accounts() -> Result<(), Box<dyn StdError>> {
    let bmc = Arc::new(Bmc::default());
    let root_id = ODataId::service_root();
    let account_service = get_account_service(bmc.clone(), &root_id, "HPE").await?;
    let maccount_id = format!("{}/Accounts/1", account_service.odata_id());
    let accounts = get_account_collection(
        bmc.clone(),
        &account_service,
        json! {[{
            ODATA_ID: maccount_id,
            ODATA_TYPE: MANAGER_ACCOUNT_DATA_TYPE,
            "Id": "1",
            "Name": "User Account",
            "UserName": "Administrator",
            "RoleId": "AdministratorRole",
        }]},
    )
    .await?;
    let accounts = accounts.all_accounts().await?;
    assert_eq!(accounts.len(), 1);
    let account = accounts.first().unwrap().raw();
    assert_eq!(account.user_name, Some("Administrator".into()));
    assert_eq!(account.account_types, vec![AccountTypes::Redfish]);
    Ok(())
}

#[test]
async fn list_no_patch_accounts() -> Result<(), Box<dyn StdError>> {
    let bmc = Arc::new(Bmc::default());
    let root_id = ODataId::service_root();
    let account_service = get_account_service(bmc.clone(), &root_id, "Contoso").await?;
    let maccount_id = format!("{}/Accounts/1", account_service.odata_id());
    assert!(get_account_collection(
        bmc.clone(),
        &account_service,
        json! {[{
            ODATA_ID: maccount_id,
            ODATA_TYPE: MANAGER_ACCOUNT_DATA_TYPE,
            "Id": "1",
            "Name": "User Account",
            "UserName": "Administrator",
            "RoleId": "AdministratorRole",
        }]},
    )
    .await
    .is_err());
    Ok(())
}

async fn get_account_service(
    bmc: Arc<Bmc>,
    root_id: &ODataId,
    vendor: &str,
) -> Result<AccountService<Bmc>, Box<dyn StdError>> {
    let account_service_id = format!("{root_id}/AccountService");
    let data_type = "#ServiceRoot.v1_13_0.ServiceRoot";
    bmc.expect(Expect::get(
        &root_id,
        json!({
            ODATA_ID: &root_id,
            ODATA_TYPE: &data_type,
            "Id": "RootService",
            "Name": "RootService",
            "AccountService": {
                ODATA_ID: &account_service_id,
            },
            "Vendor": vendor,
            "Links": {},
        }),
    ));
    let service_root = ServiceRoot::new(bmc.clone()).await?;

    let accounts_id = format!("{account_service_id}/Accounts");
    bmc.expect(Expect::get(
        &account_service_id,
        json!({
            ODATA_ID: &account_service_id,
            ODATA_TYPE: &ACCOUNT_SERVICE_DATA_TYPE,
            "Id": "AccountService",
            "Name": "AccountService",
            "Accounts": {
                ODATA_ID: &accounts_id,
            },
        }),
    ));
    Ok(service_root.account_service().await?)
}

async fn get_account_collection(
    bmc: Arc<Bmc>,
    account_service: &AccountService<Bmc>,
    members: JsonValue,
) -> Result<AccountCollection<Bmc>, Box<dyn StdError>> {
    let accounts_id = format!("{}/Accounts", account_service.odata_id());
    bmc.expect(Expect::expand(
        &accounts_id,
        json!({
            ODATA_ID: &accounts_id,
            ODATA_TYPE: &ACCOUNTS_DATA_TYPE,
            "Name": "User Accounts",
            "Members": members,
        }),
    ));
    Ok(account_service.accounts().await?)
}
