use clap::{App, AppSettings, Arg};
use console::style;
use failure::{bail, Error};
use strum::IntoEnumIterator;

use crate::archive::UnpackHelper;
use crate::formats::ArchiveType;

pub fn main() -> Result<(), Error> {
    let app = App::new("unbox")
        .about(
            "\
             unbox unpacks archives.\n\n\
             unbox is a no bullshit unpack command.  It can unpack a growing \
             range of file formats and just dumps them into the working \
             directory ensuring that only one item is created (single file or \
             folder).\
             ",
        )
        .after_help(
            "\
             Files are always unpacked into the working directory by going \
             through a temporary location first.  If you abort the tool all \
             files will be deleted.  Only one item is unpacked which is the \
             entire content of the archive.  If the archive does not have a \
             top level folder a new one is created with a name derived from \
             the archive file name.\
             ",
        )
        .author("Armin Ronacher <armin.ronacher@active-4.com>")
        .version(env!("CARGO_PKG_VERSION"))
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::ColoredHelp)
        .max_term_width(100)
        .arg(
            Arg::with_name("analyze")
                .long("analyze")
                .help("For each archive print out the format"),
        )
        .arg(
            Arg::with_name("list_formats")
                .long("list-formats")
                .help("List all supported formats"),
        )
        .arg(
            Arg::with_name("skip_unknown")
                .long("skip-unknown")
                .help("Skip silently over files that are not known archives"),
        )
        .arg(
            Arg::with_name("archives")
                .index(1)
                .multiple(true)
                .help("The archives to unpack"),
        );
    let matches = app.get_matches();

    if matches.is_present("list_formats") {
        println!("Supported file formats:");
        for variant in ArchiveType::iter() {
            println!("- {}", style(variant).cyan());
        }
        return Ok(());
    }

    let files: Vec<&str> = matches.values_of("archives").unwrap().collect();
    let skip_unknown = matches.is_present("skip_unknown");
    if matches.is_present("analyze") {
        analyze_archives(&files[..], skip_unknown)?;
    } else {
        unpack_archives(&files[..], skip_unknown)?;
    }

    Ok(())
}

pub fn analyze_archives(files: &[&str], skip_unknown: bool) -> Result<(), Error> {
    for path in files {
        if let Some(ty) = ArchiveType::for_path(&path) {
            println!("{}: {}", style(path).dim(), style(ty).cyan());
        } else if !skip_unknown {
            println!("{}: {}", style(path).dim(), style("unsupported").red());
        }
    }
    Ok(())
}

pub fn unpack_archives(files: &[&str], skip_unknown: bool) -> Result<(), Error> {
    let mut archives = vec![];

    for path in files {
        if let Some(ty) = ArchiveType::for_path(&path) {
            archives.push(ty.open(&path)?);
        } else if !skip_unknown {
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
