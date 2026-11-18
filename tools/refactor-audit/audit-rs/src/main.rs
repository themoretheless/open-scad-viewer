//! Workspace audit: per-function, per-file, per-type metrics and the resolved
//! module tree with intra-crate / cross-crate references. Read-only.
//!
//! usage: audit-rs <repo_root> <roots.tsv> <deps.tsv> <out_dir> <scan_dir>...
//!   roots.tsv: crate \t kind \t absolute src_path   (from `cargo metadata`)
//!   deps.tsv:  crate \t extern ident                 (from `cargo metadata`)
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Component, Path, PathBuf};

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

#[derive(Default, Clone, Copy)]
struct BodyMetrics {
    cyclo: usize,
    cognitive: usize,
    max_nest: usize,
    stmts: usize,
    unwraps: usize,
    expects: usize,
    clones: usize,
    alloc_in_loop: usize,
    panics: usize,
    unsafes: usize,
    tries: usize,
    closures: usize,
    max_loop_depth: usize,
    index_ops: usize,
    as_casts: usize,
}

#[derive(Default, Clone)]
struct FnRec {
    inline_mod: Vec<String>,
    ctx: String,
    trait_impl: bool,
    name: String,
    line: usize,
    loc: usize,
    body_loc: usize,
    params: usize,
    bool_params: usize,
    generics: usize,
    vis: &'static str,
    test_attr: bool,
    in_test_mod: bool,
    is_async: bool,
    is_unsafe: bool,
    has_doc: bool,
    ret: &'static str,
    m: BodyMetrics,
}

#[derive(Default, Clone)]
struct ModDecl {
    name: String,
    path_attr: Option<String>,
    inline_prefix: Vec<String>,
    cfg_test: bool,
    cfg: String,
    line: usize,
}

#[derive(Default, Clone)]
struct UseLeaf {
    inline_mod: Vec<String>,
    path: Vec<String>,
    alias: String,
    glob: bool,
    is_pub: bool,
}

#[derive(Default, Clone)]
struct TypeRec {
    kind: &'static str,
    name: String,
    extra: String,
    inline_mod: Vec<String>,
    line: usize,
    loc: usize,
    members: usize,
    pub_members: usize,
    vis: &'static str,
    in_test: bool,
}

#[derive(Default)]
struct FileInfo {
    abs: PathBuf,
    rel: String,
    krate: String,
    loc: usize,
    code: usize,
    comment: usize,
    blank: usize,
    parse_ok: bool,
    parse_err: String,
    file_cfg_test: bool,
    fns: Vec<FnRec>,
    mods: Vec<ModDecl>,
    uses: Vec<UseLeaf>,
    refs: HashMap<(String, bool, Vec<String>), usize>,
    allows: Vec<(usize, String, bool)>,
    types: Vec<TypeRec>,
    includes: Vec<String>,
    pub_items: usize,
    macro_rules: usize,
}

// ---------------------------------------------------------------- body metrics

#[derive(Default)]
struct BodyV {
    m: BodyMetrics,
    nest: usize,
    loop_depth: usize,
}

impl BodyV {
    fn enter(&mut self) {
        self.nest += 1;
        self.m.max_nest = self.m.max_nest.max(self.nest);
    }
    fn leave(&mut self) {
        self.nest -= 1;
    }
    fn enter_loop(&mut self) {
        self.loop_depth += 1;
        self.m.max_loop_depth = self.m.max_loop_depth.max(self.loop_depth);
    }
    fn leave_loop(&mut self) {
        self.loop_depth -= 1;
    }
    fn walk_if(&mut self, e: &syn::ExprIf, chained: bool) {
        self.m.cyclo += 1;
        self.m.cognitive += if chained { 1 } else { 1 + self.nest };
        self.visit_expr(&e.cond);
        self.enter();
        self.visit_block(&e.then_branch);
        self.leave();
        if let Some((_, els)) = &e.else_branch {
            match &**els {
                syn::Expr::If(inner) => self.walk_if(inner, true),
                other => {
                    self.m.cognitive += 1;
                    self.enter();
                    self.visit_expr(other);
                    self.leave();
                }
            }
        }
    }
}

const ITER_ADAPTERS: &[&str] = &[
    "map", "for_each", "filter_map", "flat_map", "fold", "filter", "any", "all", "find", "find_map",
    "position", "rposition", "try_for_each", "try_fold", "scan", "retain", "retain_mut", "sort_by",
    "sort_unstable_by", "sort_by_key", "sort_unstable_by_key", "sort_by_cached_key", "min_by",
    "max_by", "min_by_key", "max_by_key", "take_while", "skip_while", "map_while", "inspect",
    "partition", "reduce", "binary_search_by", "binary_search_by_key", "dedup_by", "dedup_by_key",
    "is_sorted_by", "group_by", "chunk_by", "and_modify", "filter_map_ok",
];

impl<'ast> Visit<'ast> for BodyV {
    fn visit_item(&mut self, _: &'ast syn::Item) {}
    fn visit_stmt(&mut self, s: &'ast syn::Stmt) {
        self.m.stmts += 1;
        visit::visit_stmt(self, s);
    }
    fn visit_local(&mut self, l: &'ast syn::Local) {
        if let Some(init) = &l.init {
            if init.diverge.is_some() {
                self.m.cyclo += 1;
                self.m.cognitive += 1;
            }
        }
        visit::visit_local(self, l);
    }
    fn visit_expr_if(&mut self, e: &'ast syn::ExprIf) {
        self.walk_if(e, false);
    }
    fn visit_expr_match(&mut self, e: &'ast syn::ExprMatch) {
        self.m.cyclo += e.arms.len().saturating_sub(1);
        self.m.cognitive += 1 + self.nest;
        self.visit_expr(&e.expr);
        self.enter();
        for arm in &e.arms {
            if let Some((_, g)) = &arm.guard {
                self.m.cyclo += 1;
                self.visit_expr(g);
            }
            self.visit_expr(&arm.body);
        }
        self.leave();
    }
    fn visit_expr_for_loop(&mut self, e: &'ast syn::ExprForLoop) {
        self.m.cyclo += 1;
        self.m.cognitive += 1 + self.nest;
        self.visit_expr(&e.expr);
        self.enter();
        self.enter_loop();
        self.visit_block(&e.body);
        self.leave_loop();
        self.leave();
    }
    fn visit_expr_while(&mut self, e: &'ast syn::ExprWhile) {
        self.m.cyclo += 1;
        self.m.cognitive += 1 + self.nest;
        self.visit_expr(&e.cond);
        self.enter();
        self.enter_loop();
        self.visit_block(&e.body);
        self.leave_loop();
        self.leave();
    }
    fn visit_expr_loop(&mut self, e: &'ast syn::ExprLoop) {
        self.m.cyclo += 1;
        self.m.cognitive += 1 + self.nest;
        self.enter();
        self.enter_loop();
        self.visit_block(&e.body);
        self.leave_loop();
        self.leave();
    }
    fn visit_expr_binary(&mut self, e: &'ast syn::ExprBinary) {
        if matches!(e.op, syn::BinOp::And(_) | syn::BinOp::Or(_)) {
            self.m.cyclo += 1;
            self.m.cognitive += 1;
        }
        visit::visit_expr_binary(self, e);
    }
    fn visit_expr_try(&mut self, e: &'ast syn::ExprTry) {
        self.m.tries += 1;
        visit::visit_expr_try(self, e);
    }
    fn visit_expr_closure(&mut self, e: &'ast syn::ExprClosure) {
        self.m.closures += 1;
        self.enter();
        visit::visit_expr_closure(self, e);
        self.leave();
    }
    fn visit_expr_unsafe(&mut self, e: &'ast syn::ExprUnsafe) {
        self.m.unsafes += 1;
        visit::visit_expr_unsafe(self, e);
    }
    fn visit_expr_index(&mut self, e: &'ast syn::ExprIndex) {
        self.m.index_ops += 1;
        visit::visit_expr_index(self, e);
    }
    fn visit_expr_cast(&mut self, e: &'ast syn::ExprCast) {
        self.m.as_casts += 1;
        visit::visit_expr_cast(self, e);
    }
    fn visit_expr_method_call(&mut self, e: &'ast syn::ExprMethodCall) {
        let name = e.method.to_string();
        match name.as_str() {
            "unwrap" => self.m.unwraps += 1,
            "expect" => self.m.expects += 1,
            "clone" => {
                self.m.clones += 1;
                if self.loop_depth > 0 {
                    self.m.alloc_in_loop += 1;
                }
            }
            "to_vec" | "to_owned" | "to_string" | "collect" | "into_owned" => {
                if self.loop_depth > 0 {
                    self.m.alloc_in_loop += 1;
                }
            }
            _ => {}
        }
        self.visit_expr(&e.receiver);
        let adapter = ITER_ADAPTERS.contains(&name.as_str());
        if adapter {
            self.enter_loop();
        }
        for a in &e.args {
            self.visit_expr(a);
        }
        if adapter {
            self.leave_loop();
        }
    }
    fn visit_expr_call(&mut self, e: &'ast syn::ExprCall) {
        if self.loop_depth > 0 {
            if let syn::Expr::Path(p) = &*e.func {
                let n = p.path.segments.len();
                if n >= 2 {
                    let ty = p.path.segments[n - 2].ident.to_string();
                    let f = p.path.segments[n - 1].ident.to_string();
                    let owning = matches!(
                        ty.as_str(),
                        "Vec" | "String" | "Box" | "HashMap" | "HashSet" | "BTreeMap" | "BTreeSet" | "Rc" | "Arc" | "VecDeque"
                    );
                    if owning && matches!(f.as_str(), "new" | "with_capacity" | "from") {
                        self.m.alloc_in_loop += 1;
                    }
                }
            }
        }
        visit::visit_expr_call(self, e);
    }
    fn visit_macro(&mut self, m: &'ast syn::Macro) {
        if let Some(seg) = m.path.segments.last() {
            match seg.ident.to_string().as_str() {
                "panic" | "unreachable" | "todo" | "unimplemented" => self.m.panics += 1,
                "vec" | "format" => {
                    if self.loop_depth > 0 {
                        self.m.alloc_in_loop += 1;
                    }
                }
                _ => {}
            }
        }
    }
}

// ---------------------------------------------------------------- file visitor

struct FileV<'a> {
    deps: &'a HashSet<String>,
    inline: Vec<String>,
    test_depth: usize,
    ctx: Vec<(String, bool)>,
    out: FileInfo,
}

fn vis_str(v: &syn::Visibility) -> &'static str {
    match v {
        syn::Visibility::Public(_) => "pub",
        syn::Visibility::Restricted(_) => "restricted",
        syn::Visibility::Inherited => "private",
    }
}

fn has_doc(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| a.path().is_ident("doc"))
}

fn has_test_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        let p = a.path();
        p.is_ident("test") || p.is_ident("bench") || p.segments.last().map(|s| s.ident == "test").unwrap_or(false)
    })
}

/// (is cfg(test)-gated, raw cfg text)
fn cfg_info(attrs: &[syn::Attribute]) -> (bool, String) {
    let mut test = false;
    let mut text = String::new();
    for a in attrs {
        if a.path().is_ident("cfg") {
            if let Ok(list) = a.meta.require_list() {
                let s = list.tokens.to_string();
                let compact: String = s.chars().filter(|c| !c.is_whitespace()).collect();
                if (compact == "test" || compact.contains("test,") || compact.contains(",test") || compact.contains("(test)"))
                    && !compact.contains("not(test")
                {
                    test = true;
                }
                if !text.is_empty() {
                    text.push(';');
                }
                text.push_str(&compact);
            }
        }
    }
    (test, text)
}

fn path_attr(attrs: &[syn::Attribute]) -> Option<String> {
    for a in attrs {
        if a.path().is_ident("path") {
            if let syn::Meta::NameValue(nv) = &a.meta {
                if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = &nv.value {
                    return Some(s.value());
                }
            }
        }
    }
    None
}

fn type_name(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(tp) => tp.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default(),
        syn::Type::Reference(r) => type_name(&r.elem),
        syn::Type::Paren(p) => type_name(&p.elem),
        syn::Type::Group(g) => type_name(&g.elem),
        other => {
            let s: String = other.to_token_stream().to_string().chars().filter(|c| !c.is_whitespace()).collect();
            s.chars().take(40).collect()
        }
    }
}

fn flatten_use(tree: &syn::UseTree, prefix: &mut Vec<String>, out: &mut Vec<(Vec<String>, String, bool)>) {
    match tree {
        syn::UseTree::Path(p) => {
            prefix.push(p.ident.to_string());
            flatten_use(&p.tree, prefix, out);
            prefix.pop();
        }
        syn::UseTree::Name(n) => {
            let mut v = prefix.clone();
            let name = n.ident.to_string();
            if name != "self" {
                v.push(name);
            }
            let alias = v.last().cloned().unwrap_or_default();
            out.push((v, alias, false));
        }
        syn::UseTree::Rename(r) => {
            let mut v = prefix.clone();
            if r.ident != "self" {
                v.push(r.ident.to_string());
            }
            out.push((v, r.rename.to_string(), false));
        }
        syn::UseTree::Glob(_) => out.push((prefix.clone(), "*".to_string(), true)),
        syn::UseTree::Group(g) => {
            for t in &g.items {
                flatten_use(t, prefix, out);
            }
        }
    }
}

impl<'a> FileV<'a> {
    fn in_test(&self) -> bool {
        self.test_depth > 0
    }
    fn note_pub(&mut self, v: &syn::Visibility) {
        if matches!(v, syn::Visibility::Public(_)) && !self.in_test() {
            self.out.pub_items += 1;
        }
    }
    fn add_ref(&mut self, segs: Vec<String>) {
        if segs.len() < 2 {
            return;
        }
        let first = segs[0].as_str();
        if !(first == "crate" || first == "super" || first == "self" || self.deps.contains(first)) {
            return;
        }
        let key = (self.inline.join("::"), self.in_test(), segs);
        *self.out.refs.entry(key).or_insert(0) += 1;
    }
    fn record_fn(&mut self, attrs: &[syn::Attribute], vis: &syn::Visibility, sig: &syn::Signature, block: &syn::Block) {
        let mut bv = BodyV::default();
        bv.m.cyclo = 1;
        bv.visit_block(block);
        let start = sig.span().start().line;
        let open = block.brace_token.span.open().start().line;
        let close = block.brace_token.span.close().start().line;
        let mut bool_params = 0;
        for a in &sig.inputs {
            if let syn::FnArg::Typed(pt) = a {
                if let syn::Type::Path(tp) = &*pt.ty {
                    if tp.path.is_ident("bool") {
                        bool_params += 1;
                    }
                }
            }
        }
        let ret = match &sig.output {
            syn::ReturnType::Default => "",
            syn::ReturnType::Type(_, ty) => match type_name(ty).as_str() {
                "Result" => "Result",
                "Option" => "Option",
                "bool" => "bool",
                _ => "other",
            },
        };
        let (ctx, trait_impl) = self.ctx.last().cloned().unwrap_or_default();
        let (cfg_test, _) = cfg_info(attrs);
        self.out.fns.push(FnRec {
            inline_mod: self.inline.clone(),
            ctx,
            trait_impl,
            name: sig.ident.to_string(),
            line: start,
            loc: close.saturating_sub(start) + 1,
            body_loc: close.saturating_sub(open) + 1,
            params: sig.inputs.len(),
            bool_params,
            generics: sig.generics.params.len(),
            vis: vis_str(vis),
            test_attr: has_test_attr(attrs),
            in_test_mod: self.in_test() || cfg_test,
            is_async: sig.asyncness.is_some(),
            is_unsafe: sig.unsafety.is_some(),
            has_doc: has_doc(attrs),
            ret,
            m: bv.m,
        });
    }
    fn record_type(&mut self, kind: &'static str, name: String, extra: String, line: usize, end: usize, members: usize, pub_members: usize, vis: &'static str) {
        self.out.types.push(TypeRec {
            kind,
            name,
            extra,
            inline_mod: self.inline.clone(),
            line,
            loc: end.saturating_sub(line) + 1,
            members,
            pub_members,
            vis,
            in_test: self.in_test(),
        });
    }
}

impl<'ast, 'a> Visit<'ast> for FileV<'a> {
    fn visit_attribute(&mut self, a: &'ast syn::Attribute) {
        let p = a.path();
        if p.is_ident("allow") || p.is_ident("expect") {
            if let Ok(list) = a.meta.require_list() {
                let line = a.span().start().line;
                for part in list.tokens.to_string().split(',') {
                    let lint: String = part.chars().filter(|c| !c.is_whitespace()).collect();
                    if !lint.is_empty() && !lint.contains('"') && !lint.starts_with("reason") {
                        self.out.allows.push((line, lint, self.in_test()));
                    }
                }
            }
        }
    }
    fn visit_item_mod(&mut self, m: &'ast syn::ItemMod) {
        let (cfg_test, cfg) = cfg_info(&m.attrs);
        for a in &m.attrs {
            self.visit_attribute(a);
        }
        self.note_pub(&m.vis);
        if let Some((_, items)) = &m.content {
            self.inline.push(m.ident.to_string());
            if cfg_test {
                self.test_depth += 1;
            }
            for it in items {
                self.visit_item(it);
            }
            if cfg_test {
                self.test_depth -= 1;
            }
            self.inline.pop();
        } else {
            self.out.mods.push(ModDecl {
                name: m.ident.to_string(),
                path_attr: path_attr(&m.attrs),
                inline_prefix: self.inline.clone(),
                cfg_test: cfg_test || self.in_test(),
                cfg,
                line: m.mod_token.span().start().line,
            });
        }
    }
    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        self.note_pub(&f.vis);
        self.record_fn(&f.attrs, &f.vis, &f.sig, &f.block);
        let saved = std::mem::take(&mut self.ctx);
        visit::visit_item_fn(self, f);
        self.ctx = saved;
    }
    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
        self.note_pub(&f.vis);
        self.record_fn(&f.attrs, &f.vis, &f.sig, &f.block);
        visit::visit_impl_item_fn(self, f);
    }
    fn visit_trait_item_fn(&mut self, f: &'ast syn::TraitItemFn) {
        if let Some(block) = &f.default {
            self.record_fn(&f.attrs, &syn::Visibility::Inherited, &f.sig, block);
        }
        visit::visit_trait_item_fn(self, f);
    }
    fn visit_item_impl(&mut self, i: &'ast syn::ItemImpl) {
        let name = type_name(&i.self_ty);
        let tr = i
            .trait_
            .as_ref()
            .and_then(|(_, p, _)| p.segments.last().map(|s| s.ident.to_string()))
            .unwrap_or_default();
        let methods = i.items.iter().filter(|it| matches!(it, syn::ImplItem::Fn(_))).count();
        let pub_methods = i
            .items
            .iter()
            .filter(|it| matches!(it, syn::ImplItem::Fn(f) if matches!(f.vis, syn::Visibility::Public(_))))
            .count();
        let line = i.impl_token.span().start().line;
        let end = i.brace_token.span.close().start().line;
        self.record_type("impl", name.clone(), tr.clone(), line, end, methods, pub_methods, "");
        self.ctx.push((name, !tr.is_empty()));
        visit::visit_item_impl(self, i);
        self.ctx.pop();
    }
    fn visit_item_trait(&mut self, t: &'ast syn::ItemTrait) {
        self.note_pub(&t.vis);
        let methods = t.items.iter().filter(|it| matches!(it, syn::TraitItem::Fn(_))).count();
        let line = t.trait_token.span().start().line;
        let end = t.brace_token.span.close().start().line;
        self.record_type("trait", t.ident.to_string(), String::new(), line, end, methods, methods, vis_str(&t.vis));
        self.ctx.push((t.ident.to_string(), false));
        visit::visit_item_trait(self, t);
        self.ctx.pop();
    }
    fn visit_item_struct(&mut self, s: &'ast syn::ItemStruct) {
        self.note_pub(&s.vis);
        let fields = s.fields.len();
        let pubf = s.fields.iter().filter(|f| matches!(f.vis, syn::Visibility::Public(_))).count();
        let line = s.struct_token.span().start().line;
        let end = s.span().end().line;
        self.record_type("struct", s.ident.to_string(), String::new(), line, end, fields, pubf, vis_str(&s.vis));
        visit::visit_item_struct(self, s);
    }
    fn visit_item_enum(&mut self, e: &'ast syn::ItemEnum) {
        self.note_pub(&e.vis);
        let line = e.enum_token.span().start().line;
        let end = e.brace_token.span.close().start().line;
        self.record_type("enum", e.ident.to_string(), String::new(), line, end, e.variants.len(), 0, vis_str(&e.vis));
        visit::visit_item_enum(self, e);
    }
    fn visit_item_const(&mut self, c: &'ast syn::ItemConst) {
        self.note_pub(&c.vis);
        visit::visit_item_const(self, c);
    }
    fn visit_item_static(&mut self, c: &'ast syn::ItemStatic) {
        self.note_pub(&c.vis);
        let line = c.static_token.span().start().line;
        let kind = if matches!(c.mutability, syn::StaticMutability::Mut(_)) { "static_mut" } else { "static" };
        self.record_type(kind, c.ident.to_string(), type_name(&c.ty), line, line, 0, 0, vis_str(&c.vis));
        visit::visit_item_static(self, c);
    }
    fn visit_item_type(&mut self, t: &'ast syn::ItemType) {
        self.note_pub(&t.vis);
        visit::visit_item_type(self, t);
    }
    fn visit_item_macro(&mut self, m: &'ast syn::ItemMacro) {
        if m.mac.path.is_ident("macro_rules") {
            self.out.macro_rules += 1;
        }
        visit::visit_item_macro(self, m);
    }
    fn visit_item_use(&mut self, u: &'ast syn::ItemUse) {
        let mut leaves = Vec::new();
        flatten_use(&u.tree, &mut Vec::new(), &mut leaves);
        let is_pub = !matches!(u.vis, syn::Visibility::Inherited);
        for (path, alias, glob) in leaves {
            if matches!(u.vis, syn::Visibility::Public(_)) && !self.in_test() {
                self.out.pub_items += 1;
            }
            let mut segs = path.clone();
            segs.truncate(6);
            self.add_ref(segs);
            self.out.uses.push(UseLeaf { inline_mod: self.inline.clone(), path, alias, glob, is_pub });
        }
    }
    fn visit_item_extern_crate(&mut self, e: &'ast syn::ItemExternCrate) {
        let name = e.ident.to_string();
        if self.deps.contains(name.as_str()) {
            let key = (self.inline.join("::"), self.in_test(), vec![name, "(extern crate)".to_string()]);
            *self.out.refs.entry(key).or_insert(0) += 1;
        }
    }
    fn visit_path(&mut self, p: &'ast syn::Path) {
        if p.segments.len() >= 2 {
            let segs: Vec<String> = p.segments.iter().take(6).map(|s| s.ident.to_string()).collect();
            self.add_ref(segs);
        }
        visit::visit_path(self, p);
    }
    fn visit_macro(&mut self, m: &'ast syn::Macro) {
        if m.path.is_ident("include") || m.path.is_ident("include_str") || m.path.is_ident("include_bytes") {
            if let Ok(s) = m.parse_body::<syn::LitStr>() {
                self.out.includes.push(s.value());
            }
        }
        visit::visit_macro(self, m);
    }
}

// ---------------------------------------------------------------- helpers

fn classify_lines(src: &str) -> (usize, usize, usize, usize) {
    let (mut loc, mut code, mut comment, mut blank) = (0, 0, 0, 0);
    let mut depth = 0usize;
    for line in src.lines() {
        loc += 1;
        let t = line.trim();
        if depth > 0 {
            comment += 1;
            depth += t.matches("/*").count();
            depth = depth.saturating_sub(t.matches("*/").count());
            continue;
        }
        if t.is_empty() {
            blank += 1;
        } else if t.starts_with("//") {
            comment += 1;
        } else if t.starts_with("/*") {
            comment += 1;
            depth += t.matches("/*").count();
            depth = depth.saturating_sub(t.matches("*/").count());
        } else {
            code += 1;
        }
    }
    (loc, code, comment, blank)
}

fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_dir() {
            if matches!(name.as_str(), "target" | "vendor" | ".git" | "node_modules") {
                continue;
            }
            walk(&p, out);
        } else if ft.is_file() && name.ends_with(".rs") {
            out.push(p);
        }
    }
}

fn analyze(abs: &Path, root: &Path, deps_by_crate: &HashMap<String, HashSet<String>>, empty: &HashSet<String>) -> FileInfo {
    let rel = abs.strip_prefix(root).unwrap_or(abs).to_string_lossy().to_string();
    let parts: Vec<&str> = rel.split('/').collect();
    let krate = if parts.len() > 2 && parts[0] == "crates" { parts[1].to_string() } else { parts[0].to_string() };
    let raw = fs::read_to_string(abs).unwrap_or_default();
    let (loc, code, comment, blank) = classify_lines(&raw);
    // syn 2 has no `yeet_expr`; `do yeet e` diverges exactly like `return e` for metrics.
    let src = raw.replace("do yeet ", "return  ");
    let deps = deps_by_crate.get(&krate).unwrap_or(empty);
    let mut v = FileV { deps, inline: Vec::new(), test_depth: 0, ctx: Vec::new(), out: FileInfo::default() };
    v.out.abs = abs.to_path_buf();
    v.out.rel = rel;
    v.out.krate = krate;
    v.out.loc = loc;
    v.out.code = code;
    v.out.comment = comment;
    v.out.blank = blank;
    match syn::parse_file(&src) {
        Ok(file) => {
            v.out.parse_ok = true;
            for a in &file.attrs {
                if a.path().is_ident("cfg") {
                    let (t, _) = cfg_info(std::slice::from_ref(a));
                    if t {
                        v.out.file_cfg_test = true;
                    }
                }
            }
            if v.out.file_cfg_test {
                v.test_depth += 1;
            }
            v.visit_file(&file);
        }
        Err(e) => {
            v.out.parse_ok = false;
            let at = e.span().start();
            v.out.parse_err = format!("{}:{}: {}", at.line, at.column, e);
        }
    }
    v.out
}

struct Placement {
    module: Vec<String>,
    kind: String,
    test: bool,
    reach: usize,
}

fn kind_rank(k: &str) -> usize {
    match k {
        "lib" => 0,
        "bin" => 1,
        "build" => 2,
        "test" => 3,
        "bench" => 4,
        "example" => 5,
        _ => 6,
    }
}

fn b(v: bool) -> u8 {
    v as u8
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 6 {
        eprintln!("usage: audit-rs <repo_root> <roots.tsv> <deps.tsv> <out_dir> <scan_dir>...");
        std::process::exit(2);
    }
    let root = normalize(Path::new(&args[1]));
    let out_dir = PathBuf::from(&args[4]);
    fs::create_dir_all(&out_dir)?;

    let mut roots: Vec<(String, String, PathBuf)> = Vec::new();
    for line in fs::read_to_string(&args[2])?.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() == 3 {
            roots.push((f[0].to_string(), f[1].to_string(), normalize(Path::new(f[2]))));
        }
    }
    roots.sort_by_key(|r| kind_rank(&r.1));
    let mut deps_by_crate: HashMap<String, HashSet<String>> = HashMap::new();
    for line in fs::read_to_string(&args[3])?.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() >= 2 {
            deps_by_crate.entry(f[0].to_string()).or_default().insert(f[1].to_string());
        }
    }

    let mut paths = Vec::new();
    for d in &args[5..] {
        walk(&root.join(d), &mut paths);
    }
    paths.sort();
    let n_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8);
    let chunk = paths.len().div_ceil(n_threads).max(1);
    let empty: HashSet<String> = HashSet::new();
    let mut infos: Vec<FileInfo> = Vec::with_capacity(paths.len());
    std::thread::scope(|s| {
        let mut handles = Vec::new();
        for part in paths.chunks(chunk) {
            let root = &root;
            let deps_by_crate = &deps_by_crate;
            let empty = &empty;
            let h = std::thread::Builder::new()
                .stack_size(512 << 20)
                .spawn_scoped(s, move || part.iter().map(|p| analyze(p, root, deps_by_crate, empty)).collect::<Vec<_>>())
                .expect("spawn");
            handles.push(h);
        }
        for h in handles {
            infos.extend(h.join().expect("worker panicked"));
        }
    });

    let index: HashMap<PathBuf, usize> = infos.iter().enumerate().map(|(i, f)| (normalize(&f.abs), i)).collect();

    // ---- pass 2: module tree
    let mut placement: HashMap<usize, Placement> = HashMap::new();
    let mut unresolved: Vec<(usize, String, usize, String)> = Vec::new();
    for (_krate, kind, path) in &roots {
        let Some(&ri) = index.get(path) else { continue };
        let root_test = matches!(kind.as_str(), "test" | "bench");
        let mut queue: VecDeque<(usize, Vec<String>, bool, bool, bool)> = VecDeque::new();
        queue.push_back((ri, Vec::new(), root_test, false, true));
        while let Some((fi, module, test, via_path, is_root)) = queue.pop_front() {
            if let Some(p) = placement.get_mut(&fi) {
                p.reach += 1;
                continue;
            }
            let info = &infos[fi];
            let test_here = test || info.file_cfg_test;
            placement.insert(fi, Placement { module: module.clone(), kind: kind.clone(), test: test_here, reach: 1 });
            let file = normalize(&info.abs);
            let dir = file.parent().map(Path::to_path_buf).unwrap_or_default();
            let fname = file.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let stem = file.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let is_mod_rs = is_root || fname == "mod.rs" || via_path;
            for md in &info.mods {
                let mut base = if is_mod_rs { dir.clone() } else { dir.join(&stem) };
                let mut m = module.clone();
                m.extend(md.inline_prefix.iter().cloned());
                m.push(md.name.clone());
                let (child, via) = if let Some(p) = &md.path_attr {
                    if md.inline_prefix.is_empty() {
                        base = dir.clone();
                    } else {
                        for c in &md.inline_prefix {
                            base.push(c);
                        }
                    }
                    (normalize(&base.join(p)), true)
                } else {
                    for c in &md.inline_prefix {
                        base.push(c);
                    }
                    let a = base.join(format!("{}.rs", md.name));
                    let bb = base.join(&md.name).join("mod.rs");
                    if index.contains_key(&a) {
                        (a, false)
                    } else {
                        (bb, false)
                    }
                };
                match index.get(&child) {
                    Some(&ci) => queue.push_back((ci, m, test_here || md.cfg_test, via, false)),
                    None => unresolved.push((fi, md.name.clone(), md.line, md.cfg.clone())),
                }
            }
        }
    }

    // files only reachable through include!()
    let mut included: HashSet<usize> = HashSet::new();
    for info in &infos {
        let dir = normalize(&info.abs).parent().map(Path::to_path_buf).unwrap_or_default();
        for inc in &info.includes {
            if let Some(&i) = index.get(&normalize(&dir.join(inc))) {
                included.insert(i);
            }
        }
    }

    let w = |name: &str| -> std::io::Result<BufWriter<fs::File>> { Ok(BufWriter::new(fs::File::create(out_dir.join(name))?)) };
    let mut f_files = w("files.tsv")?;
    let mut f_fns = w("functions.tsv")?;
    let mut f_types = w("types.tsv")?;
    let mut f_refs = w("refs.tsv")?;
    let mut f_allows = w("allows.tsv")?;
    let mut f_uses = w("rootuse.tsv")?;
    let mut f_unres = w("unresolved_mods.tsv")?;
    let mut f_err = w("parse_errors.tsv")?;
    let mut f_mods = w("moddecls.tsv")?;

    writeln!(f_files, "crate\tkind\ttest\tfile\tmodule\tloc\tcode\tcomment\tblank\tfns\ttest_fns\ttypes\tpub_items\tallows\tparse_ok\treach\tmacro_rules\tmod_decls")?;
    writeln!(f_fns, "crate\tkind\ttest\tfile\tmodule\tctx\tname\tline\tloc\tbody_loc\tparams\tbool_params\tgenerics\tcyclo\tcognitive\tmax_nest\tstmts\tunwraps\texpects\tclones\talloc_in_loop\tpanics\tunsafes\ttries\tclosures\tmax_loop_depth\tindex_ops\tas_casts\tvis\tasync\tunsafe_fn\thas_doc\tret\ttrait_impl")?;
    writeln!(f_types, "crate\tkind\ttest\tfile\tmodule\ttkind\tname\textra\tline\tloc\tmembers\tpub_members\tvis")?;
    writeln!(f_refs, "crate\tkind\ttest\tfile\tfrom_module\ttkind\tt0\tt1\tt2\tcount")?;
    writeln!(f_allows, "crate\tfile\tline\tlint\ttest")?;
    writeln!(f_uses, "crate\tkind\tfile\tmodule\talias\tpath\tis_pub\tglob")?;
    writeln!(f_unres, "crate\tfile\tmod\tline\tcfg")?;
    writeln!(f_err, "crate\tfile\terror")?;
    writeln!(f_mods, "crate\tfile\tmod\tline\tcfg\tpath_attr\tinline")?;

    for (fi, name, line, cfg) in &unresolved {
        let info = &infos[*fi];
        writeln!(f_unres, "{}\t{}\t{}\t{}\t{}", info.krate, info.rel, name, line, cfg)?;
    }

    for (i, info) in infos.iter().enumerate() {
        let (module, kind, test, reach) = match placement.get(&i) {
            Some(p) => (p.module.clone(), p.kind.clone(), p.test, p.reach),
            None => (Vec::new(), if included.contains(&i) { "included".to_string() } else { "orphan".to_string() }, false, 0),
        };
        let module_s = module.join("::");
        let test_fns = info.fns.iter().filter(|f| f.test_attr).count();
        writeln!(
            f_files,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            info.krate, kind, b(test), info.rel, module_s, info.loc, info.code, info.comment, info.blank,
            info.fns.len(), test_fns, info.types.len(), info.pub_items, info.allows.len(), b(info.parse_ok), reach,
            info.macro_rules, info.mods.len()
        )?;
        if !info.parse_ok {
            writeln!(f_err, "{}\t{}\t{}", info.krate, info.rel, info.parse_err.replace(['\t', '\n'], " "))?;
        }
        for md in &info.mods {
            writeln!(f_mods, "{}\t{}\t{}\t{}\t{}\t{}\t{}", info.krate, info.rel, md.name, md.line, md.cfg, md.path_attr.clone().unwrap_or_default(), md.inline_prefix.join("::"))?;
        }
        let full = |inline: &[String]| -> String {
            let mut m = module.clone();
            m.extend(inline.iter().cloned());
            m.join("::")
        };
        for f in &info.fns {
            let t = test || f.in_test_mod || f.test_attr;
            let m = &f.m;
            writeln!(
                f_fns,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                info.krate, kind, b(t), info.rel, full(&f.inline_mod), f.ctx, f.name, f.line, f.loc, f.body_loc, f.params,
                f.bool_params, f.generics, m.cyclo, m.cognitive, m.max_nest, m.stmts, m.unwraps, m.expects, m.clones,
                m.alloc_in_loop, m.panics, m.unsafes, m.tries, m.closures, m.max_loop_depth, m.index_ops, m.as_casts,
                f.vis, b(f.is_async), b(f.is_unsafe), b(f.has_doc), f.ret, b(f.trait_impl)
            )?;
        }
        for t in &info.types {
            writeln!(
                f_types,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                info.krate, kind, b(test || t.in_test), info.rel, full(&t.inline_mod), t.kind, t.name, t.extra, t.line, t.loc,
                t.members, t.pub_members, t.vis
            )?;
        }
        for (line, lint, t) in &info.allows {
            writeln!(f_allows, "{}\t{}\t{}\t{}\t{}", info.krate, info.rel, line, lint, b(test || *t))?;
        }
        if placement.get(&i).map(|p| p.module.is_empty()).unwrap_or(false) {
            for u in info.uses.iter().filter(|u| u.inline_mod.is_empty()) {
                writeln!(f_uses, "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", info.krate, kind, info.rel, module_s, u.alias, u.path.join("::"), b(u.is_pub), b(u.glob))?;
            }
        }
        // resolve references against the module tree
        let mut agg: HashMap<(String, bool, &'static str, String, String, String), usize> = HashMap::new();
        for ((inline, in_test, segs), count) in &info.refs {
            let mut cur = module.clone();
            if !inline.is_empty() {
                cur.extend(inline.split("::").map(str::to_string));
            }
            let from = cur.join("::");
            let first = segs[0].as_str();
            let (tkind, target): (&'static str, Vec<String>) = match first {
                "crate" => ("crate", segs[1..].to_vec()),
                "self" => {
                    let mut t = cur.clone();
                    t.extend(segs[1..].iter().cloned());
                    ("crate", t)
                }
                "super" => {
                    let mut t = cur.clone();
                    let mut k = 0;
                    let mut ok = true;
                    while k < segs.len() && segs[k] == "super" {
                        if t.pop().is_none() {
                            ok = false;
                            break;
                        }
                        k += 1;
                    }
                    if !ok {
                        continue;
                    }
                    t.extend(segs[k..].iter().cloned());
                    ("crate", t)
                }
                _ => ("extern", segs.to_vec()),
            };
            let g = |k: usize| target.get(k).cloned().unwrap_or_default();
            *agg.entry((from, test || *in_test, tkind, g(0), g(1), g(2))).or_insert(0) += count;
        }
        for ((from, t, tkind, t0, t1, t2), count) in agg {
            writeln!(f_refs, "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", info.krate, kind, b(t), info.rel, from, tkind, t0, t1, t2, count)?;
        }
    }
    eprintln!(
        "files={} placed={} unresolved_mods={} parse_errors={}",
        infos.len(),
        placement.len(),
        unresolved.len(),
        infos.iter().filter(|f| !f.parse_ok).count()
    );
    Ok(())
}
