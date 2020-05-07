use strum_macros::{Display, EnumString};

/**
 * For every addition here, make changes to src/cli.yaml possible values
 */

#[derive(Display, EnumString)]
pub enum FuzzDriver {
    Aflpp,
    Honggfuzz,
    Fuzzilli,
    Libfuzzer,
}
