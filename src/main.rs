use std::borrow::Cow;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

use clap::{Arg, Command};
use terminal_size::terminal_size;

use concat_with::concat_line;

use opencc_rust::{generate_static_dictionary, DefaultConfig, OpenCC};

use path_absolutize::Absolutize;

use tw2s::*;

const APP_NAME: &str = "tw2s";
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const CARGO_PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new(APP_NAME)
        .term_width(terminal_size().map(|(width, _)| width.0 as usize).unwrap_or(0))
        .version(CARGO_PKG_VERSION)
        .author(CARGO_PKG_AUTHORS)
        .about(concat!("A simple tool for converting Traditional Chinese(TW) to Simple Chinese.\n\nEXAMPLES:\n", concat_line!(prefix "tw2s ",
            "                               # Convert each of input lines from Traditional Chinese to Simple Chinese",
            "cht.txt chs.txt                # Convert cht.txt (in Traditional Chinese) to chs.txt (in Simple Chinese)",
            "a.cht.txt                      # Convert a.cht.txt (in Traditional Chinese) to a.chs.txt (in Simple Chinese)"
        )))
        .arg(Arg::new("FORCE")
            .long("force")
            .short('f')
            .help("Force to output if the output file exists.")
        )
        .arg(Arg::new("TW_PATH")
            .help("Assign the path of your Traditional Chinese document. It should be a file path.")
            .takes_value(true)
            .index(1)
        )
        .arg(Arg::new("S_PATH")
            .help("Assign the path of your Simple Chinese document. It should be a file path.")
            .takes_value(true)
            .index(2)
        )
        .after_help("Enjoy it! https://magiclen.org")
        .get_matches();

    let tw_path = matches.value_of("TW_PATH");
    let s_path = matches.value_of("S_PATH");

    let force = matches.is_present("FORCE");

    let temporary_path = env::temp_dir();

    generate_static_dictionary(&temporary_path, DefaultConfig::TW2SP)?;

    let opencc = OpenCC::new(Path::join(&temporary_path, DefaultConfig::TW2SP))
        .map_err(|err| err.to_string())?;
    debug_assert_eq!("测试字符串", opencc.convert("測試字串"));

    match tw_path {
        Some(tw_path) => {
            let tw_path = Path::new(tw_path);

            if tw_path.is_dir() {
                return Err(format!(
                    "`{}` is a directory!",
                    tw_path.absolutize()?.to_string_lossy()
                )
                .into());
            }

            let tw_file = File::open(&tw_path)?;

            let s_path = match s_path {
                Some(s_path) => Cow::from(Path::new(s_path)),
                None => {
                    let parent = tw_path.parent().unwrap();

                    let file_stem = match tw_path.file_stem() {
                        Some(file_stem) => {
                            let file_stem = file_stem
                                .to_str()
                                .ok_or_else(|| String::from("Unsupported path."))?;

                            file_stem.strip_suffix(".cht").unwrap_or(file_stem)
                        }
                        None => "",
                    };

                    let file_stem = opencc.convert(&file_stem);

                    let file_name = match tw_path.extension() {
                        Some(extension) => {
                            format!("{}.chs.{}", file_stem, extension.to_string_lossy())
                        }
                        None => format!("{}.chs", file_stem),
                    };

                    let s_path = Path::join(parent, file_name);

                    Cow::from(s_path)
                }
            };

            if let Ok(metadata) = s_path.metadata() {
                if metadata.is_dir() || !force {
                    return Err(
                        format!("`{}` exists!", s_path.absolutize()?.to_string_lossy()).into()
                    );
                }
            }

            let mut s_file = File::create(s_path.as_ref())?;

            let mut tw_file = BufReader::new(tw_file);

            let mut line = String::new();

            loop {
                line.clear();

                let c = tw_file.read_line(&mut line).map_err(|err| {
                    try_delete(&s_path);
                    err
                })?;

                if c == 0 {
                    break;
                }

                s_file.write(&opencc.convert(&line[0..c]).into_bytes()).map_err(|err| {
                    try_delete(&s_path);
                    err
                })?;
            }
        }
        None => {
            let mut line = String::new();
            loop {
                line.clear();

                let c = io::stdin().read_line(&mut line)?;

                if c == 0 {
                    break;
                }

                println!("{}", opencc.convert(&line[0..(c - 1)]));
            }
        }
    }

    Ok(())
}
