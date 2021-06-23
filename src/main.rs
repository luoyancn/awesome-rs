#[macro_use]
extern crate slog_logger;
extern crate slog_scope;
extern crate slog_stdlog;

use futurebase;
use procmacros::{Builder, BuilderEach};
use short_lived;

fn main() {
    let logger = slog_logger::initlogger(false, "", 0, true, false);
    let _guard = slog_scope::set_global_logger(logger);
    slog_stdlog::init().unwrap();

    /*
    static ARRAY_REF: &[i32] = &[12, 3, 45, 98, 100, 23, 878, 8765, 123, -897, 866666, 1241];
    let res = short_lived::find_max_crossbeam(ARRAY_REF);
    info!("The res is {:?}", res);
    let array = [
        12, 3, 45, 98, 100, 23, 878, 8765, 123, -897, 866666, 12411234,
    ];
    let res = short_lived::find_max_crossbeam(&array);
    info!("The res is {:?}", res);

    futurebase::async_thread();
    */

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
