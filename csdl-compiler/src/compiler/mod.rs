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

//! Compiler of multiple schemas

/// Index of schemas
pub mod schema_index;

/// Compilation stack
pub mod stack;

/// Error diagnostics
pub mod error;

/// Compiled schema bundle
pub mod compiled;

/// Qualified name
pub mod qualified_name;

/// Compiled namespace
pub mod namespace;

/// Compiled odata
pub mod odata;

/// Compiled redfish attrs
pub mod redfish;

/// Traits that are useful for compilation.
pub mod compile_traits;

/// Compiled properties of `ComplexType` or `EntityType`
pub mod compiled_properties;

/// Simple type (type definition or enum)
pub mod simple_type;

/// Compiled entity type
pub mod compiled_entity_type;

/// Compiled complex type
pub mod compiled_complex_type;

/// Compiled singleton
pub mod compiled_singleton;

use crate::compiler::odata::MustHaveId;
use crate::edmx::Edmx;
use crate::edmx::QualifiedTypeName;
use crate::edmx::attribute_values::SimpleIdentifier;
use crate::edmx::attribute_values::TypeName;
use crate::edmx::schema::Schema;
use crate::edmx::schema::Type;
use schema_index::SchemaIndex;
use stack::Stack;

/// Reexport `Compiled` to the level of the compiler.
pub type Compiled<'a> = compiled::Compiled<'a>;
/// Reexport `Error` to the level of the compiler.
pub type Error<'a> = error::Error<'a>;
/// Reexport `QualifiedName` to the level of the compiler.
pub type QualifiedName<'a> = qualified_name::QualifiedName<'a>;
/// Reexport `CompiledNamespace` to the level of the compiler.
pub type CompiledNamespace<'a> = namespace::CompiledNamespace<'a>;
/// Reexport `CompiledOData` to the level of the compiler.
pub type CompiledOData<'a> = odata::CompiledOData<'a>;
/// Reexport `CompiledProperties` to the level of the compiler.
pub type CompiledProperties<'a> = compiled_properties::CompiledProperties<'a>;
/// Reexport `CompiledProperty` to the level of the compiler.
pub type CompiledProperty<'a> = compiled_properties::CompiledProperty<'a>;
/// Reexport `CompiledNavProperty` to the level of the compiler.
pub type CompiledNavProperty<'a> = compiled_properties::CompiledNavProperty<'a>;
/// Reexport `CompiledPropertyType` to the level of the compiler.
pub type CompiledPropertyType<'a> = compiled_properties::CompiledPropertyType<'a>;
/// Reexport `SimpleType` to the level of the compiler.
pub type SimpleType<'a> = simple_type::SimpleType<'a>;
/// Reexport `SimpleTypeAttrs` to the level of the compiler.
pub type SimpleTypeAttrs<'a> = simple_type::SimpleTypeAttrs<'a>;
/// Reexport `CompiledTypeDefinition` to the level of the compiler.
pub type CompiledTypeDefinition<'a> = simple_type::CompiledTypeDefinition<'a>;
/// Reexport `CompiledEnumType` to the level of the compiler.
pub type CompiledEnumType<'a> = simple_type::CompiledEnumType<'a>;
/// Reexport `CompiledEntityType` to the level of the compiler.
pub type CompiledEntityType<'a> = compiled_entity_type::CompiledEntityType<'a>;
/// Reexport `CompiledComplexType` to the level of the compiler.
pub type CompiledComplexType<'a> = compiled_complex_type::CompiledComplexType<'a>;
/// Reexport `CompiledComplexType` to the level of the compiler.
pub type CompiledSingleton<'a> = compiled_singleton::CompiledSingleton<'a>;

/// Reexport `MapBase` to the level of the compiler.
pub use compile_traits::MapBase;
/// Reexport `MapType` to the level of the compiler.
pub use compile_traits::MapType;
/// Reexport `PropertiesManipulation` to the level of the compiler.
pub use compile_traits::PropertiesManipulation;

/// Collection of EDMX documents that are compiled together to produce
/// code.
#[derive(Default)]
pub struct SchemaBundle {
    /// Parsed and validated Edmx documents.
    pub edmx_docs: Vec<Edmx>,
}

impl SchemaBundle {
    /// Compile multiple schema, resolving all type dependencies.
    ///
    /// # Errors
    ///
    /// Returns compile error if any type cannot be resolved.
    pub fn compile(&self, singletons: &[SimpleIdentifier]) -> Result<Compiled<'_>, Error> {
        let schema_index = SchemaIndex::build(&self.edmx_docs);
        let stack = Stack::default();
        self.edmx_docs
            .iter()
            .try_fold(stack, |stack, edmx| {
                let cstack = stack.new_frame();
                let compiled = edmx
                    .data_services
                    .schemas
                    .iter()
                    .try_fold(cstack, |stack, s| {
                        Self::compile_schema(s, singletons, &schema_index, stack.new_frame())
                            .map(|v| stack.merge(v))
                    })?
                    .done();
                Ok(stack.merge(compiled))
            })
            .map(Stack::done)
    }

    fn compile_schema<'a>(
        s: &'a Schema,
        singletons: &[SimpleIdentifier],
        schema_index: &SchemaIndex<'a>,
        stack: Stack<'a, '_>,
    ) -> Result<Compiled<'a>, Error<'a>> {
        s.entity_container.as_ref().map_or_else(
            || Ok(Compiled::default()),
            |entity_container| {
                entity_container
                    .singletons
                    .iter()
                    .try_fold(stack, |stack, s| {
                        if singletons.contains(&s.name) {
                            CompiledSingleton::compile(s, schema_index, &stack)
                                .map(|v| stack.merge(v))
                        } else {
                            Ok(stack)
                        }
                    })
                    .map_err(Box::new)
                    .map_err(|e| Error::Schema(&s.namespace, e))
                    .map(Stack::done)
            },
        )
    }
}

fn is_simple_type(qtype: &QualifiedTypeName) -> bool {
    qtype.inner().namespace.is_edm()
}

fn ensure_type<'a>(
    typename: &'a TypeName,
    schema_index: &SchemaIndex<'a>,
    stack: &Stack<'a, '_>,
) -> Result<Compiled<'a>, Error<'a>> {
    let qtype = match typename {
        TypeName::One(v) | TypeName::CollectionOf(v) => v,
    };
    if stack.contains_entity(qtype.into()) || is_simple_type(qtype) {
        Ok(Compiled::default())
    } else {
        compile_type(qtype, schema_index, stack)
    }
}

fn compile_type<'a>(
    qtype: &'a QualifiedTypeName,
    schema_index: &SchemaIndex<'a>,
    stack: &Stack<'a, '_>,
) -> Result<Compiled<'a>, Error<'a>> {
    schema_index
        .find_type(qtype)
        .ok_or_else(|| Error::TypeNotFound(qtype.into()))
        .and_then(|t| match t {
            Type::TypeDefinition(td) => {
                let underlying_type = (&td.underlying_type).into();
                if is_simple_type(&td.underlying_type) {
                    Ok(Compiled::new_type_definition(CompiledTypeDefinition {
                        name: qtype.into(),
                        underlying_type,
                    }))
                } else {
                    Err(Error::TypeDefinitionOfNotPrimitiveType(underlying_type))
                }
            }
            Type::EnumType(et) => {
                let underlying_type = et.underlying_type.unwrap_or_default();
                Ok(Compiled::new_enum_type(CompiledEnumType {
                    name: qtype.into(),
                    underlying_type,
                    members: et.members.iter().map(Into::into).collect(),
                    odata: CompiledOData::new(MustHaveId::new(false), et),
                }))
            }
            Type::ComplexType(ct) => {
                let name = qtype.into();
                // Ensure that base entity type compiled if present.
                let (base, compiled) = if let Some(base_type) = &ct.base_type {
                    let compiled = compile_type(base_type, schema_index, stack)?;
                    (Some(base_type.into()), compiled)
                } else {
                    (None, Compiled::default())
                };

                let stack = stack.new_frame().merge(compiled);

                let (compiled, properties) =
                    CompiledProperties::compile(&ct.properties, schema_index, stack.new_frame())?;

                Ok(stack
                    .merge(compiled)
                    .merge(Compiled::new_complex_type(CompiledComplexType {
                        name,
                        base,
                        properties,
                        odata: CompiledOData::new(MustHaveId::new(false), ct),
                    }))
                    .done())
            }
        })
        .map_err(Box::new)
        .map_err(|e| Error::Type(qtype.into(), e))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::edmx::Edmx;

    #[test]
    fn schema_test() {
        let schema = r#"<edmx:Edmx Version="4.0">
             <edmx:DataServices>
               <Schema xmlns="http://docs.oasis-open.org/odata/ns/edm" Namespace="Resource">
                 <EntityType Name="ItemOrCollection" Abstract="true"/>
                 <EntityType Name="Item" BaseType="Resource.ItemOrCollection" Abstract="true"/>
                 <EntityType Name="Resource" BaseType="Resource.Item" Abstract="true"/>
               </Schema>
               <Schema xmlns="http://docs.oasis-open.org/odata/ns/edm" Namespace="Resource.v1_0_0">
                 <EntityType Name="Resource" BaseType="Resource.Resource" Abstract="true">
                   <Key><PropertyRef Name="Id"/></Key>
                 </EntityType>
               </Schema>
               <Schema xmlns="http://docs.oasis-open.org/odata/ns/edm" Namespace="ServiceRoot">
                 <EntityType Name="ServiceRoot" BaseType="Resource.v1_0_0.Resource" Abstract="true">
                   <Property Name="RedfishVersion" Type="Edm.String" Nullable="false">
                     <Annotation Term="OData.Description" String="The version of the Redfish service."/>
                   </Property>
                 </EntityType>
               </Schema>
               <Schema Namespace="Schema.v1_0_0">
                 <EntityContainer Name="ServiceContainer">
                   <Singleton Name="Service" Type="ServiceRoot.ServiceRoot"/>
                 </EntityContainer>
                 <EntityType Name="ServiceRoot" BaseType="ServiceRoot.ServiceRoot"/>
               </Schema>
             </edmx:DataServices>
           </edmx:Edmx>"#;
        let bundle = SchemaBundle {
            edmx_docs: vec![Edmx::parse(schema).unwrap()],
        };
        let compiled = bundle.compile(&["Service".parse().unwrap()]).unwrap();
        assert_eq!(compiled.root_singletons.len(), 1);
        let mut cur_type = &compiled.root_singletons.first().unwrap().stype;
        loop {
            let et = compiled.entity_types.get(cur_type).unwrap();
            cur_type = if let Some(t) = &et.base { t } else { break };
        }
        let qtype: QualifiedTypeName = "ServiceRoot.ServiceRoot".parse().unwrap();
        let et = compiled.entity_types.get(&(&qtype).into()).unwrap();
        assert_eq!(et.properties.properties.len(), 1);
        assert_eq!(
            et.properties.properties[0]
                .odata
                .description
                .as_ref()
                .unwrap()
                .inner(),
            &"The version of the Redfish service."
        );
    }
}
