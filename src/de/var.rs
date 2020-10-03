use std::{borrow::BorrowMut, io::Read};

use serde::de::{self, Deserializer as SerdeDeserializer, IntoDeserializer};
use xml::name::OwnedName;
use xml::reader::XmlEvent;

use de::Deserializer;
use error::{Error, Result};

use super::{buffer::BufferedXmlReader, DeserializerState};

pub struct EnumAccess<'a, R: 'a + Read, B: BufferedXmlReader<R>, S: BorrowMut<DeserializerState>> {
    de: &'a mut Deserializer<R, B, S>,
}

impl<'a, R: 'a + Read, B: BufferedXmlReader<R>, S: BorrowMut<DeserializerState>>
    EnumAccess<'a, R, B, S>
{
    pub fn new(de: &'a mut Deserializer<R, B, S>) -> Self {
        EnumAccess { de: de }
    }
}

impl<'de, 'a, R: 'a + Read, B: BufferedXmlReader<R>, S: BorrowMut<DeserializerState>>
    de::EnumAccess<'de> for EnumAccess<'a, R, B, S>
{
    type Error = Error;
    type Variant = VariantAccess<'a, R, B, S>;

    fn variant_seed<V: de::DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, VariantAccess<'a, R, B, S>)> {
        let name = expect!(
            self.de.peek()?,

            &XmlEvent::Characters(ref name) |
            &XmlEvent::StartElement { name: OwnedName { local_name: ref name, .. }, .. } => {
                seed.deserialize(name.as_str().into_deserializer())
            }
        )?;
        self.de.set_map_value();
        Ok((name, VariantAccess::new(self.de)))
    }
}

pub struct VariantAccess<'a, R: 'a + Read, B: BufferedXmlReader<R>, S: BorrowMut<DeserializerState>>
{
    de: &'a mut Deserializer<R, B, S>,
}

impl<'a, R: 'a + Read, B: BufferedXmlReader<R>, S: BorrowMut<DeserializerState>>
    VariantAccess<'a, R, B, S>
{
    pub fn new(de: &'a mut Deserializer<R, B, S>) -> Self {
        VariantAccess { de: de }
    }
}

impl<'de, 'a, R: 'a + Read, B: BufferedXmlReader<R>, S: BorrowMut<DeserializerState>>
    de::VariantAccess<'de> for VariantAccess<'a, R, B, S>
{
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        self.de.unset_map_value();
        match self.de.next()? {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                if attributes.is_empty() {
                    self.de.expect_end_element(name)
                } else {
                    Err(de::Error::invalid_length(attributes.len(), &"0"))
                }
            },
            XmlEvent::Characters(_) => Ok(()),
            _ => unreachable!(),
        }
    }

    fn newtype_variant_seed<T: de::DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value> {
        seed.deserialize(&mut *self.de)
    }

    fn tuple_variant<V: de::Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value> {
        self.de.deserialize_tuple(len, visitor)
    }

    fn struct_variant<V: de::Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        self.de.deserialize_map(visitor)
    }
}
