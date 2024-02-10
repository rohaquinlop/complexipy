use rustpython_parser::ast::{self, Stmt};

pub fn has_recursive_calls(statement: Stmt) -> bool {
    let mut ans = false;

    match statement {
        Stmt::FunctionDef(f) => {
            f.body.iter().for_each(|node| {
                match node {
                    Stmt::Return(r) => match &r.value {
                        Some(v) => match v.clone().as_ref() {
                            ast::Expr::Call(c) => match c.func.clone().as_ref() {
                                ast::Expr::Name(n) => {
                                    ans = ans || n.id == f.name;
                                }
                                _ => {}
                            },
                            _ => {}
                        },
                        _ => {}
                    },
                    Stmt::Expr(e) => match e.value.clone().as_ref() {
                        ast::Expr::Call(c) => match c.func.clone().as_ref() {
                            ast::Expr::Name(n) => {
                                ans = ans || n.id == f.name;
                            }
                            _ => {}
                        },
                        _ => {}
                    },
                    _ => {}
                };
                ans = ans || has_recursive_calls(node.clone());
            });
        }
        _ => {}
    };

    ans
}

pub fn is_decorator(statement: Stmt) -> bool {
    let mut ans = false;
    match statement {
        Stmt::FunctionDef(f) => {
            if f.body.len() == 2 {
                ans =
                    true && match f.body[0].clone() {
                        Stmt::FunctionDef(..) => true,
                        _ => false,
                    } && match f.body[1].clone() {
                        Stmt::Return(..) => true,
                        _ => false,
                    };
            }
        }
        _ => {}
    }

    ans
}

pub fn count_bool_ops(expr: ast::Expr) -> u64 {
    let mut complexity: u64 = 0;

    match expr {
        ast::Expr::BoolOp(b) => {
            complexity += b.values.len() as u64 - 1;
            for value in b.values.iter() {
                complexity += count_bool_ops(value.clone());
            }
        }
        ast::Expr::BinOp(b) => {
            complexity += 1;
            complexity += count_bool_ops(*b.left);
            complexity += count_bool_ops(*b.right);
        }
        ast::Expr::UnaryOp(u) => {
            complexity += 1;
            complexity += count_bool_ops(*u.operand);
        }
        ast::Expr::Compare(c) => {
            complexity += count_bool_ops(*c.left);
            for comparator in c.comparators.iter() {
                complexity += count_bool_ops(comparator.clone());
            }
        }
        ast::Expr::IfExp(i) => {
            complexity += 1;
            complexity += count_bool_ops(*i.test);
            complexity += count_bool_ops(*i.body);
            complexity += count_bool_ops(*i.orelse);
        }
        _ => {}
    }

    complexity
}
