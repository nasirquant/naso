//! Control-Flow Graph (CFG) construction for verification.
//!
//! This module builds CFGs from Naso AST functions for path-sensitive
//! analysis in the linearity and uncomputation provers.

use indexmap::IndexMap;
use naso_compiler::ast::{Block, Expr, Function, Ident, Span, Stmt};
use std::collections::HashMap;

/// A node in the control-flow graph.
#[derive(Debug, Clone)]
pub struct CfgNode {
    pub id: u32,
    pub kind: CfgNodeKind,
    pub span: Span,
    pub successors: Vec<u32>,
    pub predecessors: Vec<u32>,
}

/// Kind of CFG node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CfgNodeKind {
    /// Entry point of function
    Entry,
    /// Exit point of function
    Exit,
    /// Statement (assignment, call, etc.)
    Stmt(Stmt),
    /// Conditional branch
    Branch {
        condition: Expr,
        then_block: u32,
        else_block: Option<u32>,
    },
    /// Loop header
    LoopHeader {
        index: Ident,
        domain: Expr,
        body: u32,
    },
    /// Loop back-edge
    LoopBack,
    /// Merge point (after if/else)
    Merge,
}

/// Control-flow graph for a function.
#[derive(Debug, Clone)]
pub struct ControlFlowGraph {
    pub nodes: IndexMap<u32, CfgNode>,
    pub entry_id: u32,
    pub exit_id: u32,
    pub next_id: u32,
}

impl ControlFlowGraph {
    /// Build a CFG from a function.
    pub fn from_function(func: &Function) -> Result<Self, String> {
        let mut cfg = Self {
            nodes: IndexMap::new(),
            entry_id: 0,
            exit_id: 0,
            next_id: 0,
        };

        // Create entry node
        let entry_id = cfg.new_node(CfgNodeKind::Entry, Span::default());
        cfg.entry_id = entry_id;

        // Build CFG from function body
        if let Some(body) = &func.body {
            let exit_id = cfg.build_from_expr(body, entry_id)?;
            cfg.exit_id = exit_id;

            // Create explicit exit node
            let final_exit = cfg.new_node(CfgNodeKind::Exit, Span::default());
            cfg.add_edge(exit_id, final_exit);
            cfg.exit_id = final_exit;
        } else {
            // Empty function - direct entry to exit
            let exit_id = cfg.new_node(CfgNodeKind::Exit, Span::default());
            cfg.add_edge(entry_id, exit_id);
            cfg.exit_id = exit_id;
        }

        Ok(cfg)
    }

    /// Create a new node and return its ID.
    fn new_node(&mut self, kind: CfgNodeKind, span: Span) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        let node = CfgNode {
            id,
            kind,
            span,
            successors: Vec::new(),
            predecessors: Vec::new(),
        };
        self.nodes.insert(id, node);
        id
    }

    /// Add an edge between nodes.
    fn add_edge(&mut self, from: u32, to: u32) {
        if let Some(node) = self.nodes.get_mut(&from) {
            node.successors.push(to);
        }
        if let Some(node) = self.nodes.get_mut(&to) {
            node.predecessors.push(from);
        }
    }

    /// Build CFG from an expression, returning the exit node ID.
    fn build_from_expr(&mut self, expr: &Expr, entry_id: u32) -> Result<u32, String> {
        match &expr.kind {
            naso_compiler::ast::ExprKind::Let(binding) => {
                let mut current = entry_id;
                let let_node = self.new_node(
                    CfgNodeKind::Stmt(Stmt::Let {
                        name: binding.name.clone(),
                        ty: binding.ty.clone(),
                        init: binding.value.clone(),
                        span: binding.span,
                    }),
                    binding.span,
                );
                self.add_edge(current, let_node);
                current = let_node;
                self.build_from_expr(&binding.value, current)
            }
            naso_compiler::ast::ExprKind::If(cond, then_e, else_e) => {
                let branch_id = self.new_node(
                    CfgNodeKind::Branch {
                        condition: *cond.clone(),
                        then_block: 0, // Will be filled in
                        else_block: None,
                    },
                    expr.span,
                );
                self.add_edge(entry_id, branch_id);

                // Build then branch
                let then_id = self.new_node(
                    CfgNodeKind::Stmt(Stmt::Expr(then_e.as_ref().clone())),
                    expr.span,
                );
                let then_exit = self.build_from_expr(then_e, then_id)?;

                // Update branch node with then block
                if let Some(node) = self.nodes.get_mut(&branch_id) {
                    if let CfgNodeKind::Branch { then_block, .. } = &mut node.kind {
                        *then_block = then_id;
                    }
                }

                // Build else branch if present
                let else_exit = if let Some(else_e) = else_e {
                    let else_id = self.new_node(
                        CfgNodeKind::Stmt(Stmt::Expr(else_e.as_ref().clone())),
                        expr.span,
                    );
                    self.build_from_expr(else_e, else_id)?
                } else {
                    branch_id // Empty else - fall through
                };

                // Create merge node
                let merge_id = self.new_node(CfgNodeKind::Merge, expr.span);
                self.add_edge(then_exit, merge_id);
                if else_exit != branch_id {
                    self.add_edge(else_exit, merge_id);
                }

                // Update branch node with else block
                if let Some(node) = self.nodes.get_mut(&branch_id) {
                    if let CfgNodeKind::Branch { else_block, .. } = &mut node.kind {
                        *else_block = if else_exit == branch_id {
                            None
                        } else {
                            Some(else_exit)
                        };
                    }
                }

                Ok(merge_id)
            }
            naso_compiler::ast::ExprKind::For(loop_) => {
                let header_id = self.new_node(
                    CfgNodeKind::LoopHeader {
                        index: loop_.var.clone(),
                        domain: *loop_.iter.clone(),
                        body: 0, // Will be filled in
                    },
                    expr.span,
                );
                self.add_edge(entry_id, header_id);

                // Build loop body
                let body_id = self.new_node(
                    CfgNodeKind::Stmt(Stmt::Expr(loop_.body.expr.clone().unwrap_or(Expr::new(
                        naso_compiler::ast::ExprKind::Literal(naso_compiler::ast::Literal::Unit),
                        expr.span,
                        naso_compiler::ast::NodeId::default(),
                    )))),
                    expr.span,
                );
                let body_exit = self.build_from_expr(
                    &loop_.body.expr.clone().unwrap_or(Expr::new(
                        naso_compiler::ast::ExprKind::Literal(naso_compiler::ast::Literal::Unit),
                        expr.span,
                        naso_compiler::ast::NodeId::default(),
                    )),
                    body_id,
                )?;

                // Create back-edge
                let back_id = self.new_node(CfgNodeKind::LoopBack, expr.span);
                self.add_edge(body_exit, back_id);
                self.add_edge(back_id, header_id);

                // Update header with body
                if let Some(node) = self.nodes.get_mut(&header_id) {
                    if let CfgNodeKind::LoopHeader { body, .. } = &mut node.kind {
                        *body = body_id;
                    }
                }

                // Loop exit (after loop completes)
                let exit_id = self.new_node(CfgNodeKind::Merge, expr.span);
                self.add_edge(header_id, exit_id); // Exit when loop condition false

                Ok(exit_id)
            }
            naso_compiler::ast::ExprKind::Call(_, _)
            | naso_compiler::ast::ExprKind::Var(_, _)
            | naso_compiler::ast::ExprKind::Literal(_)
            | naso_compiler::ast::ExprKind::Unary(_, _)
            | naso_compiler::ast::ExprKind::Binary(_, _, _)
            | naso_compiler::ast::ExprKind::Field(_, _)
            | naso_compiler::ast::ExprKind::Index(_, _)
            | naso_compiler::ast::ExprKind::Projection(_)
            | _ => {
                let stmt_node =
                    self.new_node(CfgNodeKind::Stmt(Stmt::Expr(expr.clone())), expr.span);
                self.add_edge(entry_id, stmt_node);
                Ok(stmt_node)
            }
        }
    }

    /// Get all paths from entry to exit (for path-sensitive analysis).
    pub fn all_paths(&self) -> Vec<Vec<u32>> {
        let mut paths = Vec::new();
        let mut current_path = Vec::new();
        self.dfs_paths(self.entry_id, &mut current_path, &mut paths);
        paths
    }

    fn dfs_paths(&self, node_id: u32, current: &mut Vec<u32>, paths: &mut Vec<Vec<u32>>) {
        current.push(node_id);

        let node = &self.nodes[&node_id];
        if node_id == self.exit_id {
            paths.push(current.clone());
        } else {
            for &succ in &node.successors {
                // Avoid infinite loops in cycles (simple cycle detection)
                if !current.contains(&succ) || succ == self.exit_id {
                    self.dfs_paths(succ, current, paths);
                }
            }
        }

        current.pop();
    }

    /// Get nodes in topological order (for dataflow analysis).
    pub fn topological_order(&self) -> Vec<u32> {
        let mut visited = HashMap::new();
        let mut order = Vec::new();

        fn visit(
            cfg: &ControlFlowGraph,
            node_id: u32,
            visited: &mut HashMap<u32, bool>,
            order: &mut Vec<u32>,
        ) {
            if visited.get(&node_id).copied().unwrap_or(false) {
                return;
            }
            visited.insert(node_id, true);

            let node = &cfg.nodes[&node_id];
            for &succ in &node.successors {
                visit(cfg, succ, visited, order);
            }

            order.push(node_id);
        }

        visit(self, self.entry_id, &mut visited, &mut order);
        order.reverse();
        order
    }
}

/// Dataflow analysis framework for linearity checking.
pub struct LinearityDataflow {
    cfg: ControlFlowGraph,
    /// For each node, the set of [1] resources that are live (allocated but not consumed)
    live_in: HashMap<u32, Vec<String>>,
    live_out: HashMap<u32, Vec<String>>,
}

impl LinearityDataflow {
    pub fn new(cfg: ControlFlowGraph) -> Self {
        Self {
            cfg,
            live_in: HashMap::new(),
            live_out: HashMap::new(),
        }
    }

    /// Run the dataflow analysis to find linearity violations.
    pub fn analyze(&mut self) -> Result<Vec<LinearityViolation>, String> {
        // Initialize: at entry, all [1] parameters are live
        // This would be populated from the function signature

        // Iterate until fixed point
        let mut changed = true;
        while changed {
            changed = false;
            for &node_id in &self.cfg.topological_order() {
                if self.transfer(node_id)? {
                    changed = true;
                }
            }
        }

        // Check for violations at exit
        let mut violations = Vec::new();
        if let Some(live) = self.live_out.get(&self.cfg.exit_id) {
            for resource in live {
                violations.push(LinearityViolation {
                    kind: ViolationKind::Leak,
                    resource: resource.clone(),
                    node_id: self.cfg.exit_id,
                    message: format!("Resource '{}' leaked at function exit", resource),
                });
            }
        }

        // Check for double-consumption and unconsumed on paths
        // This would require path-sensitive analysis

        Ok(violations)
    }

    /// Transfer function for a node.
    fn transfer(&mut self, node_id: u32) -> Result<bool, String> {
        let node = &self.cfg.nodes[&node_id];
        let mut live = self.live_in.get(&node_id).cloned().unwrap_or_default();

        match &node.kind {
            CfgNodeKind::Stmt(stmt) => {
                // Check for consume operations (linear_free, qfree, etc.)
                // Check for allocation operations (qalloc, linear_alloc, etc.)
                // Update live set accordingly
            }
            CfgNodeKind::Branch { .. } => {
                // Both branches get the same live_in
            }
            CfgNodeKind::Merge => {
                // Merge live sets from predecessors
                let mut merged = Vec::new();
                for &pred in &node.predecessors {
                    if let Some(pred_live) = self.live_out.get(&pred) {
                        merged.extend(pred_live.iter().cloned());
                    }
                }
                // Remove duplicates
                merged.sort();
                merged.dedup();
                live = merged;
            }
            CfgNodeKind::LoopHeader { .. } => {
                // Loop header: merge back-edge and entry
            }
            CfgNodeKind::LoopBack => {
                // Back to header
            }
            _ => {}
        }

        let changed = self
            .live_out
            .insert(node_id, live.clone())
            .map_or(true, |old| old != live);
        Ok(changed)
    }
}

/// Linearity violation found by dataflow analysis.
#[derive(Debug, Clone)]
pub struct LinearityViolation {
    pub kind: ViolationKind,
    pub resource: String,
    pub node_id: u32,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViolationKind {
    Leak,
    DoubleConsume,
    Unconsumed,
    InvalidAccess,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cfg_creation() {
        // Smoke test - would need actual Function AST
    }
}
