use std::fmt;
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek};
use std::ops::Deref;
use std::path::{Path, PathBuf};

use cab::Cabinet;
use failure::{bail, Error};
use goblin::pe::PE;
use memmap::Mmap;
use owning_ref::OwningRef;

use crate::archive::{Archive, UnpackHelper};

trait ReadSeek: Read + Seek {}

impl<T: Read + Seek> ReadSeek for T {}

pub struct CabArchive {
    cab: Cabinet<Box<dyn ReadSeek>>,
    total_size: u64,
    path: PathBuf,
    files: Vec<String>,
}

impl fmt::Debug for CabArchive {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CabArchive")
            .field("total_size", &self.total_size)
            .field("path", &self.path)
            .finish()
    }
}

struct StableDerefMmap(Mmap);

impl Deref for StableDerefMmap {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        self.0.deref()
    }
}

unsafe impl stable_deref_trait::StableDeref for StableDerefMmap {}

impl CabArchive {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let path = path.as_ref().canonicalize()?;
        let f = BufReader::new(File::open(&path)?);
        let cab = Cabinet::new(Box::new(f) as Box<dyn ReadSeek>)?;
        CabArchive::from_cab_and_path(cab, path)
    }

    pub fn find_in_executable<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let path = path.as_ref().canonicalize()?;
        let f = File::open(&path)?;
        let mmap = unsafe { StableDerefMmap(Mmap::map(&f)?) };
        let pe = PE::parse(&mmap[..])?;

        let exesize = pe.sections.last().map_or(0, |sect| {
            u64::from(sect.pointer_to_raw_data + sect.size_of_raw_data)
        });

        if mmap.get(exesize as usize..exesize as usize + 4) == Some(&b"MSCF"[..]) {
            let owning_mmap = OwningRef::new(mmap);
            let owning_ref = owning_mmap.map(|mmap| &mmap[exesize as usize..]);
            let cab = Cabinet::new(Box::new(Cursor::new(owning_ref)) as Box<dyn ReadSeek>)?;
            CabArchive::from_cab_and_path(cab, path)
        } else {
            bail!("no cab in executable");
        }
    }

    fn from_cab_and_path(cab: Cabinet<Box<dyn ReadSeek>>, path: PathBuf) -> Result<Self, Error> {
        let mut total_size = 0;
        let mut files = vec![];
        for folder_entry in cab.folder_entries() {
            for file_entry in folder_entry.file_entries() {
                total_size += u64::from(file_entry.uncompressed_size());
                files.push(file_entry.name().to_string());
            }
        }
        Ok(CabArchive {
            path,
            cab,
            total_size,
            files,
        })
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
        for name in &self.files {
            let rdr = self.cab.read_file(&name)?;
            helper.write_file_with_progress(&name.replace('\\', "/"), rdr)?;
        }
        Ok(())
    }
}
