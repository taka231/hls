use crate::ast::{
    ANormalBaseExpr, ANormalExpr, ANormalFunDef, ANormalLet, ANormalProgram, ANormalTopLevel,
    BaseExpr, Expr, Expr_, FunDef, Ident, Let, Program, TopLevel, Type,
};

struct NormalizeState {
    temp_counter: usize,
}

impl NormalizeState {
    fn new() -> Self {
        Self { temp_counter: 0 }
    }

    fn fresh_temp(&mut self) -> Ident {
        let name = format!("_tmp_{}", self.temp_counter);
        self.temp_counter += 1;
        name
    }
}

fn normalize_base_expr(
    expr: BaseExpr,
    state: &mut NormalizeState,
) -> (Vec<ANormalLet>, ANormalBaseExpr) {
    match expr {
        BaseExpr::Int(n) => (vec![], ANormalBaseExpr::Int(n)),
        BaseExpr::Bool(b) => (vec![], ANormalBaseExpr::Bool(b)),
        BaseExpr::Var(name) => (vec![], ANormalBaseExpr::Var(name)),

        BaseExpr::Add(left, right) => {
            let (mut bindings, left_result) = normalize_base_expr(*left, state);
            let (mut right_bindings, right_result) = normalize_base_expr(*right, state);

            bindings.append(&mut right_bindings);

            let left_ident = match left_result {
                ANormalBaseExpr::Var(name) => name,
                other => {
                    let temp_name = state.fresh_temp();
                    bindings.push(ANormalLet::BindLet(crate::ast::BindLet_ {
                        name: temp_name.clone(),
                        ty: Type::I(32),
                        value: other,
                    }));
                    temp_name
                }
            };

            let right_ident = match right_result {
                ANormalBaseExpr::Var(name) => name,
                other => {
                    let temp_name = state.fresh_temp();
                    bindings.push(ANormalLet::BindLet(crate::ast::BindLet_ {
                        name: temp_name.clone(),
                        ty: Type::I(32),
                        value: other,
                    }));
                    temp_name
                }
            };

            (bindings, ANormalBaseExpr::Add(left_ident, right_ident))
        }

        BaseExpr::Mul(left, right) => {
            let (mut bindings, left_result) = normalize_base_expr(*left, state);
            let (mut right_bindings, right_result) = normalize_base_expr(*right, state);

            bindings.append(&mut right_bindings);

            let left_ident = match left_result {
                ANormalBaseExpr::Var(name) => name,
                other => {
                    let temp_name = state.fresh_temp();
                    bindings.push(ANormalLet::BindLet(crate::ast::BindLet_ {
                        name: temp_name.clone(),
                        ty: Type::I(32),
                        value: other,
                    }));
                    temp_name
                }
            };

            let right_ident = match right_result {
                ANormalBaseExpr::Var(name) => name,
                other => {
                    let temp_name = state.fresh_temp();
                    bindings.push(ANormalLet::BindLet(crate::ast::BindLet_ {
                        name: temp_name.clone(),
                        ty: Type::I(32),
                        value: other,
                    }));
                    temp_name
                }
            };

            (bindings, ANormalBaseExpr::Mul(left_ident, right_ident))
        }

        BaseExpr::NewArray(ty, size) => (vec![], ANormalBaseExpr::NewArray(ty, size)),

        BaseExpr::Call(func_name, args) => {
            let mut bindings = vec![];
            let mut normalized_args = vec![];

            for arg in args {
                let (mut arg_bindings, arg_result) = normalize_base_expr(arg, state);
                bindings.append(&mut arg_bindings);

                let arg_ident = match arg_result {
                    ANormalBaseExpr::Var(name) => name,
                    other => {
                        let temp_name = state.fresh_temp();
                        bindings.push(ANormalLet::BindLet(crate::ast::BindLet_ {
                            name: temp_name.clone(),
                            ty: Type::I(32),
                            value: other,
                        }));
                        temp_name
                    }
                };

                normalized_args.push(arg_ident);
            }

            (bindings, ANormalBaseExpr::Call(func_name, normalized_args))
        }

        BaseExpr::ArraySet(array_name, index, value) => {
            let (mut bindings, index_result) = normalize_base_expr(*index, state);
            let (mut value_bindings, value_result) = normalize_base_expr(*value, state);

            bindings.append(&mut value_bindings);

            let index_ident = match index_result {
                ANormalBaseExpr::Var(name) => name,
                other => {
                    let temp_name = state.fresh_temp();
                    bindings.push(ANormalLet::BindLet(crate::ast::BindLet_ {
                        name: temp_name.clone(),
                        ty: Type::I(32),
                        value: other,
                    }));
                    temp_name
                }
            };

            let value_ident = match value_result {
                ANormalBaseExpr::Var(name) => name,
                other => {
                    let temp_name = state.fresh_temp();
                    bindings.push(ANormalLet::BindLet(crate::ast::BindLet_ {
                        name: temp_name.clone(),
                        ty: Type::I(32),
                        value: other,
                    }));
                    temp_name
                }
            };

            (
                bindings,
                ANormalBaseExpr::ArraySet(array_name, Box::new(index_ident), Box::new(value_ident)),
            )
        }

        BaseExpr::Map(arrays, params, body) => {
            let mut bindings = vec![];
            let mut normalized_arrays = vec![];

            for array in arrays {
                let (mut array_bindings, array_result) = normalize_base_expr(array, state);
                bindings.append(&mut array_bindings);

                let array_ident = match array_result {
                    ANormalBaseExpr::Var(name) => name,
                    other => {
                        let temp_name = state.fresh_temp();
                        bindings.push(ANormalLet::BindLet(crate::ast::BindLet_ {
                            name: temp_name.clone(),
                            ty: Type::I(32),
                            value: other,
                        }));
                        temp_name
                    }
                };

                normalized_arrays.push(array_ident);
            }

            let normalized_body = normalize_expr(*body);

            (
                bindings,
                ANormalBaseExpr::Map(normalized_arrays, params, Box::new(normalized_body)),
            )
        }

        BaseExpr::Reduce(array, param1, param2, body) => {
            let (mut bindings, array_result) = normalize_base_expr(*array, state);

            let array_ident = match array_result {
                ANormalBaseExpr::Var(name) => name,
                other => {
                    let temp_name = state.fresh_temp();
                    bindings.push(ANormalLet::BindLet(crate::ast::BindLet_ {
                        name: temp_name.clone(),
                        ty: Type::I(32),
                        value: other,
                    }));
                    temp_name
                }
            };

            let normalized_body = normalize_expr(*body);

            (
                bindings,
                ANormalBaseExpr::Reduce(array_ident, param1, param2, Box::new(normalized_body)),
            )
        }
    }
}

fn normalize_let(let_binding: Let, state: &mut NormalizeState) -> Vec<ANormalLet> {
    match let_binding {
        Let::BindLet(bind_let) => {
            let (mut bindings, result) = normalize_base_expr(bind_let.value, state);
            bindings.push(ANormalLet::BindLet(crate::ast::BindLet_ {
                name: bind_let.name,
                ty: bind_let.ty,
                value: result,
            }));
            bindings
        }
        Let::NoBindLet(no_bind_let) => {
            let (mut bindings, result) = normalize_base_expr(no_bind_let.value, state);
            bindings.push(ANormalLet::NoBindLet(crate::ast::NoBindLet_ {
                value: result,
            }));
            bindings
        }
    }
}

pub fn normalize_expr(expr: Expr) -> ANormalExpr {
    let mut state = NormalizeState::new();
    let Expr_(lets, final_expr) = expr;

    let mut normalized_bindings = vec![];

    for let_binding in lets {
        let mut bindings = normalize_let(let_binding, &mut state);
        normalized_bindings.append(&mut bindings);
    }

    let (mut final_bindings, final_result) = normalize_base_expr(final_expr, &mut state);
    normalized_bindings.append(&mut final_bindings);

    Expr_(normalized_bindings, final_result)
}

pub fn normalize_base_expr_public(expr: BaseExpr) -> ANormalExpr {
    let mut state = NormalizeState::new();
    let (bindings, result) = normalize_base_expr(expr, &mut state);
    Expr_(bindings, result)
}

pub fn normalize_fundef(fundef: FunDef) -> ANormalFunDef {
    crate::ast::FunDef_ {
        name: fundef.name,
        params: fundef.params,
        return_type: fundef.return_type,
        body: normalize_expr(fundef.body),
    }
}

pub fn normalize_top_level(top_level: TopLevel) -> ANormalTopLevel {
    match top_level {
        TopLevel::ExternalDecl(external_decl) => ANormalTopLevel::ExternalDecl(external_decl),
        TopLevel::FunDef(fundef) => ANormalTopLevel::FunDef(normalize_fundef(fundef)),
    }
}

pub fn normalize_program(program: Program) -> ANormalProgram {
    program.into_iter().map(normalize_top_level).collect()
}
