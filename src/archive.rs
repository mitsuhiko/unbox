use std::fmt::Debug;
use std::fs;
use std::io::{self, BufReader, Read, Write};
use std::path::{Path, PathBuf};

use failure::Error;
use indicatif::{ProgressBar, ProgressBarRead, ProgressStyle};
use tree_magic;
use uuid::Uuid;

use crate::utils::{rename_resolving_conflict, TempDirectory};

pub fn copy_with_progress<R: ?Sized, W: ?Sized>(
    progress: &ProgressBar,
    reader: &mut R,
    writer: &mut W,
) -> io::Result<u64>
where
    R: Read,
    W: Write,
{
    let mut buf = [0; 131_072];
    let mut written = 0;
    loop {
        let len = match reader.read(&mut buf) {
            Ok(0) => return Ok(written),
            Ok(len) => len,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        writer.write_all(&buf[..len])?;
        written += len as u64;
        progress.inc(len as u64);
    }
}

pub trait Archive: Debug {
    /// The path to the archive.
    fn path(&self) -> &Path;

    /// The total size of the archive in bytes (uncompressed)
    fn total_size(&self) -> Option<u64> {
        None
    }

    /// Unpack the archive into the unpack helper.
    fn unpack(&mut self, helper: &mut UnpackHelper) -> Result<(), Error>;
}

#[derive(Debug)]
pub struct UnpackHelper {
    archive_base: String,
    dst: PathBuf,
    tmp: TempDirectory,
    pb: ProgressBar,
}

impl UnpackHelper {
    /// Creates an unpack helper for an archive.
    pub fn create<P: AsRef<Path>>(archive: &Archive, dst: &P) -> Result<UnpackHelper, Error> {
        let archive_base = archive
            .path()
            .file_stem()
            .map(|x| x.to_string_lossy().to_string())
            .unwrap_or_else(|| "Archive".to_string());
        let dst = dst.as_ref().canonicalize()?;
        let pb = match archive.total_size() {
            Some(total_size) => {
                let pb = ProgressBar::new(total_size);
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template(" {spinner} {bar:16.cyan.dim}  {wide_msg:.dim} {bytes}/{total_bytes} eta {eta}")
                        .progress_chars("█▉▊▋▌▍▎▏  ")
                );
                pb
            }
            None => {
                let pb = ProgressBar::new_spinner();
                pb.set_style(ProgressStyle::default_bar().template("{spinner}  {wide_msg:.dim}"));
                pb
            }
        };

        pb.enable_steady_tick(200);

        let tmp = TempDirectory::for_path(&dst.join(format!(".unbox-{}", Uuid::new_v4())))?;
        Ok(UnpackHelper {
            archive_base,
            dst,
            tmp,
            pb,
        })
    }

    /// Reports the temporary scratchpad path.
    pub fn path(&self) -> &Path {
        self.tmp.path()
    }

    /// Reports operating on a file.
    pub fn report_file<P: AsRef<Path>>(&mut self, filename: P) {
        self.pb
            .set_message(&format!("{}", filename.as_ref().display()));
    }

    /// Wraps a stream with the progress bar reader.
    pub fn wrap_read<R: Read>(&self, read: R) -> ProgressBarRead<R> {
        self.pb.wrap_read(read)
    }

    /// Writes into a file.
    pub fn write_file<P: AsRef<Path>>(&mut self, filename: P) -> Result<fs::File, Error> {
        let path = self.tmp.path().join(filename.as_ref());
        if let Some(ref parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        self.report_file(filename);
        Ok(fs::File::create(path)?)
    }

    /// Like `write_file` but writes directly from a reader
    /// and advances the contained progress bar by the decompressed
    /// bytes read.
    pub fn write_file_with_progress<R: Read, P: AsRef<Path>>(
        &mut self,
        filename: P,
        rdr: R,
    ) -> Result<(), Error> {
        let mut file = self.write_file(filename)?;
        copy_with_progress(&self.pb, &mut BufReader::new(rdr), &mut file)?;
        Ok(())
    }

    /// Commits the changes by moving the root of the unpacked
    /// archive to the destination folder.
    ///
    /// Returns the canonical destination path.
    pub fn commit(self) -> Result<PathBuf, Error> {
        self.pb.finish_and_clear();

        // if we found exactly one file or directory we can accept that as the
        // resulting file.
        let mut intended_dst = None;
        let mut to_move = None;
        for entry in self.tmp.path().read_dir()? {
            let entry = entry?;
            if intended_dst.is_none() {
                intended_dst = Some(self.dst.join(entry.path().strip_prefix(self.tmp.path())?));
                to_move = Some(entry.path().to_path_buf());
            } else {
                intended_dst = None;
                break;
            }
        }

        // there was only one thing in the archive, move it over.
        let rv = if let (Some(intended_dst), Some(to_move)) = (intended_dst, to_move) {
            rename_resolving_conflict(&to_move, &intended_dst)?

        // otherwise move the root
        } else {
            let intended_path = self.dst.join(&self.archive_base);
            rename_resolving_conflict(self.tmp.path(), &intended_path)?
        };

        self.tmp.cleanup()?;
        Ok(rv)
    }
}
