extern crate libc;
extern crate errno;

use super::*;

use errors::*;

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use self::errno::errno;

use self::libc::{linkat, rename, AT_FDCWD, AT_SYMLINK_FOLLOW};

/// Perform an atomic file swap by first linking the anonymous file,
/// then swapping atomically using the Linux `RENAMEAT2` syscall.
pub fn atomic_swap(original: &Path, temp: &mut fs::File) -> Result<Option<PathBuf>> {
    let input = format!("/proc/self/fd/{}", temp.as_raw_fd());
    let tmpname = get_tempfile_name(original)?;

    link_at(Path::new(&input), &tmpname);

    rename_file(&tmpname, &original);

    Ok(Some(tmpname))
}

/// Performs an explicit atomic swap using the `RENAMEAT2` syscall.
fn rename_file(old: &Path, new: &Path) {
    let src = to_cstring(OsString::from(old)).unwrap();
    let dest = to_cstring(OsString::from(new)).unwrap();
    let err = unsafe { rename(src.as_ptr(), dest.as_ptr()) };

    match err {
        0 => (),
        -1 => panic!("system error: {}", errno()),
        _ => panic!("unknown syscall error"),
    }
}

/// Link an anonymous file into the file system using the `linkat` function in `libc`.
fn link_at(old: &Path, new: &Path) {
    let src = to_cstring(OsString::from(old)).unwrap();
    let dest = to_cstring(OsString::from(&new)).unwrap();
    let err = unsafe { linkat(AT_FDCWD, src.as_ptr(), AT_FDCWD, dest.as_ptr(), AT_SYMLINK_FOLLOW) };

    match err {
        0 => (),
        -1 => panic!("system error: {}", errno()),
        _ => panic!("unknown syscall error"),
    }
}

#[cfg(test)]
mod tests {

    extern crate tempfile;

    use std::env;
    use std::fs::File;
    use std::io::{Write, Read};

    use super::*;

    use self::tempfile::NamedTempFile;

    #[test]
    fn rename_same_directory() {
        let content1 = "Hello World!";
        let content2 = "World Hello!";

        let dir = env::temp_dir();

        let mut file1 = NamedTempFile::new_in(&dir).unwrap();
        write!(file1, "{}", content1).unwrap();

        let mut file2 = NamedTempFile::new_in(&dir).unwrap();
        write!(file2, "{}", content2).unwrap();

        rename_file(file1.path(), file2.path());

        assert!(!file1.path().exists());

        let mut buffer = String::new();
        let mut open = File::open(file2.path()).unwrap();
        open.read_to_string(&mut buffer).unwrap();

        assert_eq!(content1, buffer);
    }
}