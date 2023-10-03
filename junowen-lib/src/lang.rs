use std::{collections::HashMap, fs};

use sys_locale::get_locales;

pub struct Lang {
    lang: HashMap<String, String>,
}

impl Lang {
    pub fn resolve() -> Self {
        let lang = get_locales()
            .flat_map(|tag| {
                let primary_lang = tag.split('-').next().unwrap_or(&tag).to_owned();
                [tag, primary_lang]
            })
            .filter_map(|tag| fs::read_to_string(format!("lang/{}.toml", tag)).ok())
            .find_map(|file| toml::from_str(&file).ok())
            .unwrap_or_default();
        Self { lang }
    }

    pub fn print(&self, msg: &str) {
        print!("{}", self.lang.get(msg).map(|s| s.as_str()).unwrap_or(msg));
    }

    pub fn println(&self, msg: &str) {
        println!("{}", self.lang.get(msg).map(|s| s.as_str()).unwrap_or(msg));
    }
}
