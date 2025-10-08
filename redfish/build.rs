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

use csdl_compiler::commands::process_command;
use csdl_compiler::commands::Commands;
use csdl_compiler::commands::DEFAULT_ROOT;
use csdl_compiler::features_manifest::FeaturesManifest;
use std::env::var;
use std::error::Error as StdError;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn StdError>> {
    let features_manifest = PathBuf::from("features.toml");
    let manifest = FeaturesManifest::read(&features_manifest)?;
    println!("cargo:rerun-if-changed={}", features_manifest.display());
    // Collect features that is defined by configuration
    let target_features = manifest
        .all_features()
        .into_iter()
        .filter(|f| var(format!("CARGO_FEATURE_{}", f.to_uppercase())).is_ok())
        .collect::<Vec<_>>();

    let out_dir = PathBuf::from(var("OUT_DIR").unwrap());
    let output = out_dir.join("redfish.rs");
    let schema_path = "../schemas/redfish-csdl";
    let service_root = vec![
        "Resource_v1.xml",
        "ResolutionStep_v1.xml",
        "ServiceRoot_v1.xml",
    ]
    .into_iter()
    .map(Into::into)
    .collect::<Vec<String>>();
    let service_root_pattens = vec!["ServiceRoot.*.*"]
        .into_iter()
        .map(|v| v.parse())
        .collect::<Result<Vec<_>, _>>()
        .expect("must be successfuly parsed");
    let (features_csdls, features_patterns) = manifest.collect(&target_features);
    let csdls = service_root
        .iter()
        .chain(features_csdls)
        .map(|f| format!("{schema_path}/{f}"))
        .collect::<Vec<_>>();

    for f in &csdls {
        println!("cargo:rerun-if-changed={f}");
    }

    process_command(&Commands::Compile {
        root: DEFAULT_ROOT.into(),
        output,
        csdls,
        entity_type_patterns: service_root_pattens
            .iter()
            .chain(features_patterns)
            .cloned()
            .collect(),
    })?;
    Ok(())
}
