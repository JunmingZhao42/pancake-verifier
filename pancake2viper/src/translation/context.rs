use std::collections::HashMap;

use viper::{AstFactory, Declaration, LocalVarDecl};

use crate::{pancake::Shape, viper_prelude::IArrayHelper};

pub struct ViperEncodeCtx<'a> {
    pub ast: AstFactory<'a>,
    pub stack: Vec<viper::Stmt<'a>>,
    pub declarations: Vec<viper::LocalVarDecl<'a>>,
    type_map: HashMap<String, Shape>,
    fresh_counter: u64,
    while_counter: u64,
    pub iarray: IArrayHelper<'a>,
    pub options: EncodeOptions,
    fname: String,
    var_map: HashMap<String, String>,

    pub pres: Vec<viper::Expr<'a>>,
    pub posts: Vec<viper::Expr<'a>>,
    pub invariants: Vec<viper::Expr<'a>>,
}

#[derive(Clone, Copy)]
pub struct EncodeOptions {
    pub expr_unrolling: bool,
    pub assert_aligned_accesses: bool,
    pub no_mangle: bool,
    pub is_annot: bool,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            expr_unrolling: false,
            assert_aligned_accesses: true,
            no_mangle: false,
            is_annot: false,
        }
    }
}

impl<'a> ViperEncodeCtx<'a> {
    pub fn new(fname: String, ast: AstFactory<'a>, options: EncodeOptions) -> Self {
        let mut type_map = HashMap::new();
        type_map.insert("heap".into(), Shape::Simple);
        Self {
            ast,
            stack: vec![],
            declarations: vec![],
            type_map,
            fresh_counter: 0,
            while_counter: 0,
            iarray: IArrayHelper::new(ast),
            options,
            fname,
            var_map: HashMap::new(),
            pres: vec![],
            posts: vec![],
            invariants: vec![],
        }
    }

    pub fn child(&self) -> Self {
        Self {
            ast: self.ast,
            stack: vec![],
            declarations: vec![],
            type_map: self.type_map.clone(),
            fresh_counter: self.fresh_counter,
            while_counter: self.while_counter,
            iarray: self.iarray,
            options: self.options,
            fname: self.fname.clone(),
            var_map: self.var_map.clone(),
            pres: self.pres.clone(),
            posts: self.posts.clone(),
            invariants: self.invariants.clone(),
        }
    }

    pub fn fresh_var(&mut self) -> String {
        let fresh = format!("_fr{}", self.fresh_counter);
        self.fresh_counter += 1;
        fresh
    }

    pub fn current_break_label(&self) -> String {
        format!("break_label_{}", self.while_counter)
    }

    pub fn current_continue_label(&self) -> String {
        format!("continue_label_{}", self.while_counter)
    }

    pub fn return_label(&self) -> &'static str {
        "return_label"
    }

    fn return_var_name(&self) -> &'static str {
        "retval"
    }

    pub fn return_var(&self) -> (viper::LocalVarDecl, viper::Expr) {
        (
            self.ast
                .local_var_decl(self.return_var_name(), self.ast.int_type()),
            self.ast
                .local_var(self.return_var_name(), self.ast.int_type()),
        )
    }

    pub fn new_while_ctx(&mut self) {
        self.while_counter += 1;
    }

    pub fn new_scoped_var(&mut self, var: String) -> String {
        if var == "heap" || var == "result" {
            panic!(
                "{}' is a reserved keyword and can't be used as an identifier",
                &var
            )
        }
        let mangled = format!("__{}__{}__{}", &self.fname, var, self.fresh_counter);
        self.fresh_counter += 1;

        let prev = self.var_map.insert(var, mangled.clone());

        mangled
    }

    pub fn mangle_var<'b>(&'b self, var: &'b str) -> &'b str {
        if var == "heap" {
            return "heap";
        }
        if self.options.no_mangle {
            return var;
        }
        self.var_map.get(var).unwrap_or_else(move || {
            panic!("Variable {} was not declared: \n{:?}", var, self.var_map)
        })
    }

    pub fn mangle_fn(&self, fname: &str) -> String {
        format!("f_{}", fname)
    }

    pub fn mangle_arg(&self, arg: &str) -> String {
        format!("arg_{}", arg)
    }

    pub fn pop_decls(&mut self) -> Vec<Declaration<'a>> {
        self.declarations
            .drain(..)
            .map(LocalVarDecl::into)
            .collect()
    }

    pub fn heap_type(&self) -> viper::Type {
        self.iarray.get_type()
    }

    pub fn get_type(&self, var: &str) -> Shape {
        self.type_map.get(self.mangle_var(var)).unwrap().to_owned()
    }

    pub fn set_type(&mut self, var: String, shape: Shape) {
        self.type_map.insert(var, shape);
    }

    pub fn heap_var(&self) -> (viper::LocalVarDecl, viper::Expr) {
        (
            self.ast.local_var_decl("heap", self.heap_type()),
            self.ast.local_var("heap", self.heap_type()),
        )
    }
}
