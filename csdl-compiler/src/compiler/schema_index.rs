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

use crate::compiler::Error;
use crate::compiler::QualifiedName;
use crate::edmx::Edmx;
use crate::edmx::QualifiedTypeName;
use crate::edmx::attribute_values::Namespace;
use crate::edmx::entity_type::EntityType;
use crate::edmx::schema::Schema;
use crate::edmx::schema::Type;
use std::collections::HashMap;

/// Indexing of schema across different documents
pub struct SchemaIndex<'a> {
    index: HashMap<&'a Namespace, &'a Schema>,
    /// Mapping from base entity type to all inherited entity types.
    child_map: HashMap<QualifiedName<'a>, Vec<QualifiedName<'a>>>,
}

impl<'a> SchemaIndex<'a> {
    /// Build index from docs.
    #[must_use]
    pub fn build(edmx_docs: &'a [Edmx]) -> Self {
        Self {
            index: edmx_docs
                .iter()
                .flat_map(|v| v.data_services.schemas.iter().map(|s| (&s.namespace, s)))
                .collect(),
            child_map: edmx_docs.iter().fold(HashMap::new(), |map, doc| {
                doc.data_services.schemas.iter().fold(map, |map, s| {
                    s.types.iter().fold(map, |mut map, (_, t)| match t {
                        Type::EntityType(EntityType {
                            name,
                            base_type: Some(base_type),
                            ..
                        }) => {
                            let qname = QualifiedName::new(&s.namespace, name.inner());
                            let base_type: QualifiedName = base_type.into();
                            map.entry(base_type)
                                .and_modify(|e| e.push(qname))
                                .or_insert_with(|| vec![qname]);
                            map
                        }
                        _ => map,
                    })
                })
            }),
        }
    }

    /// Find schema by namespace.
    #[must_use]
    pub fn get(&self, ns: &Namespace) -> Option<&'a Schema> {
        self.index.get(ns).map(|v| &**v)
    }

    /// Find entity type by type name
    #[must_use]
    pub fn find_entity_type(&self, qtype: &QualifiedTypeName) -> Option<&'a EntityType> {
        self.get(&qtype.inner().namespace).and_then(|ns| {
            if let Some(Type::EntityType(t)) = ns.types.get(&qtype.inner().name) {
                Some(t)
            } else {
                None
            }
        })
    }

    /// Find most specific child.
    ///
    /// # Errors
    ///
    /// Returns error if entity type is ambigous (more than one child exist).
    pub fn find_child_entity_type(
        &self,
        mut qtype: QualifiedName<'a>,
    ) -> Result<(QualifiedName<'a>, &'a EntityType), Error<'a>> {
        while let Some(children) = self.child_map.get(&qtype) {
            if children.len() > 1 {
                return Err(Error::AmbigousHeirarchy(qtype));
            }
            if let Some(child) = children.first() {
                qtype = *child;
            } else {
                break;
            }
        }
        self.get(qtype.namespace)
            .and_then(|ns| {
                if let Some(Type::EntityType(t)) = ns.types.get(qtype.name) {
                    Some(t)
                } else {
                    None
                }
            })
            // This should never happen.
            .ok_or(Error::EntityTypeNotFound(qtype))
            .map(|v| (qtype, v))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::edmx::Edmx;

    #[test]
    fn schema_index_test() {
        let schemas = [
            r#"<edmx:Edmx Version="4.0">
             <edmx:DataServices>
               <Schema Namespace="Schema.v1_0_0"/>
             </edmx:DataServices>
           </edmx:Edmx>"#,
            // Two schemas per document
            r#"<edmx:Edmx Version="4.0">
             <edmx:DataServices>
               <Schema Namespace="Schema.v1_1_0"/>
               <Schema Namespace="Schema.v1_2_0"/>
             </edmx:DataServices>
           </edmx:Edmx>"#,
        ]
        .iter()
        .map(|s| Edmx::parse(*s))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

        let index = SchemaIndex::build(&schemas);
        assert!(index.get(&"Schema.v1_1_0".parse().unwrap()).is_some());
        assert!(index.get(&"Schema.v1_0_0".parse().unwrap()).is_some());
        assert!(index.get(&"Schema.v1_2_0".parse().unwrap()).is_some());
        assert!(index.get(&"Schema.v1_3_0".parse().unwrap()).is_none());
    }
}
