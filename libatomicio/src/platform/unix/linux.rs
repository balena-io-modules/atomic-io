extern crate libc;
extern crate errno;

const RENAMEAT2: c_long = 316;
const RENAME_EXCHANGE: c_long = (1 << 1);

use super::*;

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use self::errno::errno;

use self::libc::{linkat, syscall, c_long, AT_FDCWD, AT_SYMLINK_FOLLOW};

/// Perform an atomic file swap by first linking the anonymous file,
/// then swapping atomically using the Linux `RENAMEAT2` syscall.
pub fn atomic_swap(original: &Path, temp: &mut fs::File) -> Option<PathBuf> {
    let input = format!("/proc/self/fd/{}", temp.as_raw_fd());
    let tmpname = get_tempfile_name(original);

    link_at(Path::new(&input), &tmpname);

    swap_files(&tmpname, &original);

    Some(tmpname)
}

/// Performs an explicit atomic swap using the `RENAMEAT2` syscall.
fn swap_files(old: &Path, new: &Path) {
    let src = to_cstring(OsString::from(old)).unwrap();
    let dest = to_cstring(OsString::from(new)).unwrap();
    let err = unsafe { syscall(RENAMEAT2, AT_FDCWD, src.as_ptr(), AT_FDCWD, dest.as_ptr(), RENAME_EXCHANGE) };

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
    fn swap_renameat2_same_directory() {
        let content1 = "Hello World!";
        let content2 = "World Hello!";

        let dir = env::temp_dir();

        let mut file1 = NamedTempFile::new_in(&dir).unwrap();
        write!(file1, "{}", content1).unwrap();

        let mut file2 = NamedTempFile::new_in(&dir).unwrap();
        write!(file2, "{}", content2).unwrap();

        swap_files(file1.path(), file2.path());

        let mut buffer1 = String::new();
        let mut open1 = File::open(file1.path()).unwrap();
        open1.read_to_string(&mut buffer1).unwrap();

        assert_eq!(content2, buffer1);

        let mut buffer2 = String::new();
        let mut open2 = File::open(file2.path()).unwrap();
        open2.read_to_string(&mut buffer2).unwrap();

        assert_eq!(content1, buffer2);
    }
}