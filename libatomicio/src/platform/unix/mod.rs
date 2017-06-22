
#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod imp;

pub use self::imp::*;
pub use errors::*;

use std::fs;
use std::ffi::{CString, OsString};
use std::io;
use std::os::unix::fs::MetadataExt;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::path::{Path, PathBuf};

use nix::sys::sendfile::sendfile;
use nix::sys::stat;
use nix::fcntl;

/// Used to clone the original data into the temporary file.
/// On Unix, we use sendfile to perform a fast copy.
pub fn clone(src: &mut fs::File, dest: &mut fs::File) -> Result<()> {
    let src_fd = src.as_raw_fd();
    let dest_fd = dest.as_raw_fd();
    let len = src.metadata().unwrap().len() as usize;

    sendfile(dest_fd, src_fd, None, len)?;

    Ok(())
}

/// Given a directory, return a `File` representing a temporary file
/// handle. On Unix, this returns an anonymous file that is unlinked.
pub fn get_tempfile(dir: &Path, meta: &fs::Metadata) -> Result<(fs::File, Option<PathBuf>)> {
    use self::stat::Mode;
    use self::fcntl::*;

    // mask off type bits
    let mode_bits = meta.mode() & 0o7777;
    let mode = Mode::from_bits(mode_bits)
        .ok_or_else(|| Error::from(ErrorKind::Platform))?;
    let fd = fcntl::open(dir, O_TMPFILE | O_RDWR, mode)?;
    let file = unsafe { fs::File::from_raw_fd(fd) };
    Ok((file, None))
}

/// Get a name for the temproary file. In order to swap the file,
/// we must first link it. This creates a temporary name for the 
/// link until we can swap the file.
fn get_tempfile_name(original: &Path) -> Result<PathBuf> {
    let name = original.file_name()
        .and_then(|os| os.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "invalid file name"))?;
    
    let swap = format!(".{}.swp", name);

    let mut buffer = PathBuf::from(original);
    buffer.pop();
    buffer.push(swap);

    Ok(buffer)
}

/// Helper function to convert an `OsString` to a `CString`.
fn to_cstring(input: OsString) -> Option<CString> {
    let vec = input.into_vec();
    CString::new(vec).ok()
}

#[cfg(test)]
mod tests {
    extern crate tempfile;

    use std::io::{Write, Read, Seek, SeekFrom};

    use super::*;

    #[test]
    fn clone_file_test() {
        let expected = "Hello World!";

        let mut original = tempfile::tempfile().unwrap();
        write!(original, "{}", expected).unwrap();

        original.seek(SeekFrom::Start(0)).unwrap();

        let mut cloned = tempfile::tempfile().unwrap();
        clone(&mut original, &mut cloned).unwrap();

        cloned.seek(SeekFrom::Start(0)).unwrap();

        let mut buffer = String::new();
        cloned.read_to_string(&mut buffer).unwrap();

        assert_eq!(expected, buffer);

    }
}