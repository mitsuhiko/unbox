use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use failure::Error;
use libflate::gzip;
use tar::Archive as TarArchiveReader;

use crate::archive::{Archive, UnpackHelper};

/// The compression of the tarball.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TarCompression {
    Uncompressed,
    Gzip,
}

#[derive(Debug)]
pub struct TarArchive {
    path: PathBuf,
    total_size: u64,
    compression: TarCompression,
}

impl TarArchive {
    pub fn open<P: AsRef<Path>>(path: P, compression: TarCompression) -> Result<Self, Error> {
        let path = path.as_ref().canonicalize()?;
        let total_size = path.metadata()?.len();
        Ok(TarArchive {
            path,
            total_size,
            compression,
        })
    }
}

impl Archive for TarArchive {
    fn path(&self) -> &Path {
        &self.path
    }

    fn total_size(&self) -> Option<u64> {
        Some(self.total_size)
    }

    fn unpack(&mut self, helper: &mut UnpackHelper) -> Result<(), Error> {
        match self.compression {
            TarCompression::Uncompressed => unpack_all(
                TarArchiveReader::new(BufReader::new(helper.wrap_read(File::open(&self.path)?))),
                helper,
            ),
            TarCompression::Gzip => unpack_all(
                TarArchiveReader::new(gzip::Decoder::new(BufReader::new(
                    helper.wrap_read(File::open(&self.path)?),
                ))?),
                helper,
            ),
        }
    }
}

fn unpack_all<R: Read>(
    mut rdr: TarArchiveReader<R>,
    helper: &mut UnpackHelper,
) -> Result<(), Error> {
    for entry in rdr.entries()? {
        let mut entry = entry?;
        if let Ok(path) = entry.path() {
            helper.report_file(&path);
        }
        entry.unpack_in(helper.path())?;
    }
    Ok(())
}
