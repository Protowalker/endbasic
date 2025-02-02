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

//! Trivial stdio-based console implementation for when we have nothing else.

use crate::console::{get_env_var_as_u16, read_key_from_stdin, CharsXY, ClearType, Console, Key};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::io::{self, StdoutLock, Write};

/// Default number of columns for when `COLUMNS` is not set.
const DEFAULT_COLUMNS: u16 = 80;

/// Default number of lines for when `LINES` is not set.
const DEFAULT_LINES: u16 = 24;

/// Implementation of the EndBASIC console with minimal functionality.
#[derive(Default)]
pub struct TrivialConsole {
    /// Line-oriented buffer to hold input when not operating in raw mode.
    buffer: VecDeque<Key>,

    /// Whether video syncing is enabled or not.
    sync_enabled: bool,
}

impl TrivialConsole {
    /// Flushes the console, which has already been written to via `lock`, if syncing is enabled.
    fn maybe_flush(&self, mut lock: StdoutLock<'_>) -> io::Result<()> {
        if self.sync_enabled {
            lock.flush()
        } else {
            Ok(())
        }
    }
}

#[async_trait(?Send)]
impl Console for TrivialConsole {
    fn clear(&mut self, _how: ClearType) -> io::Result<()> {
        Ok(())
    }

    fn color(&mut self, _fg: Option<u8>, _bg: Option<u8>) -> io::Result<()> {
        Ok(())
    }

    fn enter_alt(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn is_interactive(&self) -> bool {
        true
    }

    fn leave_alt(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn locate(&mut self, _pos: CharsXY) -> io::Result<()> {
        Ok(())
    }

    fn move_within_line(&mut self, _off: i16) -> io::Result<()> {
        Ok(())
    }

    fn print(&mut self, text: &str) -> io::Result<()> {
        debug_assert!(!crate::console::has_control_chars_str(text));

        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        stdout.write_all(text.as_bytes())?;
        stdout.write_all(b"\n")?;
        Ok(())
    }

    async fn poll_key(&mut self) -> io::Result<Option<Key>> {
        Ok(None)
    }

    async fn read_key(&mut self) -> io::Result<Key> {
        read_key_from_stdin(&mut self.buffer)
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn size(&self) -> io::Result<CharsXY> {
        let lines = get_env_var_as_u16("LINES").unwrap_or(DEFAULT_LINES);
        let columns = get_env_var_as_u16("COLUMNS").unwrap_or(DEFAULT_COLUMNS);
        Ok(CharsXY::new(columns, lines))
    }

    fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        debug_assert!(!crate::console::has_control_chars_u8(bytes));

        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        stdout.write_all(bytes)?;
        self.maybe_flush(stdout)
    }

    fn sync_now(&mut self) -> io::Result<()> {
        if self.sync_enabled {
            Ok(())
        } else {
            io::stdout().flush()
        }
    }

    fn set_sync(&mut self, enabled: bool) -> io::Result<()> {
        if !self.sync_enabled {
            io::stdout().flush()?;
        }
        self.sync_enabled = enabled;
        Ok(())
    }
}
