use std::fmt;
use std::fs;
use std::io::{BufReader, Cursor, Read};
use std::path::Path;

use failure::Error;
use lazy_static::lazy_static;
use petgraph::Direction;
use regex::Regex;
use strum_macros::EnumIter;

use crate::archive::Archive;

mod ar;
mod compression;
mod tar;
mod zip;

pub use self::ar::ArArchive;
pub use self::compression::{Compression, SingleFileArchive};
pub use self::tar::TarArchive;
pub use self::zip::ZipArchive;

// base types we do not care about.
const BASE_TYPES: [&str; 5] = [
    "all/all",
    "all/allfiles",
    "inode/directory",
    "text/plain",
    "application/octet-stream",
];

/// An enum of supported archive types.
#[derive(Copy, Clone, PartialEq, Eq, EnumIter)]
pub enum ArchiveType {
    Ar,
    Zip,
    Tar,
    TarGz,
    TarXz,
    TarBz2,
    SingleFileGz,
    SingleFileXz,
    SingleFileBz2,
}

impl fmt::Display for ArchiveType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ArchiveType::Ar => write!(f, "unix ar archive"),
            ArchiveType::Zip => write!(f, "zip archive"),
            ArchiveType::Tar => write!(f, "uncompressed tarball"),
            ArchiveType::TarGz => write!(f, "gzip-compressed tarball"),
            ArchiveType::TarXz => write!(f, "xz-compressed tarball"),
            ArchiveType::TarBz2 => write!(f, "bzip2-compressed tarball"),
            ArchiveType::SingleFileGz => write!(f, "gzip-compressed file"),
            ArchiveType::SingleFileBz2 => write!(f, "bzip2-compressed file"),
            ArchiveType::SingleFileXz => write!(f, "xz-compressed file"),
        }
    }
}

/// Given some types this tries to determine the mimetype of the item.
///
/// It does not return child mimetypes which means that for instance an
/// open office text document is determined to be a zip archive.
fn get_mimetype(bytes: &[u8]) -> &'static str {
    let mut mimetype = tree_magic::from_u8(bytes);

    // walk up the graph until we hit the first non base type
    let graph = &tree_magic::TYPE.graph;
    let node_index = tree_magic::TYPE.hash[&mimetype];
    for index in graph.neighbors_directed(node_index, Direction::Incoming) {
        let parent_mimetype = graph[index];
        if BASE_TYPES.contains(&parent_mimetype) {
            break;
        }
        mimetype = parent_mimetype;
    }

    mimetype
}

impl ArchiveType {
    /// Determines the archive type for the given path.
    ///
    /// This first tries to determine the file contents purely by reading the
    /// first few hundred of kilobytes and then falls back to guessing based
    /// on the filename.
    pub fn for_path<P: AsRef<Path>>(path: &P) -> Option<ArchiveType> {
        ArchiveType::determine_by_magic(path).or_else(|| ArchiveType::determine_by_filename(path))
    }

    fn determine_by_filename<P: AsRef<Path>>(path: &P) -> Option<ArchiveType> {
        // determine by filename
        if let Some(filename) = path.as_ref().file_name().and_then(|x| x.to_str()) {
            for &(ref regex, ty) in BY_PATTERN.iter() {
                if regex.is_match(filename) {
                    return Some(ty);
                }
            }
        };
        None
    }

    fn determine_by_magic<P: AsRef<Path>>(path: &P) -> Option<ArchiveType> {
        // determine by magic
        let mut buf = [0u8; 131_072];
        let f = fs::File::open(path).ok()?;
        let mut reader = BufReader::new(f);
        let size = reader.read(&mut buf[..]).ok()?;
        let mimetype = get_mimetype(&buf[..size]);

        // if we get a direct hit, then we know what we are dealing with.  These
        // intentionally do not include mimetypes for pure compession algorithms
        // such as gzip
        if let Some(&rv) = BY_MIMETYPE.get(mimetype) {
            return Some(rv);
        }

        // if the mimetype points to a compression we unpack a bit of the magic
        // to see if we can detect an interior archive.
        let compression = Compression::for_mimetype(mimetype)?;
        let inner_ty = ArchiveType::determine_behind_compession(&buf[..size], compression);
        compression.as_archive_type(inner_ty)
    }

    fn determine_behind_compession(buf: &[u8], compression: Compression) -> Option<ArchiveType> {
        let mut rdr = compression.decompress(Cursor::new(buf.to_vec())).ok()?;
        let mut zbuf = [0u8; 131_072];
        let size = rdr.read(&mut zbuf[..]).ok()?;
        let mimetype = get_mimetype(&zbuf[..size]);

        if let Some(&ty) = BY_MIMETYPE.get(mimetype) {
            return compression.as_archive_type(Some(ty));
        }

        None
    }

    /// Opens the given path as an archive of the type.
    pub fn open<P: AsRef<Path>>(self, path: &P) -> Result<Box<dyn Archive>, Error> {
        match self {
            ArchiveType::Ar => Ok(Box::new(ArArchive::open(path)?)),
            ArchiveType::Zip => Ok(Box::new(ZipArchive::open(path)?)),
            ArchiveType::Tar => Ok(Box::new(TarArchive::open(path, Compression::Uncompressed)?)),
            ArchiveType::TarGz => Ok(Box::new(TarArchive::open(path, Compression::Gz)?)),
            ArchiveType::TarXz => Ok(Box::new(TarArchive::open(path, Compression::Xz)?)),
            ArchiveType::TarBz2 => Ok(Box::new(TarArchive::open(path, Compression::Bz2)?)),
            ArchiveType::SingleFileGz => {
                Ok(Box::new(SingleFileArchive::open(path, Compression::Gz)?))
            }
            ArchiveType::SingleFileBz2 => {
                Ok(Box::new(SingleFileArchive::open(path, Compression::Bz2)?))
            }
            ArchiveType::SingleFileXz => {
                Ok(Box::new(SingleFileArchive::open(path, Compression::Xz)?))
            }
        }
    }
}

lazy_static! {
    /// A mapping of mimetype to archive type.
    ///
    /// These do not contain mimetypes for compression algorithms as they are
    /// specially handled.
    static ref BY_MIMETYPE: std::collections::HashMap<&'static str, ArchiveType> = {
        let mut rv = std::collections::HashMap::new();
        rv.insert("application/zip", ArchiveType::Zip);
        rv.insert("application/x-tar", ArchiveType::Tar);
        rv.insert("application/x-archive", ArchiveType::Ar);
        rv
    };

    /// Mapping of regexes to filenames.
    static ref BY_PATTERN: Vec<(Regex, ArchiveType)> = vec![
        (Regex::new(r"(?i)\.ar?$").unwrap(), ArchiveType::Ar),
        (Regex::new(r"(?i)\.zip$").unwrap(), ArchiveType::Zip),
        (Regex::new(r"(?i)\.tar$").unwrap(), ArchiveType::Tar),
        (Regex::new(r"(?i)\.t(ar\.gz|gz)$").unwrap(), ArchiveType::TarGz),
        (Regex::new(r"(?i)\.t(ar\.xz|xz)$").unwrap(), ArchiveType::TarXz),
        (Regex::new(r"(?i)\.t(ar\.bz2|bz2?)$").unwrap(), ArchiveType::TarBz2),
    ];
}
