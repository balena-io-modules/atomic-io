
use super::*;

use errors::*;

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use libc::{linkat, rename, AT_FDCWD, AT_SYMLINK_FOLLOW};

use nix;

/// Perform an atomic file swap by first linking the anonymous file,
/// then swapping atomically using the Linux `RENAMEAT2` syscall.
pub fn atomic_swap(original: &Path, temp: &mut fs::File) -> Result<Option<PathBuf>> {
    let input = format!("/proc/self/fd/{}", temp.as_raw_fd());
    let tmpname = get_tempfile_name(original)?;

    link_at(Path::new(&input), &tmpname)?;

    rename_file(&tmpname, &original)?;

    Ok(Some(tmpname))
}

/// Performs an explicit atomic swap using the `rename` function in `libc`.
fn rename_file(old: &Path, new: &Path) -> Result<()> {
    let src = to_cstring(OsString::from(old)).unwrap();
    let dest = to_cstring(OsString::from(new)).unwrap();
    let err = unsafe { rename(src.as_ptr(), dest.as_ptr()) };

    match err {
        0 => Ok(()),
        _ => Err(nix::Error::last().into()),
    }
}

/// Link a file into the file system using the `linkat` function in `libc`.
fn link_at(old: &Path, new: &Path) -> Result<()> {
    let src = to_cstring(OsString::from(old)).unwrap();
    let dest = to_cstring(OsString::from(&new)).unwrap();
    let err = unsafe { linkat(AT_FDCWD, src.as_ptr(), AT_FDCWD, dest.as_ptr(), AT_SYMLINK_FOLLOW) };

    match err {
        0 => Ok(()),
        _ => Err(nix::Error::last().into()),
    }
}

#[cfg(test)]
mod tests {

    extern crate tempfile;
    extern crate tempdir;

    use std::fs::{self, File};
    use std::io::{Write, Read};
    use std::os::unix;

    use super::*;

    use self::tempfile::NamedTempFile;
    
    use self::tempdir::TempDir;

    fn tmpdir() -> TempDir {
        TempDir::new("atomic_test").unwrap()
    }

    #[test]
    fn rename_same_directory() {
        let content1 = "Hello World!";
        let content2 = "World Hello!";

        let dir = tmpdir();

        let mut file1 = NamedTempFile::new_in(dir.path()).unwrap();
        write!(file1, "{}", content1).unwrap();

        let mut file2 = NamedTempFile::new_in(dir.path()).unwrap();
        write!(file2, "{}", content2).unwrap();

        rename_file(file1.path(), file2.path()).unwrap();

        assert!(!file1.path().exists());

        let mut buffer = String::new();
        let mut open = File::open(file2.path()).unwrap();
        open.read_to_string(&mut buffer).unwrap();

        assert_eq!(content1, buffer);
    }

    #[test]
    fn rename_not_exists() {
        let dir = tmpdir();

        let path1 = dir.path().join("notfound1");
        let path2 = dir.path().join("notfound2");

        let result = rename_file(&path1, &path2);

        match *result.unwrap_err().kind() {
            ErrorKind::Unix(x) => assert_eq!(x, nix::Error::Sys(nix::errno::Errno::ENOENT)),
            _ => assert!(false)
        }
    }

    #[test]
    fn hard_link() {
        let dir = tmpdir();

        let mut file = NamedTempFile::new_in(dir.path()).unwrap();
        write!(file, "{}", "content").unwrap();

        let link_path = dir.path().join("tmplink");
        
        link_at(file.path(), &link_path).unwrap();

        let meta = file.metadata().unwrap();
        let link_meta = fs::metadata(link_path).unwrap();

        assert_eq!(meta.ino(), link_meta.ino());
        assert_eq!(meta.nlink(), 2);
    }

    #[test]
    fn hard_link_follow_symlink() {
        let dir = tmpdir();

        let mut file = NamedTempFile::new_in(dir.path()).unwrap();
        write!(file, "{}", "content").unwrap();

        let symlink_path = dir.path().join("symlink");
        unix::fs::symlink(file.path(), &symlink_path).unwrap();

        let hardlink_path = dir.path().join("hardlink");
        link_at(&symlink_path, &hardlink_path).unwrap();

        let meta = file.metadata().unwrap();
        let link_meta = fs::metadata(hardlink_path).unwrap();

        assert_eq!(meta.ino(), link_meta.ino());
        assert_eq!(meta.nlink(), 2);
    }

}