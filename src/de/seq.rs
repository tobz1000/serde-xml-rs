use std::io::Read;

use serde::de;
use xml::reader::XmlEvent;

use de::ChildDeserializer;
use error::{Error, Result};

pub struct SeqAccess<'a, R: 'a + Read> {
    de: ChildDeserializer<'a, R>,
    starting_depth: usize,
    max_size: Option<usize>,
    seq_type: SeqType,
}

pub enum SeqType {
    Elements { expected_name: String },
    Text,
}

impl<'a, R: 'a + Read> SeqAccess<'a, R> {
    pub fn new(
        mut de: ChildDeserializer<'a, R>,
        starting_depth: usize,
        max_size: Option<usize>,
    ) -> Self {
        let seq_type = if de.unset_map_value() {
            debug_expect!(de.peek(), Ok(&XmlEvent::StartElement { ref name, .. }) => {
                SeqType::Elements { expected_name: name.local_name.clone() }
            })
        } else {
            SeqType::Text
        };
        SeqAccess {
            de,
            starting_depth,
            max_size,
            seq_type,
        }
    }
}

impl<'de, 'a, R: 'a + Read> de::SeqAccess<'de> for SeqAccess<'a, R> {
    type Error = Error;

    fn next_element_seed<T: de::DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>> {
        match self.max_size.as_mut() {
            Some(&mut 0) => {
                return Ok(None);
            },
            Some(max_size) => {
                *max_size -= 1;
            },
            None => {},
        }

        let more = match (self.de.peek()?, &self.seq_type) {
            (&XmlEvent::StartElement { ref name, .. }, SeqType::Elements { expected_name }) => {
                &name.local_name == expected_name
            },
            (&XmlEvent::EndElement { .. }, SeqType::Text)
            | (_, SeqType::Elements { .. })
            | (&XmlEvent::EndDocument { .. }, _) => false,
            (_, SeqType::Text) => true,
        };

        if more {
            if let SeqType::Elements { .. } = self.seq_type {
                self.de.set_map_value();
            }
            seed.deserialize(&mut self.de).map(Some)
        } else {
            Ok(None)
        }
    }

    fn size_hint(&self) -> Option<usize> {
        self.max_size
    }
}
