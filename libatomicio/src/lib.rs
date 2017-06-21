
#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;

mod platform;

use std::fs;
use std::io::{self, Read, Write, Seek, SeekFrom};
use std::path::{Path, PathBuf};

mod errors {
    use std::io;

    error_chain! {
        foreign_links {
            Io(io::Error) #[doc = "Error during I/O"];
        }

        errors {
            Platform {
                description("platform-specific error occured")
            }
        }
    }
}

pub use errors::*;

/// The `AtomicFile` struct represents a copy of the underlying file that
/// is readable and writable. When the required changes have been made,
/// calling `commit()` the file to apply the changes.
pub struct AtomicFile {
    orig: PathBuf,
    tmpfile: fs::File,
    tmpname: Option<PathBuf>,
}

impl AtomicFile {
    /// Opens an atomic file for reading and writing
    pub fn open<P: AsRef<Path>>(path: P) -> Result<AtomicFile> {
        let metadata = fs::metadata(path.as_ref())?;
        let absolute = path.as_ref().canonicalize()?;
        let dir = absolute.parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no parent directory"))?;
        let (mut temp, name) = platform::get_tempfile(dir, &metadata)?;

        {
            let mut original = fs::File::open(path.as_ref())?;
            platform::clone(&mut original, &mut temp)?;
        }

        temp.seek(SeekFrom::Start(0))?;

        Ok(AtomicFile { 
            tmpfile: temp, 
            orig: path.as_ref().to_path_buf(),
            tmpname: name,
        })
    }

    /// Commits the changes made to the original file
    pub fn commit(mut self) -> Result<()> {
        self.tmpfile.flush()?;
        self.tmpname = platform::atomic_swap(&self.orig, &mut self.tmpfile)?;
        Ok(())
    }
}

impl Write for AtomicFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.tmpfile.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> { self.tmpfile.flush() }
}

impl Read for AtomicFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.tmpfile.read(buf)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.tmpfile.read_to_end(buf)
    }
}

impl Seek for AtomicFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.tmpfile.seek(pos)
    }
}

impl Drop for AtomicFile {
    #[allow(unused_must_use)]
    fn drop(&mut self) {
        if let Some(ref name) = self.tmpname {
            fs::remove_file(name);
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate tempfile;

    use super::*;

    use std::fs::File;

    use self::tempfile::NamedTempFile;

    fn init(buf: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", buf).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        file
    }

    fn read<R>(mut file: R) -> String 
        where R: Read
    {
        let mut buffer = String::new();
        file.read_to_string(&mut buffer).unwrap();
        buffer
    }

    #[test]
    fn atomic_file_read() {
        let expected = "Hello World!";

        let file = init(expected);
        let atomic = AtomicFile::open(file.path()).unwrap();
        
        let result = read(atomic);
        assert_eq!(expected, result);
    }

    #[test]
    fn atomic_file_write_read() {
        let expected = "foo";

        let file = init(expected);
        let mut atomic = AtomicFile::open(file.path()).unwrap();
        write!(atomic, "{}", "bar").unwrap();
        
        let result = read(file.reopen().unwrap());

        assert_eq!(expected, result);
    }

    #[test]
    fn atomic_file_write_rollback_read() {
        let expected = "foo";

        let file = init(expected);

        let mut atomic = AtomicFile::open(file.path()).unwrap();
        write!(atomic, "{}", "bar").unwrap();

        drop(atomic);
        
        let result = read(file.reopen().unwrap());

        assert_eq!(expected, result);
    }

    #[test]
    fn atomic_file_write_commit_read() {
        let expected = "bar";

        let file = init("foo");
        let mut atomic = AtomicFile::open(file.path()).unwrap();
        write!(atomic, "{}", expected).unwrap();

        atomic.commit().unwrap();
        
        let opened = File::open(file.path()).unwrap();
        let result = read(opened);

        assert_eq!(expected, result);
    }
}
