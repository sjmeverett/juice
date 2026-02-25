use crate::tree::{DimensionValue, StyleProps, WidgetNode};
use fontdue::Font;
use taffy::prelude::*;

#[derive(Debug, Clone)]
pub enum NodeContext {
    Container { background: Option<RgbColor>, node_id: Option<u32> },
    Text { content: String, color: RgbColor, font_name: String, font_size: f32 },
}

#[derive(Debug, Clone, Copy)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub fn from_hex(hex: &str) -> Self {
        let hex = hex.strip_prefix('#').unwrap_or(hex);
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
            RgbColor { r, g, b }
        } else {
            RgbColor { r: 255, g: 255, b: 255 }
        }
    }
}

pub struct LayoutTree {
    pub taffy: TaffyTree<NodeContext>,
    pub root: NodeId,
}

#[derive(Clone)]
struct InheritedStyle {
    color: RgbColor,
    font_name: String,
    font_size: f32,
}

impl InheritedStyle {
    fn new(default_font: &str) -> Self {
        InheritedStyle {
            color: RgbColor { r: 255, g: 255, b: 255 },
            font_name: default_font.to_string(),
            font_size: 24.0,
        }
    }
}

pub fn build_layout_tree(widget: &WidgetNode, default_font: &str) -> LayoutTree {
    let mut taffy = TaffyTree::new();
    let root = build_node(&mut taffy, widget, &InheritedStyle::new(default_font));
    LayoutTree { taffy, root }
}

fn build_node(
    taffy: &mut TaffyTree<NodeContext>,
    widget: &WidgetNode,
    inherited: &InheritedStyle,
) -> NodeId {
    match widget {
        WidgetNode::Text(content) => {
            let context = NodeContext::Text {
                content: content.clone(),
                color: inherited.color,
                font_name: inherited.font_name.clone(),
                font_size: inherited.font_size,
            };
            taffy
                .new_leaf_with_context(Style::default(), context)
                .unwrap()
        }
        WidgetNode::Element { props, children, .. } => {
            let mut child_inherited = inherited.clone();
            let mut background = None;

            if let Some(style) = &props.style {
                if let Some(bg) = &style.background {
                    background = Some(RgbColor::from_hex(bg));
                }
                if let Some(color) = &style.color {
                    child_inherited.color = RgbColor::from_hex(color);
                }
                if let Some(font) = &style.font {
                    child_inherited.font_name = font.clone();
                }
                if let Some(font_size) = style.font_size {
                    child_inherited.font_size = font_size;
                }
            }

            let child_ids: Vec<NodeId> = children
                .iter()
                .map(|c| build_node(taffy, c, &child_inherited))
                .collect();

            let taffy_style = style_from_props(props.style.as_ref());
            let node_id = taffy.new_with_children(taffy_style, &child_ids).unwrap();
            taffy
                .set_node_context(node_id, Some(NodeContext::Container { background, node_id: props.id }))
                .unwrap();
            node_id
        }
    }
}

fn style_from_props(style: Option<&StyleProps>) -> Style {
    let Some(s) = style else {
        return Style {
            display: Display::Flex,
            ..Default::default()
        };
    };

    let flex_direction = match s.flex_direction.as_deref() {
        Some("column") => FlexDirection::Column,
        _ => FlexDirection::Row,
    };

    let pad_all = s.padding.unwrap_or(0.0);
    let padding = Rect {
        left: LengthPercentage::length(s.padding_left.unwrap_or(pad_all)),
        right: LengthPercentage::length(s.padding_right.unwrap_or(pad_all)),
        top: LengthPercentage::length(s.padding_top.unwrap_or(pad_all)),
        bottom: LengthPercentage::length(s.padding_bottom.unwrap_or(pad_all)),
    };

    let gap_val = s.gap.unwrap_or(0.0);
    let gap = Size {
        width: LengthPercentage::length(gap_val),
        height: LengthPercentage::length(gap_val),
    };

    let width = dimension_from_value(s.width.as_ref());
    let height = dimension_from_value(s.height.as_ref());

    Style {
        display: Display::Flex,
        flex_direction,
        flex_grow: s.flex_grow.unwrap_or(0.0),
        flex_shrink: s.flex_shrink.unwrap_or(1.0),
        size: Size { width, height },
        padding,
        gap,
        ..Default::default()
    }
}

fn dimension_from_value(value: Option<&DimensionValue>) -> Dimension {
    match value {
        Some(DimensionValue::Length(v)) => Dimension::length(*v),
        Some(DimensionValue::Percent(v)) => Dimension::percent(*v),
        None => Dimension::auto(),
    }
}

pub fn hit_test(layout_tree: &LayoutTree, x: f32, y: f32) -> Option<u32> {
    hit_test_node(&layout_tree.taffy, layout_tree.root, x, y, 0.0, 0.0)
}

fn hit_test_node(
    taffy: &TaffyTree<NodeContext>,
    taffy_node: NodeId,
    target_x: f32,
    target_y: f32,
    parent_x: f32,
    parent_y: f32,
) -> Option<u32> {
    let layout = taffy.layout(taffy_node).unwrap();
    let abs_x = parent_x + layout.location.x;
    let abs_y = parent_y + layout.location.y;
    let w = layout.size.width;
    let h = layout.size.height;

    if target_x < abs_x || target_x >= abs_x + w || target_y < abs_y || target_y >= abs_y + h {
        return None;
    }

    // Check children in reverse order (last drawn = frontmost)
    if let Ok(children) = taffy.children(taffy_node) {
        for &child_id in children.iter().rev() {
            if let Some(hit) = hit_test_node(taffy, child_id, target_x, target_y, abs_x, abs_y) {
                return Some(hit);
            }
        }
    }

    // Return this node's JS ID if it's a container with one
    if let Some(NodeContext::Container { node_id: Some(js_id), .. }) = taffy.get_node_context(taffy_node) {
        Some(*js_id)
    } else {
        None
    }
}

pub fn compute_layout(
    layout_tree: &mut LayoutTree,
    fonts: &std::collections::HashMap<String, Font>,
    width: f32,
    height: f32,
) {
    let available = Size {
        width: AvailableSpace::Definite(width),
        height: AvailableSpace::Definite(height),
    };

    layout_tree
        .taffy
        .compute_layout_with_measure(
            layout_tree.root,
            available,
            |known_size, _available_space, _node_id, context, _style| {
                if let Some(NodeContext::Text { content, font_name, font_size, .. }) = context {
                    let fs = *font_size;
                    if let Some(font) = fonts.get(font_name) {
                        let w = known_size.width.unwrap_or_else(|| {
                            content
                                .chars()
                                .map(|c| font.metrics(c, fs).advance_width)
                                .sum()
                        });
                        let h = known_size.height.unwrap_or_else(|| {
                            font.horizontal_line_metrics(fs)
                                .map(|m| m.ascent - m.descent + m.line_gap)
                                .unwrap_or(fs)
                        });
                        Size { width: w, height: h }
                    } else {
                        Size::ZERO
                    }
                } else {
                    Size::ZERO
                }
            },
        )
        .unwrap();
}
