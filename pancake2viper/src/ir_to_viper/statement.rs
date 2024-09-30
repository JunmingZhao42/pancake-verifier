use crate::ir::{self, FunctionCall};
use crate::ir_to_viper::TranslationMode;
use crate::utils::ViperUtils;

use crate::{ToShape, ToViper, ToViperType};

use super::utils::ViperEncodeCtx;

impl<'a> ToViper<'a> for ir::Stmt {
    type Output = viper::Stmt<'a>;
    fn to_viper(self, ctx: &mut ViperEncodeCtx<'a>) -> Self::Output {
        let ast = ctx.ast;
        use ir::Stmt::*;
        let stmt = match self {
            Annotation(annot) => annot.to_viper(ctx),
            Skip => ast.comment("skip"),
            Definition(def) => def.to_viper(ctx),
            Assign(ass) => ass.to_viper(ctx),
            Break => ast.goto(&ctx.current_break_label()),
            Continue => ast.goto(&ctx.current_continue_label()),
            Return(ret) => ret.to_viper(ctx),
            If(ifs) => ifs.to_viper(ctx),
            While(whiles) => whiles.to_viper(ctx),
            Seq(seq) => seq.to_viper(ctx),
            Call(call) => call.to_viper(ctx),
            ExtCall(ext) => ext.to_viper(ctx),
            Store(store) => store.to_viper(ctx),
            StoreBits(store) => store.to_viper(ctx),
            SharedStore(store) => store.to_viper(ctx),
            SharedStoreBits(store) => store.to_viper(ctx),
            SharedLoad(load) => load.to_viper(ctx),
            SharedLoadBits(load) => load.to_viper(ctx),
        };
        ctx.stack.push(stmt);

        let decls = ctx
            .declarations
            .drain(..)
            .map(|d| d.into())
            .collect::<Vec<_>>();
        let seq = ast.seqn(&ctx.stack, &decls);
        ctx.stack.clear();
        seq
    }
}

impl<'a> ToViper<'a> for ir::Return {
    type Output = viper::Stmt<'a>;
    fn to_viper(self, ctx: &mut ViperEncodeCtx<'a>) -> Self::Output {
        let ast = ctx.ast;
        let value = self.value.to_viper(ctx);
        let ass = ast.local_var_assign(ctx.return_var().1, value);
        let goto = ast.goto(ctx.return_label());

        let decls = ctx.pop_decls();

        ctx.stack.push(ass);
        ctx.stack.push(goto);
        let seq = ast.seqn(&ctx.stack, &decls);
        ctx.stack.clear();
        seq
    }
}

impl<'a> ToViper<'a> for ir::If {
    type Output = viper::Stmt<'a>;
    fn to_viper(self, ctx: &mut ViperEncodeCtx<'a>) -> Self::Output {
        let ast = ctx.ast;

        let cond = self.cond.cond_to_viper(ctx);
        let mut then_ctx = ctx.child();
        let then_body = self.if_branch.to_viper(&mut then_ctx);
        let mut else_ctx = then_ctx.child();
        let else_body = self.else_branch.to_viper(&mut else_ctx);

        let decls = ctx.pop_decls();

        ctx.stack.push(ast.if_stmt(cond, then_body, else_body));
        let seq = ast.seqn(&ctx.stack, &decls);
        ctx.stack.clear();
        seq
    }
}

impl<'a> ToViper<'a> for ir::While {
    type Output = viper::Stmt<'a>;
    fn to_viper(self, ctx: &mut ViperEncodeCtx<'a>) -> Self::Output {
        let ast = ctx.ast;

        let cond = self.cond.cond_to_viper(ctx);
        let mut body_ctx = ctx.child();
        let body = self.body.to_viper(&mut body_ctx);

        let decls = ctx
            .declarations
            .drain(..)
            .map(|d| d.into())
            .collect::<Vec<_>>();

        let mut body_seq = ctx.stack.clone();
        body_seq.push(body);
        body_seq.push(ast.label(&ctx.current_continue_label(), &[]));
        let body = ast.seqn(&body_seq, &[]);

        ctx.stack
            .push(ast.while_stmt(cond, &body_ctx.invariants, body));
        ctx.stack.push(ast.label(&ctx.current_break_label(), &[]));
        let seq = ast.seqn(&ctx.stack, &decls);
        ctx.stack.clear();
        seq
    }
}

impl<'a> ToViper<'a> for ir::Seq {
    type Output = viper::Stmt<'a>;
    fn to_viper(self, ctx: &mut ViperEncodeCtx<'a>) -> Self::Output {
        let ast = ctx.ast;
        let stmts = self
            .stmts
            .into_iter()
            .map(|s| s.to_viper(ctx))
            .collect::<Vec<_>>();
        ast.seqn(&stmts, &[])
    }
}

impl<'a> ToViper<'a> for ir::Definition {
    type Output = viper::Stmt<'a>;
    fn to_viper(self, ctx: &mut ViperEncodeCtx<'a>) -> Self::Output {
        let ast = ctx.ast;
        let name = ctx.mangler.new_scoped_var(self.lhs);
        let shape = self.rhs.shape(ctx);
        let var = ast.new_var(&name, shape.to_viper_type(ctx));
        ctx.declarations.push(var.0);

        ctx.set_type(name, shape);
        let ass = ast.local_var_assign(var.1, self.rhs.to_viper(ctx));
        let scope = self.scope.to_viper(&mut ctx.child());

        let decls = ctx.pop_decls();

        ctx.stack.push(ass);
        ctx.stack.push(scope);
        let seq = ast.seqn(&ctx.stack, &decls);
        ctx.stack.clear();
        seq
    }
}

impl<'a> ToViper<'a> for ir::Assign {
    type Output = viper::Stmt<'a>;
    fn to_viper(self, ctx: &mut ViperEncodeCtx<'a>) -> Self::Output {
        let ast = ctx.ast;
        let lhs_shape = ctx.get_type(&self.lhs);
        let name = ctx.mangler.mangle_var(&self.lhs);
        assert_eq!(lhs_shape, self.rhs.shape(ctx));
        let var = ast.new_var(name, lhs_shape.to_viper_type(ctx));

        let ass = ast.local_var_assign(var.1, self.rhs.to_viper(ctx));
        let decls = ctx.pop_decls();

        ctx.stack.push(ass);
        let seq = ast.seqn(&ctx.stack, &decls);
        ctx.stack.clear();
        seq
    }
}

impl<'a> ToViper<'a> for ir::Call {
    type Output = viper::Stmt<'a>;
    fn to_viper(self, ctx: &mut ViperEncodeCtx<'a>) -> Self::Output {
        ir::Definition {
            lhs: ctx.fresh_var(),
            rhs: self.call,
            scope: Box::new(ir::Stmt::Skip),
        }
        .to_viper(ctx)
    }
}

impl<'a> ToViper<'a> for ir::ExtCall {
    type Output = viper::Stmt<'a>;
    fn to_viper(self, ctx: &mut ViperEncodeCtx<'a>) -> Self::Output {
        let ast = ctx.ast;
        let args = self.args.to_viper(ctx);
        ast.method_call(&format!("ffi_{}", self.fname), &args, &[])
    }
}

impl<'a> ToViper<'a> for ir::Annotation {
    type Output = viper::Stmt<'a>;
    fn to_viper(self, ctx: &mut ViperEncodeCtx<'a>) -> Self::Output {
        let ast = ctx.ast;

        use crate::ir::AnnotationType::*;
        match self.typ {
            fold @ (Fold | Unfold) => match self.expr {
                ir::Expr::FunctionCall(access) => {
                    let ast_node = |e| match fold {
                        Unfold => ast.unfold(e),
                        Fold => ast.fold(e),
                        _ => unreachable!(),
                    };
                    ast_node(ast.predicate_access_predicate(
                        ast.predicate_access(&access.args.to_viper(ctx), &access.fname),
                        ast.full_perm(),
                    ))
                }
                _ => panic!(
                    "Invalid Fold/Unfold. Expected predicate access, got {:?}",
                    self.expr
                ),
            },
            x => {
                let no_pos = ast.no_position();

                ctx.set_mode(self.typ.into());
                let body = self.expr.to_viper(ctx);
                ctx.set_mode(TranslationMode::Normal);
                ctx.mangler.clear_annot_var();
                match x {
                    Assertion => ast.assert(body, no_pos),
                    Inhale => ast.inhale(body, no_pos),
                    Exhale => ast.exhale(body, no_pos),
                    x @ (Invariant | Precondition | Postcondition) => {
                        match x {
                            Invariant => ctx.invariants.push(body),
                            Precondition => ctx.pres.push(body),
                            Postcondition => ctx.posts.push(body),
                            _ => unreachable!(),
                        };
                        ast.comment("annotation pushed")
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}
