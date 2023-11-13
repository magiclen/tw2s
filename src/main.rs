mod cli;

use std::{
    env, fs,
    fs::File,
    io,
    io::{BufRead, BufReader, Write},
};

use anyhow::{anyhow, Context};
use cli::*;
use opencc_rust::{generate_static_dictionary, DefaultConfig, OpenCC};

fn main() -> anyhow::Result<()> {
    let args = get_args();

    let temporary_path = env::temp_dir();

    generate_static_dictionary(&temporary_path, DefaultConfig::TW2SP).unwrap();

    let opencc = OpenCC::new(temporary_path.join(DefaultConfig::TW2SP)).unwrap();
    debug_assert_eq!("测试字符串", opencc.convert("測試字串"));

    match args.tw_path {
        Some(tw_path) => {
            if tw_path.is_dir() {
                return Err(anyhow!("{tw_path:?} is a directory!"));
            }

            let tw_file = File::open(tw_path.as_path()).with_context(|| anyhow!("{tw_path:?}"))?;

            let s_path = match args.s_path {
                Some(s_path) => s_path,
                None => {
                    let parent = tw_path.parent().unwrap();

                    let file_stem = match tw_path.file_stem() {
                        Some(file_stem) => {
                            let file_stem = file_stem
                                .to_str()
                                .ok_or_else(|| anyhow!("{tw_path:?} is an unsupported path."))?;

                            file_stem.strip_suffix(".cht").unwrap_or(file_stem.as_ref())
                        },
                        None => "",
                    };

                    let file_stem = opencc.convert(file_stem);

                    let file_name = match tw_path.extension() {
                        Some(extension) => {
                            format!("{file_stem}.chs.{}", extension.to_string_lossy())
                        },
                        None => format!("{file_stem}.chs"),
                    };

                    parent.join(file_name)
                },
            };

            match s_path.metadata() {
                Ok(metadata) => {
                    if metadata.is_dir() {
                        return Err(anyhow!("{s_path:?} is a directory!"));
                    } else if !args.force {
                        return Err(anyhow!("{s_path:?} exists!"));
                    }
                },
                Err(error) if error.kind() == io::ErrorKind::NotFound => (),
                Err(error) => {
                    return Err(error).with_context(|| anyhow!("{s_path:?}"));
                },
            }

            let mut s_file =
                File::create(s_path.as_path()).with_context(|| anyhow!("{s_path:?}"))?;

            let mut tw_file = BufReader::new(tw_file);

            let mut line = String::new();

            loop {
                line.clear();

                let c = tw_file
                    .read_line(&mut line)
                    .map_err(|error| {
                        let _ = fs::remove_file(s_path.as_path());

                        error
                    })
                    .with_context(|| anyhow!("{tw_path:?}"))?;

                if c == 0 {
                    break;
                }

                s_file.write(&opencc.convert(&line[0..c]).into_bytes()).map_err(|error| {
                    let _ = fs::remove_file(s_path.as_path());

                    error
                })?;
            }
        },
        None => {
            let mut line = String::new();

            loop {
                line.clear();

                let c = io::stdin().read_line(&mut line).with_context(|| anyhow!("stdin"))?;

                if c == 0 {
                    break;
                }

                println!("{}", opencc.convert(&line[0..(c - 1)]));
            }
        },
    }

    Ok(())
}
