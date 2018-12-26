use std::ffi::OsStr;
use std::fs::File;
use std::io::{copy, BufReader, Read};
use std::path::{Path, PathBuf};

use bzip2::read::BzDecoder;
use failure::Error;
use libflate::gzip;
use xz2::read::XzDecoder;

use crate::archive::{Archive, UnpackHelper};
use crate::formats::ArchiveType;

/// The compression of a normal file.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Compression {
    Uncompressed,
    Gz,
    Xz,
    Bz2,
}

#[derive(Debug)]
pub struct SingleFileArchive {
    path: PathBuf,
    compression: Compression,
    total_size: u64,
}

impl SingleFileArchive {
    pub fn open<P: AsRef<Path>>(path: P, compression: Compression) -> Result<Self, Error> {
        let path = path.as_ref().canonicalize()?;
        let total_size = path.metadata()?.len();
        Ok(SingleFileArchive {
            path,
            compression,
            total_size,
        })
    }
}

impl Archive for SingleFileArchive {
    fn path(&self) -> &Path {
        &self.path
    }

    fn total_size(&self) -> Option<u64> {
        Some(self.total_size)
    }

    fn unpack(&mut self, helper: &mut UnpackHelper) -> Result<(), Error> {
        let f = BufReader::new(helper.wrap_read(File::open(&self.path)?));
        let mut rdr = self.compression.decompress(f)?;
        helper.report_file(&self.path);
        let filename = self
            .path
            .file_stem()
            .unwrap_or_else(|| OsStr::new("Unknown"));
        let mut w = File::create(helper.path().join(filename))?;
        copy(&mut rdr, &mut w)?;
        Ok(())
    }
}

impl Compression {
    /// Returns the compression for a mimetype.
    pub fn for_mimetype(mimetype: &str) -> Option<Compression> {
        match mimetype {
            "application/gzip" => Some(Compression::Gz),
            "application/x-xz" => Some(Compression::Xz),
            "application/bzip2" => Some(Compression::Bz2),
            _ => None,
        }
    }

    /// Wraps a reader for transparent decompression.
    pub fn decompress<R: Read + 'static>(self, rdr: R) -> Result<Box<dyn Read>, Error> {
        match self {
            Compression::Uncompressed => Ok(Box::new(rdr)),
            Compression::Gz => Ok(Box::new(gzip::Decoder::new(rdr)?)),
            Compression::Xz => Ok(Box::new(XzDecoder::new(rdr))),
            Compression::Bz2 => Ok(Box::new(BzDecoder::new(rdr))),
        }
    }

    /// Returns the single file archive type.
    pub fn as_archive_type(self, parent: Option<ArchiveType>) -> Option<ArchiveType> {
        match parent {
            None => match self {
                Compression::Uncompressed => None,
                Compression::Gz => Some(ArchiveType::SingleFileGz),
                Compression::Bz2 => Some(ArchiveType::SingleFileBz2),
                Compression::Xz => Some(ArchiveType::SingleFileXz),
            },
            Some(ArchiveType::Tar) => match self {
                Compression::Uncompressed => Some(ArchiveType::Tar),
                Compression::Gz => Some(ArchiveType::TarGz),
                Compression::Bz2 => Some(ArchiveType::TarBz2),
                Compression::Xz => Some(ArchiveType::TarXz),
            },
            Some(..) => None,
        }
    }
}
