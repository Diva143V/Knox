//! Interactive 2D Knowledge Graph canvas with force-directed physics layout and node inspection.

use std::collections::HashMap;

use eframe::egui::{
    self, Color32, CornerRadius, Painter, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2,
};

use crate::core::types::KnoxAction;
use crate::gui::theme::{
    COLOR_DEBUG, COLOR_FILE, COLOR_PLAN, COLOR_PRIMARY, action_color, actor_color,
};

/// Type of node displayed on the graph canvas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Project,
    File,
    Action,
}

/// A visual node in the 2D Knowledge Graph canvas.
#[derive(Debug, Clone)]
pub struct VisualNode {
    pub id: String,
    pub label: String,
    pub kind: NodeKind,
    pub action_type: Option<String>,
    pub actor: Option<String>,
    pub pos: Pos2,
    pub velocity: Vec2,
    pub is_current: bool,
    pub action: Option<KnoxAction>,
}

/// A visual edge connecting two nodes.
#[derive(Debug, Clone)]
pub struct VisualEdge {
    pub from: usize,
    pub to: usize,
    pub label: String,
    pub is_superseded: bool,
}

/// Interactive Knowledge Graph canvas with zoom, pan, and force-directed simulation.
pub struct GraphCanvas {
    pub nodes: Vec<VisualNode>,
    pub edges: Vec<VisualEdge>,
    pub pan: Vec2,
    pub zoom: f32,
    pub is_simulating: bool,
    pub dragged_node: Option<usize>,
    pub hovered_node: Option<usize>,
}

impl Default for GraphCanvas {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            pan: Vec2::ZERO,
            zoom: 1.0,
            is_simulating: true,
            dragged_node: None,
            hovered_node: None,
        }
    }
}

impl GraphCanvas {
    /// Builds graph nodes and edges from a list of KnoxActions.
    pub fn build_from_actions(&mut self, actions: &[KnoxAction], hide_superseded: bool) {
        self.nodes.clear();
        self.edges.clear();

        let mut node_map: HashMap<String, usize> = HashMap::new();
        let center = Pos2::new(400.0, 300.0);
        let count = actions.len().max(1);

        for (i, action) in actions.iter().enumerate() {
            if hide_superseded && !action.is_current {
                continue;
            }

            let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
            let radius = 120.0 + (i as f32 % 4.0) * 40.0;
            let pos = center + Vec2::new(angle.cos() * radius, angle.sin() * radius);

            let act_idx = self.nodes.len();
            let label = format!(
                "{}: {}",
                action.action_type,
                truncate_str(&action.summary, 22)
            );
            self.nodes.push(VisualNode {
                id: action.id.0.clone(),
                label,
                kind: NodeKind::Action,
                action_type: Some(action.action_type.clone()),
                actor: Some(action.actor.clone()),
                pos,
                velocity: Vec2::ZERO,
                is_current: action.is_current,
                action: Some(action.clone()),
            });
            node_map.insert(action.id.0.clone(), act_idx);

            self.link_action_targets(action, act_idx, &mut node_map, center);
        }

        self.link_action_dependencies(actions, &node_map);
    }

    fn link_action_targets(
        &mut self,
        action: &KnoxAction,
        act_idx: usize,
        node_map: &mut HashMap<String, usize>,
        center: Pos2,
    ) {
        for file in &action.files {
            let file_key = format!("file:{}", file);
            let file_idx = *node_map.entry(file_key.clone()).or_insert_with(|| {
                let idx = self.nodes.len();
                let fpos = center + Vec2::new(rand_offset(idx), rand_offset(idx + 1));
                self.nodes.push(VisualNode {
                    id: file_key,
                    label: file.clone(),
                    kind: NodeKind::File,
                    action_type: None,
                    actor: None,
                    pos: fpos,
                    velocity: Vec2::ZERO,
                    is_current: true,
                    action: None,
                });
                idx
            });

            self.edges.push(VisualEdge {
                from: act_idx,
                to: file_idx,
                label: "targets".to_string(),
                is_superseded: false,
            });
        }
    }

    fn link_action_dependencies(
        &mut self,
        actions: &[KnoxAction],
        node_map: &HashMap<String, usize>,
    ) {
        for action in actions {
            let act_idx = match node_map.get(&action.id.0) {
                Some(&i) => i,
                None => continue,
            };

            if let Some(ref pid) = action.parent_action_id {
                if let Some(&p_idx) = node_map.get(&pid.0) {
                    self.edges.push(VisualEdge {
                        from: p_idx,
                        to: act_idx,
                        label: "follows".to_string(),
                        is_superseded: false,
                    });
                }
            }

            if let Some(ref sid) = action.supersedes {
                if let Some(&s_idx) = node_map.get(&sid.0) {
                    self.edges.push(VisualEdge {
                        from: act_idx,
                        to: s_idx,
                        label: "supersedes".to_string(),
                        is_superseded: true,
                    });
                }
            }
        }
    }

    /// Advances the spring-embedder force simulation by one tick.
    pub fn step_simulation(&mut self) {
        if !self.is_simulating || self.nodes.is_empty() {
            return;
        }
        self.apply_forces(4000.0, 0.04, 110.0);
        self.apply_velocities();
    }

    fn apply_forces(&mut self, k_repulse: f32, k_spring: f32, natural_len: f32) {
        for i in 0..self.nodes.len() {
            for j in (i + 1)..self.nodes.len() {
                let delta = self.nodes[i].pos - self.nodes[j].pos;
                let dist = delta.length().max(10.0);
                let force = delta.normalized() * (k_repulse / (dist * dist));
                self.nodes[i].velocity += force;
                self.nodes[j].velocity -= force;
            }
        }

        for edge in &self.edges {
            if edge.from >= self.nodes.len() || edge.to >= self.nodes.len() {
                continue;
            }
            let delta = self.nodes[edge.to].pos - self.nodes[edge.from].pos;
            let dist = delta.length().max(1.0);
            let force = delta.normalized() * ((dist - natural_len) * k_spring);
            self.nodes[edge.from].velocity += force;
            self.nodes[edge.to].velocity -= force;
        }
    }

    fn apply_velocities(&mut self) {
        let center = Pos2::new(450.0, 320.0);
        for (i, node) in self.nodes.iter_mut().enumerate() {
            if Some(i) == self.dragged_node {
                node.velocity = Vec2::ZERO;
                continue;
            }
            let to_center = center - node.pos;
            node.velocity += to_center * 0.005;
            node.pos += node.velocity * 0.6;
            node.velocity *= 0.82;
        }
    }

    /// Renders the canvas inside the given UI rect and handles user interaction.
    pub fn render(&mut self, ui: &mut Ui) -> Option<KnoxAction> {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        self.handle_input(&response, rect);

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, CornerRadius::ZERO, Color32::from_rgb(15, 23, 42));

        self.draw_grid(&painter, rect);
        self.draw_edges(&painter, rect);
        let selected = self.draw_nodes(&painter, rect, &response);

        if self.is_simulating {
            self.step_simulation();
            ui.ctx().request_repaint();
        }

        selected
    }

    fn handle_input(&mut self, response: &Response, rect: Rect) {
        if response.dragged_by(egui::PointerButton::Secondary)
            || response.dragged_by(egui::PointerButton::Middle)
        {
            self.pan += response.drag_delta();
        }

        let scroll_delta = response.ctx.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta != 0.0 && response.hovered() {
            let factor = if scroll_delta > 0.0 { 1.08 } else { 0.92 };
            self.zoom = (self.zoom * factor).clamp(0.2, 3.5);
        }

        if let Some(hover_pos) = response.hover_pos() {
            let world_pos = self.screen_to_world(hover_pos, rect);
            self.hovered_node = self.find_node_at(world_pos);

            if response.drag_started_by(egui::PointerButton::Primary) {
                self.dragged_node = self.hovered_node;
            }
        }

        if response.dragged_by(egui::PointerButton::Primary) {
            if let Some(node_idx) = self.dragged_node {
                if let Some(hover_pos) = response.hover_pos() {
                    let world_pos = self.screen_to_world(hover_pos, rect);
                    if node_idx < self.nodes.len() {
                        self.nodes[node_idx].pos = world_pos;
                    }
                }
            } else {
                self.pan += response.drag_delta();
            }
        }

        if response.drag_stopped() {
            self.dragged_node = None;
        }
    }

    fn draw_grid(&self, painter: &Painter, rect: Rect) {
        let grid_size = 40.0 * self.zoom;
        let stroke = Stroke::new(1.0_f32, Color32::from_rgb(30, 41, 59));
        let offset_x = (self.pan.x * self.zoom) % grid_size;
        let offset_y = (self.pan.y * self.zoom) % grid_size;

        let mut x = rect.left() + offset_x;
        while x < rect.right() {
            painter.line_segment(
                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                stroke,
            );
            x += grid_size;
        }

        let mut y = rect.top() + offset_y;
        while y < rect.bottom() {
            painter.line_segment(
                [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                stroke,
            );
            y += grid_size;
        }
    }

    fn draw_edges(&self, painter: &Painter, rect: Rect) {
        for edge in &self.edges {
            if edge.from >= self.nodes.len() || edge.to >= self.nodes.len() {
                continue;
            }
            let p1 = self.world_to_screen(self.nodes[edge.from].pos, rect);
            let p2 = self.world_to_screen(self.nodes[edge.to].pos, rect);

            let stroke = if edge.is_superseded {
                Stroke::new(1.5_f32, COLOR_DEBUG)
            } else if edge.label == "targets" {
                Stroke::new(1.2_f32, COLOR_FILE)
            } else {
                Stroke::new(1.2_f32, COLOR_PLAN)
            };

            painter.line_segment([p1, p2], stroke);
            self.draw_arrowhead(painter, p1, p2, stroke.color);
        }
    }

    fn draw_arrowhead(&self, painter: &Painter, from: Pos2, to: Pos2, color: Color32) {
        let dir = (to - from).normalized();
        let arrow_len = 8.0 * self.zoom.clamp(0.6, 1.4);
        let arrow_pos = to - dir * (18.0 * self.zoom.clamp(0.6, 1.4));
        let perp = Vec2::new(-dir.y, dir.x) * (arrow_len * 0.5);

        painter.line_segment(
            [arrow_pos - dir * arrow_len + perp, arrow_pos],
            Stroke::new(1.5_f32, color),
        );
        painter.line_segment(
            [arrow_pos - dir * arrow_len - perp, arrow_pos],
            Stroke::new(1.5_f32, color),
        );
    }

    fn draw_nodes(&self, painter: &Painter, rect: Rect, response: &Response) -> Option<KnoxAction> {
        let mut clicked_action = None;

        for (i, node) in self.nodes.iter().enumerate() {
            let screen_pos = self.world_to_screen(node.pos, rect);
            let radius = match node.kind {
                NodeKind::Project => 22.0,
                NodeKind::File => 14.0,
                NodeKind::Action => 16.0,
            } * self.zoom.clamp(0.6, 1.8);

            let is_hovered = Some(i) == self.hovered_node;
            self.render_node_shape(painter, screen_pos, radius, node, is_hovered);

            if is_hovered && response.clicked() {
                if let Some(ref act) = node.action {
                    clicked_action = Some(act.clone());
                }
            }

            self.render_node_label(painter, screen_pos, radius, node);
        }

        clicked_action
    }

    fn render_node_shape(
        &self,
        painter: &Painter,
        pos: Pos2,
        radius: f32,
        node: &VisualNode,
        is_hovered: bool,
    ) {
        let (fill, stroke_color) = match node.kind {
            NodeKind::Project => (COLOR_PRIMARY, Color32::WHITE),
            NodeKind::File => (COLOR_FILE, Color32::WHITE),
            NodeKind::Action => {
                let col = action_color(node.action_type.as_deref().unwrap_or(""));
                let stroke = if node.is_current {
                    actor_color(node.actor.as_deref().unwrap_or(""))
                } else {
                    COLOR_DEBUG
                };
                (col, stroke)
            }
        };

        let stroke_width = if is_hovered { 3.0_f32 } else { 1.5_f32 };
        painter.circle(pos, radius, fill, Stroke::new(stroke_width, stroke_color));
    }

    fn render_node_label(&self, painter: &Painter, pos: Pos2, radius: f32, node: &VisualNode) {
        let font_size = (11.0 * self.zoom.clamp(0.7, 1.2)).max(8.0);
        let text_color = if node.is_current {
            Color32::from_rgb(226, 232, 240)
        } else {
            Color32::from_rgb(148, 163, 184)
        };

        let label_pos = pos + Vec2::new(0.0, radius + 3.0);
        painter.text(
            label_pos,
            egui::Align2::CENTER_TOP,
            &node.label,
            egui::FontId::proportional(font_size),
            text_color,
        );
    }

    fn screen_to_world(&self, screen_pos: Pos2, rect: Rect) -> Pos2 {
        let center = rect.center();
        let centered = screen_pos - center;
        Pos2::new(
            (centered.x - self.pan.x) / self.zoom + center.x,
            (centered.y - self.pan.y) / self.zoom + center.y,
        )
    }

    fn world_to_screen(&self, world_pos: Pos2, rect: Rect) -> Pos2 {
        let center = rect.center();
        let centered = world_pos - center;
        Pos2::new(
            centered.x * self.zoom + self.pan.x + center.x,
            centered.y * self.zoom + self.pan.y + center.y,
        )
    }

    fn find_node_at(&self, world_pos: Pos2) -> Option<usize> {
        let hit_dist = 22.0;
        self.nodes
            .iter()
            .position(|node| (node.pos - world_pos).length() < hit_dist)
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len])
    } else {
        s.to_string()
    }
}

fn rand_offset(seed: usize) -> f32 {
    let s = (seed * 1103515245 + 12345) % 1000;
    (s as f32 - 500.0) * 0.4
}
