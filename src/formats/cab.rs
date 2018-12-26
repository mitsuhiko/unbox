use std::fmt;
use std::fs::File;
use std::io::{BufReader};
use std::path::{Path, PathBuf};

use cab::Cabinet;
use failure::Error;

use crate::archive::{Archive, UnpackHelper};

pub struct CabArchive {
    cab: Cabinet<BufReader<File>>,
    total_size: u64,
    path: PathBuf,
}

impl fmt::Debug for CabArchive {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CabArchive")
            .field("total_size", &self.total_size)
            .field("path", &self.path)
            .finish()
    }
}

impl CabArchive {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let path = path.as_ref().canonicalize()?;
        let f = BufReader::new(File::open(&path)?);
        let cab = Cabinet::new(f)?;
        let mut total_size = 0;
        for folder_entry in cab.folder_entries() {
            for file_entry in folder_entry.file_entries() {
                total_size += u64::from(file_entry.uncompressed_size());
            }
        }
        Ok(CabArchive { path, cab, total_size })
    }
}

impl Archive for CabArchive {
    fn path(&self) -> &Path {
        &self.path
    }

    fn total_size(&self) -> Option<u64> {
        Some(self.total_size)
    }

    fn unpack(&mut self, helper: &mut UnpackHelper) -> Result<(), Error> {
        let mut entries = vec![];
        for folder_entry in self.cab.folder_entries() {
            for file_entry in folder_entry.file_entries() {
                entries.push(file_entry.name().to_string());
            }
        }

        for name in entries {
            let rdr = self.cab.read_file(&name)?;
            helper.write_file_with_progress(&name.replace('\\', "/"), rdr)?;
        }

        Ok(())
    }
}
