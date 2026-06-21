use std::path::PathBuf;

fn main() {
    if std::env::args().any(|a| a == "--smoke-test") {
        let out = std::env::var("ROBBO_SMOKE_OUT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("target/smoke"));
        if let Err(e) = robbo_app::test_harness::run_smoke_scenario(&out) {
            eprintln!("smoke test failed: {e}");
            std::process::exit(1);
        }
        return;
    }

    robbo_app::build_app().run();
}
