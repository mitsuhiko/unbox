use std::fs::File;
use std::io::{copy, BufReader};
use std::path::{Component, Path, PathBuf};

#[cfg(unix)]
use {std::ffi::OsStr, std::os::unix::ffi::OsStrExt};

use ar::Archive as ArArchiveReader;
use failure::Error;

use crate::archive::{Archive, UnpackHelper};

#[derive(Debug)]
pub struct ArArchive {
    path: PathBuf,
    total_size: u64,
}

impl ArArchive {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let path = path.as_ref().canonicalize()?;
        let total_size = path.metadata()?.len();
        Ok(ArArchive { path, total_size })
    }
}

impl Archive for ArArchive {
    fn path(&self) -> &Path {
        &self.path
    }

    fn total_size(&self) -> Option<u64> {
        Some(self.total_size)
    }

    fn unpack(&mut self, helper: &mut UnpackHelper) -> Result<(), Error> {
        let f = BufReader::new(helper.wrap_read(File::open(&self.path)?));
        let mut archive = ArArchiveReader::new(f);

        while let Some(entry) = archive.next_entry() {
            let mut entry = entry?;
            let header = entry.header();
            let path = {
                #[cfg(windows)]
                {
                    PathBuf::from(String::from_utf8(header.identifier().into())?)
                }
                #[cfg(unix)]
                {
                    PathBuf::from(OsStr::from_bytes(header.identifier()))
                }
            };

            if path.components().any(|component| match component {
                Component::ParentDir | Component::RootDir | Component::Prefix(..) => true,
                Component::Normal(..) | Component::CurDir => false,
            }) {
                continue;
            }
            helper.report_file(&path);

            let mut f = File::create(helper.path().join(&path))?;
            copy(&mut entry, &mut f)?;
        }
        Ok(())
    }
}
