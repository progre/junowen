use std::{collections::HashMap, fs};

use anyhow::Error;

pub struct Lang {
    lang: HashMap<String, String>,
}

impl Lang {
    pub fn new(lang: &str) -> Self {
        Self {
            lang: fs::read_to_string(format!("lang.{lang}"))
                .map_err(Error::new)
                .and_then(|s| toml::from_str(&s).map_err(Error::new))
                .unwrap_or_default(),
        }
    }

    pub fn print(&self, msg: &str) {
        print!("{}", self.lang.get(msg).map(|s| s.as_str()).unwrap_or(msg));
    }

    pub fn println(&self, msg: &str) {
        println!("{}", self.lang.get(msg).map(|s| s.as_str()).unwrap_or(msg));
    }
}
