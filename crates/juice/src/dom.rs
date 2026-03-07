use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use fontdue::Font;
use fontdue::layout::{CoordinateSystem, Layout as TextLayout, LayoutSettings, TextStyle};
use rquickjs::function::{Func, MutFn};
use rquickjs::{Ctx, IntoJs, Object, Value};
use taffy::{
    AlignContent, AlignItems, AvailableSpace, BoxSizing, Dimension, Display, FlexDirection,
    FlexWrap, Layout, LengthPercentage, LengthPercentageAuto, NodeId, Overflow, Position, Size,
    Style, TaffyTree,
};

use crate::{
    canvas::RgbColor,
    engine::JsModule,
    inherited_style::{InheritedStyle, InheritedStyleOverrides, TextAlign},
};

pub struct CachedRaster {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub struct NodeContext {
    pub kind: NodeKind,
    pub resolved_style: InheritedStyle,
    pub overrides: InheritedStyleOverrides,
    pub render_dirty: bool,
    pub cached_raster: Option<CachedRaster>,
}

pub enum NodeKind {
    Element {
        tag: String,
        background: Option<RgbColor>,
        border_radius: f32,
    },
    Text {
        text: String,
        wrap_width: Option<f32>,
    },
    Svg {
        width: Dimension,
        height: Dimension,
        markup: String,
    },
    Image {
        width: Dimension,
        height: Dimension,
        src: String,
        data: Vec<u8>,
        img_width: u32,
        img_height: u32,
    },
}

pub struct Dom {
    tree: TaffyTree<NodeContext>,
    inherited_style: InheritedStyle,
    pub root_node_id: Option<NodeId>,
}

impl Dom {
    pub fn new(inherited_style: InheritedStyle) -> Self {
        Self {
            tree: TaffyTree::new(),
            inherited_style,
            root_node_id: None,
        }
    }

    pub fn create_element(&mut self, tag: String) -> u64 {
        let style = Style::default();

        let kind = match tag.as_str() {
            "svg" => NodeKind::Svg {
                width: Dimension::auto(),
                height: Dimension::auto(),
                markup: "".to_string(),
            },
            "img" => NodeKind::Image {
                width: Dimension::auto(),
                height: Dimension::auto(),
                src: "".to_string(),
                data: vec![],
                img_width: 0,
                img_height: 0,
            },
            tag => NodeKind::Element {
                tag: tag.to_string(),
                background: None,
                border_radius: 0.0,
            },
        };

        let node_id = self
            .tree
            .new_leaf_with_context(
                style,
                NodeContext {
                    kind,
                    resolved_style: self.inherited_style.clone(),
                    overrides: InheritedStyleOverrides::default(),

                    render_dirty: true,
                    cached_raster: None,
                },
            )
            .unwrap();

        if tag == "document" {
            self.root_node_id = Some(node_id);
        }

        u64::from(node_id)
    }

    pub fn create_text_node(&mut self, text: String) -> u64 {
        let style = Style {
            min_size: Size {
                width: Dimension::length(0.0),
                height: Dimension::length(0.0),
            },
            ..Style::default()
        };

        let node_id = self
            .tree
            .new_leaf_with_context(
                style,
                NodeContext {
                    kind: NodeKind::Text {
                        text,
                        wrap_width: None,
                    },
                    resolved_style: self.inherited_style.clone(),
                    overrides: InheritedStyleOverrides::default(),

                    render_dirty: true,
                    cached_raster: None,
                },
            )
            .unwrap();

        u64::from(node_id)
    }

    pub fn append_child(&mut self, parent_id: u64, child_id: u64) -> Result<(), DomError> {
        let parent_id = NodeId::from(parent_id);
        let child_id = NodeId::from(child_id);

        self.tree
            .add_child(parent_id, child_id)
            .map_err(|_| DomError {
                message: "Invalid NodeId".to_string(),
            })?;

        let parent_resolved = self.get_resolved_style(parent_id);
        self.resolve_subtree(&parent_resolved, child_id);
        Ok(())
    }

    pub fn insert_child_at(
        &mut self,
        index: usize,
        parent_id: u64,
        child_id: u64,
    ) -> Result<(), DomError> {
        let parent_id = NodeId::from(parent_id);
        let child_id = NodeId::from(child_id);

        self.tree
            .insert_child_at_index(parent_id, index, child_id)
            .map_err(|_| DomError {
                message: "Invalid NodeId".to_string(),
            })?;

        let parent_resolved = self.get_resolved_style(parent_id);
        self.resolve_subtree(&parent_resolved, child_id);
        Ok(())
    }

    pub fn remove_child(&mut self, parent_id: u64, child_id: u64) -> Result<(), DomError> {
        let parent_id = NodeId::from(parent_id);
        let child_id = NodeId::from(child_id);

        self.tree
            .remove_child(parent_id, child_id)
            .map(|_| ())
            .map_err(|_| DomError {
                message: "Invalid NodeId".to_string(),
            })
    }

    pub fn delete_node(&mut self, node_id: u64) -> Result<(), DomError> {
        let node_id = NodeId::from(node_id);

        self.tree.remove(node_id).map(|_| ()).map_err(|_| DomError {
            message: "Invalid NodeId".to_string(),
        })
    }

    pub fn set_attribute_string(
        &mut self,
        node_id: u64,
        key: String,
        value: String,
    ) -> Result<(), DomError> {
        let node_id = NodeId::from(node_id);
        let mut needs_cascade = false;

        let ctx = self
            .tree
            .get_node_context_mut(node_id)
            .ok_or_else(|| DomError {
                message: "Invalid NodeId".to_string(),
            })?;

        match &mut ctx.kind {
            NodeKind::Element { background, .. } => match key.as_str() {
                "color" => {
                    ctx.overrides.color = RgbColor::from_string(&value);
                    needs_cascade = true;
                }
                "font" => {
                    ctx.overrides.font_name = Some(value);
                    needs_cascade = true;
                }
                "textAlign" => {
                    ctx.overrides.text_align = Some(parse_text_align(&value));
                    needs_cascade = true;
                }
                "background" => {
                    *background = RgbColor::from_string(&value);
                    ctx.render_dirty = true;
                }
                _ => {}
            },
            NodeKind::Text { text, .. } => match key.as_str() {
                "text" => {
                    *text = value;
                    ctx.render_dirty = true;
                    // Text content change affects measurement
                    let _ = self.tree.mark_dirty(node_id);
                }
                _ => {}
            },
            NodeKind::Svg { markup, .. } => match key.as_str() {
                "markup" => {
                    *markup = value;
                    ctx.render_dirty = true;
                }
                "color" => {
                    ctx.overrides.color = RgbColor::from_string(&value);
                    needs_cascade = true;
                }
                "font" => {
                    ctx.overrides.font_name = Some(value);
                    needs_cascade = true;
                }
                "textAlign" => {
                    ctx.overrides.text_align = Some(parse_text_align(&value));
                    needs_cascade = true;
                }
                _ => {}
            },
            NodeKind::Image {
                src,
                data,
                img_width,
                img_height,
                ..
            } => match key.as_str() {
                "src" => {
                    *src = value.clone();
                    ctx.render_dirty = true;
                    // Decode base64 data URL: "data:image/png;base64,..."
                    if let Some(base64_data) = value.split(',').nth(1).and_then(|s| {
                        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s).ok()
                    }) {
                        match image::load_from_memory(&base64_data) {
                            Ok(img) => {
                                let rgba = img.to_rgba8();
                                *img_width = rgba.width();
                                *img_height = rgba.height();
                                *data = rgba.to_vec();
                            }
                            Err(err) => {
                                println!("Error loading image: {:?}", err);
                                *data = vec![];
                                *img_width = 0;
                                *img_height = 0;
                            }
                        }
                    }
                }
                _ => {}
            },
        };

        if needs_cascade {
            self.cascade_resolved_style(node_id);
        }

        Ok(())
    }

    pub fn set_attribute_number(
        &mut self,
        node_id: u64,
        key: String,
        value: f32,
    ) -> Result<(), DomError> {
        let node_id = NodeId::from(node_id);
        let mut needs_cascade = false;

        let ctx = self
            .tree
            .get_node_context_mut(node_id)
            .ok_or_else(|| DomError {
                message: "Invalid NodeId".to_string(),
            })?;

        match &mut ctx.kind {
            NodeKind::Element { border_radius, .. } => match key.as_str() {
                "fontSize" => {
                    ctx.overrides.font_size = Some(value);
                    needs_cascade = true;
                }
                "borderRadius" => {
                    *border_radius = value;
                    ctx.render_dirty = true;
                }
                _ => {}
            },
            _ => {}
        };

        if needs_cascade {
            self.cascade_resolved_style(node_id);
        }

        Ok(())
    }

    pub fn set_style_string(
        &mut self,
        node_id: u64,
        key: String,
        value: String,
    ) -> Result<(), DomError> {
        let node_id = NodeId::from(node_id);

        let style = self.tree.style(node_id).map_err(|_| DomError {
            message: "Could not update style".to_string(),
        })?;

        let mut style = style.clone();

        if value == "auto" {
            match key.as_str() {
                "flexBasis" => style.flex_basis = Dimension::auto(),
                "width" => style.size.width = Dimension::auto(),
                "height" => style.size.height = Dimension::auto(),
                "marginTop" => style.margin.top = LengthPercentageAuto::auto(),
                "marginRight" => style.margin.right = LengthPercentageAuto::auto(),
                "marginBottom" => style.margin.bottom = LengthPercentageAuto::auto(),
                "marginLeft" => style.margin.left = LengthPercentageAuto::auto(),
                _ => {}
            }
        } else {
            match key.as_str() {
                "alignContent" => style.align_content = parse_align_content(&value),
                "alignItems" => style.align_items = parse_align_items(&value),
                "alignSelf" => style.align_self = parse_align_items(&value),
                "boxSizing" => style.box_sizing = parse_box_sizing(&value),
                "display" => style.display = parse_display(&value),
                "flexDirection" => style.flex_direction = parse_flex_direction(&value),
                "flexWrap" => style.flex_wrap = parse_flex_wrap(&value),
                "justifyContent" => style.justify_content = parse_align_content(&value),
                "justifyItems" => style.justify_items = parse_align_items(&value),
                "justifySelf" => style.justify_self = parse_align_items(&value),
                "overflowX" => style.overflow.x = parse_overflow(&value),
                "overflowY" => style.overflow.y = parse_overflow(&value),
                "position" => style.position = parse_position(&value),
                _ => {}
            }
        }

        self.tree.set_style(node_id, style).map_err(|_| DomError {
            message: "Could not update style".to_string(),
        })
    }

    pub fn set_style_number(
        &mut self,
        node_id: u64,
        key: String,
        value: f32,
    ) -> Result<(), DomError> {
        let node_id = NodeId::from(node_id);

        // Handle non-layout style properties stored on the NodeContext
        if key == "borderRadius" {
            if let Some(ctx) = self.tree.get_node_context_mut(node_id) {
                if let NodeKind::Element { border_radius, .. } = &mut ctx.kind {
                    *border_radius = value;
                    ctx.render_dirty = true;
                }
            }
            return Ok(());
        }

        let style = self.tree.style(node_id).map_err(|_| DomError {
            message: "Could not update style".to_string(),
        })?;

        let mut style = style.clone();

        match key.as_str() {
            "flexBasis" => style.flex_basis = Dimension::length(value),
            "flexGrow" => style.flex_grow = value,
            "flexShrink" => style.flex_shrink = value,
            "gapHeight" => style.gap.height = LengthPercentage::length(value),
            "gapWidth" => style.gap.width = LengthPercentage::length(value),
            "height" => style.size.height = Dimension::length(value),
            "marginBottom" => style.margin.bottom = LengthPercentageAuto::length(value),
            "marginLeft" => style.margin.left = LengthPercentageAuto::length(value),
            "marginRight" => style.margin.right = LengthPercentageAuto::length(value),
            "marginTop" => style.margin.top = LengthPercentageAuto::length(value),
            "maxHeight" => style.max_size.height = Dimension::length(value),
            "maxWidth" => style.max_size.width = Dimension::length(value),
            "paddingBottom" => style.padding.bottom = LengthPercentage::length(value),
            "paddingLeft" => style.padding.left = LengthPercentage::length(value),
            "paddingRight" => style.padding.right = LengthPercentage::length(value),
            "paddingTop" => style.padding.top = LengthPercentage::length(value),
            "width" => style.size.width = Dimension::length(value),
            _ => {}
        };

        self.tree.set_style(node_id, style).map_err(|_| DomError {
            message: "Could not update style".to_string(),
        })
    }

    pub fn set_style_percent(
        &mut self,
        node_id: u64,
        key: String,
        value: f32,
    ) -> Result<(), DomError> {
        let node_id = NodeId::from(node_id);

        let style = self.tree.style(node_id).map_err(|_| DomError {
            message: "Could not update style".to_string(),
        })?;

        let mut style = style.clone();
        let fraction = value / 100.0;

        match key.as_str() {
            "flexBasis" => style.flex_basis = Dimension::percent(fraction),
            "gapHeight" => style.gap.height = LengthPercentage::percent(fraction),
            "gapWidth" => style.gap.width = LengthPercentage::percent(fraction),
            "height" => style.size.height = Dimension::percent(fraction),
            "marginBottom" => style.margin.bottom = LengthPercentageAuto::percent(fraction),
            "marginLeft" => style.margin.left = LengthPercentageAuto::percent(fraction),
            "marginRight" => style.margin.right = LengthPercentageAuto::percent(fraction),
            "marginTop" => style.margin.top = LengthPercentageAuto::percent(fraction),
            "maxHeight" => style.max_size.height = Dimension::percent(fraction),
            "maxWidth" => style.max_size.width = Dimension::percent(fraction),
            "paddingBottom" => style.padding.bottom = LengthPercentage::percent(fraction),
            "paddingLeft" => style.padding.left = LengthPercentage::percent(fraction),
            "paddingRight" => style.padding.right = LengthPercentage::percent(fraction),
            "paddingTop" => style.padding.top = LengthPercentage::percent(fraction),
            "width" => style.size.width = Dimension::percent(fraction),
            _ => {}
        }

        self.tree.set_style(node_id, style).map_err(|_| DomError {
            message: "Could not update style".to_string(),
        })
    }

    pub fn set_style_em(&mut self, node_id: u64, key: String, value: f32) -> Result<(), DomError> {
        let node_id = NodeId::from(node_id);

        let style = self.tree.style(node_id).map_err(|_| DomError {
            message: "Could not update style".to_string(),
        })?;

        let ctx = self
            .tree
            .get_node_context(node_id)
            .ok_or_else(|| DomError {
                message: "Invalid NodeId".to_string(),
            })?;

        let inherited_style = ctx.resolved_style.with_overrides(&ctx.overrides);

        let mut style = style.clone();
        let length = value * inherited_style.font_size;

        match key.as_str() {
            "flexBasis" => style.flex_basis = Dimension::length(length),
            "gapHeight" => style.gap.height = LengthPercentage::length(length),
            "gapWidth" => style.gap.width = LengthPercentage::length(length),
            "height" => style.size.height = Dimension::length(length),
            "marginBottom" => style.margin.bottom = LengthPercentageAuto::length(length),
            "marginLeft" => style.margin.left = LengthPercentageAuto::length(length),
            "marginRight" => style.margin.right = LengthPercentageAuto::length(length),
            "marginTop" => style.margin.top = LengthPercentageAuto::length(length),
            "maxHeight" => style.max_size.height = Dimension::length(length),
            "maxWidth" => style.max_size.width = Dimension::length(length),
            "paddingBottom" => style.padding.bottom = LengthPercentage::length(length),
            "paddingLeft" => style.padding.left = LengthPercentage::length(length),
            "paddingRight" => style.padding.right = LengthPercentage::length(length),
            "paddingTop" => style.padding.top = LengthPercentage::length(length),
            "width" => style.size.width = Dimension::length(length),
            _ => {}
        }

        self.tree.set_style(node_id, style).map_err(|_| DomError {
            message: "Could not update style".to_string(),
        })
    }

    pub fn compute_layout(&mut self, fonts: &HashMap<String, Font>, width: f32, height: f32) {
        let Some(root) = self.root_node_id else {
            return;
        };

        self.tree
            .compute_layout_with_measure(
                root,
                Size {
                    width: AvailableSpace::Definite(width),
                    height: AvailableSpace::Definite(height),
                },
                |known_size, available_space, _node_id, context, _style| {
                    if let Some(NodeContext {
                        kind: NodeKind::Text { text, wrap_width },
                        resolved_style,
                        ..
                    }) = context
                    {
                        let fs = resolved_style.font_size;

                        if let Some(font) = fonts.get(&resolved_style.font_name) {
                            let single_line_width: f32 = text
                                .chars()
                                .map(|c| font.metrics(c, fs).advance_width)
                                .sum();

                            let line_height = font
                                .horizontal_line_metrics(fs)
                                .map(|m| m.ascent - m.descent + m.line_gap)
                                .unwrap_or(fs);

                            // Determine width following the canonical Taffy pattern:
                            // known_size is a hard constraint, available_space is
                            // clamped between min-content and max-content.
                            let width =
                                known_size
                                    .width
                                    .unwrap_or_else(|| match available_space.width {
                                        AvailableSpace::MinContent => single_line_width,
                                        AvailableSpace::MaxContent => single_line_width,
                                        AvailableSpace::Definite(w) => w.min(single_line_width),
                                    });

                            if single_line_width > width + 1.0 {
                                let mut text_layout =
                                    TextLayout::new(CoordinateSystem::PositiveYDown);
                                text_layout.reset(&LayoutSettings {
                                    max_width: Some(width),
                                    ..LayoutSettings::default()
                                });
                                text_layout.append(
                                    std::slice::from_ref(font),
                                    &TextStyle::new(text, fs, 0),
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
                                *wrap_width = Some(width);
                                Size { width, height: h }
                            } else {
                                *wrap_width = None;
                                Size {
                                    width,
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
    }

    pub fn get_layout(&self, node_id: NodeId) -> Option<&Layout> {
        self.tree.layout(node_id).ok()
    }

    pub fn get_node(&self, node_id: NodeId) -> Option<&NodeContext> {
        self.tree.get_node_context(node_id)
    }

    pub fn get_node_mut(&mut self, node_id: NodeId) -> Option<&mut NodeContext> {
        self.tree.get_node_context_mut(node_id)
    }

    pub fn get_children(&self, node_id: NodeId) -> Option<Vec<NodeId>> {
        self.tree.children(node_id).ok()
    }

    pub fn node_at_point(&self, x: f32, y: f32) -> Option<u64> {
        let root = self.root_node_id?;
        self._node_at_point(root, x, y, 0.0, 0.0)
    }

    fn _node_at_point(
        &self,
        node_id: NodeId,
        x: f32,
        y: f32,
        parent_x: f32,
        parent_y: f32,
    ) -> Option<u64> {
        let layout = self.tree.layout(node_id).ok()?;

        let node_x = parent_x + layout.location.x;
        let node_y = parent_y + layout.location.y;
        let Size { width, height } = layout.size;

        if x < node_x || x >= node_x + width || y < node_y || y >= node_y + height {
            return None;
        }

        // Check children in reverse order (last drawn = foremost)
        if let Ok(children) = self.tree.children(node_id) {
            for &child_id in children.iter().rev() {
                if let Some(id) = self._node_at_point(child_id, x, y, node_x, node_y) {
                    return Some(id);
                }
            }
        }

        Some(u64::from(node_id))
    }

    /// Recompute an element's resolved_style from its parent and cascade to children.
    fn cascade_resolved_style(&mut self, node_id: NodeId) {
        let parent_resolved = self
            .tree
            .parent(node_id)
            .map(|pid| self.get_resolved_style(pid))
            .unwrap_or_else(|| self.inherited_style.clone());

        self.resolve_subtree(&parent_resolved, node_id);
    }

    /// Get the resolved inherited style for a node, falling back to the global default.
    fn get_resolved_style(&self, node_id: NodeId) -> InheritedStyle {
        match self.tree.get_node_context(node_id) {
            Some(ctx) => ctx.resolved_style.clone(),
            None => self.inherited_style.clone(),
        }
    }

    /// Resolve inherited styles for a subtree rooted at `node_id` given the parent's resolved style.
    fn resolve_subtree(&mut self, parent_resolved: &InheritedStyle, node_id: NodeId) {
        let Some(ctx) = self.tree.get_node_context_mut(node_id) else {
            return;
        };

        let old_font = ctx.resolved_style.font_name.clone();
        let old_size = ctx.resolved_style.font_size;

        ctx.resolved_style = parent_resolved.with_overrides(&ctx.overrides);

        let resolved = ctx.resolved_style.clone();
        let is_text = matches!(ctx.kind, NodeKind::Text { .. });

        // Mark dirty if font properties changed (affects measurement)
        if is_text && (resolved.font_name != old_font || resolved.font_size != old_size) {
            let _ = self.tree.mark_dirty(node_id);
        }

        // Text nodes have no children
        if is_text {
            return;
        }

        if let Ok(children) = self.tree.children(node_id) {
            for child_id in children {
                self.resolve_subtree(&resolved, child_id);
            }
        }
    }
}

pub struct DomError {
    pub message: String,
}

impl<'js> IntoJs<'js> for DomError {
    fn into_js(self, ctx: &Ctx<'js>) -> rquickjs::Result<Value<'js>> {
        let error = Object::new(ctx.clone())?;
        error.set("message", self.message.into_js(ctx))?;
        error.into_js(ctx)
    }
}

fn parse_display(str: &str) -> Display {
    match str {
        "block" => Display::Block,
        "flex" => Display::Flex,
        "grid" => Display::Grid,
        "none" => Display::None,
        _ => Display::Block,
    }
}

fn parse_align_items(str: &str) -> Option<AlignItems> {
    let result = match str {
        "baseline" => AlignItems::Baseline,
        "center" => AlignItems::Center,
        "end" => AlignItems::End,
        "flex-end" => AlignItems::FlexEnd,
        "flex-start" => AlignItems::FlexStart,
        "start" => AlignItems::Start,
        "stretch" => AlignItems::Stretch,
        _ => return None,
    };

    Some(result)
}

fn parse_align_content(str: &str) -> Option<AlignContent> {
    let result = match str {
        "center" => AlignContent::Center,
        "end" => AlignContent::End,
        "flex-end" => AlignContent::FlexEnd,
        "flex-start" => AlignContent::FlexStart,
        "space-around" => AlignContent::SpaceAround,
        "space-between" => AlignContent::SpaceAround,
        "space-evenly" => AlignContent::SpaceEvenly,
        "start" => AlignContent::Start,
        "stretch" => AlignContent::Stretch,
        _ => return None,
    };

    Some(result)
}

fn parse_box_sizing(str: &str) -> BoxSizing {
    match str {
        "border-box" => BoxSizing::BorderBox,
        "content-box" => BoxSizing::ContentBox,
        _ => BoxSizing::ContentBox,
    }
}

fn parse_flex_wrap(str: &str) -> FlexWrap {
    match str {
        "nowrap" => FlexWrap::NoWrap,
        "wrap" => FlexWrap::Wrap,
        "wrap-reverse" => FlexWrap::WrapReverse,
        _ => FlexWrap::NoWrap,
    }
}

fn parse_flex_direction(str: &str) -> FlexDirection {
    match str {
        "column" => FlexDirection::Column,
        "column-reverse" => FlexDirection::ColumnReverse,
        "row" => FlexDirection::Row,
        "row-reverse" => FlexDirection::RowReverse,
        _ => FlexDirection::Row,
    }
}

fn parse_overflow(str: &str) -> Overflow {
    match str {
        "clip" => Overflow::Clip,
        "hidden" => Overflow::Hidden,
        "scroll" => Overflow::Scroll,
        "visible" => Overflow::Visible,
        _ => Overflow::Visible,
    }
}

fn parse_position(str: &str) -> Position {
    match str {
        "absolute" => Position::Absolute,
        "relative" => Position::Relative,
        _ => Position::Relative,
    }
}

fn parse_text_align(str: &str) -> TextAlign {
    match str {
        "center" => TextAlign::Center,
        "right" => TextAlign::Right,
        _ => TextAlign::Left,
    }
}

impl JsModule for Rc<RefCell<Dom>> {
    fn register(&self, ctx: &Ctx<'_>) {
        let js_dom = Object::new(ctx.clone()).unwrap();

        let dom = self.clone();
        js_dom
            .set(
                "createElement",
                Func::from(MutFn::from(move |tag: String| {
                    dom.borrow_mut().create_element(tag)
                })),
            )
            .unwrap();

        let dom = self.clone();
        js_dom
            .set(
                "createTextNode",
                Func::from(MutFn::from(move |text: String| {
                    dom.borrow_mut().create_text_node(text)
                })),
            )
            .unwrap();

        let dom = self.clone();
        js_dom
            .set(
                "appendChild",
                Func::from(MutFn::from(
                    move |ctx: Ctx<'_>, parent_id: u64, child_id: u64| -> rquickjs::Result<()> {
                        dom.borrow_mut()
                            .append_child(parent_id, child_id)
                            .map_err(|err| ctx.throw(err.into_js(&ctx).unwrap()))
                    },
                )),
            )
            .unwrap();

        let dom = self.clone();
        js_dom
            .set(
                "insertChildAt",
                Func::from(MutFn::from(
                    move |ctx: Ctx<'_>,
                          index: usize,
                          parent_id: u64,
                          child_id: u64|
                          -> rquickjs::Result<()> {
                        dom.borrow_mut()
                            .insert_child_at(index, parent_id, child_id)
                            .map_err(|err| ctx.throw(err.into_js(&ctx).unwrap()))
                    },
                )),
            )
            .unwrap();

        let dom = self.clone();
        js_dom
            .set(
                "removeChild",
                Func::from(MutFn::from(
                    move |ctx: Ctx<'_>, parent_id: u64, child_id: u64| -> rquickjs::Result<()> {
                        dom.borrow_mut()
                            .remove_child(parent_id, child_id)
                            .map_err(|err| ctx.throw(err.into_js(&ctx).unwrap()))
                    },
                )),
            )
            .unwrap();

        let dom = self.clone();
        js_dom
            .set(
                "deleteNode",
                Func::from(MutFn::from(
                    move |ctx: Ctx<'_>, node_id: u64| -> rquickjs::Result<()> {
                        dom.borrow_mut()
                            .delete_node(node_id)
                            .map_err(|err| ctx.throw(err.into_js(&ctx).unwrap()))
                    },
                )),
            )
            .unwrap();

        let dom = self.clone();
        js_dom
            .set(
                "setAttributeString",
                Func::from(MutFn::from(
                    move |ctx: Ctx<'_>,
                          node_id: u64,
                          key: String,
                          value: String|
                          -> rquickjs::Result<()> {
                        dom.borrow_mut()
                            .set_attribute_string(node_id, key, value)
                            .map_err(|err| ctx.throw(err.into_js(&ctx).unwrap()))
                    },
                )),
            )
            .unwrap();

        let dom = self.clone();
        js_dom
            .set(
                "setAttributeNumber",
                Func::from(MutFn::from(
                    move |ctx: Ctx<'_>,
                          node_id: u64,
                          key: String,
                          value: f32|
                          -> rquickjs::Result<()> {
                        dom.borrow_mut()
                            .set_attribute_number(node_id, key, value)
                            .map_err(|err| ctx.throw(err.into_js(&ctx).unwrap()))
                    },
                )),
            )
            .unwrap();

        let dom = self.clone();
        js_dom
            .set(
                "setStyleString",
                Func::from(MutFn::from(
                    move |ctx: Ctx<'_>,
                          node_id: u64,
                          key: String,
                          value: String|
                          -> rquickjs::Result<()> {
                        dom.borrow_mut()
                            .set_style_string(node_id, key, value)
                            .map_err(|err| ctx.throw(err.into_js(&ctx).unwrap()))
                    },
                )),
            )
            .unwrap();

        let dom = self.clone();
        js_dom
            .set(
                "setStyleNumber",
                Func::from(MutFn::from(
                    move |ctx: Ctx<'_>,
                          node_id: u64,
                          key: String,
                          value: f32|
                          -> rquickjs::Result<()> {
                        dom.borrow_mut()
                            .set_style_number(node_id, key, value)
                            .map_err(|err| ctx.throw(err.into_js(&ctx).unwrap()))
                    },
                )),
            )
            .unwrap();
        let dom = self.clone();

        js_dom
            .set(
                "setStyleEm",
                Func::from(MutFn::from(
                    move |ctx: Ctx<'_>,
                          node_id: u64,
                          key: String,
                          value: f32|
                          -> rquickjs::Result<()> {
                        dom.borrow_mut()
                            .set_style_em(node_id, key, value)
                            .map_err(|err| ctx.throw(err.into_js(&ctx).unwrap()))
                    },
                )),
            )
            .unwrap();

        let dom = self.clone();
        js_dom
            .set(
                "setStylePercent",
                Func::from(MutFn::from(
                    move |ctx: Ctx<'_>,
                          node_id: u64,
                          key: String,
                          value: f32|
                          -> rquickjs::Result<()> {
                        dom.borrow_mut()
                            .set_style_percent(node_id, key, value)
                            .map_err(|err| ctx.throw(err.into_js(&ctx).unwrap()))
                    },
                )),
            )
            .unwrap();

        ctx.globals().set("dom", js_dom).unwrap();
    }
}
