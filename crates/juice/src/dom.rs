use fontdue::Font;
use fontdue::layout::{CoordinateSystem, Layout as TextLayout, LayoutSettings, TextStyle};
use serde::Deserialize;
use std::collections::HashMap;
use taffy::{
    AlignContent, AlignItems, AlignSelf, AvailableSpace, Dimension, Display, FlexDirection, Layout,
    LengthPercentage, LengthPercentageAuto, NodeId, Rect, Size, Style, TaffyTree,
};

use crate::{
    canvas::RgbColor,
    inherited_style::{InheritedStyle, TextAlign},
};

pub struct Dom {
    pub root_node_id: NodeId,
    pub root_id: Option<u32>,
    pub tree: TaffyTree<NodeContext>,
}

impl Dom {
    pub fn new(
        content_json: &str,
        mut inherited_style: InheritedStyle,
        fonts: &HashMap<String, Font>,
        width: f32,
        height: f32,
    ) -> Result<Self, serde_json::Error> {
        let content: Node = serde_json::from_str(content_json)?;

        let root_id = if let Node::Element { id, .. } = content {
            id
        } else {
            None
        };

        let mut tree = TaffyTree::new();
        let root = build_node(&mut tree, &content, &mut inherited_style);

        tree.compute_layout_with_measure(
            root,
            Size {
                width: AvailableSpace::Definite(width),
                height: AvailableSpace::Definite(height),
            },
            |known_size, available_space, _node_id, context, _style| {
                if let Some(NodeContext::Text {
                    content,
                    font_name,
                    font_size,
                    wrap_width,
                    ..
                }) = context
                {
                    let fs = *font_size;

                    if let Some(font) = fonts.get(font_name) {
                        // Use advance widths for accurate measurement (not glyph bitmap extents)
                        let single_line_width: f32 = content
                            .chars()
                            .map(|c| font.metrics(c, fs).advance_width)
                            .sum();

                        let line_height = font
                            .horizontal_line_metrics(fs)
                            .map(|m| m.ascent - m.descent + m.line_gap)
                            .unwrap_or(fs);

                        let constraint = known_size.width.or(match available_space.width {
                            AvailableSpace::Definite(w) => Some(w),
                            _ => None,
                        });

                        // Only wrap if the text actually overflows the constraint
                        let needs_wrap = constraint.map_or(false, |c| single_line_width > c);

                        if needs_wrap {
                            let max_w = constraint.unwrap();

                            let mut text_layout = TextLayout::new(CoordinateSystem::PositiveYDown);

                            text_layout.reset(&LayoutSettings {
                                max_width: Some(max_w),
                                ..LayoutSettings::default()
                            });

                            text_layout.append(
                                std::slice::from_ref(font),
                                &TextStyle::new(content, fs, 0),
                            );

                            let glyphs = text_layout.glyphs();

                            let h = known_size.height.unwrap_or_else(|| {
                                if glyphs.is_empty() {
                                    line_height
                                } else {
                                    let last_line_y =
                                        glyphs.iter().map(|g| g.y).fold(0.0f32, f32::max);
                                    last_line_y + line_height
                                }
                            });

                            *wrap_width = Some(max_w);

                            Size {
                                width: known_size.width.unwrap_or(max_w),
                                height: h,
                            }
                        } else {
                            *wrap_width = None;
                            Size {
                                width: known_size.width.unwrap_or(single_line_width),
                                height: known_size.height.unwrap_or(line_height),
                            }
                        }
                    } else {
                        Size::ZERO
                    }
                } else {
                    Size::ZERO
                }
            },
        )
        .unwrap();

        Ok(Self {
            root_node_id: root,
            root_id,
            tree,
        })
    }

    pub fn node_at_point(&self, x: f32, y: f32) -> Option<u32> {
        self._node_at_point(self.root_node_id, x, y, 0.0, 0.0)
    }

    pub fn get_layout(&self, node_id: NodeId) -> Option<&Layout> {
        self.tree.layout(node_id).ok()
    }

    pub fn get_context(&self, node_id: NodeId) -> Option<&NodeContext> {
        self.tree.get_node_context(node_id)
    }

    pub fn get_children(&self, node_id: NodeId) -> Option<Vec<NodeId>> {
        self.tree.children(node_id).ok()
    }

    fn _node_at_point(
        &self,
        node_id: NodeId,
        x: f32,
        y: f32,
        parent_x: f32,
        parent_y: f32,
    ) -> Option<u32> {
        let layout = self.tree.layout(node_id).ok()?;

        // get the absolute position of the current node
        let node_x = parent_x + layout.location.x;
        let node_y = parent_y + layout.location.y;

        let Size { width, height } = layout.size;

        // check if x,y is inside the current node
        if x < node_x || x >= node_x + width || y < node_y || y >= node_y + height {
            return None;
        }

        // check the children in reverse order (last drawn is foremost)
        if let Ok(children) = self.tree.children(node_id) {
            for &child_id in children.iter().rev() {
                if let Some(id) = self._node_at_point(child_id, x, y, node_x, node_y) {
                    return Some(id);
                }
            }
        }

        // none of the children were hit, which means we can say we were hit
        // it's only relevant if we're a non-text node with an ID on the JavaScript side though
        match self.tree.get_node_context(node_id) {
            Some(NodeContext::Element {
                js_id: Some(js_id), ..
            }) => Some(*js_id),
            Some(NodeContext::Svg {
                js_id: Some(js_id), ..
            }) => Some(*js_id),
            Some(NodeContext::Image {
                js_id: Some(js_id), ..
            }) => Some(*js_id),
            _ => None,
        }
    }
}

fn build_node(
    tree: &mut TaffyTree<NodeContext>,
    node: &Node,
    inherited_style: &mut InheritedStyle,
) -> NodeId {
    match node {
        Node::Text { text } => {
            let context = NodeContext::Text {
                content: text.clone(),
                color: inherited_style.color,
                font_name: inherited_style.font_name.clone(),
                font_size: inherited_style.font_size,
                text_align: inherited_style.text_align,
                wrap_width: None,
            };

            tree.new_leaf_with_context(
                Style {
                    flex_grow: 1.0,
                    min_size: Size {
                        width: Dimension::length(0.0),
                        height: Dimension::length(0.0),
                    },
                    ..Style::default()
                },
                context,
            )
            .unwrap()
        }

        Node::Svg {
            id,
            markup,
            width,
            height,
        } => {
            let context = NodeContext::Svg {
                markup: markup.clone(),
                js_id: *id,
                inherited_color: inherited_style.color,
            };

            let style = Style {
                size: Size {
                    width: parse_dimension(width, inherited_style.font_size),
                    height: parse_dimension(height, inherited_style.font_size),
                },
                ..Default::default()
            };

            tree.new_leaf_with_context(style, context).unwrap()
        }

        Node::Image {
            id,
            src,
            width,
            height,
        } => {
            // Decode base64 data URL: "data:image/png;base64,..."
            let (data, img_width, img_height) = if let Some(base64_data) =
                src.split(',').nth(1).and_then(|s| {
                    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s).ok()
                }) {
                match image::load_from_memory(&base64_data) {
                    Ok(img) => {
                        let rgba = img.to_rgba8();
                        (rgba.to_vec(), rgba.width(), rgba.height())
                    }
                    Err(err) => {
                        println!("Error loading image: {:?}", err);
                        (vec![], 0, 0)
                    }
                }
            } else {
                (vec![], 0, 0)
            };

            let context = NodeContext::Image {
                data,
                img_width,
                img_height,
                js_id: *id,
            };

            let style = Style {
                size: Size {
                    width: parse_dimension(width, inherited_style.font_size),
                    height: parse_dimension(height, inherited_style.font_size),
                },
                ..Default::default()
            };

            tree.new_leaf_with_context(style, context).unwrap()
        }

        Node::Element {
            align_items,
            align_self,
            background,
            border_radius,
            children,
            color,
            flex_direction,
            flex_grow,
            flex_shrink,
            font,
            font_size,
            gap,
            height,
            id,
            justify_content,
            justify_self,
            margin,
            padding,
            text_align,
            width,
        } => {
            let text_align = match text_align.as_deref() {
                Some("center") => Some(TextAlign::Center),
                Some("right") => Some(TextAlign::Right),
                Some("left") => Some(TextAlign::Left),
                _ => None,
            };

            let mut child_style = inherited_style.clone_and_override(
                color.map(RgbColor::from_array),
                font.clone(),
                *font_size,
                text_align,
            );

            let child_ids: Vec<NodeId> = children
                .iter()
                .map(|child| build_node(tree, child, &mut child_style))
                .collect();

            let gap = gap.unwrap_or(0.0);

            let style = Style {
                display: Display::Flex,
                align_items: match align_items.as_deref() {
                    Some("flex-start") => Some(AlignItems::FlexStart),
                    Some("center") => Some(AlignItems::Center),
                    Some("flex-end") => Some(AlignItems::FlexEnd),
                    Some("stretch") => Some(AlignItems::Stretch),
                    _ => None,
                },
                align_self: match align_self.as_deref() {
                    Some("flex-start") => Some(AlignSelf::FlexStart),
                    Some("center") => Some(AlignSelf::Center),
                    Some("flex-end") => Some(AlignSelf::FlexEnd),
                    Some("stretch") => Some(AlignSelf::Stretch),
                    _ => None,
                },
                flex_direction: match flex_direction.as_deref() {
                    Some("column") => FlexDirection::Column,
                    _ => FlexDirection::Row,
                },
                flex_grow: flex_grow.unwrap_or(0.0),
                flex_shrink: flex_shrink.unwrap_or(1.0),
                justify_content: match justify_content.as_deref() {
                    Some("flex-start") => Some(AlignContent::FlexStart),
                    Some("center") => Some(AlignContent::Center),
                    Some("flex-end") => Some(AlignContent::FlexEnd),
                    Some("stretch") => Some(AlignContent::Stretch),
                    Some("space-between") => Some(AlignContent::SpaceBetween),
                    Some("space-around") => Some(AlignContent::SpaceAround),
                    _ => None,
                },
                justify_self: match justify_self.as_deref() {
                    Some("flex-start") => Some(AlignSelf::FlexStart),
                    Some("center") => Some(AlignSelf::Center),
                    Some("flex-end") => Some(AlignSelf::FlexEnd),
                    Some("stretch") => Some(AlignSelf::Stretch),
                    _ => None,
                },
                size: Size {
                    width: parse_dimension(width, inherited_style.font_size),
                    height: parse_dimension(height, inherited_style.font_size),
                },
                padding: match padding {
                    Some([top, right, bottom, left]) => Rect {
                        top: LengthPercentage::length(*top),
                        right: LengthPercentage::length(*right),
                        bottom: LengthPercentage::length(*bottom),
                        left: LengthPercentage::length(*left),
                    },
                    None => Rect::zero(),
                },
                margin: match margin {
                    Some([top, right, bottom, left]) => Rect {
                        top: LengthPercentageAuto::length(*top),
                        right: LengthPercentageAuto::length(*right),
                        bottom: LengthPercentageAuto::length(*bottom),
                        left: LengthPercentageAuto::length(*left),
                    },
                    None => Rect::zero(),
                },
                gap: Size {
                    width: LengthPercentage::length(gap),
                    height: LengthPercentage::length(gap),
                },
                ..Default::default()
            };

            let taffy_node = tree.new_with_children(style, &child_ids).unwrap();

            tree.set_node_context(
                taffy_node,
                Some(NodeContext::Element {
                    background: background.map(RgbColor::from_array),
                    border_radius: border_radius.unwrap_or(0.0),
                    js_id: *id,
                }),
            )
            .unwrap();

            taffy_node
        }
    }
}

fn parse_dimension(s: &str, font_height: f32) -> Dimension {
    if let Some(n) = s.strip_suffix("px") {
        n.parse::<f32>()
            .map(Dimension::length)
            .unwrap_or_else(|_| Dimension::auto())
    } else if let Some(n) = s.strip_suffix("%") {
        n.parse::<f32>()
            .map(|v| Dimension::percent(v / 100.0))
            .unwrap_or_else(|_| Dimension::auto())
    } else if let Some(n) = s.strip_suffix("em") {
        n.parse::<f32>()
            .map(|v| Dimension::length(v * font_height))
            .unwrap_or_else(|_| Dimension::auto())
    } else {
        Dimension::auto()
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Node {
    #[serde(rename = "element")]
    Element {
        #[serde(default)]
        id: Option<u32>,
        #[serde(default, rename = "alignItems")]
        align_items: Option<String>,
        #[serde(default, rename = "alignSelf")]
        align_self: Option<String>,
        #[serde(default)]
        background: Option<[u8; 3]>,
        #[serde(default, rename = "borderRadius")]
        border_radius: Option<f32>,
        #[serde(default)]
        color: Option<[u8; 3]>,
        #[serde(default)]
        font: Option<String>,
        #[serde(default, rename = "fontSize")]
        font_size: Option<f32>,
        #[serde(default, rename = "flexDirection")]
        flex_direction: Option<String>,
        #[serde(default, rename = "flexGrow")]
        flex_grow: Option<f32>,
        #[serde(default, rename = "flexShrink")]
        flex_shrink: Option<f32>,
        #[serde(default, rename = "justifyContent")]
        justify_content: Option<String>,
        #[serde(default, rename = "justifySelf")]
        justify_self: Option<String>,
        #[serde(default)]
        width: String,
        #[serde(default)]
        height: String,
        #[serde(default)]
        padding: Option<[f32; 4]>,
        #[serde(default)]
        margin: Option<[f32; 4]>,
        #[serde(default)]
        gap: Option<f32>,
        #[serde(default, rename = "textAlign")]
        text_align: Option<String>,
        #[serde(default)]
        children: Vec<Node>,
    },
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "svg")]
    Svg {
        #[serde(default)]
        id: Option<u32>,
        markup: String,
        #[serde(default)]
        width: String,
        #[serde(default)]
        height: String,
    },
    #[serde(rename = "image")]
    Image {
        #[serde(default)]
        id: Option<u32>,
        #[serde(default)]
        src: String,
        #[serde(default)]
        width: String,
        #[serde(default)]
        height: String,
    },
}

#[derive(Debug, Clone)]
pub enum NodeContext {
    Element {
        background: Option<RgbColor>,
        border_radius: f32,
        js_id: Option<u32>,
    },
    Text {
        content: String,
        color: RgbColor,
        font_name: String,
        font_size: f32,
        text_align: TextAlign,
        wrap_width: Option<f32>,
    },
    Svg {
        markup: String,
        js_id: Option<u32>,
        inherited_color: RgbColor,
    },
    Image {
        data: Vec<u8>,
        img_width: u32,
        img_height: u32,
        js_id: Option<u32>,
    },
}
