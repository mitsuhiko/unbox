use clap::{App, Arg};
use failure::{bail, Error};
use tree_magic;

use crate::archive::{ArchiveType, UnpackHelper};

pub fn main() -> Result<(), Error> {
    let app = App::new("unbox")
        .about("Unpacks various archives")
        .author("Armin Ronacher <armin.ronacher@active-4.com>")
        .arg(
            Arg::with_name("archives")
                .index(1)
                .multiple(true)
                .help("The archives to unpack"),
        );
    let matches = app.get_matches();

    let mut archives = vec![];

    for path in matches.values_of("archives").unwrap() {
        if let Some(ty) = ArchiveType::for_path(&path) {
            archives.push(ty.open(&path)?);
        } else {
            bail!("Could not determine archive type of '{}'", path);
        }
    }

    for mut archive in archives {
        let mut helper = UnpackHelper::create(&*archive, &".")?;
        archive.unpack(&mut helper)?;
        let path = helper.commit()?;
        println!("{}", path.display());
    }

    Ok(())
}
