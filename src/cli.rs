use clap::{App, AppSettings, Arg};
use console::style;
use failure::{bail, Error};
use tree_magic;

use crate::archive::UnpackHelper;
use crate::formats::ArchiveType;

pub fn main() -> Result<(), Error> {
    let app = App::new("unbox")
        .about("Unpacks various archives")
        .author("Armin Ronacher <armin.ronacher@active-4.com>")
        .setting(AppSettings::ArgRequiredElseHelp)
        .arg(
            Arg::with_name("analyze")
                .long("analyze")
                .help("For each archive print out the format"),
        )
        .arg(
            Arg::with_name("archives")
                .index(1)
                .multiple(true)
                .help("The archives to unpack"),
        );
    let matches = app.get_matches();
    let files: Vec<&str> = matches.values_of("archives").unwrap().collect();

    if matches.is_present("analyze") {
        analyze_archives(&files[..])?;
    } else {
        unpack_archives(&files[..])?;
    }

    Ok(())
}

pub fn analyze_archives(files: &[&str]) -> Result<(), Error> {
    for path in files {
        if let Some(ty) = ArchiveType::for_path(&path) {
            println!("{}: {}", style(path).dim(), style(ty).cyan());
        } else {
            println!("{}: {}", style(path).dim(), style("unsupported").red());
        }
    }
    Ok(())
}

pub fn unpack_archives(files: &[&str]) -> Result<(), Error> {
    let mut archives = vec![];

    for path in files {
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
