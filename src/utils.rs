use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use lazy_static::lazy_static;
use regex::Regex;
use uuid::Uuid;

lazy_static! {
    static ref INCR_REGEX: Regex = Regex::new(
        r#"(?x)
        ^(.+?)(\d+)(.*?)$
    "#
    )
    .unwrap();
}

/// Increments the last number in a string.
pub fn increment_string(s: &str) -> String {
    if let Some(caps) = INCR_REGEX.captures(s) {
        let num: u64 = caps[2].parse().unwrap();
        format!("{}{}{}", &caps[1], num + 1, &caps[3])
    } else {
        format!("{}-2", s)
    }
}

pub fn rename_resolving_conflict(src: &Path, dst: &Path) -> io::Result<PathBuf> {
    // simple case: dst does not exist yet
    if !dst.exists() {
        fs::rename(src, dst)?;
        return Ok(dst.to_path_buf());
    }

    let dst = env::current_dir()?.join(dst);
    let parent = dst.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::Other,
            "Could not determine parent for rename",
        )
    })?;
    let mut basename = dst
        .file_name()
        .map(|x| x.to_string_lossy().to_string())
        .unwrap();
    loop {
        let new_basename = increment_string(&basename);
        let new_dst = parent.join(&new_basename);
        if !new_dst.exists() {
            fs::rename(src, &new_dst)?;
            return Ok(new_dst);
        }
        basename = new_basename;
    }
}

/// When constructed with a path creates a temporary directory that can be
/// atomically moved over.
#[derive(Debug)]
pub struct TempDirectory {
    tmp: PathBuf,
    dst: PathBuf,
}

impl TempDirectory {
    /// Creates a temp directory that can be moved over to the given dst path.
    ///
    /// The parent directory of the dst folder must exist.
    pub fn for_path<P: AsRef<Path>>(dst: &P) -> io::Result<TempDirectory> {
        let mut dst = dst.as_ref().to_path_buf();
        if !dst.is_absolute() {
            dst = env::current_dir()?.join(dst);
        }

        let parent = match dst.parent() {
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "destination folder cannot be toplevel directory",
                ));
            }
            Some(parent) => {
                if !parent.exists() {
                    return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        "parent path of destination folder does not exist",
                    ));
                }
                parent
            }
        };

        let mut tmp = env::temp_dir();
        let basename = format!(".unbox-{}", Uuid::new_v4());
        tmp.push(&basename);
        let dummy_path = parent.join(&basename);

        // if we can successfully move from the temporary folder to our
        // destination folder in an atomic move we can use it as our
        // temp directory.
        if fs::create_dir(&tmp).is_ok()
            && fs::rename(&tmp, &dummy_path).is_ok()
            && fs::remove_dir(&dummy_path).is_ok()
        {
            fs::create_dir(&tmp)?;
            Ok(TempDirectory { tmp, dst })

        // otherwise we use a temporary folder within the destination path.
        } else {
            Ok(TempDirectory {
                tmp: parent.join(&basename),
                dst,
            })
        }
    }

    /// Returns the path to the temporary directory
    pub fn path(&self) -> &Path {
        &self.tmp
    }

    /// Removes everything.
    pub fn cleanup(self) -> io::Result<()> {
        match fs::remove_dir_all(&self.tmp) {
            Err(ref err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            other => other,
        }
    }
}

#[test]
fn test_increment_string() {
    assert_eq!(increment_string("foo"), "foo-2");
    assert_eq!(increment_string("foo-2"), "foo-3");
    assert_eq!(increment_string("foo-100"), "foo-101");
    assert_eq!(increment_string("foo-2.txt"), "foo-3.txt");
    assert_eq!(increment_string("Something (2)"), "Something (3)");
}
