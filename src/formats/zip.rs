use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use failure::Error;
use zip::read::ZipArchive as ZipArchiveReader;

use crate::archive::{Archive, UnpackHelper};

#[derive(Debug)]
pub struct ZipArchive {
    path: PathBuf,
    rdr: ZipArchiveReader<BufReader<File>>,
    total_size: u64,
}

impl Archive for ZipArchive {
    fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let path = path.as_ref().canonicalize()?;
        let mut rdr = ZipArchiveReader::new(BufReader::new(File::open(&path)?))?;
        let total_size = (0..rdr.len())
            .map(|x| rdr.by_index(x).ok().map_or(0, |x| x.size()))
            .sum();
        Ok(ZipArchive {
            path,
            rdr,
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
        for idx in 0..self.rdr.len() {
            let file = self.rdr.by_index(idx)?;
            let name = file.sanitized_name();
            if file.unix_mode().unwrap_or(0) & 16384 == 0 && !name.ends_with("/") {
                helper.write_file_with_progress(name, file)?;
            }
        }
        Ok(())
    }
}
