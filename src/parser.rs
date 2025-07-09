use crate::ast::*;
use peg::parser;

parser! {
    pub grammar hls() for str {
        pub rule program() -> Program
            = _ items:item() ** whitespace() _ { items }

        rule item() -> TopLevel
            = ed:external_decl() { TopLevel::ExternalDecl(ed) }
            / fd:fundef() { TopLevel::FunDef(fd) }

        rule external_decl() -> ExternalDecl
            = "external" _ name:identifier() _ ":" _ ty:type_annotation() _ ";" {
                ExternalDecl { name, ty }
            }

        pub rule fundef() -> FunDef
            = "fn" _ name:identifier() _ "(" _ params:param_list() _ ")" _ return_type:return_type_annotation()? _ "=" _ body:expr() _ {
                FunDef { name, params, return_type, body }
            }
            / "fn" _ name:identifier() _ "()" _ return_type:return_type_annotation()? _ "=" _ body:expr() _  {
                FunDef { name, params: vec![], return_type, body }
            }
            / "fn" _ name:identifier() _ return_type:return_type_annotation()? _ "=" _ body:expr() _ {
                FunDef { name, params: vec![], return_type, body }
            }

        rule param_list() -> Vec<(Ident, Type)>
            = params:param() ** (_ "," _) { params }
            / { vec![] }

        rule param() -> (Ident, Type)
            = name:identifier() _ ":" _ ty:type_annotation() { (name, ty) }

        rule return_type_annotation() -> Type
            = "->" _ ty:type_annotation() { ty }

        pub rule type_annotation() -> Type
            = inner:basic_type() _ "[" _ size:number() _ "]" {
                Type::Array(Box::new(inner), size as usize)
            }
            / basic_type()

        rule basic_type() -> Type
            = "i32" { Type::I(32) }
            / "bool" { Type::I(1) }

        pub rule expr() -> Expr
            = lets:let_bindings() _ "in" _ base:base_expr() {
                Expr_(lets, base)
            }
            / base:base_expr() {
                Expr_(vec![], base)
            }

        rule let_bindings() -> Vec<Let>
            = lets:let_binding() ++ (_ "in" _) { lets }

        pub rule let_binding() -> Let
            = "let" _ "_" _ "=" _ value:base_expr() {
                Let::NoBindLet(NoBindLet { value })
            }
            / "let" _ name:identifier() _ ":" _ ty:type_annotation() _ "=" _ value:base_expr() {
                Let::BindLet(BindLet { name, ty, value })
            }

        pub rule base_expr() -> BaseExpr
            = array:identifier() _ "[" _ index:base_expr() _ "]" _ ":=" _ value:base_expr() {
                BaseExpr::ArraySet(array, Box::new(index), Box::new(value))
            }
            / precedence! {
                left:(@) _ "+" _ right:@ { BaseExpr::Add(Box::new(left), Box::new(right)) }
                left:(@) _ "*" _ right:@ { BaseExpr::Mul(Box::new(left), Box::new(right)) }
                --
                t:term() { t }
            }

        rule term() -> BaseExpr
            = n:number() { BaseExpr::Int(n) }
            / b:boolean() { BaseExpr::Bool(b) }
            / func_call:function_call() { func_call }
            / id:identifier() { BaseExpr::Var(id) }
            / "(" _ e:base_expr() _ ")" { e }

        rule function_call() -> BaseExpr
            = "new_array" _ "<" _ ty:type_annotation() _ ">" _ "[" _ size:number() _ "]" {
                BaseExpr::NewArray(Box::new(ty), size as usize)
            }
            / "map" _ "(" _ arrays:array_list() _ "," _ lambda:lambda_expr_multi() _ ")" {
                BaseExpr::Map(arrays, lambda.0, Box::new(lambda.1))
            }
            / "map" _ "(" _ array:base_expr() _ "," _ lambda:lambda_expr() _ ")" {
                BaseExpr::Map(vec![array], vec![lambda.0], Box::new(lambda.1))
            }
            / "reduce" _ "(" _ array:base_expr() _ "," _ lambda:lambda_expr_2() _ ")" {
                BaseExpr::Reduce(Box::new(array), lambda.0, lambda.1, Box::new(lambda.2))
            }
            / name:identifier() _ "(" _ args:argument_list() _ ")" {
                BaseExpr::Call(name, args)
            }

        rule argument_list() -> Vec<BaseExpr>
            = args:base_expr() ** (_ "," _) { args }
            / { vec![] }

        rule array_list() -> Vec<BaseExpr>
            = arrays:base_expr() ++ (_ "," _) { arrays }

        rule lambda_expr() -> (String, Expr)
            = "(" _ param:identifier() _ ")" _ "=>" _ body:base_expr() {
                (param, Expr_(vec![], body))
            }

        rule lambda_expr_multi() -> (Vec<String>, Expr)
            = "(" _ params:param_list_lambda() _ ")" _ "=>" _ body:base_expr() {
                (params, Expr_(vec![], body))
            }

        rule param_list_lambda() -> Vec<String>
            = params:identifier() ++ (_ "," _) { params }

        rule lambda_expr_2() -> (String, String, Expr)
            = "(" _ param1:identifier() _ "," _ param2:identifier() _ ")" _ "=>" _ body:base_expr() {
                (param1, param2, Expr_(vec![], body))
            }

        rule number() -> i32
            = n:$(['0'..='9']+) {? n.parse().or(Err("number")) }

        rule boolean() -> bool
            = "true" { true }
            / "false" { false }

        rule identifier() -> String
            = quiet!{
                !reserved() id:$(['a'..='z' | 'A'..='Z' | '_']['a'..='z' | 'A'..='Z' | '0'..='9' | '_']*) {
                    id.to_string()
                }
            }

        rule reserved()
            = "fn" / "let" / "in" / "map" / "reduce" / "new_array" / "true" / "false" / "i32" / "bool" / "array" / "=>" / "external"

        rule _() = quiet!{ (whitespace_char() / line_comment())* }
        rule whitespace() = quiet!{ (whitespace_char() / line_comment())+ }
        rule whitespace_char() = [' ' | '\t' | '\n' | '\r']
        rule line_comment() = "//" (!"\n" [_])* "\n"?
    }
}
