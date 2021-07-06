use std::marker::PhantomData;

use procmacros::{Builder, BuilderEach, CustomDebug};

#[macro_use]
extern crate slog_logger;
extern crate slog_scope;

fn main() {
    slog_logger::setup_logger(false, "", 0, true, false);
    let builder = Command::builder()
        .executable("lucifer".to_owned())
        .args(vec![])
        .env(vec![])
        .current_dir("target".to_owned())
        .build()
        .unwrap();
    info!("{:#?}", builder);

    let builder = CommandEach::builder()
        .executable("lucifer".to_owned())
        .arg("lucifer".to_owned())
        .arg("titans".to_owned())
        .env("zhangjl".to_owned())
        .env("luoyan".to_owned())
        .others(vec![])
        .build()
        .unwrap();
    info!("{:#?}", builder);

    let builder = CommandEach::builder()
        .executable("lucifer".to_owned())
        .others(vec![])
        .build()
        .unwrap();
    info!("{:#?}", builder);
}

#[derive(Debug, Builder)]
pub struct Command {
    executable: String,
    args: Vec<String>,
    env: Vec<String>,
    current_dir: Option<String>,
}

#[derive(Debug, BuilderEach)]
pub struct CommandEach {
    executable: String,
    #[builder(each = "arg")]
    args: Vec<String>,
    #[builder(each = "env")]
    env: Vec<String>,
    others: Vec<String>,
    current_dir: Option<String>,
}

pub trait TraitA {
    type Value;
}
#[derive(CustomDebug)]
pub struct Custom<'a, T, U: TraitA> {
    name: &'a str,
    #[debug = "0b{:08b}"]
    age: u8,
    other: T,
    complex: PhantomData<U::Value>,
}
