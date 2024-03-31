use std::path::{Path, PathBuf};

use once_cell::sync::Lazy;

pub struct LanguageDef<'def> {
    pub(crate) name: &'def str,
    pub(crate) extensions: &'def [&'def str],
    pub(crate) comments: &'def [&'def Lazy<regex::Regex>],
    pub(crate) indicators: &'def [&'def Lazy<glob::Pattern>],
    pub(crate) exclude: &'def [&'def Lazy<glob::Pattern>],
}

impl<'def> LanguageDef<'def> {
    pub fn match_against(&self, dirs: &[PathBuf]) -> bool {
        for dir in dirs {
            if let Some(file_ext) = dir.extension() {
                if self.extensions.iter().find(|ext| Some(**ext) == file_ext.to_str()).is_some() { return true; };
            }

            for indicator in self.indicators {
                if let Some(file_name) = dir.file_name() {
                    if indicator.matches(file_name.to_str().unwrap()) { return true; }
                }
            }
        }
        return false;
    }

    pub fn remove_comments(&self, code: String) -> String {
        let mut code = code;
        for comment_def in self.comments {
            let tmp = comment_def.replace_all(&code, "").to_string();
            code = tmp;
        }
        code
    }

    pub fn is_excluded(&self, file: &Path) -> bool {
        if let Some(file_name) = file.file_name() {
            self.exclude.iter().find(|e| e.matches(file_name.to_str().unwrap())).is_some()
        } else { false }
    }
}

include!(concat!(env!("OUT_DIR"), "/langdefs.rs"));
