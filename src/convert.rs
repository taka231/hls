use std::collections::HashMap;

use crate::{
    ast::{self, ANormalBindLet, ANormalNoBindLet, Type},
    calyx_ast,
};
use anyhow::Result;

const ADDRESS_WIDTH: usize = 32;

#[derive(Debug, Clone)]
pub struct Converter {
    pub program: calyx_ast::Program,
    pub fresh_idx: usize,
    pub current_func: Option<String>,
    // HashMap<VariableName, CellName>
    pub env: HashMap<String, calyx_ast::Src>,
    pub type_env: HashMap<String, ast::Type>,
    pub fun_type_env: HashMap<String, (Vec<(String, Type)>, Option<Type>)>,
}

impl Converter {
    pub fn init() -> Self {
        let import_names = vec![
            "primitives/core.futil".to_string(),
            "primitives/binary_operators.futil".to_string(),
            "primitives/memories/comb.futil".to_string(),
        ];
        let program = calyx_ast::Program {
            import_names,
            main: calyx_ast::Component::new("main", vec![], vec![]),
            components: vec![],
        };
        Converter {
            program,
            fresh_idx: 0,
            current_func: None,
            env: HashMap::new(),
            type_env: HashMap::new(),
            fun_type_env: HashMap::new(),
        }
    }

    fn fresh_name(&mut self) -> String {
        let name = format!("_tmp_{}", self.fresh_idx);
        self.fresh_idx += 1;
        name
    }

    fn find_src_by_var(&self, var: &str) -> Result<calyx_ast::Src> {
        if let Some(cell_name) = self.env.get(var) {
            Ok(cell_name.clone())
        } else {
            Err(anyhow::anyhow!("Variable {} not found in cell map", var))
        }
    }

    fn get_current_func(&mut self) -> Result<&mut calyx_ast::Component> {
        if let Some(func_name) = &self.current_func {
            if func_name == "main" {
                return Ok(&mut self.program.main);
            }
            if let Some(component) = self
                .program
                .components
                .iter_mut()
                .find(|c| c.name == *func_name)
            {
                return Ok(component);
            }
        }
        Err(anyhow::anyhow!(
            "No current function set or function not found"
        ))
    }

    fn new_group(&mut self) -> calyx_ast::Group {
        let name = format!("_group_{}", self.fresh_idx);
        self.fresh_idx += 1;
        calyx_ast::Group {
            name,
            done: None,
            wires: vec![],
        }
    }

    pub fn convert(&mut self, ast: ast::ANormalProgram) -> Result<()> {
        for decl in ast {
            match decl {
                ast::ANormalTopLevel::ExternalDecl(decl) => {
                    self.convert_external_decl(&decl)?;
                }
                ast::ANormalTopLevel::FunDef(fundef) => {
                    self.convert_fundef(&fundef)?;
                }
            }
        }
        Ok(())
    }

    const FUN_OUT_NAME: &'static str = "_out";

    fn convert_fundef(&mut self, fundef: &ast::ANormalFunDef) -> Result<()> {
        let ast::FunDef_ {
            name,
            params,
            return_type,
            body,
        } = fundef;
        self.fun_type_env
            .insert(name.clone(), (params.clone(), return_type.clone()));
        self.current_func = Some(name.clone());
        let is_main = name == "main";
        if is_main && !(params.is_empty() && return_type.is_none()) {
            return Err(anyhow::anyhow!(
                "Main function must have no parameters and no return type"
            ));
        }
        if !is_main {
            let mut component_params: Vec<(String, usize)> = vec![];
            let mut cells: Vec<calyx_ast::Cell> = vec![];
            for (param_name, param_type) in params {
                self.type_env.insert(param_name.clone(), param_type.clone());
                match param_type {
                    ast::Type::I(width) => {
                        component_params.push((param_name.clone(), *width));
                        self.env.insert(
                            param_name.clone(),
                            calyx_ast::Src::Port(calyx_ast::Port {
                                cell: param_name.clone(),
                                port: "".to_string(),
                            }),
                        );
                    }
                    ast::Type::Array(content_ty, size) => {
                        if let ast::Type::I(width) = &**content_ty {
                            let array_ref_cell = calyx_ast::Cell {
                                name: param_name.clone(),
                                is_external: false,
                                is_ref: true,
                                circuit: calyx_ast::Circuit::CombMemD1 {
                                    data_width: *width,
                                    len: *size,
                                    address_width: ADDRESS_WIDTH,
                                },
                            };
                            cells.push(array_ref_cell);
                            self.env.insert(
                                param_name.clone(),
                                calyx_ast::Src::Port(calyx_ast::Port {
                                    cell: param_name.clone(),
                                    port: "read_data".to_string(),
                                }),
                            );
                        } else {
                            return Err(anyhow::anyhow!(
                                "Expected an integer type for array parameter"
                            ));
                        }
                    }
                };
            }

            let result = if let Some(ty) = return_type {
                let name = Converter::FUN_OUT_NAME.to_string();
                match ty {
                    ast::Type::I(width) => {
                        self.type_env.insert(name.clone(), ty.clone());
                        self.env.insert(
                            name.clone(),
                            calyx_ast::Src::Port(calyx_ast::Port {
                                cell: name.clone(),
                                port: "out".to_string(),
                            }),
                        );
                        vec![(name.clone(), *width)]
                    }
                    ast::Type::Array(content_ty, size) => {
                        if let ast::Type::I(width) = &**content_ty {
                            let array_cell = calyx_ast::Cell {
                                name: name.clone(),
                                is_external: false,
                                is_ref: false,
                                circuit: calyx_ast::Circuit::CombMemD1 {
                                    data_width: *width,
                                    len: *size,
                                    address_width: ADDRESS_WIDTH,
                                },
                            };
                            cells.push(array_cell);
                            self.env.insert(
                                name.clone(),
                                calyx_ast::Src::Port(calyx_ast::Port {
                                    cell: name.clone(),
                                    port: "read_data".to_string(),
                                }),
                            );
                            vec![]
                        } else {
                            return Err(anyhow::anyhow!(
                                "Expected an integer type for array return type"
                            ));
                        }
                    }
                }
            } else {
                vec![]
            };

            let component = calyx_ast::Component {
                name: name.clone(),
                params: component_params,
                result,
                wires: calyx_ast::Wires::default(),
                cells,
                control: vec![],
            };
            self.program.components.push(component);
        }

        let out = if return_type.is_none() || matches!(return_type, Some(ast::Type::Array(..))) {
            None
        } else {
            let name = self.fresh_name();
            Some(name.clone())
        };

        let control = self.convert_expr(body, out.clone())?;
        self.get_current_func()?.push_control(control);

        if return_type.is_some() && matches!(return_type, Some(ast::Type::I(..))) {
            let result = self.find_src_by_var(out.as_ref().unwrap())?;
            let output_cell = calyx_ast::Cell {
                name: self.fresh_name(),
                is_external: false,
                is_ref: false,
                circuit: calyx_ast::Circuit::StdReg {
                    width: match return_type.as_ref().unwrap() {
                        ast::Type::I(width) => *width,
                        _ => unreachable!("Expected an integer type for output"),
                    },
                },
            };
            self.get_current_func()?.cells.push(output_cell.clone());
            let mut group = self.new_group();
            group.wires.push(calyx_ast::Wire {
                dest: calyx_ast::Port {
                    cell: output_cell.name.clone(),
                    port: "in".to_string(),
                },
                src: result,
            });
            group.wires.push(calyx_ast::Wire {
                dest: calyx_ast::Port {
                    cell: output_cell.name.clone(),
                    port: "write_en".to_string(),
                },
                src: calyx_ast::Src::Int { value: 1, width: 1 },
            });
            group.done = Some(calyx_ast::Src::Port(calyx_ast::Port {
                cell: output_cell.name.clone(),
                port: "done".to_string(),
            }));
            self.get_current_func()?
                .control
                .push(calyx_ast::Control::GroupName(group.name.clone()));
            self.get_current_func()?.wires.groups.push(group);
            self.get_current_func()?
                .wires
                .static_wires
                .push(calyx_ast::Wire {
                    dest: calyx_ast::Port {
                        cell: Converter::FUN_OUT_NAME.to_string(),
                        port: "".to_string(),
                    },
                    src: calyx_ast::Src::Port(calyx_ast::Port {
                        cell: output_cell.name.clone(),
                        port: "out".to_string(),
                    }),
                });
        }

        Ok(())
    }

    fn convert_expr(
        &mut self,
        expr: &ast::ANormalExpr,
        out: Option<String>,
    ) -> Result<calyx_ast::Control> {
        let ast::Expr_(lets, body) = expr;
        let mut seq_vec = vec![];
        for let_binding in lets {
            let control = self.convert_let(let_binding)?;
            if !control.is_empty() {
                seq_vec.push(control);
            }
        }
        let control = self.convert_base_expr(body)?(out)?;
        if !control.is_empty() {
            seq_vec.push(control);
        }
        Ok(calyx_ast::Control::Seq(seq_vec))
    }

    fn convert_let(&mut self, let_binding: &ast::ANormalLet) -> Result<calyx_ast::Control> {
        match let_binding {
            ast::ANormalLet::BindLet(ANormalBindLet { name, value, ty }) => {
                self.type_env.insert(name.clone(), ty.clone());
                self.convert_base_expr(value)?(Some(name.clone()))
            }
            ast::ANormalLet::NoBindLet(ANormalNoBindLet { value }) => {
                self.convert_base_expr(value)?(None)
            }
        }
    }

    fn convert_base_expr<'a: 'b, 'b>(
        &'a mut self,
        base_expr: &'a ast::ANormalBaseExpr,
    ) -> Result<Box<dyn FnOnce(Option<String>) -> Result<calyx_ast::Control> + 'b>> {
        match base_expr {
            ast::ANormalBaseExpr::Int(n) => Ok(Box::new(|dest: Option<String>| {
                if let Some(dest) = dest {
                    self.env.insert(
                        dest.clone(),
                        calyx_ast::Src::Int {
                            value: *n as isize,
                            width: 32,
                        },
                    );
                }
                Ok(calyx_ast::Control::empty())
            })),
            ast::ANormalBaseExpr::Bool(b) => Ok(Box::new(|dest: Option<String>| {
                if let Some(dest) = dest {
                    self.env.insert(
                        dest.clone(),
                        calyx_ast::Src::Int {
                            value: if *b { 1 } else { 0 },
                            width: 1,
                        },
                    );
                }
                Ok(calyx_ast::Control::empty())
            })),
            ast::ANormalBaseExpr::Var(var) => Ok(Box::new(move |dest: Option<String>| {
                let src = self.find_src_by_var(var)?;
                if let Some(dest) = dest {
                    self.env.insert(dest.clone(), src);
                }
                Ok(calyx_ast::Control::empty())
            })),
            ast::ANormalBaseExpr::Add(var1, var2) => {
                let var1 = self.find_src_by_var(var1)?;
                let var2 = self.find_src_by_var(var2)?;
                Ok(Box::new(move |dest: Option<String>| {
                    if let Some(dest) = dest {
                        let new_add_cell_name = self.fresh_name();
                        let new_add_cell = calyx_ast::Cell {
                            name: new_add_cell_name.clone(),
                            is_external: false,
                            is_ref: false,
                            circuit: calyx_ast::Circuit::StdAdd { width: 32 },
                        };
                        self.get_current_func()?.cells.push(new_add_cell);
                        let left_wire = calyx_ast::Wire {
                            dest: calyx_ast::Port {
                                cell: new_add_cell_name.clone(),
                                port: "left".to_string(),
                            },
                            src: var1.clone(),
                        };
                        let right_wire = calyx_ast::Wire {
                            dest: calyx_ast::Port {
                                cell: new_add_cell_name.clone(),
                                port: "right".to_string(),
                            },
                            src: var2.clone(),
                        };
                        self.get_current_func()?.wires.static_wires.push(left_wire);
                        self.get_current_func()?.wires.static_wires.push(right_wire);
                        self.env.insert(
                            dest.clone(),
                            calyx_ast::Src::Port(calyx_ast::Port {
                                cell: new_add_cell_name,
                                port: "out".to_string(),
                            }),
                        );
                    }
                    Ok(calyx_ast::Control::empty())
                }))
            }
            ast::ANormalBaseExpr::Mul(var1, var2) => {
                let var1 = self.find_src_by_var(var1)?;
                let var2 = self.find_src_by_var(var2)?;
                Ok(Box::new(move |dest: Option<String>| {
                    if let Some(dest) = dest {
                        let mult_cell = self.get_current_func()?.get_mult_cell(32);
                        let dest_cell = calyx_ast::Cell {
                            name: dest.clone(),
                            is_external: false,
                            is_ref: false,
                            circuit: calyx_ast::Circuit::StdReg { width: 32 },
                        };
                        self.env.insert(
                            dest.clone(),
                            calyx_ast::Src::Port(calyx_ast::Port {
                                cell: dest.clone(),
                                port: "out".to_string(),
                            }),
                        );
                        self.get_current_func()?.cells.push(dest_cell);
                        let mut group = self.new_group();
                        group.wires.push(calyx_ast::Wire {
                            dest: calyx_ast::Port {
                                cell: mult_cell.name.clone(),
                                port: "left".to_string(),
                            },
                            src: var1.clone(),
                        });
                        group.wires.push(calyx_ast::Wire {
                            dest: calyx_ast::Port {
                                cell: mult_cell.name.clone(),
                                port: "right".to_string(),
                            },
                            src: var2.clone(),
                        });
                        group.wires.push(calyx_ast::Wire {
                            dest: calyx_ast::Port {
                                cell: mult_cell.name.clone(),
                                port: "go".to_string(),
                            },
                            src: calyx_ast::Src::Int { value: 1, width: 1 },
                        });
                        group.wires.push(calyx_ast::Wire {
                            dest: calyx_ast::Port {
                                cell: dest.clone(),
                                port: "in".to_string(),
                            },
                            src: calyx_ast::Src::Port(calyx_ast::Port {
                                cell: mult_cell.name.clone(),
                                port: "out".to_string(),
                            }),
                        });
                        group.wires.push(calyx_ast::Wire {
                            dest: calyx_ast::Port {
                                cell: dest.clone(),
                                port: "write_en".to_string(),
                            },
                            src: calyx_ast::Port {
                                cell: mult_cell.name.clone(),
                                port: "done".to_string(),
                            }
                            .into(),
                        });
                        group.done = Some(calyx_ast::Src::Port(calyx_ast::Port {
                            cell: dest.clone(),
                            port: "done".to_string(),
                        }));
                        let group_name = group.name.clone();
                        self.get_current_func()?.wires.groups.push(group);
                        Ok(calyx_ast::Control::GroupName(group_name))
                    } else {
                        Ok(calyx_ast::Control::empty())
                    }
                }))
            }
            ast::ANormalBaseExpr::NewArray(_, _) => todo!(),
            ast::ANormalBaseExpr::Map(vars, args, expr) => {
                let Some(Type::Array(content_ty, size)) = self.type_env.get(vars.first().unwrap())
                else {
                    return Err(anyhow::anyhow!("Expected an array type for map"));
                };
                let Type::I(width) = &**content_ty else {
                    return Err(anyhow::anyhow!("Expected an integer type for map"));
                };
                let size = *size;
                let width = *width;
                let args: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
                let vars: Vec<calyx_ast::Port> = vars
                    .iter()
                    .map(|var| match self.find_src_by_var(var)? {
                        calyx_ast::Src::Port(port) => Ok(port),
                        _ => Err(anyhow::anyhow!("Expected a port for variable {}", var)),
                    })
                    .collect::<Result<_>>()?;
                for arg in &args {
                    self.type_env.insert(arg.clone(), Type::I(width));
                }
                Ok(Box::new(move |dest: Option<String>| {
                    let mut seq_vec = vec![];
                    let add_cell = self.get_current_func()?.get_add_cell(32);
                    let new_vec = calyx_ast::Cell {
                        name: self.fresh_name(),
                        is_external: false,
                        is_ref: false,
                        circuit: calyx_ast::Circuit::CombMemD1 {
                            data_width: width,
                            len: size,
                            address_width: ADDRESS_WIDTH,
                        },
                    };
                    self.get_current_func()?.cells.push(new_vec.clone());
                    let count_reg = calyx_ast::Cell {
                        name: self.fresh_name(),
                        is_external: false,
                        is_ref: false,
                        circuit: calyx_ast::Circuit::StdReg {
                            width: ADDRESS_WIDTH,
                        },
                    };
                    self.get_current_func()?.cells.push(count_reg.clone());
                    let arg_regs: Vec<calyx_ast::Cell> = args
                        .iter()
                        .map(|arg| {
                            let arg_reg = calyx_ast::Cell {
                                name: self.fresh_name(),
                                is_external: false,
                                is_ref: false,
                                circuit: calyx_ast::Circuit::StdReg { width },
                            };
                            self.get_current_func()?.cells.push(arg_reg.clone());
                            self.env.insert(
                                arg.clone(),
                                calyx_ast::Src::Port(calyx_ast::Port {
                                    cell: arg_reg.name.clone(),
                                    port: "out".to_string(),
                                }),
                            );
                            Ok(arg_reg)
                        })
                        .collect::<Result<_>>()?;

                    let cond_lt = calyx_ast::Cell {
                        name: self.fresh_name(),
                        is_external: false,
                        is_ref: false,
                        circuit: calyx_ast::Circuit::StdLt {
                            width: ADDRESS_WIDTH,
                        },
                    };
                    self.get_current_func()?.cells.push(cond_lt.clone());
                    if let Some(dest) = dest {
                        self.env.insert(
                            dest.clone(),
                            calyx_ast::Src::Port(calyx_ast::Port {
                                cell: new_vec.name.clone(),
                                port: "read_data".to_string(),
                            }),
                        );
                    }

                    let mut init_count_reg_group = self.new_group();
                    init_count_reg_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: count_reg.name.clone(),
                            port: "in".to_string(),
                        },
                        src: calyx_ast::Src::Int {
                            value: 0,
                            width: ADDRESS_WIDTH,
                        },
                    });
                    init_count_reg_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: count_reg.name.clone(),
                            port: "write_en".to_string(),
                        },
                        src: calyx_ast::Src::Int { value: 1, width: 1 },
                    });
                    init_count_reg_group.done = Some(calyx_ast::Src::Port(calyx_ast::Port {
                        cell: count_reg.name.clone(),
                        port: "done".to_string(),
                    }));
                    seq_vec.push(calyx_ast::Control::GroupName(
                        init_count_reg_group.name.clone(),
                    ));
                    self.get_current_func()?
                        .wires
                        .groups
                        .push(init_count_reg_group);

                    let mut cond_lt_group = self.new_group();
                    cond_lt_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: cond_lt.name.clone(),
                            port: "left".to_string(),
                        },
                        src: calyx_ast::Src::Port(calyx_ast::Port {
                            cell: count_reg.name.clone(),
                            port: "out".to_string(),
                        }),
                    });
                    cond_lt_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: cond_lt.name.clone(),
                            port: "right".to_string(),
                        },
                        src: calyx_ast::Src::Int {
                            value: size as isize,
                            width: ADDRESS_WIDTH,
                        },
                    });
                    let cond_lt_group_name = cond_lt_group.name.clone();
                    self.get_current_func()?
                        .wires
                        .groups
                        .push(cond_lt_group.clone());

                    let mut init_args_groups = vec![];
                    for (i, arg_reg) in arg_regs.iter().enumerate() {
                        let mut init_arg_group = self.new_group();
                        init_arg_group.wires.push(calyx_ast::Wire {
                            dest: vars[i].port("addr0"),
                            src: calyx_ast::Src::Port(calyx_ast::Port {
                                cell: count_reg.name.clone(),
                                port: "out".to_string(),
                            }),
                        });
                        init_arg_group.wires.push(calyx_ast::Wire {
                            dest: calyx_ast::Port {
                                cell: arg_reg.name.clone(),
                                port: "in".to_string(),
                            },
                            src: calyx_ast::Src::Port(vars[i].clone()),
                        });
                        init_arg_group.wires.push(calyx_ast::Wire {
                            dest: calyx_ast::Port {
                                cell: arg_reg.name.clone(),
                                port: "write_en".to_string(),
                            },
                            src: calyx_ast::Src::Int { value: 1, width: 1 },
                        });
                        init_arg_group.done = Some(calyx_ast::Src::Port(calyx_ast::Port {
                            cell: arg_reg.name.clone(),
                            port: "done".to_string(),
                        }));
                        init_args_groups.push(init_arg_group.name.clone());
                        self.get_current_func()?.wires.groups.push(init_arg_group);
                    }
                    let result_var = self.fresh_name();
                    let body_control = self.convert_expr(expr, Some(result_var.clone()))?;
                    let result = self.env.get(&result_var).cloned().ok_or_else(|| {
                        anyhow::anyhow!(
                            "internal error: Expected result variable {} to be in environment",
                            result_var
                        )
                    })?;
                    let mut result_reg_group = self.new_group();
                    result_reg_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: new_vec.name.clone(),
                            port: "addr0".to_string(),
                        },
                        src: calyx_ast::Src::Port(calyx_ast::Port {
                            cell: count_reg.name.clone(),
                            port: "out".to_string(),
                        }),
                    });
                    result_reg_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: new_vec.name.clone(),
                            port: "write_data".to_string(),
                        },
                        src: result,
                    });
                    result_reg_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: new_vec.name.clone(),
                            port: "write_en".to_string(),
                        },
                        src: calyx_ast::Src::Int { value: 1, width: 1 },
                    });
                    result_reg_group.done = Some(calyx_ast::Src::Port(calyx_ast::Port {
                        cell: new_vec.name.clone(),
                        port: "done".to_string(),
                    }));
                    self.get_current_func()?
                        .wires
                        .groups
                        .push(result_reg_group.clone());

                    let mut inc_count_group = self.new_group();
                    inc_count_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: add_cell.name.clone(),
                            port: "left".to_string(),
                        },
                        src: calyx_ast::Src::Port(calyx_ast::Port {
                            cell: count_reg.name.clone(),
                            port: "out".to_string(),
                        }),
                    });
                    inc_count_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: add_cell.name.clone(),
                            port: "right".to_string(),
                        },
                        src: calyx_ast::Src::Int {
                            value: 1,
                            width: ADDRESS_WIDTH,
                        },
                    });
                    inc_count_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: count_reg.name.clone(),
                            port: "in".to_string(),
                        },
                        src: calyx_ast::Src::Port(calyx_ast::Port {
                            cell: add_cell.name.clone(),
                            port: "out".to_string(),
                        }),
                    });
                    inc_count_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: count_reg.name.clone(),
                            port: "write_en".to_string(),
                        },
                        src: calyx_ast::Src::Int { value: 1, width: 1 },
                    });
                    inc_count_group.done = Some(calyx_ast::Src::Port(calyx_ast::Port {
                        cell: count_reg.name.clone(),
                        port: "done".to_string(),
                    }));
                    self.get_current_func()?
                        .wires
                        .groups
                        .push(inc_count_group.clone());

                    let mut loop_body = vec![];
                    loop_body.push(calyx_ast::Control::Seq(
                        init_args_groups
                            .into_iter()
                            .map(calyx_ast::Control::GroupName)
                            .collect::<Vec<_>>(),
                    ));
                    if !body_control.is_empty() {
                        loop_body.push(body_control);
                    }
                    loop_body.push(calyx_ast::Control::GroupName(result_reg_group.name.clone()));
                    loop_body.push(calyx_ast::Control::GroupName(inc_count_group.name.clone()));

                    let loop_control = calyx_ast::Control::While {
                        condition: calyx_ast::Port {
                            cell: cond_lt.name.clone(),
                            port: "out".to_string(),
                        },
                        with: Some(cond_lt_group_name),
                        body: loop_body,
                    };

                    seq_vec.push(loop_control);

                    Ok(calyx_ast::Control::Seq(seq_vec))
                }))
            }
            ast::ANormalBaseExpr::Reduce(array, init_value, acm, arg, expr) => {
                let Some(Type::Array(content_ty, size)) = &self.type_env.get(array) else {
                    return Err(anyhow::anyhow!("Expected an array type for reduction"));
                };
                let Type::I(width) = &**content_ty else {
                    return Err(anyhow::anyhow!("Expected an integer type for reduction"));
                };
                let size = *size;
                let width = *width;
                let calyx_ast::Src::Port(array) = self.find_src_by_var(array)? else {
                    return Err(anyhow::anyhow!("Expected a port for array variable"));
                };
                let init_value = self.find_src_by_var(init_value)?;
                self.type_env.insert(acm.clone(), Type::I(width));
                self.type_env.insert(arg.clone(), Type::I(width));
                Ok(Box::new(move |dest: Option<String>| {
                    let mut seq_vec = vec![];
                    let add_cell = self.get_current_func()?.get_add_cell(width);
                    let acm_reg = calyx_ast::Cell {
                        name: self.fresh_name(),
                        is_external: false,
                        is_ref: false,
                        circuit: calyx_ast::Circuit::StdReg { width },
                    };
                    self.env.insert(
                        acm.clone(),
                        calyx_ast::Src::Port(calyx_ast::Port {
                            cell: acm_reg.name.clone(),
                            port: "out".to_string(),
                        }),
                    );
                    if let Some(dest) = dest {
                        self.env.insert(
                            dest.clone(),
                            calyx_ast::Src::Port(calyx_ast::Port {
                                cell: acm_reg.name.clone(),
                                port: "out".to_string(),
                            }),
                        );
                    }
                    let count_reg = calyx_ast::Cell {
                        name: self.fresh_name(),
                        is_external: false,
                        is_ref: false,
                        circuit: calyx_ast::Circuit::StdReg {
                            width: ADDRESS_WIDTH,
                        },
                    };
                    let arg_reg = calyx_ast::Cell {
                        name: self.fresh_name(),
                        is_external: false,
                        is_ref: false,
                        circuit: calyx_ast::Circuit::StdReg { width },
                    };
                    self.env.insert(
                        arg.clone(),
                        calyx_ast::Src::Port(calyx_ast::Port {
                            cell: arg_reg.name.clone(),
                            port: "out".to_string(),
                        }),
                    );
                    let cond_lt = calyx_ast::Cell {
                        name: self.fresh_name(),
                        is_external: false,
                        is_ref: false,
                        circuit: calyx_ast::Circuit::StdLt {
                            width: ADDRESS_WIDTH,
                        },
                    };
                    let mut init_count_reg_group = self.new_group();
                    init_count_reg_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: count_reg.name.clone(),
                            port: "in".to_string(),
                        },
                        src: calyx_ast::Src::Int {
                            value: 0,
                            width: ADDRESS_WIDTH,
                        },
                    });
                    init_count_reg_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: count_reg.name.clone(),
                            port: "write_en".to_string(),
                        },
                        src: calyx_ast::Src::Int { value: 1, width: 1 },
                    });
                    init_count_reg_group.done = Some(calyx_ast::Src::Port(calyx_ast::Port {
                        cell: count_reg.name.clone(),
                        port: "done".to_string(),
                    }));

                    let mut init_acm_reg_group = self.new_group();
                    init_acm_reg_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: acm_reg.name.clone(),
                            port: "in".to_string(),
                        },
                        src: init_value.clone(),
                    });
                    init_acm_reg_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: acm_reg.name.clone(),
                            port: "write_en".to_string(),
                        },
                        src: calyx_ast::Src::Int { value: 1, width: 1 },
                    });
                    init_acm_reg_group.done = Some(calyx_ast::Src::Port(calyx_ast::Port {
                        cell: acm_reg.name.clone(),
                        port: "done".to_string(),
                    }));
                    let init_control = calyx_ast::Control::Par(vec![
                        calyx_ast::Control::GroupName(init_count_reg_group.name.clone()),
                        calyx_ast::Control::GroupName(init_acm_reg_group.name.clone()),
                    ]);
                    self.get_current_func()?
                        .wires
                        .groups
                        .push(init_count_reg_group);
                    self.get_current_func()?
                        .wires
                        .groups
                        .push(init_acm_reg_group);

                    seq_vec.push(init_control);

                    let mut cond_lt_group = self.new_group();
                    cond_lt_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: cond_lt.name.clone(),
                            port: "left".to_string(),
                        },
                        src: calyx_ast::Src::Port(calyx_ast::Port {
                            cell: count_reg.name.clone(),
                            port: "out".to_string(),
                        }),
                    });
                    cond_lt_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: cond_lt.name.clone(),
                            port: "right".to_string(),
                        },
                        src: calyx_ast::Src::Int {
                            value: size as isize,
                            width: ADDRESS_WIDTH,
                        },
                    });
                    let cond_lt_group_name = cond_lt_group.name.clone();
                    self.get_current_func()?
                        .wires
                        .groups
                        .push(cond_lt_group.clone());

                    let mut read_array_group = self.new_group();
                    read_array_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: array.cell.clone(),
                            port: "addr0".to_string(),
                        },
                        src: calyx_ast::Src::Port(calyx_ast::Port {
                            cell: count_reg.name.clone(),
                            port: "out".to_string(),
                        }),
                    });
                    read_array_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: arg_reg.name.clone(),
                            port: "in".to_string(),
                        },
                        src: calyx_ast::Src::Port(calyx_ast::Port {
                            cell: array.cell.clone(),
                            port: "read_data".to_string(),
                        }),
                    });
                    read_array_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: arg_reg.name.clone(),
                            port: "write_en".to_string(),
                        },
                        src: calyx_ast::Src::Int { value: 1, width: 1 },
                    });
                    read_array_group.done = Some(calyx_ast::Src::Port(calyx_ast::Port {
                        cell: arg_reg.name.clone(),
                        port: "done".to_string(),
                    }));

                    let result_var = self.fresh_name();
                    self.type_env.insert(result_var.clone(), Type::I(width));

                    let body_group = self.convert_expr(expr, Some(result_var.clone()))?;

                    let result = self.env.get(&result_var).cloned().ok_or_else(|| {
                        anyhow::anyhow!(
                            "internal error: Expected result variable {} to be in environment",
                            result_var
                        )
                    })?;

                    let mut result_reg_group = self.new_group();
                    result_reg_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: acm_reg.name.clone(),
                            port: "in".to_string(),
                        },
                        src: result,
                    });
                    result_reg_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: acm_reg.name.clone(),
                            port: "write_en".to_string(),
                        },
                        src: calyx_ast::Src::Int { value: 1, width: 1 },
                    });
                    result_reg_group.done = Some(calyx_ast::Src::Port(calyx_ast::Port {
                        cell: acm_reg.name.clone(),
                        port: "done".to_string(),
                    }));

                    let mut inc_count_group = self.new_group();
                    inc_count_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: add_cell.name.clone(),
                            port: "left".to_string(),
                        },
                        src: calyx_ast::Src::Port(calyx_ast::Port {
                            cell: count_reg.name.clone(),
                            port: "out".to_string(),
                        }),
                    });
                    inc_count_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: add_cell.name.clone(),
                            port: "right".to_string(),
                        },
                        src: calyx_ast::Src::Int {
                            value: 1,
                            width: ADDRESS_WIDTH,
                        },
                    });
                    inc_count_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: count_reg.name.clone(),
                            port: "in".to_string(),
                        },
                        src: calyx_ast::Src::Port(calyx_ast::Port {
                            cell: add_cell.name.clone(),
                            port: "out".to_string(),
                        }),
                    });
                    inc_count_group.wires.push(calyx_ast::Wire {
                        dest: calyx_ast::Port {
                            cell: count_reg.name.clone(),
                            port: "write_en".to_string(),
                        },
                        src: calyx_ast::Src::Int { value: 1, width: 1 },
                    });
                    inc_count_group.done = Some(calyx_ast::Src::Port(calyx_ast::Port {
                        cell: count_reg.name.clone(),
                        port: "done".to_string(),
                    }));

                    let mut while_body = vec![];
                    while_body.push(calyx_ast::Control::GroupName(read_array_group.name.clone()));
                    if !body_group.is_empty() {
                        while_body.push(body_group);
                    }
                    while_body.push(calyx_ast::Control::GroupName(result_reg_group.name.clone()));
                    while_body.push(calyx_ast::Control::GroupName(inc_count_group.name.clone()));

                    seq_vec.push(calyx_ast::Control::While {
                        condition: calyx_ast::Port {
                            cell: cond_lt.name.clone(),
                            port: "out".to_string(),
                        },
                        with: Some(cond_lt_group_name),
                        body: while_body,
                    });

                    self.get_current_func()?.wires.groups.push(read_array_group);
                    self.get_current_func()?.wires.groups.push(result_reg_group);
                    self.get_current_func()?.wires.groups.push(inc_count_group);

                    self.get_current_func()?.cells.push(acm_reg);
                    self.get_current_func()?.cells.push(count_reg);
                    self.get_current_func()?.cells.push(arg_reg);
                    self.get_current_func()?.cells.push(cond_lt);

                    Ok(calyx_ast::Control::Seq(seq_vec))
                }))
            }
            ast::ANormalBaseExpr::Call(fun_name, args) => {
                let args: Vec<calyx_ast::Src> = args
                    .iter()
                    .map(|arg| self.find_src_by_var(arg))
                    .collect::<Result<_>>()?;
                let fun_name = fun_name.to_string();
                Ok(Box::new(move |dest: Option<String>| {
                    let (params, result_ty) = self
                        .fun_type_env
                        .get(&fun_name)
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "Function {} not found in function environment",
                                fun_name
                            )
                        })?
                        .clone();
                    let is_contain_array =
                        params.iter().any(|(_, ty)| matches!(ty, Type::Array(_, _)))
                            || result_ty
                                .as_ref()
                                .map_or(false, |ty| matches!(ty, Type::Array(_, _)));
                    if is_contain_array {
                        todo!()
                    } else {
                        let fun = self.fresh_name();
                        let fun_cell = calyx_ast::Cell {
                            name: fun,
                            is_external: false,
                            is_ref: false,
                            circuit: calyx_ast::Circuit::FunInstance {
                                name: fun_name.clone(),
                            },
                        };
                        self.get_current_func()?.cells.push(fun_cell.clone());
                        let mut group = self.new_group();
                        for (i, (param_name, _)) in params.iter().enumerate() {
                            group.wires.push(calyx_ast::Wire {
                                dest: calyx_ast::Port {
                                    cell: fun_cell.name.clone(),
                                    port: param_name.clone(),
                                },
                                src: args[i].clone(),
                            });
                        }
                        group.wires.push(calyx_ast::Wire {
                            dest: calyx_ast::Port {
                                cell: fun_cell.name.clone(),
                                port: "go".to_string(),
                            },
                            src: calyx_ast::Src::Int { value: 1, width: 1 },
                        });
                        if let Some(dest) = dest {
                            self.env.insert(
                                dest.clone(),
                                calyx_ast::Src::Port(calyx_ast::Port {
                                    cell: fun_cell.name.clone(),
                                    port: Converter::FUN_OUT_NAME.to_string(),
                                }),
                            );
                        }
                        group.done = Some(calyx_ast::Src::Port(calyx_ast::Port {
                            cell: fun_cell.name.clone(),
                            port: "done".to_string(),
                        }));
                        let group_name = group.name.clone();
                        self.get_current_func()?.wires.groups.push(group);
                        Ok(calyx_ast::Control::GroupName(group_name))
                    }
                }))
            }
            ast::ANormalBaseExpr::ArraySet(array, index, value) => {
                let calyx_ast::Src::Port(array) = self.find_src_by_var(array)? else {
                    return Err(anyhow::anyhow!("Expected a port for array variable"));
                };
                let index = self.find_src_by_var(index)?;
                let value = self.find_src_by_var(value)?;

                Ok(Box::new(move |dest: Option<String>| {
                    let mut group = self.new_group();
                    group.wires.push(calyx_ast::Wire {
                        dest: array.port("addr0"),
                        src: index.clone(),
                    });
                    group.wires.push(calyx_ast::Wire {
                        dest: array.port("write_data"),
                        src: value.clone(),
                    });
                    group.wires.push(calyx_ast::Wire {
                        dest: array.port("write_en"),
                        src: calyx_ast::Src::Int { value: 1, width: 1 },
                    });
                    group.done = Some(array.port("done").into());
                    let group_name = group.name.clone();
                    self.get_current_func()?.wires.groups.push(group);
                    if let Some(dest) = dest {
                        self.env.insert(dest.clone(), value);
                    }
                    Ok(calyx_ast::Control::GroupName(group_name))
                }))
            }
        }
    }

    fn convert_external_decl(&mut self, decl: &ast::ExternalDecl) -> Result<()> {
        let ast::Type::Array(ty, size) = &decl.ty else {
            return Err(anyhow::anyhow!("Unsupported type in external declaration"));
        };
        let ast::Type::I(width) = &**ty else {
            return Err(anyhow::anyhow!("Unsupported type in external declaration"));
        };
        self.program.main.cells.push(calyx_ast::Cell {
            name: decl.name.clone(),
            is_external: true,
            is_ref: false,
            circuit: calyx_ast::Circuit::CombMemD1 {
                data_width: *width,
                len: *size,
                address_width: ADDRESS_WIDTH,
            },
        });
        let mem_port: calyx_ast::Src = calyx_ast::Port {
            cell: decl.name.clone(),
            port: "read_data".to_string(),
        }
        .into();
        self.env.insert(decl.name.clone(), mem_port);
        self.type_env.insert(decl.name.clone(), decl.ty.clone());
        Ok(())
    }
}
