#![cfg(feature = "cbor")]
// CBOR Encode functions
//
// Because we have all the type information for declared types,
// we can invoke the appropriate encode_* functions for each
// simple type, structure, map, and array. (later: enums).
// If we had leveraged minicbor's Encoder trait , we could have let
// the rust compiler do the work to invoke nested encoders for complex types.
// Instead, we generate encode_* functions for each non-simple data type
// and call them directly. It is hoped that this will simplify ports to other
// target languages, if those languages don't have traits, and the cbor libraries
// in those languages can't #[derive] encoders and decoders for arbitrary
// structures.

// The encoder is written as a plain function "encode_<S>" where S is the type name
// (camel cased for the fn name), and scoped to the module where S is defined.
use crate::codegen_rust::RustCodeGen;
use crate::{
    codegen_rust::is_optional_type,
    error::{Error, Result},
    gen::CodeGen,
    model::wasmcloud_model_namespace,
    writer::Writer,
};
use atelier_core::model::shapes::ShapeKind;
use atelier_core::{
    model::{
        shapes::{HasTraits, Simple, StructureOrUnion},
        HasIdentity, ShapeID,
    },
    prelude::{
        prelude_namespace_id, SHAPE_BIGDECIMAL, SHAPE_BIGINTEGER, SHAPE_BLOB, SHAPE_BOOLEAN,
        SHAPE_BYTE, SHAPE_DOCUMENT, SHAPE_DOUBLE, SHAPE_FLOAT, SHAPE_INTEGER, SHAPE_LONG,
        SHAPE_PRIMITIVEBOOLEAN, SHAPE_PRIMITIVEBYTE, SHAPE_PRIMITIVEDOUBLE, SHAPE_PRIMITIVEFLOAT,
        SHAPE_PRIMITIVEINTEGER, SHAPE_PRIMITIVELONG, SHAPE_PRIMITIVESHORT, SHAPE_SHORT,
        SHAPE_STRING, SHAPE_TIMESTAMP,
    },
};
use std::string::ToString;

type IsEmptyStruct = bool;

#[derive(Clone, Copy)]
pub(crate) enum ValExpr<'s> {
    Plain(&'s str),
    Ref(&'s str),
}
impl<'s> ValExpr<'s> {
    /// returns borrowed reference to value
    pub(crate) fn as_ref(&self) -> String {
        match self {
            ValExpr::Plain(s) => format!("&{}", s),
            ValExpr::Ref(s) => s.to_string(),
        }
    }

    /// returns value as-is
    pub(crate) fn as_str(&self) -> &str {
        match self {
            ValExpr::Plain(s) => s,
            ValExpr::Ref(s) => s,
        }
    }

    /// returns value for copyable types
    pub(crate) fn as_copy(&self) -> String {
        match self {
            ValExpr::Plain(s) => s.to_string(),
            ValExpr::Ref(s) => format!("*{}", s),
        }
    }
}

// encode_* methods encode a base/simple type

fn encode_blob(val: ValExpr) -> String {
    format!("e.bytes({})?;\n", &val.as_ref())
}
fn encode_boolean(val: ValExpr) -> String {
    format!("e.bool({})?;\n", val.as_copy())
}
fn encode_str(val: ValExpr) -> String {
    format!("e.str({})?;\n", val.as_ref())
}
fn encode_byte(val: ValExpr) -> String {
    format!("e.i8({})?;\n", val.as_copy())
}
fn encode_unsigned_byte(val: ValExpr) -> String {
    format!("e.u8({})?;\n", val.as_copy())
}
fn encode_short(val: ValExpr) -> String {
    format!("e.i16({})?;\n", val.as_copy())
}
fn encode_unsigned_short(val: ValExpr) -> String {
    format!("e.u16({})?;\n", val.as_copy())
}
fn encode_integer(val: ValExpr) -> String {
    format!("e.i32({})?;\n", val.as_copy())
}
fn encode_unsigned_integer(val: ValExpr) -> String {
    format!("e.u32({})?;\n", val.as_copy())
}
fn encode_long(val: ValExpr) -> String {
    format!("e.i64({})?;\n", val.as_copy())
}
fn encode_unsigned_long(val: ValExpr) -> String {
    format!("e.u64({})?;\n", val.as_copy())
}
fn encode_float(val: ValExpr) -> String {
    format!("e.f32({})?;\n", val.as_str())
}
fn encode_double(val: ValExpr) -> String {
    format!("e.f64({})?;\n", val.as_str())
}
fn encode_document(val: ValExpr) -> String {
    format!("e.bytes({})?;\n", val.as_ref())
}
fn encode_timestamp(_val: ValExpr) -> String {
    todo!(); // tag timestamp
}
fn encode_big_integer(_val: ValExpr) -> String {
    todo!(); // tag big int
}
fn encode_big_decimal(_val: ValExpr) -> String {
    todo!() // tag big decimal
}

impl<'model> RustCodeGen<'model> {
    /// Generates cbor encode statements "e.func()" for the id.
    /// If id is a primitive type, writes the direct encode function, otherwise,
    /// delegates to an encode_* function created in the same module where the symbol is defined
    pub(crate) fn encode_shape_id(&self, id: &ShapeID, val: ValExpr) -> Result<String> {
        let name = id.shape_name().to_string();
        let stmt = if id.namespace() == prelude_namespace_id() {
            match name.as_ref() {
                SHAPE_BLOB => encode_blob(val),
                SHAPE_BOOLEAN | SHAPE_PRIMITIVEBOOLEAN => encode_boolean(val),
                SHAPE_STRING => encode_str(val),
                SHAPE_BYTE | SHAPE_PRIMITIVEBYTE => encode_byte(val),
                SHAPE_SHORT | SHAPE_PRIMITIVESHORT => encode_short(val),
                SHAPE_INTEGER | SHAPE_PRIMITIVEINTEGER => encode_integer(val),
                SHAPE_LONG | SHAPE_PRIMITIVELONG => encode_long(val),
                SHAPE_FLOAT | SHAPE_PRIMITIVEFLOAT => encode_float(val),
                SHAPE_DOUBLE | SHAPE_PRIMITIVEDOUBLE => encode_double(val),
                SHAPE_TIMESTAMP => encode_timestamp(val),
                SHAPE_BIGINTEGER => encode_big_integer(val),
                SHAPE_BIGDECIMAL => encode_big_decimal(val),
                SHAPE_DOCUMENT => encode_document(val),
                _ => return Err(Error::UnsupportedType(name)),
            }
        } else if id.namespace() == wasmcloud_model_namespace() {
            match name.as_bytes() {
                b"U64" => encode_unsigned_long(val),
                b"U32" => encode_unsigned_integer(val),
                b"U16" => encode_unsigned_short(val),
                b"U8" => encode_unsigned_byte(val),
                b"I64" => encode_long(val),
                b"I32" => encode_integer(val),
                b"I16" => encode_short(val),
                b"I8" => encode_byte(val),
                _ => {
                    let mut s = String::new();
                    if self.namespace.is_none()
                        || self.namespace.as_ref().unwrap() != wasmcloud_model_namespace()
                    {
                        s.push_str(&self.import_core);
                        s.push_str("::model::");
                    }
                    s.push_str(&format!(
                        "encode_{}(e, {})?;\n",
                        crate::strings::to_snake_case(&id.shape_name().to_string()),
                        val.as_ref()
                    ));
                    s
                }
            }
        } else if self.namespace.is_some() && id.namespace() == self.namespace.as_ref().unwrap() {
            format!(
                "encode_{}(e, {})?;\n",
                crate::strings::to_snake_case(&id.shape_name().to_string()),
                val.as_ref()
            )
        } else {
            match self.packages.get(&id.namespace().to_string()) {
                Some(crate::model::PackageName {
                    crate_name: Some(crate_name),
                    ..
                }) => {
                    // the crate name should be valid rust syntax. If not, they'll get an error with rustc
                    format!(
                        "{}::encode_{}(e, {})?;\n",
                        &crate_name,
                        crate::strings::to_snake_case(&id.shape_name().to_string()),
                        val.as_ref(),
                    )
                }
                _ => {
                    return Err(Error::Model(format!("undefined crate for namespace {} for symbol {}. Make sure codegen.toml includes all dependent namespaces, and that the dependent .smithy file contains package metadata with crate: value",
                                                    &id.namespace(), &id)));
                }
            }
        };
        Ok(stmt)
    }

    /// Generates statements to encode the shape.
    /// Second Result field is true if structure has no fields, e.g., "MyStruct {}"
    fn encode_shape_kind(
        &self,
        id: &ShapeID,
        kind: &ShapeKind,
        val: ValExpr,
    ) -> Result<(String, IsEmptyStruct)> {
        let mut empty_struct: IsEmptyStruct = false;
        let s = match kind {
            ShapeKind::Simple(simple) => match simple {
                Simple::Blob => encode_blob(val),
                Simple::Boolean => encode_boolean(val),
                Simple::String => encode_str(val),
                Simple::Byte => encode_byte(val),
                Simple::Short => encode_short(val),
                Simple::Integer => encode_integer(val),
                Simple::Long => encode_long(val),
                Simple::Float => encode_float(val),
                Simple::Double => encode_double(val),
                Simple::Timestamp => encode_timestamp(val),
                Simple::BigInteger => encode_big_integer(val),
                Simple::BigDecimal => encode_big_decimal(val),
                Simple::Document => encode_document(val),
            },
            ShapeKind::Map(map) => {
                let mut s = format!(
                    r#"
                    e.map({}.len() as u64)?;
                    for (k,v) in {} {{
                    "#,
                    val.as_str(),
                    val.as_str()
                );
                s.push_str(&self.encode_shape_id(map.key().target(), ValExpr::Ref("k"))?);
                s.push_str(&self.encode_shape_id(map.value().target(), ValExpr::Ref("v"))?);
                s.push_str(
                    r#"
                    }
                    "#,
                );
                s
            }
            ShapeKind::List(list) => {
                let mut s = format!(
                    r#"
                    e.array({}.len() as u64)?;
                    for item in {}.iter() {{
                    "#,
                    val.as_str(),
                    val.as_str()
                );
                s.push_str(&self.encode_shape_id(list.member().target(), ValExpr::Ref("item"))?);
                s.push_str(
                    r#"
                    }
                    "#,
                );
                s
            }
            ShapeKind::Set(set) => {
                let mut s = format!(
                    r#"
                    e.array({}.len() as u64)?;
                    for v in {}.iter() {{
                    "#,
                    val.as_str(),
                    val.as_str()
                );
                s.push_str(&self.encode_shape_id(set.member().target(), ValExpr::Ref("v"))?);
                s.push_str(
                    r#"
                    }
                    "#,
                );
                s
            }
            ShapeKind::Structure(struct_) => {
                let (s, is_empty_struct) = self.encode_struct(id, struct_, val)?;
                empty_struct = is_empty_struct;
                s
            }
            ShapeKind::Operation(_)
            | ShapeKind::Resource(_)
            | ShapeKind::Service(_)
            | ShapeKind::Unresolved => String::new(),

            ShapeKind::Union(_) => {
                unimplemented!();
            }
        };
        Ok((s, empty_struct))
    }

    /// Generate string to encode structure.
    /// Second Result field is true if structure has no fields, e.g., "MyStruct {}"
    fn encode_struct(
        &self,
        id: &ShapeID,
        strukt: &StructureOrUnion,
        val: ValExpr,
    ) -> Result<(String, IsEmptyStruct)> {
        let (fields, is_numbered) = crate::model::get_sorted_fields(id.shape_name(), strukt)?;
        // use array encoding if fields are declared with numbers
        let as_array = is_numbered;
        let field_max_index = if as_array && !fields.is_empty() {
            fields.iter().map(|f| f.field_num().unwrap()).max().unwrap()
        } else {
            fields.len() as u16
        };
        let mut s = String::new();
        if as_array {
            s.push_str(&format!("e.array({})?;\n", field_max_index + 1));
        } else {
            s.push_str(&format!("e.map({})?;\n", fields.len()));
        }
        let mut current_index = 0;
        for field in fields.iter() {
            if let Some(field_num) = field.field_num() {
                if as_array {
                    while current_index < *field_num {
                        s.push_str("e.null()?;\n");
                        current_index += 1;
                    }
                }
            }
            let field_name = self.to_field_name(field.id(), field.traits())?;
            let field_val = self.encode_shape_id(field.target(), ValExpr::Ref("val"))?;
            if is_optional_type(field) {
                s.push_str(&format!(
                    "if let Some(val) =  {}.{}.as_ref() {{\n",
                    val.as_str(),
                    &field_name
                ));
                if !as_array {
                    // map key is declared name, not target language name
                    s.push_str(&format!("e.str(\"{}\")?;\n", field.id()));
                }
                s.push_str(&field_val);
                s.push_str("} else { e.null()?; }\n");
            } else {
                if !as_array {
                    // map key is declared name, not target language name
                    s.push_str(&format!("e.str(\"{}\")?;\n", field.id()));
                }
                let val = format!("{}.{}", val.as_str(), &field_name);
                s.push_str(&self.encode_shape_id(field.target(), ValExpr::Plain(&val))?);
            }
            current_index += 1;
        }
        Ok((s, fields.is_empty()))
    }

    pub(crate) fn declare_shape_encoder(
        &self,
        w: &mut Writer,
        id: &ShapeID,
        kind: &ShapeKind,
    ) -> Result<()> {
        // The encoder is written as a plain function "encode_<S>" where S is the type name
        // (camel cased for the fn name), and scoped to the module where S is defined. This could
        // have been implemented as 'impl Encode for TYPE ...', but that would make the code more
        // rust-specific. This code is structured to be easier to port to other target languages.
        match kind {
            ShapeKind::Simple(_)
            | ShapeKind::Structure(_)
            | ShapeKind::Map(_)
            | ShapeKind::List(_)
            | ShapeKind::Set(_) => {
                let name = id.shape_name();
                // use val-by-copy as param to encode if type is rust primitive "copy" type
                // This is only relevant for aliases of primitive types in wasmbus-model namespace
                let is_rust_copy = vec!["U8", "I8", "U16", "I16", "U32", "I32", "U64", "I64"]
                    .contains(&name.to_string().as_str());
                //let val_or_copy = if is_rust_copy { "*val" } else { "val" };
                // The purpose of is_empty_struct is to determine when the parameter is unused
                // in the function body, and append '_' to the name to avoid a compiler warning.
                let (body, is_empty_struct) =
                    self.encode_shape_kind(id, kind, ValExpr::Ref("val"))?;
                let mut s = format!(
                    r#" 
                // Encode {} as CBOR and append to output stream
                // This is part of experimental cbor support
                #[doc(hidden)] {}
                pub fn encode_{}<W>(e: &mut minicbor::Encoder<W>, {}: &{}) -> Result<(),minicbor::encode::Error<W::Error>>
                where
                    W: minicbor::encode::Write,
                {{
                "#,
                    &name,
                    if is_rust_copy { "#[inline]" } else { "" },
                    crate::strings::to_snake_case(&name.to_string()),
                    if is_empty_struct { "_val" } else { "val" },
                    &id.shape_name()
                );
                s.push_str(&body);
                s.push_str("Ok(())\n}\n");
                w.write(s.as_bytes());
            }
            ShapeKind::Operation(_)
            | ShapeKind::Resource(_)
            | ShapeKind::Service(_)
            | ShapeKind::Union(_)
            | ShapeKind::Unresolved => { /* write nothing */ }
        }
        Ok(())
    }
}
