use std::fs::File;
use std::io::{BufReader, Read};

use clap::{App, Arg};
use failure::Error;
use tree_magic;

use crate::archive::{open_archive, UnpackHelper};

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
    let mut buf = vec![0u8; 4096];

    for path in matches.values_of("archives").unwrap() {
        let f = File::open(path)?;
        let mut reader = BufReader::new(f);
        let size = reader.read(&mut buf[..])?;
        let magic = tree_magic::from_u8(&buf[..size]);
        archives.push(open_archive(path, Some(magic))?);
    }

    for mut archive in archives {
        let mut helper = UnpackHelper::create(&*archive, &".")?;
        archive.unpack(&mut helper)?;
        let path = helper.commit()?;
        println!("{}", path.display());
    }

    Ok(())
}
