// EndBASIC
// Copyright 2021 Julio Merino
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not
// use this file except in compliance with the License.  You may obtain a copy
// of the License at:
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.  See the
// License for the specific language governing permissions and limitations
// under the License.

//! Test utilities for consumers of the EndBASIC interpreter.

use crate::console::{ClearType, Console, Key, Position};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::io;

/// A captured command or messages sent to the mock console.
#[derive(Debug, Eq, PartialEq)]
pub enum CapturedOut {
    /// Represents a call to `Console::clear`.
    Clear(ClearType),

    /// Represents a call to `Console::color`.
    Color(Option<u8>, Option<u8>),

    /// Represents a call to `Console::enter_alt`.
    EnterAlt,

    /// Represents a call to `Console::hide_cursor`.
    HideCursor,

    /// Represents a call to `Console::leave_alt`.
    LeaveAlt,

    /// Represents a call to `Console::locate`.
    Locate(Position),

    /// Represents a call to `Console::move_within_line`.
    MoveWithinLine(i16),

    /// Represents a call to `Console::print`.
    Print(String),

    /// Represents a call to `Console::show_cursor`.
    ShowCursor,

    /// Represents a call to `Console::write`.
    Write(Vec<u8>),
}

/// A console that supplies golden input and captures all output.
pub struct MockConsole {
    /// Sequence of keys to yield on `read_key` calls.
    golden_in: VecDeque<Key>,

    /// Sequence of all messages printed.
    captured_out: Vec<CapturedOut>,

    /// The size of the mock console.
    size: Position,
}

impl MockConsole {
    /// Obtains a reference to the captured output.
    pub fn captured_out(&self) -> &[CapturedOut] {
        self.captured_out.as_slice()
    }
}

impl Drop for MockConsole {
    fn drop(&mut self) {
        assert!(
            self.golden_in.is_empty(),
            "Not all golden input chars were consumed; {} left",
            self.golden_in.len()
        );
    }
}

#[async_trait(?Send)]
impl Console for MockConsole {
    fn clear(&mut self, how: ClearType) -> io::Result<()> {
        self.captured_out.push(CapturedOut::Clear(how));
        Ok(())
    }

    fn color(&mut self, fg: Option<u8>, bg: Option<u8>) -> io::Result<()> {
        self.captured_out.push(CapturedOut::Color(fg, bg));
        Ok(())
    }

    fn enter_alt(&mut self) -> io::Result<()> {
        self.captured_out.push(CapturedOut::EnterAlt);
        Ok(())
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.captured_out.push(CapturedOut::HideCursor);
        Ok(())
    }

    fn is_interactive(&self) -> bool {
        false
    }

    fn leave_alt(&mut self) -> io::Result<()> {
        self.captured_out.push(CapturedOut::LeaveAlt);
        Ok(())
    }

    fn locate(&mut self, pos: Position) -> io::Result<()> {
        self.captured_out.push(CapturedOut::Locate(pos));
        Ok(())
    }

    fn move_within_line(&mut self, off: i16) -> io::Result<()> {
        self.captured_out.push(CapturedOut::MoveWithinLine(off));
        Ok(())
    }

    fn print(&mut self, text: &str) -> io::Result<()> {
        self.captured_out.push(CapturedOut::Print(text.to_owned()));
        Ok(())
    }

    async fn read_key(&mut self) -> io::Result<Key> {
        match self.golden_in.pop_front() {
            Some(ch) => Ok(ch),
            None => Ok(Key::Eof),
        }
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.captured_out.push(CapturedOut::ShowCursor);
        Ok(())
    }

    fn size(&self) -> io::Result<Position> {
        Ok(self.size)
    }

    fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.captured_out.push(CapturedOut::Write(bytes.to_owned()));
        Ok(())
    }
}

/// Builder pattern for a `MockConsole`.
pub struct MockConsoleBuilder {
    golden_in: VecDeque<Key>,
    size: Position,
}

impl Default for MockConsoleBuilder {
    /// Creates a new console builder, with no golden input and an infinite size.
    fn default() -> Self {
        Self { golden_in: VecDeque::new(), size: Position { row: usize::MAX, column: usize::MAX } }
    }
}

impl MockConsoleBuilder {
    /// Adds a bunch of characters as golden input keys.
    ///
    /// Note that some escape characters within `s` are interpreted and added as their
    /// corresponding `Key`s for simplicity.
    pub fn add_input_chars(mut self, s: &str) -> Self {
        for ch in s.chars() {
            match ch {
                '\n' => self.golden_in.push_back(Key::NewLine),
                '\r' => self.golden_in.push_back(Key::CarriageReturn),
                ch => self.golden_in.push_back(Key::Char(ch)),
            }
        }
        self
    }

    /// Adds a bunch of keys as golden input.
    pub fn add_input_keys(mut self, keys: &[Key]) -> Self {
        self.golden_in.extend(keys.iter().cloned());
        self
    }

    /// Sets the size of the mock console.
    pub fn with_size(mut self, size: Position) -> Self {
        self.size = size;
        self
    }

    /// Builds a `MockConsole` instance as configured in the builder.
    pub fn build(self) -> MockConsole {
        MockConsole { golden_in: self.golden_in, captured_out: vec![], size: self.size }
    }
}