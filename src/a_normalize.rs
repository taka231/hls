use crate::ast::{
    ANormalBaseExpr, ANormalExpr, ANormalFunDef, ANormalLet, ANormalProgram, ANormalTopLevel,
    BaseExpr, Expr, Expr_, FunDef, Ident, Let, Program, TopLevel, Type,
};
use anyhow::Result;
use std::collections::HashMap;

struct NormalizeState {
    temp_counter: usize,
    type_env: HashMap<Ident, Type>,
}

impl NormalizeState {
    fn new() -> Self {
        Self {
            temp_counter: 0,
            type_env: HashMap::new(),
        }
    }

    fn fresh_temp(&mut self) -> Ident {
        let name = format!("_tmp_{}", self.temp_counter);
        self.temp_counter += 1;
        name
    }

    fn insert_type(&mut self, name: Ident, ty: Type) {
        self.type_env.insert(name, ty);
    }

    fn get_type(&self, name: &str) -> Result<Type> {
        self.type_env
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Variable '{}' not found in type environment", name))
    }
}

fn infer_anormal_type(expr: &ANormalBaseExpr, state: &NormalizeState) -> Result<Type> {
    match expr {
        ANormalBaseExpr::Int(_) => Ok(Type::I(32)),
        ANormalBaseExpr::Bool(_) => Ok(Type::I(1)),
        ANormalBaseExpr::Var(name) => state.get_type(name),
        ANormalBaseExpr::Add(left, right) => {
            let left_ty = state.get_type(left)?;
            let right_ty = state.get_type(right)?;
            match (&left_ty, &right_ty) {
                (Type::I(w1), Type::I(w2)) if w1 == w2 => Ok(Type::I(*w1)),
                _ => Err(anyhow::anyhow!(
                    "Cannot add types {:?} and {:?}",
                    left_ty,
                    right_ty
                )),
            }
        }
        ANormalBaseExpr::Mul(left, right) => {
            let left_ty = state.get_type(left)?;
            let right_ty = state.get_type(right)?;
            match (&left_ty, &right_ty) {
                (Type::I(w1), Type::I(w2)) if w1 == w2 => Ok(Type::I(*w1)),
                _ => Err(anyhow::anyhow!(
                    "Cannot multiply types {:?} and {:?}",
                    left_ty,
                    right_ty
                )),
            }
        }
        ANormalBaseExpr::NewArray(ty, size) => Ok(Type::Array(ty.clone(), *size)),
        ANormalBaseExpr::Call(_, _) => Ok(Type::I(32)),
        ANormalBaseExpr::ArraySet(array_name, _, _) => {
            let array_ty = state.get_type(array_name)?;
            match array_ty {
                Type::Array(element_ty, _) => Ok((*element_ty).clone()),
                _ => Err(anyhow::anyhow!(
                    "ArraySet: Variable '{}' is not an array type",
                    array_name
                )),
            }
        }
        ANormalBaseExpr::Map(arrays, _, _) => {
            if let Some(array) = arrays.first() {
                state.get_type(array)
            } else {
                Err(anyhow::anyhow!("Map: Empty array list"))
            }
        }
        ANormalBaseExpr::Reduce(array, _, _, _) => {
            let array_ty = state.get_type(array)?;
            match array_ty {
                Type::Array(element_ty, _) => Ok((*element_ty).clone()),
                _ => Err(anyhow::anyhow!(
                    "Reduce: Variable '{}' is not an array type",
                    array
                )),
            }
        }
    }
}

fn normalize_base_expr(
    expr: BaseExpr,
    state: &mut NormalizeState,
) -> Result<(Vec<ANormalLet>, ANormalBaseExpr)> {
    match expr {
        BaseExpr::Int(n) => Ok((vec![], ANormalBaseExpr::Int(n))),
        BaseExpr::Bool(b) => Ok((vec![], ANormalBaseExpr::Bool(b))),
        BaseExpr::Var(name) => Ok((vec![], ANormalBaseExpr::Var(name))),

        BaseExpr::Add(left, right) => {
            let (mut bindings, left_result) = normalize_base_expr(*left, state)?;
            let (mut right_bindings, right_result) = normalize_base_expr(*right, state)?;

            bindings.append(&mut right_bindings);

            let left_ident = match left_result {
                ANormalBaseExpr::Var(name) => name,
                other => {
                    let temp_name = state.fresh_temp();
                    let inferred_ty = infer_anormal_type(&other, state)?;
                    state.insert_type(temp_name.clone(), inferred_ty.clone());
                    bindings.push(ANormalLet::BindLet(crate::ast::BindLet_ {
                        name: temp_name.clone(),
                        ty: inferred_ty,
                        value: other,
                    }));
                    temp_name
                }
            };

            let right_ident = match right_result {
                ANormalBaseExpr::Var(name) => name,
                other => {
                    let temp_name = state.fresh_temp();
                    let inferred_ty = infer_anormal_type(&other, state)?;
                    state.insert_type(temp_name.clone(), inferred_ty.clone());
                    bindings.push(ANormalLet::BindLet(crate::ast::BindLet_ {
                        name: temp_name.clone(),
                        ty: inferred_ty,
                        value: other,
                    }));
                    temp_name
                }
            };

            Ok((bindings, ANormalBaseExpr::Add(left_ident, right_ident)))
        }

        BaseExpr::Mul(left, right) => {
            let (mut bindings, left_result) = normalize_base_expr(*left, state)?;
            let (mut right_bindings, right_result) = normalize_base_expr(*right, state)?;

            bindings.append(&mut right_bindings);

            let left_ident = match left_result {
                ANormalBaseExpr::Var(name) => name,
                other => {
                    let temp_name = state.fresh_temp();
                    let inferred_ty = infer_anormal_type(&other, state)?;
                    state.insert_type(temp_name.clone(), inferred_ty.clone());
                    bindings.push(ANormalLet::BindLet(crate::ast::BindLet_ {
                        name: temp_name.clone(),
                        ty: inferred_ty,
                        value: other,
                    }));
                    temp_name
                }
            };

            let right_ident = match right_result {
                ANormalBaseExpr::Var(name) => name,
                other => {
                    let temp_name = state.fresh_temp();
                    let inferred_ty = infer_anormal_type(&other, state)?;
                    state.insert_type(temp_name.clone(), inferred_ty.clone());
                    bindings.push(ANormalLet::BindLet(crate::ast::BindLet_ {
                        name: temp_name.clone(),
                        ty: inferred_ty,
                        value: other,
                    }));
                    temp_name
                }
            };

            Ok((bindings, ANormalBaseExpr::Mul(left_ident, right_ident)))
        }

        BaseExpr::NewArray(ty, size) => Ok((vec![], ANormalBaseExpr::NewArray(ty, size))),

        BaseExpr::Call(func_name, args) => {
            let mut bindings = vec![];
            let mut normalized_args = vec![];

            for arg in args {
                let (mut arg_bindings, arg_result) = normalize_base_expr(arg, state)?;
                bindings.append(&mut arg_bindings);

                let arg_ident = match arg_result {
                    ANormalBaseExpr::Var(name) => name,
                    other => {
                        let temp_name = state.fresh_temp();
                        let inferred_ty = infer_anormal_type(&other, state)?;
                        state.insert_type(temp_name.clone(), inferred_ty.clone());
                        bindings.push(ANormalLet::BindLet(crate::ast::BindLet_ {
                            name: temp_name.clone(),
                            ty: inferred_ty,
                            value: other,
                        }));
                        temp_name
                    }
                };

                normalized_args.push(arg_ident);
            }

            Ok((bindings, ANormalBaseExpr::Call(func_name, normalized_args)))
        }

        BaseExpr::ArraySet(array_name, index, value) => {
            let (mut bindings, index_result) = normalize_base_expr(*index, state)?;
            let (mut value_bindings, value_result) = normalize_base_expr(*value, state)?;

            bindings.append(&mut value_bindings);

            let index_ident = match index_result {
                ANormalBaseExpr::Var(name) => name,
                other => {
                    let temp_name = state.fresh_temp();
                    let inferred_ty = infer_anormal_type(&other, state)?;
                    state.insert_type(temp_name.clone(), inferred_ty.clone());
                    bindings.push(ANormalLet::BindLet(crate::ast::BindLet_ {
                        name: temp_name.clone(),
                        ty: inferred_ty,
                        value: other,
                    }));
                    temp_name
                }
            };

            let value_ident = match value_result {
                ANormalBaseExpr::Var(name) => name,
                other => {
                    let temp_name = state.fresh_temp();
                    let inferred_ty = infer_anormal_type(&other, state)?;
                    state.insert_type(temp_name.clone(), inferred_ty.clone());
                    bindings.push(ANormalLet::BindLet(crate::ast::BindLet_ {
                        name: temp_name.clone(),
                        ty: inferred_ty,
                        value: other,
                    }));
                    temp_name
                }
            };

            Ok((
                bindings,
                ANormalBaseExpr::ArraySet(array_name, Box::new(index_ident), Box::new(value_ident)),
            ))
        }

        BaseExpr::Map(arrays, params, body) => {
            let mut bindings = vec![];
            let mut normalized_arrays = vec![];

            for array in arrays {
                let (mut array_bindings, array_result) = normalize_base_expr(array, state)?;
                bindings.append(&mut array_bindings);

                let array_ident = match array_result {
                    ANormalBaseExpr::Var(name) => name,
                    other => {
                        let temp_name = state.fresh_temp();
                        let inferred_ty = infer_anormal_type(&other, state)?;
                        state.insert_type(temp_name.clone(), inferred_ty.clone());
                        bindings.push(ANormalLet::BindLet(crate::ast::BindLet_ {
                            name: temp_name.clone(),
                            ty: inferred_ty,
                            value: other,
                        }));
                        temp_name
                    }
                };

                normalized_arrays.push(array_ident);
            }

            let normalized_body = normalize_expr_with_state(*body, state)?;

            Ok((
                bindings,
                ANormalBaseExpr::Map(normalized_arrays, params, Box::new(normalized_body)),
            ))
        }

        BaseExpr::Reduce(array, param1, param2, body) => {
            let (mut bindings, array_result) = normalize_base_expr(*array, state)?;

            let array_ident = match array_result {
                ANormalBaseExpr::Var(name) => name,
                other => {
                    let temp_name = state.fresh_temp();
                    let inferred_ty = infer_anormal_type(&other, state)?;
                    state.insert_type(temp_name.clone(), inferred_ty.clone());
                    bindings.push(ANormalLet::BindLet(crate::ast::BindLet_ {
                        name: temp_name.clone(),
                        ty: inferred_ty,
                        value: other,
                    }));
                    temp_name
                }
            };

            let normalized_body = normalize_expr_with_state(*body, state)?;

            Ok((
                bindings,
                ANormalBaseExpr::Reduce(array_ident, param1, param2, Box::new(normalized_body)),
            ))
        }
    }
}

fn normalize_let(let_binding: Let, state: &mut NormalizeState) -> Result<Vec<ANormalLet>> {
    match let_binding {
        Let::BindLet(bind_let) => {
            let (mut bindings, result) = normalize_base_expr(bind_let.value, state)?;
            state.insert_type(bind_let.name.clone(), bind_let.ty.clone());
            bindings.push(ANormalLet::BindLet(crate::ast::BindLet_ {
                name: bind_let.name,
                ty: bind_let.ty,
                value: result,
            }));
            Ok(bindings)
        }
        Let::NoBindLet(no_bind_let) => {
            let (mut bindings, result) = normalize_base_expr(no_bind_let.value, state)?;
            bindings.push(ANormalLet::NoBindLet(crate::ast::NoBindLet_ {
                value: result,
            }));
            Ok(bindings)
        }
    }
}

pub fn normalize_expr(expr: Expr) -> Result<ANormalExpr> {
    let mut state = NormalizeState::new();
    normalize_expr_with_state(expr, &mut state)
}

fn normalize_expr_with_state(expr: Expr, state: &mut NormalizeState) -> Result<ANormalExpr> {
    let Expr_(lets, final_expr) = expr;

    let mut normalized_bindings = vec![];

    for let_binding in lets {
        let mut bindings = normalize_let(let_binding, state)?;
        normalized_bindings.append(&mut bindings);
    }

    let (mut final_bindings, final_result) = normalize_base_expr(final_expr, state)?;
    normalized_bindings.append(&mut final_bindings);

    Ok(Expr_(normalized_bindings, final_result))
}

pub fn normalize_base_expr_public(expr: BaseExpr) -> Result<ANormalExpr> {
    let mut state = NormalizeState::new();
    let (bindings, result) = normalize_base_expr(expr, &mut state)?;
    Ok(Expr_(bindings, result))
}

pub fn normalize_fundef(fundef: FunDef) -> Result<ANormalFunDef> {
    let mut state = NormalizeState::new();

    for (param_name, param_type) in &fundef.params {
        state.insert_type(param_name.clone(), param_type.clone());
    }

    let normalized_body = normalize_expr_with_state(fundef.body, &mut state)?;

    Ok(crate::ast::FunDef_ {
        name: fundef.name,
        params: fundef.params,
        return_type: fundef.return_type,
        body: normalized_body,
    })
}

pub fn normalize_top_level(top_level: TopLevel) -> Result<ANormalTopLevel> {
    match top_level {
        TopLevel::ExternalDecl(external_decl) => Ok(ANormalTopLevel::ExternalDecl(external_decl)),
        TopLevel::FunDef(fundef) => Ok(ANormalTopLevel::FunDef(normalize_fundef(fundef)?)),
    }
}

pub fn normalize_program(program: Program) -> Result<ANormalProgram> {
    program.into_iter().map(normalize_top_level).collect()
}
