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
            println!("Original AST:");
            println!("{:?}", program);

            println!("\nAfter Alpha Conversion:");
            let alpha_converted = alpha_convert_program(&program);
            println!("{:?}", alpha_converted);
        }
        Err(e) => {
            println!("Parse error: {}", e);
        }
    }
}

static IMPUT: &str = r#"
external a: i32[16];
external b: i32[16];
external out: i32[1];

fn main() = 
    let _ = out[0] := 0 in
    let sum_a_b: i32[16] = map(a, b, (x, y) => x + y) in
    let squared: i32[16] = map(sum_a_b, (x) => x * x) in
    let result: i32 = reduce(squared, (x, y) => x + y) in
    out[0] := result
"#;
