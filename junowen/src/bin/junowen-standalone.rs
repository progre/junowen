mod lang;

use std::{env::current_exe, process::ExitCode};

use junowen_lib::{hook_utils::do_dll_injection, lang::Lang};
use sys_locale::get_locales;

use crate::lang::to_lang_source;

fn create_lang() -> Lang {
    let lang = get_locales()
        .flat_map(|tag| {
            let primary_lang = tag.split('-').next().unwrap_or(&tag).to_owned();
            [tag, primary_lang]
        })
        .filter_map(|tag| to_lang_source(&tag))
        .find_map(|file| toml::from_str(file).ok())
        .unwrap_or_default();
    Lang::new(lang)
}

fn main() -> ExitCode {
    let lang = create_lang();

    let dll_path = current_exe()
        .unwrap()
        .as_path()
        .parent()
        .unwrap()
        .join(concat!(env!("CARGO_PKG_NAME"), ".dll"));
    if let Err(err) = do_dll_injection("th19.exe", &dll_path) {
        lang.print("failed injection into th19.exe");
        println!(": {}", err);
        println!();
        lang.println("you can close this window by pressing enter...");
        let _ = std::io::stdin().read_line(&mut String::new());
        return ExitCode::FAILURE;
    }

    lang.println("completed injection into th19.exe");
    println!();
    lang.println("you can close this window by pressing enter...");
    let _ = std::io::stdin().read_line(&mut String::new());
    ExitCode::SUCCESS
}
