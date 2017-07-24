use std::str::FromStr;
use syntax::ast::*;
use syntax::codemap::{Span, BytePos};
use syntax::ext::hygiene::SyntaxContext;
use syntax::visit::{self, Visitor, FnKind};

use driver;
use visit::Visit;


#[derive(Debug)]
pub struct NodeInfo {
    pub id: NodeId,
    pub span: Span,
}


struct PickVisitor {
    node_info: Option<NodeInfo>,
    kind: NodeKind,
    target: Span,
}

impl<'a> Visitor<'a> for PickVisitor {
    fn visit_item(&mut self, x: &'a Item) {
        // Recurse first, so that the deepest node gets visited first.  This way we get
        // the function and not its containing module, for example.
        visit::walk_item(self, x);
        if self.node_info.is_none() &&
           self.kind.includes(NodeKind::Item) &&
           x.span.contains(self.target) {
            self.node_info = Some(NodeInfo { id: x.id, span: x.span });
        }

        // Special case for modules.  If the cursor lies within the inner span of a mod item
        // (meaning inside the included file), then we mark the mod item itself.  This is because
        // `Mod` nodes don't have their own IDs.
        if self.node_info.is_none() {
            if let ItemKind::Mod(ref m) = x.node {
                if m.inner.contains(self.target) {
                    self.node_info = Some(NodeInfo { id: x.id, span: x.span });
                }
            }
        }
    }

    fn visit_trait_item(&mut self, x: &'a TraitItem) {
        visit::walk_trait_item(self, x);
        if self.node_info.is_none() &&
           self.kind.includes(NodeKind::TraitItem) &&
           x.span.contains(self.target) {
            self.node_info = Some(NodeInfo { id: x.id, span: x.span });
        }
    }

    fn visit_impl_item(&mut self, x: &'a ImplItem) {
        visit::walk_impl_item(self, x);
        if self.node_info.is_none() &&
           self.kind.includes(NodeKind::ImplItem) &&
           x.span.contains(self.target) {
            self.node_info = Some(NodeInfo { id: x.id, span: x.span });
        }
    }

    fn visit_foreign_item(&mut self, x: &'a ForeignItem) {
        visit::walk_foreign_item(self, x);
        if self.node_info.is_none() &&
           self.kind.includes(NodeKind::ForeignItem) &&
           x.span.contains(self.target) {
            self.node_info = Some(NodeInfo { id: x.id, span: x.span });
        }
    }

    fn visit_stmt(&mut self, x: &'a Stmt) {
        visit::walk_stmt(self, x);
        if self.node_info.is_none() &&
           self.kind.includes(NodeKind::Stmt) &&
           x.span.contains(self.target) {
            self.node_info = Some(NodeInfo { id: x.id, span: x.span });
        }
    }

    fn visit_expr(&mut self, x: &'a Expr) {
        visit::walk_expr(self, x);
        if self.node_info.is_none() &&
           self.kind.includes(NodeKind::Expr) &&
           x.span.contains(self.target) {
            self.node_info = Some(NodeInfo { id: x.id, span: x.span });
        }
    }

    fn visit_pat(&mut self, x: &'a Pat) {
        visit::walk_pat(self, x);
        if self.node_info.is_none() &&
           self.kind.includes(NodeKind::Pat) &&
           x.span.contains(self.target) {
            self.node_info = Some(NodeInfo { id: x.id, span: x.span });
        }
    }

    fn visit_ty(&mut self, x: &'a Ty) {
        visit::walk_ty(self, x);
        if self.node_info.is_none() &&
           self.kind.includes(NodeKind::Ty) &&
           x.span.contains(self.target) {
            self.node_info = Some(NodeInfo { id: x.id, span: x.span });
        }
    }

    // There's no `visit_arg`, unfortunately, so we have to do this instead.
    fn visit_fn(&mut self, fk: FnKind<'a>, fd: &'a FnDecl, s: Span, _id: NodeId) {
        visit::walk_fn(self, fk, fd, s);

        if self.node_info.is_none() &&
           self.kind.includes(NodeKind::Arg) {
            for arg in &fd.inputs {
                if arg.ty.span.contains(self.target) ||
                   arg.pat.span.contains(self.target) ||
                   (arg.ty.span.ctxt == arg.pat.span.ctxt &&
                    arg.ty.span.between(arg.pat.span).contains(self.target)) {
                    self.node_info = Some(NodeInfo {
                        id: arg.id,
                        span: arg.ty.span.to(arg.pat.span),
                    });
                }
            }
        }
    }

    fn visit_mac(&mut self, mac: &'a Mac) {
        visit::walk_mac(self, mac);
    }
}


#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NodeKind {
    Any,
    ItemLike,

    Item,
    TraitItem,
    ImplItem,
    ForeignItem,
    Stmt,
    Expr,
    Pat,
    Ty,
    Arg,
}

impl NodeKind {
    fn includes(self, other: NodeKind) -> bool {
        match self {
            NodeKind::Any => true,
            NodeKind::ItemLike => match other {
                NodeKind::Item |
                NodeKind::TraitItem |
                NodeKind::ImplItem |
                NodeKind::ForeignItem => true,
                _ => false,
            },
            _ => self == other,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match *self {
            NodeKind::Any => "any",
            NodeKind::ItemLike => "itemlike",
            NodeKind::Item => "item",
            NodeKind::TraitItem => "trait_item",
            NodeKind::ImplItem => "impl_item",
            NodeKind::ForeignItem => "foreign_item",
            NodeKind::Stmt => "stmt",
            NodeKind::Expr => "expr",
            NodeKind::Pat => "pat",
            NodeKind::Ty => "ty",
            NodeKind::Arg => "arg",
        }
    }
}

impl FromStr for NodeKind {
    type Err = ();
    fn from_str(s: &str) -> Result<NodeKind, ()> {
        let kind =
            match s {
                "any" => NodeKind::Any,
                "itemlike" => NodeKind::ItemLike,

                "item" => NodeKind::Item,
                "trait_item" => NodeKind::TraitItem,
                "impl_item" => NodeKind::ImplItem,
                "foreign_item" => NodeKind::ForeignItem,
                "stmt" => NodeKind::Stmt,
                "expr" => NodeKind::Expr,
                "pat" => NodeKind::Pat,
                "ty" => NodeKind::Ty,
                "arg" => NodeKind::Arg,

                _ => return Err(()),
            };
        Ok(kind)
    }
}

pub fn pick_node(krate: &Crate, kind: NodeKind, pos: BytePos) -> Option<NodeInfo> {
    let mut v = PickVisitor {
        node_info: None,
        kind: kind,
        target: Span { lo: pos, hi: pos, ctxt: SyntaxContext::empty() },
    };
    krate.visit(&mut v);

    // If the cursor falls inside the crate's module, then mark the crate itself.
    if v.node_info.is_none() {
        if krate.module.inner.contains(v.target) {
            v.node_info = Some(NodeInfo { id: CRATE_NODE_ID, span: krate.span });
        }
    }

    v.node_info
}

pub fn pick_node_at_loc(krate: &Crate,
                        cx: &driver::Ctxt,
                        kind: NodeKind,
                        file: &str,
                        line: u32,
                        col: u32) -> Option<NodeInfo> {
    let fm = match cx.session().codemap().get_filemap(file) {
        Some(x) => x,
        None => {
            panic!("target position lies in nonexistent file {:?}", file);
        },
    };

    if line == 0 || line as usize - 1 >= fm.lines.borrow().len() {
        panic!("line {} is outside the bounds of {}", line, file);
    };
    let (lo, hi) = fm.line_bounds(line as usize - 1);

    let line_len = hi.0 - lo.0;
    if col >= line_len {
        panic!("column {} is outside the bounds of {} line {}", col, file, line);
    }

    // TODO: make this work when the line contains multibyte characters
    let pos = lo + BytePos(col);

    pick_node(krate, kind, pos)
}

pub fn pick_node_command(krate: &Crate, cx: &driver::Ctxt, args: &[String]) {
    let kind = NodeKind::from_str(&args[0]).unwrap();
    let file = &args[1];
    let line = u32::from_str(&args[2]).unwrap();
    let col = u32::from_str(&args[3]).unwrap();

    let result = pick_node_at_loc(krate, cx, kind, file, line, col);

    if let Some(ref result) = result {
        let lo_loc = cx.session().codemap().lookup_char_pos(result.span.lo);
        let hi_loc = cx.session().codemap().lookup_char_pos(result.span.hi - BytePos(1));
        info!("{{ \
            found: true, \
            node_id: {}, \
            span_lo: [{}, {}], \
            span_hi: [{}, {}] \
            }}", result.id, lo_loc.line, lo_loc.col.0 + 1, hi_loc.line, hi_loc.col.0 + 1);
    } else {
        info!("{{ found: false }}");
    }
}
