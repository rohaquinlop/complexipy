#[cfg(any(feature = "python", feature = "wasm"))]
use ruff_python_ast::{self as ast, Expr, Stmt};

#[cfg(any(feature = "python", feature = "wasm"))]
pub fn function_level_cyclomatic(body: &[Stmt]) -> u64 {
    let mut total: u64 = 1;
    for stmt in body {
        total += count_stmt(stmt, true);
    }
    total
}

#[cfg(any(feature = "python", feature = "wasm"))]
pub fn module_level_cyclomatic(body: &[Stmt]) -> u64 {
    let mut total: u64 = 1;
    for stmt in body {
        if matches!(stmt, Stmt::FunctionDef(_) | Stmt::ClassDef(_)) {
            continue;
        }
        total += count_stmt(stmt, true);
    }
    total
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn count_stmt(stmt: &Stmt, skip_nested_defs: bool) -> u64 {
    let mut c: u64 = 0;
    match stmt {
        Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {
            if skip_nested_defs {
                return 0;
            }
        }
        Stmt::If(s) => {
            c += 1;
            c += count_expr(&s.test);
            for sub in s.body.iter() {
                c += count_stmt(sub, skip_nested_defs);
            }
            for clause in s.elif_else_clauses.iter() {
                if let Some(test) = &clause.test {
                    c += 1;
                    c += count_expr(test);
                }
                for sub in clause.body.iter() {
                    c += count_stmt(sub, skip_nested_defs);
                }
            }
        }
        Stmt::For(s) => {
            c += 1;
            c += count_expr(&s.iter);
            for sub in s.body.iter() {
                c += count_stmt(sub, skip_nested_defs);
            }
            for sub in s.orelse.iter() {
                c += count_stmt(sub, skip_nested_defs);
            }
        }
        Stmt::While(s) => {
            c += 1;
            c += count_expr(&s.test);
            for sub in s.body.iter() {
                c += count_stmt(sub, skip_nested_defs);
            }
            for sub in s.orelse.iter() {
                c += count_stmt(sub, skip_nested_defs);
            }
        }
        Stmt::Try(s) => {
            for sub in s.body.iter() {
                c += count_stmt(sub, skip_nested_defs);
            }
            for handler in s.handlers.iter() {
                c += 1;
                let ast::ExceptHandler::ExceptHandler(h) = handler;
                for sub in h.body.iter() {
                    c += count_stmt(sub, skip_nested_defs);
                }
            }
            for sub in s.orelse.iter() {
                c += count_stmt(sub, skip_nested_defs);
            }
            for sub in s.finalbody.iter() {
                c += count_stmt(sub, skip_nested_defs);
            }
        }
        Stmt::Match(s) => {
            c += count_expr(&s.subject);
            for case in s.cases.iter() {
                c += 1;
                if let Some(guard) = &case.guard {
                    c += count_expr(guard);
                }
                for sub in case.body.iter() {
                    c += count_stmt(sub, skip_nested_defs);
                }
            }
        }
        Stmt::With(s) => {
            for item in s.items.iter() {
                c += count_expr(&item.context_expr);
            }
            for sub in s.body.iter() {
                c += count_stmt(sub, skip_nested_defs);
            }
        }
        Stmt::Assert(s) => {
            c += count_expr(&s.test);
            if let Some(msg) = &s.msg {
                c += count_expr(msg);
            }
        }
        Stmt::Return(s) => {
            if let Some(v) = &s.value {
                c += count_expr(v);
            }
        }
        Stmt::Expr(s) => c += count_expr(&s.value),
        Stmt::Assign(s) => c += count_expr(&s.value),
        Stmt::AugAssign(s) => c += count_expr(&s.value),
        Stmt::AnnAssign(s) => {
            if let Some(v) = &s.value {
                c += count_expr(v);
            }
        }
        Stmt::Raise(s) => {
            if let Some(exc) = &s.exc {
                c += count_expr(exc);
            }
            if let Some(cause) = &s.cause {
                c += count_expr(cause);
            }
        }
        Stmt::Delete(s) => {
            for t in s.targets.iter() {
                c += count_expr(t);
            }
        }
        _ => {}
    }
    c
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn count_expr(expr: &Expr) -> u64 {
    let mut c: u64 = 0;
    match expr {
        Expr::BoolOp(b) => {
            let n = b.values.len() as u64;
            if n > 1 {
                c += n - 1;
            }
            for v in b.values.iter() {
                c += count_expr(v);
            }
        }
        Expr::If(i) => {
            c += 1;
            c += count_expr(&i.test);
            c += count_expr(&i.body);
            c += count_expr(&i.orelse);
        }
        Expr::UnaryOp(u) => c += count_expr(&u.operand),
        Expr::BinOp(b) => {
            c += count_expr(&b.left);
            c += count_expr(&b.right);
        }
        Expr::Compare(cmp) => {
            c += count_expr(&cmp.left);
            for cm in cmp.comparators.iter() {
                c += count_expr(cm);
            }
        }
        Expr::Lambda(l) => {
            c += count_expr(&l.body);
        }
        Expr::Call(call) => {
            c += count_expr(&call.func);
            for arg in call.arguments.args.iter() {
                c += count_expr(arg);
            }
            for kw in call.arguments.keywords.iter() {
                c += count_expr(&kw.value);
            }
        }
        Expr::Tuple(t) => {
            for e in t.elts.iter() {
                c += count_expr(e);
            }
        }
        Expr::List(l) => {
            for e in l.elts.iter() {
                c += count_expr(e);
            }
        }
        Expr::Set(s) => {
            for e in s.elts.iter() {
                c += count_expr(e);
            }
        }
        Expr::Dict(d) => {
            for v in d.iter_values() {
                c += count_expr(v);
            }
        }
        Expr::ListComp(lc) => {
            c += count_comprehension(&lc.generators);
            c += count_expr(&lc.elt);
        }
        Expr::SetComp(sc) => {
            c += count_comprehension(&sc.generators);
            c += count_expr(&sc.elt);
        }
        Expr::DictComp(dc) => {
            c += count_comprehension(&dc.generators);
            c += count_expr(&dc.key);
            c += count_expr(&dc.value);
        }
        Expr::Generator(g) => {
            c += count_comprehension(&g.generators);
            c += count_expr(&g.elt);
        }
        Expr::Subscript(s) => {
            c += count_expr(&s.value);
            c += count_expr(&s.slice);
        }
        Expr::Attribute(a) => c += count_expr(&a.value),
        Expr::Starred(s) => c += count_expr(&s.value),
        Expr::Await(a) => c += count_expr(&a.value),
        Expr::Yield(y) => {
            if let Some(v) = &y.value {
                c += count_expr(v);
            }
        }
        Expr::YieldFrom(y) => c += count_expr(&y.value),
        Expr::Named(n) => {
            c += count_expr(&n.target);
            c += count_expr(&n.value);
        }
        Expr::FString(f) => {
            for element in f.value.elements() {
                if let Some(inter) = element.as_interpolation() {
                    c += count_expr(&inter.expression);
                }
            }
        }
        _ => {}
    }
    c
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn count_comprehension(generators: &[ast::Comprehension]) -> u64 {
    let mut c: u64 = 0;
    for g in generators.iter() {
        c += count_expr(&g.iter);
        for if_clause in g.ifs.iter() {
            c += 1;
            c += count_expr(if_clause);
        }
    }
    c
}

#[cfg(all(test, any(feature = "python", feature = "wasm")))]
mod tests {
    use super::*;
    use ruff_python_parser::parse_module;

    fn cyclo(code: &str) -> u64 {
        let parsed = parse_module(code).expect("parse");
        let module = parsed.syntax();
        match module.body.first() {
            Some(Stmt::FunctionDef(f)) => function_level_cyclomatic(&f.body),
            _ => module_level_cyclomatic(&module.body),
        }
    }

    #[test]
    fn empty_function_is_one() {
        assert_eq!(cyclo("def f():\n    pass\n"), 1);
    }

    #[test]
    fn single_if_is_two() {
        assert_eq!(cyclo("def f(x):\n    if x:\n        return 1\n"), 2);
    }

    #[test]
    fn if_elif_else_is_three() {
        let code = "def f(x):\n    if x == 1:\n        return 1\n    elif x == 2:\n        return 2\n    else:\n        return 3\n";
        assert_eq!(cyclo(code), 3);
    }

    #[test]
    fn for_plus_and_or_counts_boolops() {
        let code = "def f(xs):\n    for x in xs:\n        if x > 0 and x < 10 or x == -1:\n            pass\n";
        // base 1 + for 1 + if 1 + (and/or: 3 operands-1 + 2 operands-1 = 2+1 -> actually single BoolOp? )
        // BoolOp flat: (x>0 and x<10) is one BoolOp with 2 values => +1. `or x==-1` wraps: outer BoolOp 2 values => +1
        // Total = 1 + 1 + 1 + 1 + 1 = 5
        assert_eq!(cyclo(code), 5);
    }

    #[test]
    fn nested_comprehension_two_if_clauses() {
        let code = "def f(xs):\n    return [x for x in xs if x > 0 if x < 10]\n";
        // base 1 + 2 if clauses = 3
        assert_eq!(cyclo(code), 3);
    }

    #[test]
    fn ternary_adds_one() {
        let code = "def f(x):\n    return 1 if x else 0\n";
        assert_eq!(cyclo(code), 2);
    }

    #[test]
    fn match_three_cases_is_four() {
        let code = "def f(x):\n    match x:\n        case 1:\n            pass\n        case 2:\n            pass\n        case _:\n            pass\n";
        assert_eq!(cyclo(code), 4);
    }

    #[test]
    fn try_with_two_excepts_is_three() {
        let code = "def f():\n    try:\n        pass\n    except ValueError:\n        pass\n    except KeyError:\n        pass\n    else:\n        pass\n    finally:\n        pass\n";
        assert_eq!(cyclo(code), 3);
    }

    #[test]
    fn assert_with_bare_else_no_contribution() {
        let code = "def f(x):\n    with open('f') as fp:\n        assert x\n";
        assert_eq!(cyclo(code), 1);
    }

    #[test]
    fn while_loop_adds_one() {
        assert_eq!(cyclo("def f(x):\n    while x:\n        x -= 1\n"), 2);
    }

    #[test]
    fn nested_function_does_not_leak() {
        let code = "def f(x):\n    def g(y):\n        if y:\n            return 1\n    return g\n";
        assert_eq!(cyclo(code), 1);
    }

    #[test]
    fn module_level_ignores_top_functions() {
        let code = "x = 1\nif x:\n    pass\ndef f():\n    if x:\n        pass\n";
        // module: base 1 + if 1 = 2 (function def skipped)
        let parsed = parse_module(code).expect("parse");
        assert_eq!(module_level_cyclomatic(&parsed.syntax().body), 2);
    }
}
