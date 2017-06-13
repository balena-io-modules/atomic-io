
mod platform;

use std::fs;
use std::io::{Result, Read, Write, Seek, SeekFrom};
use std::path::{Path, PathBuf};

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
    pub fn open<P: AsRef<Path>>(path: P) -> AtomicFile {
        let dir = path.as_ref().parent().unwrap();
        let (mut temp, name) = platform::get_tempfile(dir);

        {
            let mut original = fs::File::open(path.as_ref()).unwrap();
            platform::clone(&mut original, &mut temp);
        }

        temp.seek(SeekFrom::Start(0)).unwrap();

        AtomicFile { 
            tmpfile: temp, 
            orig: path.as_ref().to_path_buf(),
            tmpname: name,    
        }
    }

    /// Commits the changes made to the original file
    pub fn commit(mut self) {
        self.tmpfile.flush().unwrap();
        self.tmpname = platform::atomic_swap(&self.orig, &mut self.tmpfile);
    }
}

impl Write for AtomicFile {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.tmpfile.write(buf)
    }

    fn flush(&mut self) -> Result<()> { self.tmpfile.flush() }
}

impl Read for AtomicFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.tmpfile.read(buf)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        self.tmpfile.read_to_end(buf)
    }
}

impl Seek for AtomicFile {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.tmpfile.seek(pos)
    }
}

impl Drop for AtomicFile {
    fn drop(&mut self) {
        if let Some(ref name) = self.tmpname {
            fs::remove_file(name).unwrap();
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
        let atomic = AtomicFile::open(file.path());
        
        let result = read(atomic);
        assert_eq!(expected, result);
    }

    #[test]
    fn atomic_file_write_read() {
        let expected = "foo";

        let file = init(expected);
        let mut atomic = AtomicFile::open(file.path());
        write!(atomic, "{}", "bar").unwrap();
        
        let result = read(file.reopen().unwrap());

        assert_eq!(expected, result);
    }

    #[test]
    fn atomic_file_write_rollback_read() {
        let expected = "foo";

        let file = init(expected);

        let mut atomic = AtomicFile::open(file.path());
        write!(atomic, "{}", "bar").unwrap();

        drop(atomic);
        
        let result = read(file.reopen().unwrap());

        assert_eq!(expected, result);
    }

    #[test]
    fn atomic_file_write_commit_read() {
        let expected = "bar";

        let file = init("foo");
        let mut atomic = AtomicFile::open(file.path());
        write!(atomic, "{}", expected).unwrap();

        atomic.commit();
        
        let opened = File::open(file.path()).unwrap();
        let result = read(opened);

        assert_eq!(expected, result);
    }
}
