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

use nv_redfish_core::Bmc;
use std::error::Error as StdError;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

pub enum Error<B: Bmc> {
    Bmc(B::Error),
    #[cfg(feature = "accounts")]
    AccountServiceNotSupported,
}

impl<B: Bmc> Display for Error<B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Bmc(err) => write!(f, "BMC error: {err}"),
            #[cfg(feature = "accounts")]
            Self::AccountServiceNotSupported => {
                write!(f, "Account service is not supported by system")
            }
        }
    }
}

impl<B: Bmc> Debug for Error<B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(self, f)
    }
}

impl<B: Bmc> StdError for Error<B> {}
