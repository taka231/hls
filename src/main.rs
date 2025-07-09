pub mod a_normalize;
pub mod alpha;
pub mod ast;
pub mod calyx_ast;
pub mod convert;
pub mod parser;

use alpha::alpha_convert_program;
use anyhow::Result;
use parser::hls;

fn main() -> Result<()> {
    let program = hls::program(INPUT)?;
    let alpha_converted = alpha_convert_program(&program);
    let normalized = a_normalize::normalize_program(alpha_converted)?;
    let mut converter = convert::Converter::init();
    converter.convert(normalized)?;
    println!("{}", converter.program);
    Ok(())
}

static INPUT: &str = r#"
external a: i32[4];
external b: i32[4];
external out: i32[1];

fn main() = 
    let sum_a_b: i32[4] = map(a, b, (x, y) => x + y) in
    let squared: i32[4] = map(sum_a_b, (x) => x * x) in
    let result: i32 = reduce(squared, 0, (x, y) => x + y) in
    out[0] := result
"#;
