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

//! File system-based implementation of the storage system.

use crate::storage::{Drive, Metadata};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::str;

/// A drive that is backed by an on-disk directory.
pub struct DirectoryDrive {
    /// Path to the directory containing all entries backed by this drive.  The directory may
    /// contain files that are not EndBASIC programs, and that's OK, but those files will not be
    /// accessible through this interface.
    dir: PathBuf,
}

impl DirectoryDrive {
    /// Creates a new drive backed by the `dir` directory.
    pub fn new<P: Into<PathBuf>>(dir: P) -> io::Result<Self> {
        let dir = dir.into();

        // Obtain the canonical path to the underlying directory, which we need for system_path to
        // make sense.  Unfortunately, we must ensure the directory exists in order to do this.
        let dir = match dir.canonicalize() {
            Ok(dir) => dir,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                fs::create_dir_all(&dir)?;
                dir.canonicalize()?
            }
            Err(e) => return Err(e),
        };

        Ok(Self { dir })
    }
}

impl Drive for DirectoryDrive {
    fn delete(&mut self, name: &str) -> io::Result<()> {
        let path = self.dir.join(name);
        fs::remove_file(path)
    }

    fn enumerate(&self) -> io::Result<BTreeMap<String, Metadata>> {
        let mut entries = BTreeMap::default();
        match fs::read_dir(&self.dir) {
            Ok(dirents) => {
                for de in dirents {
                    let de = de?;

                    let file_type = de.file_type()?;
                    if !file_type.is_file() && !file_type.is_symlink() {
                        // Silently ignore entries we cannot handle.
                        continue;
                    }

                    // This follows symlinks for cross-platform simplicity, but it is ugly.  I don't
                    // expect symlinks in the programs directory anyway.  If we want to handle this
                    // better, we'll have to add a way to report file types.
                    let metadata = fs::metadata(de.path())?;
                    let offset = match time::UtcOffset::try_current_local_offset() {
                        Ok(offset) => offset,
                        Err(_) => time::UtcOffset::UTC,
                    };
                    let date = time::OffsetDateTime::from(metadata.modified()?).to_offset(offset);
                    let length = metadata.len();

                    entries.insert(
                        de.file_name().to_string_lossy().to_string(),
                        Metadata { date, length },
                    );
                }
            }
            Err(e) => {
                if e.kind() != io::ErrorKind::NotFound {
                    return Err(e);
                }
            }
        }
        Ok(entries)
    }

    fn get(&self, name: &str) -> io::Result<String> {
        let path = self.dir.join(name);
        let input = File::open(&path)?;
        let mut content = String::new();
        io::BufReader::new(input).read_to_string(&mut content)?;
        Ok(content)
    }

    fn put(&mut self, name: &str, content: &str) -> io::Result<()> {
        let path = self.dir.join(name);
        let output = OpenOptions::new().create(true).write(true).truncate(true).open(path)?;
        let mut writer = io::BufWriter::new(output);
        writer.write_all(content.as_bytes())
    }

    fn system_path(&self, name: &str) -> Option<PathBuf> {
        Some(self.dir.join(name))
    }
}

/// Instantiates a directory drive backed by `target`.
pub fn directory_drive_factory(target: &str) -> io::Result<Box<dyn Drive>> {
    if !target.is_empty() {
        Ok(Box::from(DirectoryDrive::new(target)?))
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Must specify a directory mount an disk drive",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::{BufRead, Write};
    use std::path::Path;

    /// Reads `path` and checks that its contents match `exp_lines`.
    fn check_file(path: &Path, exp_lines: &[&str]) {
        let file = File::open(path).unwrap();
        let reader = io::BufReader::new(file);
        let mut lines = vec![];
        for line in reader.lines() {
            lines.push(line.unwrap());
        }
        assert_eq!(exp_lines, lines.as_slice());
    }

    /// Creates `path` with the contents in `lines` and with a deterministic modification time.
    fn write_file(path: &Path, lines: &[&str]) {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .truncate(false) // Should not be creating the same file more than once.
            .write(true)
            .open(path)
            .unwrap();
        for line in lines {
            file.write_fmt(format_args!("{}\n", line)).unwrap();
        }
        drop(file);

        filetime::set_file_mtime(path, filetime::FileTime::from_unix_time(1_588_757_875, 0))
            .unwrap();
    }

    #[test]
    fn test_directorydrive_delete_ok() {
        let dir = tempfile::tempdir().unwrap();
        write_file(&dir.path().join("a.bas"), &[]);
        write_file(&dir.path().join("a.bat"), &[]);

        let mut drive = DirectoryDrive::new(&dir.path()).unwrap();
        drive.delete("a.bas").unwrap();
        assert!(!dir.path().join("a.bas").exists());
        assert!(dir.path().join("a.bat").exists());
    }

    #[test]
    fn test_directorydrive_delete_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut drive = DirectoryDrive::new(&dir.path()).unwrap();
        assert_eq!(io::ErrorKind::NotFound, drive.delete("a.bas").unwrap_err().kind());
    }

    #[test]
    fn test_directorydrive_enumerate_nothing() {
        let dir = tempfile::tempdir().unwrap();

        let drive = DirectoryDrive::new(&dir.path()).unwrap();
        assert!(drive.enumerate().unwrap().is_empty());
    }

    #[test]
    fn test_directorydrive_enumerate_some_files() {
        let dir = tempfile::tempdir().unwrap();
        write_file(&dir.path().join("empty.bas"), &[]);
        write_file(&dir.path().join("some file.bas"), &["this is not empty"]);

        let drive = DirectoryDrive::new(&dir.path()).unwrap();
        let entries = drive.enumerate().unwrap();
        assert_eq!(2, entries.len());
        let date = time::OffsetDateTime::from_unix_timestamp(1_588_757_875);
        assert_eq!(&Metadata { date, length: 0 }, entries.get("empty.bas").unwrap());
        assert_eq!(&Metadata { date, length: 18 }, entries.get("some file.bas").unwrap());
    }

    #[test]
    fn test_directorydrive_enumerate_treats_missing_dir_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        let drive = DirectoryDrive::new(dir.path().join("does-not-exist")).unwrap();
        assert!(drive.enumerate().unwrap().is_empty());
    }

    #[test]
    fn test_directorydrive_enumerate_ignores_non_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("will-be-ignored")).unwrap();
        let drive = DirectoryDrive::new(&dir.path()).unwrap();
        assert!(drive.enumerate().unwrap().is_empty());
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_directorydrive_enumerate_follows_symlinks() {
        use std::os::unix::fs as unix_fs;

        let dir = tempfile::tempdir().unwrap();
        write_file(&dir.path().join("some file.bas"), &["this is not empty"]);
        unix_fs::symlink(&Path::new("some file.bas"), &dir.path().join("a link.bas")).unwrap();

        let drive = DirectoryDrive::new(&dir.path()).unwrap();
        let entries = drive.enumerate().unwrap();
        assert_eq!(2, entries.len());
        let metadata =
            Metadata { date: time::OffsetDateTime::from_unix_timestamp(1_588_757_875), length: 18 };
        assert_eq!(&metadata, entries.get("some file.bas").unwrap());
        assert_eq!(&metadata, entries.get("a link.bas").unwrap());
    }

    #[test]
    fn test_directorydrive_enumerate_fails_on_non_directory() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("not-a-dir");
        write_file(&file, &[]);
        let drive = DirectoryDrive::new(&file).unwrap();
        assert_eq!(io::ErrorKind::Other, drive.enumerate().unwrap_err().kind());
    }

    #[test]
    fn test_directorydrive_get() {
        let dir = tempfile::tempdir().unwrap();
        write_file(&dir.path().join("some file.bas"), &["one line", "two lines"]);

        let drive = DirectoryDrive::new(&dir.path()).unwrap();
        assert_eq!("one line\ntwo lines\n", drive.get("some file.bas").unwrap());
    }

    #[test]
    fn test_directorydrive_put() {
        let dir = tempfile::tempdir().unwrap();

        let mut drive = DirectoryDrive::new(&dir.path()).unwrap();
        drive.put("some file.bas", "a b c\nd e\n").unwrap();
        check_file(&dir.path().join("some file.bas"), &["a b c", "d e"]);
    }

    #[test]
    fn test_directorydrive_system_path() {
        let dir = tempfile::tempdir().unwrap();

        let drive = DirectoryDrive::new(&dir.path()).unwrap();
        assert_eq!(
            dir.path().canonicalize().unwrap().join("foo"),
            drive.system_path("foo").unwrap()
        );
    }
}
