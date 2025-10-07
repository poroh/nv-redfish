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

use crate::compiler::EntityTypeFilterPattern;
use serde::Deserialize;
use std::error::Error as StdError;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::fs::File;
use std::io::Error as IoError;
use std::io::Read as _;
use std::path::PathBuf;
use toml::de::Error as TomlError;

/// Manifest that defines features schema compilation.
#[derive(Deserialize, Debug)]
pub struct FeaturesManifest {
    pub features: Vec<Feature>,
}

impl FeaturesManifest {
    /// Read features manifest from toml file.
    ///
    /// # Errors
    ///
    /// - `Error::Io` if failed to read file
    /// - `Error::Toml` if failed to parse content as TOML / invalid features manifest.
    pub fn read(fname: &PathBuf) -> Result<Self, Error> {
        let mut file = File::open(fname).map_err(Error::Io)?;
        let mut content = String::new();
        file.read_to_string(&mut content).map_err(Error::Io)?;
        toml::from_str(&content).map_err(Error::Toml)
    }

    /// All features that defined in manifest.
    #[must_use]
    pub fn all_features(&self) -> Vec<&String> {
        self.features.iter().map(|f| &f.name).collect()
    }

    /// Collect CSDLs and patterns to be compiled.
    #[must_use]
    pub fn collect<'a>(
        &'a self,
        features: &[&String],
    ) -> (Vec<&'a String>, Vec<&'a EntityTypeFilterPattern>) {
        self.features
            .iter()
            .fold((Vec::new(), Vec::new()), |(mut files, mut patterns), f| {
                if features.contains(&&f.name) {
                    files.extend(f.csdl_files.iter());
                    patterns.extend(f.patterns.iter());
                }
                (files, patterns)
            })
    }
}

#[derive(Deserialize, Debug)]
pub struct Feature {
    pub name: String,
    pub csdl_files: Vec<String>,
    pub patterns: Vec<EntityTypeFilterPattern>,
}

#[derive(Debug)]
pub enum Error {
    Io(IoError),
    Toml(TomlError),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Io(err) => write!(f, "input/output error: {err}"),
            Self::Toml(err) => write!(f, "manifest file format error: {err}"),
        }
    }
}

impl StdError for Error {}
