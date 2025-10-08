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

use nv_redfish::ServiceRoot;
use nv_redfish_core::ODataId;
use nv_redfish_tests::Bmc;
use nv_redfish_tests::Expect;
use nv_redfish_tests::ODATA_ID;
use nv_redfish_tests::ODATA_TYPE;
use serde_json::json;
use std::error::Error as StdError;
use std::sync::Arc;
use tokio::test;

#[test]
async fn list_accounts() -> Result<(), Box<dyn StdError>> {
    let bmc = Arc::new(Bmc::default());
    let root_id = ODataId::service_root();
    let account_service_id = format!("{root_id}/AccountService");
    let data_type = "#ServiceRoot.v1_13_0.ServiceRoot";
    let account_service_data_type = "#AccountService.v1_5_0.AccountService";
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
            "Links": {},
        }),
    ));
    let service_root = ServiceRoot::new(bmc.clone()).await?;

    let accounts_id = format!("{account_service_id}/Accounts");
    let accounts_data_type = "#ManagerAccountCollection.ManagerAccountCollection";
    bmc.expect(Expect::get(
        &account_service_id,
        json!({
            ODATA_ID: &account_service_id,
            ODATA_TYPE: &account_service_data_type,
            "Id": "AccountService",
            "Name": "AccountService",
            "Accounts": {
                ODATA_ID: &accounts_id,
            },
        }),
    ));

    let maccount_id = format!("{account_service_id}/Accounts/1");
    let maccount_data_type = "#ManagerAccount.v1_3_0.ManagerAccount";
    let account_service = service_root.account_service().await?;
    bmc.expect(Expect::expand(
        &accounts_id,
        json!({
            ODATA_ID: &accounts_id,
            ODATA_TYPE: &accounts_data_type,
            "Name": "User Accounts",
            "Members": [
                {
                    ODATA_ID: maccount_id,
                    ODATA_TYPE: maccount_data_type,
                    "Id": "1",
                    "Name": "User Account",
                    "UserName": "Administrator",
                    "RoleId": "AdministratorRole",
                    "AccountTypes": []
                }
            ],
        }),
    ));
    let accounts = account_service.accounts().await?.all_accounts().await?;
    assert_eq!(accounts.len(), 1);
    let account = accounts.first().unwrap().raw();
    assert_eq!(account.user_name, Some("Administrator".into()));
    assert_eq!(account.role_id, Some("AdministratorRole".into()));
    assert_eq!(account.base.name, "User Account");
    assert_eq!(account.base.id, "1");
    Ok(())
}
