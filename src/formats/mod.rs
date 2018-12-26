use std::fmt;
use std::fs;
use std::io::{BufReader, Read};
use std::path::Path;

use failure::Error;
use lazy_static::lazy_static;
use regex::Regex;

use crate::archive::Archive;

mod tar;
mod zip;

pub use self::tar::{TarArchive, TarCompression};
pub use self::zip::ZipArchive;

/// An enum of supported archive types.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ArchiveType {
    Zip,
    Tar,
    TarGz,
    TarXz,
    TarBz2,
}

impl fmt::Display for ArchiveType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ArchiveType::Zip => write!(f, "zip archive"),
            ArchiveType::Tar => write!(f, "uncompressed tarball"),
            ArchiveType::TarGz => write!(f, "gzip-compressed tarball"),
            ArchiveType::TarXz => write!(f, "xz-compressed tarball"),
            ArchiveType::TarBz2 => write!(f, "bzip2-compressed tarball"),
        }
    }
}

impl ArchiveType {
    /// Determines the archive type for the given path.
    pub fn for_path<P: AsRef<Path>>(path: &P) -> Option<ArchiveType> {
        // determine by filename
        if let Some(filename) = path.as_ref().file_name().and_then(|x| x.to_str()) {
            for &(ref regex, ty) in BY_PATTERN.iter() {
                if regex.is_match(filename) {
                    return Some(ty);
                }
            }
        };

        // determine by magic
        let mut buf = [0u8; 4096];
        let f = fs::File::open(path).ok()?;
        let mut reader = BufReader::new(f);
        let size = reader.read(&mut buf[..]).ok()?;
        let mimetype = tree_magic::from_u8(&buf[..size]);

        BY_MIMETYPE.get(mimetype).cloned()
    }

    /// Opens the given path as an archive of the type.
    pub fn open<P: AsRef<Path>>(self, path: &P) -> Result<Box<dyn Archive>, Error> {
        match self {
            ArchiveType::Zip => Ok(Box::new(ZipArchive::open(path)?)),
            ArchiveType::Tar => Ok(Box::new(TarArchive::open(
                path,
                TarCompression::Uncompressed,
            )?)),
            ArchiveType::TarGz => Ok(Box::new(TarArchive::open(path, TarCompression::Gzip)?)),
            ArchiveType::TarXz => Ok(Box::new(TarArchive::open(path, TarCompression::Xz)?)),
            ArchiveType::TarBz2 => Ok(Box::new(TarArchive::open(path, TarCompression::Bz2)?)),
        }
    }
}

lazy_static! {
    /// A mapping of mimetype to archive type.
    pub static ref BY_MIMETYPE: std::collections::HashMap<&'static str, ArchiveType> = {
        let mut rv = std::collections::HashMap::new();
        rv.insert("application/zip", ArchiveType::Zip);
        rv.insert("application/x-tar", ArchiveType::Tar);
        rv
    };

    /// Mapping of regexes to filenames.
    pub static ref BY_PATTERN: Vec<(Regex, ArchiveType)> = vec![
        (Regex::new(r"(?i)\.zip$").unwrap(), ArchiveType::Zip),
        (Regex::new(r"(?i)\.tar$").unwrap(), ArchiveType::Tar),
        (Regex::new(r"(?i)\.t(ar.gz|gz)$").unwrap(), ArchiveType::TarGz),
        (Regex::new(r"(?i)\.tar.xz$").unwrap(), ArchiveType::TarXz),
        (Regex::new(r"(?i)\.tar.bz2$").unwrap(), ArchiveType::TarBz2),
    ];
}
