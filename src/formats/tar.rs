use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use failure::Error;
use tar::Archive as TarArchiveReader;

use crate::archive::{Archive, UnpackHelper};

#[derive(Debug)]
pub struct TarArchive {
    path: PathBuf,
    total_size: u64,
}

impl Archive for TarArchive {
    fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let path = path.as_ref().canonicalize()?;
        let total_size = path.metadata()?.len();
        Ok(TarArchive {
            path,
            total_size,
        })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn total_size(&self) -> Option<u64> {
        Some(self.total_size)
    }

    fn unpack(&mut self, helper: &mut UnpackHelper) -> Result<(), Error> {
        let mut rdr = TarArchiveReader::new(BufReader::new(helper.wrap_read(File::open(&self.path)?)));
        for entry in rdr.entries()? {
            let mut entry = entry?;
            if let Ok(path) = entry.path() {
                helper.report_file(&path);
            }
            entry.unpack_in(helper.path())?;
        }
        Ok(())
    }
}
