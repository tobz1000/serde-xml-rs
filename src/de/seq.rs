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
    /// Sequence is of elements with the same name.
    SameElement { expected_name: String },
    /// Sequence if of elements with multiple names (enums).
    /// TODO: Need to find somewhere to obtain the names we're looking for, to support out-of-order
    /// elements
    AnyType,
}

impl<'a, R: 'a + Read> SeqAccess<'a, R> {
    pub fn new(
        mut de: ChildDeserializer<'a, R>,
        starting_depth: usize,
        max_size: Option<usize>,
    ) -> Self {
        let seq_type = if de.unset_map_value() {
            debug_expect!(de.peek(), Ok(&XmlEvent::StartElement { ref name, .. }) => {
                SeqType::SameElement { expected_name: name.local_name.clone() }
            })
        } else {
            SeqType::AnyType
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
        debug_assert_eq!(
            self.de.state.depth, self.starting_depth,
            "Element depth should be equal on each re-entry to sequence"
        );

        match self.max_size.as_mut() {
            Some(&mut 0) => {
                return Ok(None);
            }
            Some(max_size) => {
                *max_size -= 1;
            }
            None => {}
        }

        match &self.seq_type {
            SeqType::SameElement { expected_name } => loop {
                debug_assert!(
                    self.de.state.depth >= self.starting_depth,
                    "Element depth should only go below starting depth after end of sequence"
                );

                dbg!((self.de.state.depth, self.starting_depth));

                if self.de.state.depth != self.starting_depth {
                    self.de.peek()?;
                    self.de.buffered_reader.skip();
                }

                let next_element = self.de.peek()?;

                match next_element {
                    XmlEvent::StartElement { name, .. } if &name.local_name == expected_name => {
                        self.de.set_map_value();
                        return seed.deserialize(&mut self.de).map(Some);
                    }
                    XmlEvent::EndElement { .. } | XmlEvent::EndDocument => {
                        println!("Ending seq");
                        return Ok(None);
                    }
                    _ => {
                        self.de.buffered_reader.skip();
                    }
                }
            },
            SeqType::AnyType => loop {
                todo!();
                debug_assert!(
                    self.de.state.depth >= self.starting_depth,
                    "Element depth should only go below starting depth after end of sequence"
                );

                if self.de.state.depth != self.starting_depth {
                    self.de.peek()?;
                    self.de.buffered_reader.skip();
                }

                let next_element = self.de.peek()?;

                match next_element {
                    XmlEvent::EndElement { .. } | XmlEvent::EndDocument => return Ok(None),
                    _ => {
                        return seed.deserialize(&mut self.de).map(Some);
                    }
                }
            },
        }

        // let more = match (&self.seq_type, self.de.peek()?) {
        //     (SeqType::SameElement { expected_name }, XmlEvent::StartElement { name, .. }) => {
        //         &name.local_name == expected_name
        //     }
        //     (SeqType::SameElement { .. }, _) => false,
        //     (SeqType::AnyType, XmlEvent::EndElement { .. }) => false,
        //     (SeqType::AnyType, XmlEvent::EndDocument) => false,
        //     (SeqType::AnyType, _) => true,
        // };

        // if more {
        //     if let SeqType::SameElement { .. } = self.seq_type {
        //         self.de.set_map_value();
        //     }
        //     seed.deserialize(&mut self.de).map(Some)
        // } else {
        //     Ok(None)
        // }
    }

    fn size_hint(&self) -> Option<usize> {
        self.max_size
    }
}
