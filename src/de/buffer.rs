use error::Result;
use std::{collections::VecDeque, io::Read};
use xml::reader::{EventReader, XmlEvent};

/// Retrieve XML events from an underlying reader.
pub trait BufferedXmlReader {
    /// Get and "consume" the next event.
    fn next(&mut self) -> Result<XmlEvent>;

    /// Get the next event without consuming.
    fn peek(&mut self) -> Result<&XmlEvent>;
}

pub struct RootXmlBuffer<R: Read> {
    reader: EventReader<R>,
    buffer: VecDeque<CachedXmlEvent>,
}

impl<R: Read> RootXmlBuffer<R> {
    pub fn new(reader: EventReader<R>) -> Self {
        RootXmlBuffer {
            reader,
            buffer: VecDeque::new(),
        }
    }

    pub fn child_buffer<'root>(&'root mut self) -> ChildXmlBuffer<'root, R> {
        let RootXmlBuffer { reader, buffer } = self;
        ChildXmlBuffer {
            reader,
            buffer,
            cursor: 0,
        }
    }
}

impl<R: Read> BufferedXmlReader for RootXmlBuffer<R> {
    /// Used XML events in the root buffer are moved to the caller
    fn next(&mut self) -> Result<XmlEvent> {
        loop {
            match self.buffer.pop_front() {
                Some(CachedXmlEvent::Unused(ev)) => break Ok(ev),
                Some(CachedXmlEvent::Used) => continue,
                None => break next_significant_event(&mut self.reader),
            }
        }
    }

    fn peek(&mut self) -> Result<&XmlEvent> {
        get_from_buffer_or_reader(&mut self.buffer, &mut self.reader, 0)
    }
}

pub struct ChildXmlBuffer<'root, R: Read> {
    reader: &'root mut EventReader<R>,
    buffer: &'root mut VecDeque<CachedXmlEvent>,
    cursor: usize,
}

impl<'root, R: Read> ChildXmlBuffer<'root, R> {
    /// Advance the child buffer without marking an event as "used". Should only be called after `.peek()`.
    fn skip(&mut self) {
        debug_assert!(self.cursor < self.buffer.len());

        self.cursor += 1;
    }
}

impl<'root, R: Read> BufferedXmlReader for ChildXmlBuffer<'root, R> {
    /// Consumed XML events in a child buffer are marked as "used"
    fn next(&mut self) -> Result<XmlEvent> {
        loop {
            match self.buffer.get_mut(self.cursor) {
                Some(entry @ CachedXmlEvent::Unused(_)) => {
                    let taken = std::mem::replace(entry, CachedXmlEvent::Used);

                    return debug_expect!(taken, CachedXmlEvent::Unused(ev) => Ok(ev));
                }
                Some(CachedXmlEvent::Used) => {
                    self.cursor += 1;
                    continue;
                }
                None => {
                    debug_assert_eq!(self.buffer.len(), self.cursor);

                    // Skip creation of buffer entry when consuming event straight away
                    return next_significant_event(&mut self.reader);
                }
            }
        }
    }

    fn peek(&mut self) -> Result<&XmlEvent> {
        get_from_buffer_or_reader(self.buffer, self.reader, self.cursor)
    }
}

#[derive(Debug)]
enum CachedXmlEvent {
    Unused(XmlEvent),
    Used,
}

fn get_from_buffer_or_reader<'buf>(
    buffer: &'buf mut VecDeque<CachedXmlEvent>,
    reader: &mut EventReader<impl Read>,
    index: usize,
) -> Result<&'buf XmlEvent> {
    // We should only be attempting to get an event already in the buffer, or the next event to place in the buffer
    debug_assert!(index <= buffer.len());

    loop {
        match buffer.get_mut(index) {
            Some(CachedXmlEvent::Unused(_)) => break,
            Some(CachedXmlEvent::Used) => {
                buffer.pop_front();
                continue;
            }
            None => {
                let next = next_significant_event(reader)?;
                buffer.push_back(CachedXmlEvent::Unused(next));
                let next_ref = debug_expect!(
                    buffer.front(),
                    Some(CachedXmlEvent::Unused(next)) => next
                );
                return Ok(next_ref);
            }
        }
    }

    // Returning of borrowed data must be done after of loop/match due to current limitation of borrow checker
    debug_expect!(buffer.front(), Some(CachedXmlEvent::Unused(next)) => Ok(next))
}

/// Reads the next XML event from the underlying reader, skipping events we're not interested in.
fn next_significant_event(reader: &mut EventReader<impl Read>) -> Result<XmlEvent> {
    loop {
        match reader.next()? {
            XmlEvent::StartDocument { .. }
            | XmlEvent::ProcessingInstruction { .. }
            | XmlEvent::Whitespace { .. }
            | XmlEvent::Comment(_) => { /* skip */ }
            other => return Ok(other),
        }
    }
}
