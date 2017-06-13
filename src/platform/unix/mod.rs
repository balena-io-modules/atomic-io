
extern crate nix;
extern crate libc;

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod imp;

pub use self::imp::*;

use std::fs;
use std::ffi::{CString, OsString};
use std::os::unix::ffi::OsStringExt;
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::path::{Path, PathBuf};

use self::nix::sys::sendfile::sendfile;
use self::nix::sys::stat;
use self::nix::fcntl;

/// Used to clone the original data into the temporary file.
/// On Unix, we use sendfile to perform a fast copy.
pub fn clone(src: &mut fs::File, dest: &mut fs::File) {
    let src_fd = src.as_raw_fd();
    let dest_fd = dest.as_raw_fd();
    let len = src.metadata().unwrap().len() as usize;

    sendfile(dest_fd, src_fd, None, len).unwrap();
}

/// Given a directory, return a `File` representing a temporary file
/// handle. On Unix, this returns an anonymous file that is unlinked.
pub fn get_tempfile(dir: &Path) -> (fs::File, Option<PathBuf>) {
    use self::stat::*;
    use self::fcntl::*;

    let fd = fcntl::open(dir, O_TMPFILE | O_RDWR, S_IRUSR | S_IWUSR).unwrap();
    let file = unsafe { fs::File::from_raw_fd(fd) };
    (file, None)
}

/// Get a name for the temproary file. In order to swap the file,
/// we must first link it. This creates a temporary name for the 
/// link until we can swap the file.
fn get_tempfile_name(original: &Path) -> PathBuf {
    let name = original.file_name().unwrap();
    let swap = format!(".{}.swp", name.to_str().unwrap());

    let mut buffer = PathBuf::from(original);
    buffer.pop();
    buffer.push(swap);

    buffer
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
        clone(&mut original, &mut cloned);

        cloned.seek(SeekFrom::Start(0)).unwrap();

        let mut buffer = String::new();
        cloned.read_to_string(&mut buffer).unwrap();

        assert_eq!(expected, buffer);

    }
}