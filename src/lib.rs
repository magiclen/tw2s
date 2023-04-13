//! # tw2s
//! A simple tool for converting Traditional Chinese(TW) to Simple Chinese.

use std::{fs, path::Path};

pub fn try_delete<P: AsRef<Path>>(path: P) {
    if fs::remove_file(path.as_ref()).is_err() {}
}
