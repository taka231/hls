pub mod a_normalize;
pub mod alpha;
pub mod ast;
pub mod calyx_ast;
pub mod convert;
pub mod parser;

use alpha::alpha_convert_program;
use parser::hls;

fn main() {
    let ast = hls::program(IMPUT);
    match ast {
        Ok(program) => {
            let alpha_converted = alpha_convert_program(&program);
            match a_normalize::normalize_program(alpha_converted) {
                Ok(normalized) => {
                    let mut converter = convert::Converter::init();
                    match converter.convert(normalized) {
                        Ok(()) => {
                            println!("{}", converter.program);
                        }
                        Err(e) => {
                            println!("Conversion error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("A-normalization error: {}", e);
                }
            }
        }
        Err(e) => {
            println!("Parse error: {}", e);
        }
    }
}

static IMPUT: &str = r#"
// external a: i32[16];
// external b: i32[16];
external out: i32[1];

fn main() = 
    // let sum_a_b: i32[16] = map(a, b, (x, y) => x + y) in
    // let squared: i32[16] = map(sum_a_b, (x) => x * x) in
    // let result: i32 = reduce(squared, (x, y) => x + y) in
    // out[0] := result
    let x: i32 = 1 in
    let y: i32 = 2 in
    out[0] := x + y * 3
"#;
