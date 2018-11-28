//! # tw2s
//! A simple tool for converting Traditional Chinese(TW) to Simple Chinese.

extern crate clap;
extern crate opencc_rust;
extern crate path_absolutize;

use path_absolutize::*;

use std::env;
use std::path::Path;
use std::io::{self, Write, BufReader, BufRead};
use std::fs::{self, File};

use clap::{App, Arg};

use opencc_rust::*;

// TODO -----Config START-----

const APP_NAME: &str = "tw2s";
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const CARGO_PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

#[derive(Debug)]
pub struct Config {
    pub tw_path: Option<String>,
    pub s_path: Option<String>,
    pub force: bool,
}

impl Config {
    pub fn from_cli() -> Result<Config, String> {
        let arg0 = env::args().next().unwrap();
        let arg0 = Path::new(&arg0).file_stem().unwrap().to_str().unwrap();

        let examples = vec![
            "                               # Convert each of input lines from Traditional Chinese to Simple Chinese",
            "cht.txt chs.txt                # Convert cht.txt (in Traditional Chinese) to chs.txt (in Simple Chinese)",
            "a.cht.txt                      # Convert a.cht.txt (in Traditional Chinese) to a.chs.txt (in Simple Chinese)"
        ];

        let matches = App::new(APP_NAME)
            .version(CARGO_PKG_VERSION)
            .author(CARGO_PKG_AUTHORS)
            .about(format!("A simple tool for converting Traditional Chinese(TW) to Simple Chinese.\n\nEXAMPLES:\n{}", examples.iter()
                .map(|e| format!("  {} {}\n", arg0, e))
                .collect::<Vec<String>>()
                .concat()
            ).as_str()
            )
            .arg(Arg::with_name("FORCE")
                .long("force")
                .short("f")
                .help("Forces to output if the output file exists.")
            )
            .arg(Arg::with_name("TW_PATH")
                .help("Assigns the path of your Traditional Chinese document. It should be a file path.")
                .takes_value(true)
                .index(1)
            )
            .arg(Arg::with_name("S_PATH")
                .help("Assigns the path of your Simple Chinese document. It should be a file path.")
                .takes_value(true)
                .index(2)
            )
            .after_help("Enjoy it! https://magiclen.org")
            .get_matches();

        let tw_path = matches.value_of("TW_PATH").map(|s| s.to_string());

        let s_path = matches.value_of("S_PATH").map(|s| s.to_string());

        let force = matches.is_present("FORCE");

        Ok(Config {
            tw_path,
            s_path,
            force,
        })
    }
}

// TODO -----Config END-----

pub fn run(config: Config) -> Result<i32, String> {
    let temporary_path = env::temp_dir();

    generate_static_dictionary(&temporary_path, DefaultConfig::TW2SP).unwrap();

    let opencc = OpenCC::new(Path::join(&temporary_path, DefaultConfig::TW2SP)).unwrap();
    assert_eq!("测试字符串", opencc.convert("測試字串"));

    match config.tw_path {
        Some(tw_path) => {
            let tw_path = Path::new(&tw_path).absolutize().unwrap();

            if !tw_path.exists() {
                return Err(format!("`{}` does not exist!", tw_path.to_str().unwrap()));
            }

            if !tw_path.is_file() {
                return Err(format!("`{}` is not a file!", tw_path.to_str().unwrap()));
            }

            let s_path = match config.s_path {
                Some(s_path) => {
                    let s_path = Path::new(&s_path).absolutize().unwrap();

                    if s_path.exists() {
                        if config.force {
                            if !s_path.is_file() {
                                return Err(format!("`{}` is not a file!", s_path.to_str().unwrap()));
                            }
                        } else {
                            return Err(format!("`{}` exists!", s_path.to_str().unwrap()));
                        }
                    }

                    s_path
                }
                None => {
                    let parent = tw_path.parent().unwrap();

                    let file_stem = match tw_path.file_stem() {
                        Some(file_stem) => {
                            let file_stem = file_stem.to_str().unwrap();

                            if file_stem.ends_with(".chs") {
                                &file_stem[..file_stem.len() - 4]
                            } else {
                                file_stem
                            }
                        }
                        None => ""
                    };

                    let file_stem = opencc.convert(&file_stem);

                    let file_name = match tw_path.extension() {
                        Some(extension) => format!("{}.chs.{}", file_stem, extension.to_str().unwrap()),
                        None => format!("{}.chs", file_stem)
                    };

                    Path::join(parent, file_name)
                }
            };

            let tw_file = File::open(&tw_path).map_err(|_| format!("Cannot open {}.", tw_path.to_str().unwrap()))?;

            let mut tw_file = BufReader::new(tw_file);

            let mut s_file = File::create(&s_path).map_err(|_| format!("Cannot create {}.", s_path.to_str().unwrap()))?;

            let mut line = String::new();

            loop {
                line.clear();

                let c = tw_file.read_line(&mut line).map_err(|err| {
                    try_delete(&s_path);
                    err.to_string()
                })?;

                if c == 0 {
                    break;
                }

                s_file.write(&opencc.convert(&line[0..c]).into_bytes()).map_err(|err| {
                    try_delete(&s_path);
                    err.to_string()
                })?;
            }
        }
        None => {
            let mut line = String::new();
            loop {
                line.clear();

                let c = io::stdin().read_line(&mut line).map_err(|err| err.to_string())?;

                if c == 0 {
                    break;
                }

                println!("{}", opencc.convert(&line[0..(c - 1)]));
            }
        }
    }

    Ok(0)
}

fn try_delete<P: AsRef<Path>>(path: P) {
    if let Err(_) = fs::remove_file(path.as_ref()) {}
}