use std::{collections::HashMap, fs::File, io::{BufWriter, Write}, path::Path};
use genco::prelude::*;

fn main() {
    println!("cargo:rerun-if-changed=langs.toml");

    let parsed: HashMap<String, Language> = toml::from_str(
        &std::fs::read_to_string("./langs.toml")
            .expect("'langs.toml' file should be present in project root.")
    ).expect("langs.toml should be valid toml syntax.");

    let mut tokens = rust::Tokens::new();
    let mut extension_map = phf_codegen::Map::new();
    let mut all_langs = Vec::new();
    for (ref language, def) in parsed.into_iter() {
        let lang = language.to_uppercase();
        
        quote_in! { tokens =>
            // Indicators
            $(if def.indicators.is_some() => $(for (i, ind) in def.indicators.as_ref().unwrap().iter().enumerate() {
                static $(format!("{}_INDICATOR_{}", lang, i)): Lazy<::glob::Pattern> =
                    Lazy::new(|| ::glob::Pattern::new($[str]($[const](ind))).unwrap());
            }))

            // Excludes
            $(if def.exclude.is_some() => $(for (i, exc) in def.exclude.as_ref().unwrap().iter().enumerate() {
                static $(format!("{}_EXCLUDE_{}", lang, i)): Lazy<::glob::Pattern> =
                    Lazy::new(|| ::glob::Pattern::new($[str]($[const](exc))).unwrap());
            }))

            // Comments
            $(for (i, cmt) in def.comments.iter().enumerate() {
                static $(format!("{}_COMMENT_{}", lang, i)): Lazy<::regex::Regex> =
                    Lazy::new(|| ::regex::Regex::new($[str]($[const](cmt))).unwrap());
            })

            pub static $(lang.clone()): LanguageDef<'static> = LanguageDef {
                name: $[str]($[const](language)),
                extensions: &[$(for r in &def.extensions => $[str]($[const](r)),)],
                comments: &[$(for i in 0..def.comments.len() => &$(format!("{}_COMMENT_{}", lang, i)),)],
                indicators: $(if def.indicators.is_some() { 
                    &[$(for i in 0..def.indicators.unwrap().len() => &$(format!("{}_INDICATOR_{}", lang, i)),)]
                } else { &[] }),
                exclude: $(if def.exclude.is_some() { 
                    &[$(for i in 0..def.exclude.unwrap().len() => &$(format!("{}_EXCLUDE_{}", lang, i)),)]
                } else { &[] }),
            };
        };

        all_langs.push(lang.clone());
        for ext in def.extensions {
            extension_map.entry(ext, &format!("&{}", lang));
        }
    }

    quote_in! { tokens =>
        pub static ALL_LANGS: &'static [&'static LanguageDef<'static>] =
            &[$(for lang in all_langs => &$lang,)];
        pub static EXTENSIONS_MAP: ::phf::Map<&'static str, &'static LanguageDef> = 
            $(format!("{}", extension_map.build()));
    };

    let target_path = Path::new(&std::env::var("OUT_DIR").unwrap()).join("langdefs.rs");
    let mut file = BufWriter::new(File::create(&target_path).unwrap());

    write!(
        &mut file,
        "{}",
        tokens.to_file_string().expect("Could not generate langdefs.rs")
    ).unwrap();
}

#[derive(serde::Deserialize, Debug)]
struct Language {
    extensions: Vec<String>,
    comments: Vec<String>,
    indicators: Option<Vec<String>>,
    exclude: Option<Vec<String>>
}
