use crate::ast::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct AlphaContext {
    env: HashMap<String, String>,
    counter: usize,
}

impl AlphaContext {
    fn fresh_name(&mut self, original: &str) -> String {
        let fresh = format!("{}_{}", original, self.counter);
        self.counter += 1;
        fresh
    }

    fn bind(&mut self, original: &str) -> String {
        let fresh = self.fresh_name(original);
        self.env.insert(original.to_string(), fresh.clone());
        fresh
    }

    fn lookup(&self, name: &str) -> String {
        self.env
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_string())
    }

    fn alpha_convert_top(&mut self, item: &TopLevel) -> TopLevel {
        match item {
            TopLevel::ExternalDecl(decl) => {
                TopLevel::ExternalDecl(self.alpha_convert_external_decl(decl))
            }
            TopLevel::FunDef(fundef) => TopLevel::FunDef(self.alpha_convert_fundef(fundef)),
        }
    }

    fn alpha_convert_external_decl(&mut self, decl: &ExternalDecl) -> ExternalDecl {
        ExternalDecl {
            name: decl.name.clone(),
            ty: decl.ty.clone(),
        }
    }

    fn alpha_convert_fundef(&mut self, fundef: &FunDef) -> FunDef {
        let new_name = if fundef.name == "main" {
            fundef.name.clone()
        } else {
            self.bind(&fundef.name)
        };

        let saved_env = self.env.clone();

        let new_params: Vec<(Ident, Type)> = fundef
            .params
            .iter()
            .map(|(name, ty)| {
                let new_name = self.bind(name);
                (new_name, ty.clone())
            })
            .collect();

        let new_body = self.alpha_convert_expr(&fundef.body);

        self.env = saved_env;
        if fundef.name != "main" {
            self.env.insert(fundef.name.clone(), new_name.clone());
        }

        FunDef {
            name: new_name,
            params: new_params,
            return_type: fundef.return_type.clone(),
            body: new_body,
        }
    }

    fn alpha_convert_expr(&mut self, expr: &Expr) -> Expr {
        let Expr_(lets, base) = expr;

        let mut new_lets = Vec::new();

        for let_binding in lets {
            let new_let = self.alpha_convert_let(let_binding);
            new_lets.push(new_let);
        }

        let new_base = self.alpha_convert_base_expr(base);

        Expr_(new_lets, new_base)
    }

    fn alpha_convert_let(&mut self, let_binding: &Let) -> Let {
        match let_binding {
            Let::BindLet(bind_let) => {
                let new_value = self.alpha_convert_base_expr(&bind_let.value);
                let new_name = self.bind(&bind_let.name);

                Let::BindLet(BindLet {
                    name: new_name,
                    ty: bind_let.ty.clone(),
                    value: new_value,
                })
            }
            Let::NoBindLet(no_bind_let) => {
                let new_value = self.alpha_convert_base_expr(&no_bind_let.value);

                Let::NoBindLet(NoBindLet { value: new_value })
            }
        }
    }

    fn alpha_convert_base_expr(&mut self, expr: &BaseExpr) -> BaseExpr {
        match expr {
            BaseExpr::Int(n) => BaseExpr::Int(*n),
            BaseExpr::Bool(b) => BaseExpr::Bool(*b),
            BaseExpr::Var(name) => BaseExpr::Var(self.lookup(name)),

            BaseExpr::Add(left, right) => {
                let new_left = self.alpha_convert_base_expr(left);
                let new_right = self.alpha_convert_base_expr(right);
                BaseExpr::Add(Box::new(new_left), Box::new(new_right))
            }

            BaseExpr::Mul(left, right) => {
                let new_left = self.alpha_convert_base_expr(left);
                let new_right = self.alpha_convert_base_expr(right);
                BaseExpr::Mul(Box::new(new_left), Box::new(new_right))
            }

            BaseExpr::NewArray(ty, size) => BaseExpr::NewArray(ty.clone(), *size),

            BaseExpr::Map(arrays, params, body) => {
                let new_arrays: Vec<BaseExpr> = arrays
                    .iter()
                    .map(|array| self.alpha_convert_base_expr(array))
                    .collect();

                let saved_env = self.env.clone();
                let new_params: Vec<Ident> = params.iter().map(|param| self.bind(param)).collect();
                let new_body = self.alpha_convert_expr(body);
                self.env = saved_env;

                BaseExpr::Map(new_arrays, new_params, Box::new(new_body))
            }

            BaseExpr::Reduce(array, init_value, param1, param2, body) => {
                let new_array = self.alpha_convert_base_expr(array);
                let new_init_value = self.alpha_convert_base_expr(init_value);

                let saved_env = self.env.clone();
                let new_param1 = self.bind(param1);
                let new_param2 = self.bind(param2);
                let new_body = self.alpha_convert_expr(body);
                self.env = saved_env;

                BaseExpr::Reduce(
                    Box::new(new_array),
                    Box::new(new_init_value),
                    new_param1,
                    new_param2,
                    Box::new(new_body),
                )
            }

            BaseExpr::Call(name, args) => {
                let new_name = self.lookup(name);
                let new_args: Vec<BaseExpr> = args
                    .iter()
                    .map(|arg| self.alpha_convert_base_expr(arg))
                    .collect();
                BaseExpr::Call(new_name, new_args)
            }

            BaseExpr::ArraySet(name, index, value) => {
                let new_name = self.lookup(name);
                let new_index = self.alpha_convert_base_expr(index);
                let new_value = self.alpha_convert_base_expr(value);
                BaseExpr::ArraySet(new_name, Box::new(new_index), Box::new(new_value))
            }
        }
    }
}

pub fn alpha_convert_program(program: &Program) -> Program {
    let mut ctx = AlphaContext::default();

    for item in program {
        if let TopLevel::ExternalDecl(decl) = item {
            ctx.env.insert(decl.name.clone(), decl.name.clone());
        }
    }

    program
        .iter()
        .map(|item| ctx.alpha_convert_top(item))
        .collect()
}
