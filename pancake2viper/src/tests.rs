use std::env;

use app::App;

use super::*;

use lazy_static::lazy_static;

lazy_static! {
    static ref VIPER: viper::Viper =
        viper::Viper::new_with_args(&env::var("VIPER_HOME").unwrap(), vec![]);
}

fn verify_file(path: &str) -> anyhow::Result<()> {
    let mut app = App::default();
    app.code = std::fs::read_to_string(path)?;
    app.verify = true;
    app.run(&VIPER)
}

fn verify_file_model(path: &str) -> anyhow::Result<()> {
    let mut app = App::default();
    app.code = std::fs::read_to_string(path)?;
    app.model = Some(std::fs::read_to_string("./tests/shared/model.vpr")?);
    app.verify = true;
    app.run(&VIPER)
}

include!(concat!(env!("OUT_DIR"), "/generated_tests.rs"));
