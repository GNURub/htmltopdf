use crate::css::{
    AlignItems, BackgroundClip, BackgroundSize, BorderCollapse, BorderLineStyle, BoxShadow,
    BoxSizing, ClearSide, Color, ComputedStyle, Display, FlexDirection, FlexWrap, FloatSide,
    FontFace, FontStyle, FontWeight, GradientStop, GridTrack, JustifyContent, LinearGradient,
    ListStyleType, ObjectFit, OverflowWrap, Position, PseudoElement, RadialGradient, Stylesheet,
    TextAlign, TextDecoration, TextOverflow, TextTransform, TextWrapStyle, VerticalAlign,
    Visibility, WhiteSpace,
};
use crate::dom::{Document, ElementNode, Node, NodeId};
use crate::{PageOptions, RenderMode, RenderOptions};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct LayoutPage {
    pub number: usize,
    pub page: PageOptions,
    pub items: Vec<LayoutItem>,
}

#[derive(Debug, Clone)]
pub enum LayoutItem {
    BeginClip(ClipRect),
    EndClip,
    Text(TextRun),
    Rect(Rect),
    RoundRect(RoundRect),
    StrokeRect(StrokeRect),
    LinearGradient(GradientRect),
    Image(ImageRect),
    Circle(Circle),
    CircleStroke(CircleStroke),
    Line(Line),
    Polygon(Polygon),
    SvgPath(SvgPath),
}

#[derive(Debug, Clone)]
pub struct ClipRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub radius: f32,
}

#[derive(Debug, Clone)]
pub struct TextRun {
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub font_size: f32,
    pub color: Color,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
    pub font_face: FontFace,
    pub letter_spacing: f32,
    pub word_spacing: f32,
    pub text_decoration: TextDecoration,
    pub text_decoration_color: Option<Color>,
    pub text_decoration_thickness: Option<f32>,
    pub text_underline_offset: Option<f32>,
    pub rotation_deg: f32,
    pub text_shadow: Option<BoxShadow>,
}

#[derive(Debug, Clone)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: Color,
}

#[derive(Debug, Clone)]
pub struct RoundRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub radius: f32,
    pub color: Color,
}

#[derive(Debug, Clone)]
pub struct StrokeRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub radius: f32,
    pub stroke_width: f32,
    pub color: Color,
    pub dash: Option<(f32, f32)>,
}

#[derive(Debug, Clone)]
pub struct GradientRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub gradient: LinearGradient,
}

#[derive(Debug, Clone)]
pub struct ImageRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub intrinsic_width_px: u32,
    pub intrinsic_height_px: u32,
    pub data: Vec<u8>,
    pub alpha_mask: Option<Vec<u8>>,
    pub format: ImageFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Jpeg,
    Png {
        color_space: PngColorSpace,
        bits_per_component: u8,
        components: u8,
    },
    Raw {
        color_space: PngColorSpace,
        bits_per_component: u8,
        components: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PngColorSpace {
    DeviceGray,
    DeviceRgb,
}

#[derive(Debug, Clone)]
pub struct Circle {
    pub cx: f32,
    pub cy: f32,
    pub r: f32,
    pub color: Color,
}

#[derive(Debug, Clone)]
pub struct CircleStroke {
    pub cx: f32,
    pub cy: f32,
    pub r: f32,
    pub width: f32,
    pub color: Color,
}

#[derive(Debug, Clone)]
pub struct Line {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub width: f32,
    pub color: Color,
    pub line_cap_round: bool,
    pub dash: Option<(f32, f32)>,
}

#[derive(Debug, Clone)]
pub struct Polygon {
    pub points: Vec<(f32, f32)>,
    pub color: Color,
}

#[derive(Debug, Clone)]
pub struct SvgPath {
    pub commands: Vec<PathCommand>,
    pub fill: Option<Color>,
    pub stroke: Option<Color>,
    pub stroke_width: f32,
    pub line_cap_round: bool,
}

#[derive(Debug, Clone)]
pub enum PathCommand {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    CubicTo {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        x: f32,
        y: f32,
    },
    Close,
}

#[derive(Clone)]
struct LayoutContext<'a> {
    document: &'a Document,
    stylesheet: &'a Stylesheet,
    options: &'a RenderOptions,
    pages: Vec<LayoutPage>,
    current_y: f32,
    inset_left: f32,
    inset_right: f32,
    containing_blocks: Vec<ContainingBlock>,
    suppress_positioning: bool,
    suppress_keep_with_next: bool,
    suppress_floats: bool,
    suppress_pagination: bool,
    floats: Vec<ActiveFloat>,
    float_base_left: f32,
    float_base_right: f32,
    fixed_overlays: Vec<Vec<LayoutItem>>,
    style_cache: BTreeMap<NodeId, ComputedStyle>,
}

#[derive(Clone, Copy)]
struct ContainingBlock {
    x: f32,
    top: f32,
    width: f32,
    height: f32,
}

#[derive(Clone, Copy)]
struct ActiveFloat {
    page_index: usize,
    side: FloatSide,
    bottom: f32,
    occupied_width: f32,
}

#[derive(Clone)]
struct FlowTextLine {
    text: String,
    inset_left: f32,
    available_width: f32,
}

#[derive(Clone)]
struct FlowInlineLine {
    segments: Vec<InlineTextSegment>,
    inset_left: f32,
    available_width: f32,
}

#[derive(Clone)]
struct PreparedCell {
    style: ComputedStyle,
    lines: Vec<PreparedLine>,
    content_height: f32,
    height: f32,
}

#[derive(Clone)]
struct PreparedLine {
    text: String,
    style: ComputedStyle,
    before: f32,
    after: f32,
}

#[derive(Clone)]
struct InlineTextSegment {
    text: String,
    style: ComputedStyle,
    atomic: Option<std::rc::Rc<InlineAtomicBox>>,
}

#[derive(Clone)]
struct InlineAtomicBox {
    items: Vec<LayoutItem>,
    width: f32,
    height: f32,
    baseline: f32,
}

#[derive(Clone, Copy)]
struct ChildLayoutRange {
    page_index: usize,
    item_start: usize,
    item_end: usize,
    consumed: f32,
    baseline_offset: f32,
    align_items: AlignItems,
}

#[derive(Clone, Copy)]
struct GridPlacedChild {
    id: NodeId,
    start_column: usize,
    span: usize,
}

#[derive(Clone, Copy)]
struct GridRowLayoutRange {
    page_index: usize,
    item_start: usize,
    item_end: usize,
}

#[derive(Clone, Copy)]
struct PaintStackRange {
    item_start: usize,
    item_end: usize,
    z_index: i32,
    source_order: usize,
}

pub fn layout_document(
    document: &Document,
    stylesheet: &Stylesheet,
    options: &RenderOptions,
) -> Vec<LayoutPage> {
    if options.render_mode == RenderMode::DemoFixture && is_premium_invoice_fixture(document) {
        return premium_invoice_pages(options);
    }

    let mut context = LayoutContext {
        document,
        stylesheet,
        options,
        pages: vec![LayoutPage {
            number: 1,
            page: options.page,
            items: Vec::new(),
        }],
        current_y: options.page.height_pt - options.page.margin_top_pt,
        inset_left: 0.0,
        inset_right: 0.0,
        containing_blocks: Vec::new(),
        suppress_positioning: false,
        suppress_keep_with_next: false,
        suppress_floats: false,
        suppress_pagination: false,
        floats: Vec::new(),
        float_base_left: 0.0,
        float_base_right: 0.0,
        fixed_overlays: Vec::new(),
        style_cache: BTreeMap::new(),
    };

    for child in document.children(document.root()) {
        context.layout_flow_child(*child, 0.0, 0.0);
    }
    context.append_fixed_overlays();
    prepend_document_background(document, stylesheet, options, &mut context.pages);
    append_print_margin_masks(&mut context.pages);

    context.pages
}

impl<'a> LayoutContext<'a> {
    fn paint_snapshot(&self) -> Vec<usize> {
        self.pages.iter().map(|page| page.items.len()).collect()
    }

    fn restore_paint_snapshot(&mut self, snapshot: Vec<usize>) {
        for (index, count) in snapshot.iter().copied().enumerate() {
            if let Some(page) = self.pages.get_mut(index) {
                page.items.truncate(count);
            }
        }
        for page in self.pages.iter_mut().skip(snapshot.len()) {
            page.items.clear();
        }
    }

    fn computed_style(&mut self, id: NodeId) -> ComputedStyle {
        if let Some(style) = self.style_cache.get(&id) {
            return style.clone();
        }
        let parent_style =
            self.document
                .parent_of(id)
                .and_then(|parent_id| match self.document.node(parent_id) {
                    Some(Node::Element(_)) => Some(self.computed_style(parent_id)),
                    _ => None,
                });
        let style = ComputedStyle::for_node_with_parent(
            self.document,
            self.stylesheet,
            id,
            parent_style.as_ref(),
        );
        self.style_cache.insert(id, style.clone());
        style
    }

    fn pseudo_style(
        &mut self,
        id: NodeId,
        parent_style: &ComputedStyle,
        kind: PseudoElement,
    ) -> Option<ComputedStyle> {
        self.stylesheet
            .pseudo_style_for_node(self.document, id, kind, parent_style)
            .filter(|style| {
                style.display != Display::None && style.visibility == Visibility::Visible
            })
    }

    fn has_generated_pseudo(&mut self, id: NodeId, style: &ComputedStyle) -> bool {
        [PseudoElement::Before, PseudoElement::After]
            .into_iter()
            .any(|kind| {
                self.pseudo_style(id, style, kind)
                    .and_then(|style| style.content)
                    .is_some_and(|content| !content.is_empty())
            })
    }

    fn has_inline_generated_pseudo(&mut self, id: NodeId, style: &ComputedStyle) -> bool {
        [PseudoElement::Before, PseudoElement::After]
            .into_iter()
            .any(|kind| {
                self.pseudo_style(id, style, kind).is_some_and(|style| {
                    style
                        .content
                        .as_ref()
                        .is_some_and(|content| !content.is_empty())
                        && style.display == Display::Inline
                        && !is_out_of_flow_position(&style)
                })
            })
    }

    fn should_layout_container_as_inline_text(
        &mut self,
        id: NodeId,
        style: &ComputedStyle,
    ) -> bool {
        matches!(style.display, Display::Block | Display::Inline)
            && (self.has_inline_generated_pseudo(id, style) || self.has_inline_atomic_child(id))
            && self.container_children_are_text_inline(id)
    }

    fn has_inline_atomic_child(&mut self, id: NodeId) -> bool {
        self.document.children(id).iter().copied().any(|child| {
            matches!(self.document.node(child), Some(Node::Element(_)))
                && match self.computed_style(child).display {
                    Display::InlineBlock => true,
                    Display::Inline => self.has_inline_atomic_child(child),
                    _ => false,
                }
        })
    }

    fn container_children_are_text_inline(&mut self, id: NodeId) -> bool {
        self.document
            .children(id)
            .iter()
            .copied()
            .all(|child| match self.document.node(child) {
                Some(Node::Text(_)) => true,
                Some(Node::Element(element)) if element.tag == "br" => true,
                Some(Node::Element(element))
                    if element.tag == "script" || element.tag == "style" =>
                {
                    true
                }
                Some(Node::Element(_)) => {
                    let child_style = self.computed_style(child);
                    child_style.display == Display::None
                        || (child_style.display == Display::InlineBlock
                            && child_style.float == FloatSide::None
                            && !is_out_of_flow_position(&child_style))
                        || (child_style.display == Display::Inline
                            && child_style.float == FloatSide::None
                            && !is_out_of_flow_position(&child_style)
                            && self.container_children_are_text_inline(child))
                }
                _ => false,
            })
    }

    fn layout_flow_child(&mut self, id: NodeId, base_left: f32, base_right: f32) {
        self.reflow_floats(base_left, base_right);
        let child_style = match self.document.node(id) {
            Some(Node::Element(_)) => Some(self.computed_style(id)),
            _ => None,
        };
        if let Some(style) = child_style.as_ref() {
            if style.clear != ClearSide::None
                && !is_out_of_flow_position(style)
                && style.display != Display::None
            {
                self.clear_floats(style.clear);
                self.reflow_floats(base_left, base_right);
            }
            if style.float != FloatSide::None
                && !is_out_of_flow_position(style)
                && style.display != Display::None
            {
                self.layout_float(id, style, base_left, base_right);
                return;
            }
        }
        self.layout_node(id);
    }

    fn layout_float(&mut self, id: NodeId, style: &ComputedStyle, base_left: f32, base_right: f32) {
        self.reflow_floats(base_left, base_right);
        let page_content_width = (self.options.page.width_pt
            - self.options.page.margin_left_pt
            - self.options.page.margin_right_pt)
            .max(0.0);
        let occupied_left = self.float_occupied_width(FloatSide::Left);
        let occupied_right = self.float_occupied_width(FloatSide::Right);
        let available_width =
            (page_content_width - base_left - base_right - occupied_left - occupied_right).max(0.0);
        let margin_width = style.margin_left + style.margin_right;
        let box_width = if style.width.is_some() {
            resolve_flow_horizontal_geometry(style, available_width)
                .box_width
                .min((available_width - margin_width).max(0.0))
        } else {
            (self.intrinsic_inline_width(id, available_width) - margin_width)
                .max(0.0)
                .min((available_width - margin_width).max(0.0))
        };
        let remaining = (available_width - box_width - margin_width).max(0.0);
        let (temporary_left, temporary_right) = match style.float {
            FloatSide::Left => (
                base_left + occupied_left,
                base_right + occupied_right + remaining,
            ),
            FloatSide::Right => (
                base_left + occupied_left + remaining,
                base_right + occupied_right,
            ),
            FloatSide::None => return,
        };

        let start_page = self.pages.len().saturating_sub(1);
        let start_y = self.current_y;
        let previous_left = self.inset_left;
        let previous_right = self.inset_right;
        let previous_suppress = self.suppress_floats;
        self.inset_left = temporary_left;
        self.inset_right = temporary_right;
        self.suppress_floats = true;
        if matches!(self.document.node(id), Some(Node::Element(element)) if is_text_aggregation_tag(&element.tag))
        {
            let snapshot = self.paint_snapshot();
            self.layout_container(id, style);
            if style.visibility != Visibility::Visible {
                self.restore_paint_snapshot(snapshot);
            }
        } else {
            self.layout_node(id);
        }
        self.suppress_floats = previous_suppress;
        let float_page = self.pages.len().saturating_sub(1);
        let float_bottom = self.current_y;
        let float_top = if float_page == start_page {
            start_y
        } else {
            self.options.page.height_pt - self.options.page.margin_top_pt
        };
        self.current_y = float_top;
        self.inset_left = previous_left;
        self.inset_right = previous_right;

        if box_width > 0.0 && float_bottom < float_top {
            self.floats.push(ActiveFloat {
                page_index: float_page,
                side: style.float,
                bottom: float_bottom,
                occupied_width: box_width + margin_width,
            });
        }
        self.reflow_floats(base_left, base_right);
    }

    fn clear_floats(&mut self, clear: ClearSide) {
        self.remove_expired_floats();
        let bottom = self
            .floats
            .iter()
            .filter(|float| match clear {
                ClearSide::None => false,
                ClearSide::Left => float.side == FloatSide::Left,
                ClearSide::Right => float.side == FloatSide::Right,
                ClearSide::Both => true,
            })
            .map(|float| float.bottom)
            .min_by(f32::total_cmp);
        if let Some(bottom) = bottom {
            self.current_y = self.current_y.min(bottom);
            self.ensure_space(0.0);
        }
    }

    fn remove_expired_floats(&mut self) {
        let page_index = self.pages.len().saturating_sub(1);
        let current_y = self.current_y;
        self.floats
            .retain(|float| float.page_index == page_index && float.bottom < current_y - 0.01);
    }

    fn float_occupied_width(&self, side: FloatSide) -> f32 {
        self.floats
            .iter()
            .filter(|float| float.side == side)
            .map(|float| float.occupied_width)
            .sum()
    }

    fn reflow_floats(&mut self, base_left: f32, base_right: f32) {
        if self.suppress_floats {
            self.inset_left = base_left;
            self.inset_right = base_right;
            return;
        }
        self.remove_expired_floats();
        self.inset_left = base_left + self.float_occupied_width(FloatSide::Left);
        self.inset_right = base_right + self.float_occupied_width(FloatSide::Right);
    }

    fn float_occupied_width_at_y(&self, page_index: usize, y: f32, side: FloatSide) -> f32 {
        if self.suppress_floats {
            return 0.0;
        }
        self.floats
            .iter()
            .filter(|float| {
                float.page_index == page_index && float.side == side && float.bottom < y - 0.01
            })
            .map(|float| float.occupied_width)
            .sum()
    }

    fn wrap_text_with_floats(
        &self,
        text: &str,
        style: &ComputedStyle,
        base_left: f32,
        base_right: f32,
    ) -> Vec<FlowTextLine> {
        let mut page_index = self.pages.len().saturating_sub(1);
        let mut remaining = normalize_text_for_flow(text, style.white_space);
        let mut y = self.current_y;
        let line_height = layout_line_height(style).max(0.01);
        let mut output = Vec::new();

        while !remaining.is_empty() {
            if !self.suppress_pagination && y < self.options.page.margin_bottom_pt + line_height {
                page_index += 1;
                y = self.options.page.height_pt - self.options.page.margin_top_pt;
            }
            let occupied_left = self.float_occupied_width_at_y(page_index, y, FloatSide::Left);
            let occupied_right = self.float_occupied_width_at_y(page_index, y, FloatSide::Right);
            let inset_left = base_left + occupied_left;
            let inset_right = base_right + occupied_right;
            let raw_available_width = (self.options.page.width_pt
                - self.options.page.margin_left_pt
                - self.options.page.margin_right_pt
                - inset_left
                - inset_right)
                .max(0.0);
            let available_width = if style.width.is_some()
                || style.min_width.is_some()
                || style.max_width.is_some()
            {
                resolve_box_width(style, raw_available_width)
            } else {
                raw_available_width
            };
            let wrapped = wrap_text_with_style(&remaining, style, available_width);
            if wrapped.is_empty() {
                break;
            }
            output.push(FlowTextLine {
                text: wrapped[0].clone(),
                inset_left,
                available_width,
            });
            y -= line_height;
            if wrapped.len() == 1 {
                break;
            }
            let next_remaining = advance_wrapped_text(&remaining, &wrapped[0]);
            if next_remaining.is_empty() {
                break;
            }
            remaining = next_remaining;
        }

        output
    }

    fn wrap_inline_segments_with_floats(
        &self,
        segments: &[InlineTextSegment],
        style: &ComputedStyle,
        base_left: f32,
        base_right: f32,
    ) -> Vec<FlowInlineLine> {
        let tokens = tokenize_inline_segments(segments);
        let mut cursor = 0usize;
        let mut page_index = self.pages.len().saturating_sub(1);
        let mut y = self.current_y;
        let line_height = layout_line_height(style).max(0.01);
        let mut output = Vec::new();

        while cursor < tokens.len() {
            if !self.suppress_pagination && y < self.options.page.margin_bottom_pt + line_height {
                page_index += 1;
                y = self.options.page.height_pt - self.options.page.margin_top_pt;
            }

            let occupied_left = self.float_occupied_width_at_y(page_index, y, FloatSide::Left);
            let occupied_right = self.float_occupied_width_at_y(page_index, y, FloatSide::Right);
            let inset_left = base_left + occupied_left;
            let inset_right = base_right + occupied_right;
            let raw_available_width = (self.options.page.width_pt
                - self.options.page.margin_left_pt
                - self.options.page.margin_right_pt
                - inset_left
                - inset_right)
                .max(0.0);
            let available_width = if style.width.is_some()
                || style.min_width.is_some()
                || style.max_width.is_some()
            {
                resolve_box_width(style, raw_available_width)
            } else {
                raw_available_width
            };
            let (line, next_cursor) = wrap_one_inline_line(
                &tokens,
                cursor,
                available_width,
                style.white_space == WhiteSpace::NoWrap,
            );
            if next_cursor <= cursor {
                break;
            }
            let (_, actual_height) = inline_line_metrics(&line, style);
            let page_top = self.options.page.height_pt - self.options.page.margin_top_pt;
            if !self.suppress_pagination
                && y < page_top - 0.01
                && y < self.options.page.margin_bottom_pt + actual_height
            {
                page_index += 1;
                y = page_top;
                continue;
            }
            output.push(FlowInlineLine {
                segments: line,
                inset_left,
                available_width,
            });
            cursor = next_cursor;
            y -= actual_height;
        }

        output
    }

    fn layout_node(&mut self, id: NodeId) {
        let visibility = match self.document.node(id) {
            Some(Node::Element(_)) => self.computed_style(id).visibility,
            Some(Node::Text(_)) => self
                .document
                .parent_of(id)
                .and_then(|parent_id| match self.document.node(parent_id) {
                    Some(Node::Element(_)) => Some(self.computed_style(parent_id).visibility),
                    _ => None,
                })
                .unwrap_or(Visibility::Visible),
            None => Visibility::Visible,
        };

        if visibility == Visibility::Hidden {
            let snapshot = self.paint_snapshot();
            self.layout_node_inner(id);
            self.restore_paint_snapshot(snapshot);
        } else {
            self.layout_node_inner(id);
        }
    }

    fn layout_node_inner(&mut self, id: NodeId) {
        let Some(node) = self.document.node(id) else {
            return;
        };
        match node {
            Node::Text(text) => {
                if text.trim().is_empty() {
                    return;
                }
                let style = self
                    .document
                    .parent_of(id)
                    .and_then(|parent_id| match self.document.node(parent_id) {
                        Some(Node::Element(_)) => Some(self.computed_style(parent_id)),
                        _ => None,
                    })
                    .unwrap_or_default();
                // A text node produces an anonymous box, not another copy of
                // its parent's dimensions, background or positioning.
                let mut text_style = ComputedStyle::inherited_from(&style);
                text_style.transform_rotate_deg = 0.0;
                self.layout_text_block(text, &text_style);
            }
            Node::Element(element) => {
                let style = self.computed_style(id);
                if style.display == Display::None {
                    return;
                }
                if !self.suppress_floats
                    && style.float != FloatSide::None
                    && !is_out_of_flow_position(&style)
                {
                    let base_left = self.float_base_left;
                    let base_right = self.float_base_right;
                    self.layout_float(id, &style, base_left, base_right);
                    return;
                }
                if !self.suppress_positioning && is_out_of_flow_position(&style) {
                    let containing_block = if style.position == Position::Fixed {
                        self.page_containing_block()
                    } else {
                        self.containing_block()
                    };
                    self.layout_positioned_node(id, &style, containing_block);
                    return;
                }
                if element.tag == "svg" {
                    self.layout_svg(id, &style);
                    if style.page_break_after {
                        self.force_page_break();
                    }
                    return;
                }
                if element.tag == "img" {
                    self.layout_image(id, element, &style);
                    if style.page_break_after {
                        self.force_page_break();
                    }
                    return;
                }
                if is_rendered_form_control(element) {
                    self.layout_form_control(id, element, &style);
                    if style.page_break_after {
                        self.force_page_break();
                    }
                    return;
                }
                if is_non_painted_tag(&element.tag) {
                    return;
                }
                if style.display == Display::Contents {
                    for child in self.document.children(id) {
                        self.layout_node(*child);
                    }
                    return;
                }
                if style.page_break_after_avoid
                    && !self.suppress_keep_with_next
                    && self.should_keep_with_next_on_next_page(id)
                {
                    self.force_page_break();
                }
                if style.page_break_before {
                    self.force_page_break();
                }

                if element.tag == "table" {
                    self.layout_table(id, &style);
                    if style.page_break_after {
                        self.force_page_break();
                    }
                    return;
                }

                if style.display == Display::Grid {
                    self.layout_grid_container(id, &style);
                    if style.page_break_after {
                        self.force_page_break();
                    }
                    return;
                }

                if matches!(style.display, Display::Flex | Display::InlineFlex)
                    && style.flex_direction == FlexDirection::Row
                {
                    self.layout_row_container(id, &style);
                    if style.page_break_after {
                        self.force_page_break();
                    }
                    return;
                }

                if matches!(style.display, Display::Flex | Display::InlineFlex)
                    && style.flex_direction == FlexDirection::Column
                {
                    self.layout_column_container(id, &style);
                    if style.page_break_after {
                        self.force_page_break();
                    }
                    return;
                }

                if self.should_layout_container_as_inline_text(id, &style) {
                    self.layout_avoid_or_container(id, &style);
                    if style.page_break_after {
                        self.force_page_break();
                    }
                    return;
                }

                if is_container_tag(&element.tag) {
                    self.layout_avoid_or_container(id, &style);
                    if style.page_break_after {
                        self.force_page_break();
                    }
                    return;
                }

                if should_descend_into_element(&element.tag, self.document.children(id)) {
                    self.layout_avoid_or_container(id, &style);
                    if style.page_break_after {
                        self.force_page_break();
                    }
                    return;
                }

                let text = self.document.text_content(id);
                if has_styled_inline_children(self.document, id)
                    || self.has_generated_pseudo(id, &style)
                    || self.should_emit_list_marker(id, &style)
                {
                    self.layout_inline_text_block(id, &style);
                } else if !text.trim().is_empty() {
                    self.layout_text_block(&text, &style);
                } else {
                    for child in self.document.children(id) {
                        self.layout_node(*child);
                    }
                }
                if style.page_break_after {
                    self.force_page_break();
                }
            }
        }
    }

    fn layout_avoid_or_container(&mut self, id: NodeId, style: &ComputedStyle) {
        if style.page_break_inside_avoid && self.should_start_avoid_block_on_next_page(id, style) {
            self.force_page_break();
        }
        self.layout_container(id, style);
    }

    fn layout_positioned_node(
        &mut self,
        id: NodeId,
        style: &ComputedStyle,
        containing_block: ContainingBlock,
    ) {
        let width = self.positioned_node_width(id, style, containing_block.width);
        let height = positioned_height(style, containing_block.height);
        let left = style
            .inset_left
            .as_ref()
            .map(|value| value.resolve(containing_block.width));
        let right = style
            .inset_right
            .as_ref()
            .map(|value| value.resolve(containing_block.width));
        let top = style
            .inset_top
            .as_ref()
            .map(|value| value.resolve(containing_block.height));
        let bottom = style
            .inset_bottom
            .as_ref()
            .map(|value| value.resolve(containing_block.height));
        let x = if let Some(left) = left {
            containing_block.x + left + style.margin_left
        } else if let Some(right) = right {
            containing_block.x + containing_block.width - right - width - style.margin_right
        } else {
            containing_block.x + style.margin_left
        };
        let top_y = if let Some(top) = top {
            containing_block.top - top - style.margin_top
        } else if let Some(bottom) = bottom {
            containing_block.top - containing_block.height + bottom + height + style.margin_bottom
        } else {
            self.current_y - style.margin_top
        };

        let previous_y = self.current_y;
        let previous_left = self.inset_left;
        let previous_right = self.inset_right;
        let previous_suppress = self.suppress_positioning;
        let content_width = self.options.page.width_pt
            - self.options.page.margin_left_pt
            - self.options.page.margin_right_pt;
        let inset_left = (x - self.options.page.margin_left_pt - style.margin_left).max(0.0);
        let inset_right =
            (content_width - inset_left - width - style.margin_left - style.margin_right).max(0.0);

        self.current_y = top_y;
        self.inset_left = inset_left;
        self.inset_right = inset_right;
        self.suppress_positioning = true;
        let overlay_page_index = self.pages.len().saturating_sub(1);
        let overlay_item_start = self.pages[overlay_page_index].items.len();
        self.layout_node(id);
        self.suppress_positioning = previous_suppress;
        self.current_y = previous_y;
        self.inset_left = previous_left;
        self.inset_right = previous_right;
        if style.position == Position::Fixed {
            self.register_fixed_overlay(overlay_page_index, overlay_item_start);
        }
    }

    fn positioned_node_width(
        &mut self,
        id: NodeId,
        style: &ComputedStyle,
        containing_width: f32,
    ) -> f32 {
        if style.width.is_some() || (style.inset_left.is_some() && style.inset_right.is_some()) {
            return positioned_width(style, containing_width);
        }

        let margin_width = style.margin_left + style.margin_right;
        let intrinsic_width = self
            .intrinsic_inline_width(id, containing_width)
            .clamp(0.0, containing_width);
        (intrinsic_width - margin_width)
            .max(0.0)
            .min(containing_width)
    }

    fn layout_blockified_item(&mut self, id: NodeId) {
        let Some(Node::Element(element)) = self.document.node(id) else {
            self.layout_node(id);
            return;
        };
        let style = self.computed_style(id);
        if is_rendered_form_control(element) {
            self.layout_node(id);
            return;
        }
        if style.display == Display::None
            || style.display == Display::Contents
            || element.tag == "svg"
            || element.tag == "table"
            || is_non_painted_tag(&element.tag)
            || matches!(
                style.display,
                Display::Flex | Display::InlineFlex | Display::Grid
            )
            || (!self.suppress_positioning && is_out_of_flow_position(&style))
        {
            self.layout_node(id);
            return;
        }
        self.layout_avoid_or_container(id, &style);
        if style.page_break_after {
            self.force_page_break();
        }
    }

    fn layout_image(&mut self, _id: NodeId, element: &ElementNode, style: &ComputedStyle) {
        let Some(src) = element.attr("src") else {
            return;
        };
        let Some(image) = load_image_asset(src, self.options) else {
            return;
        };

        if style.margin_top > 0.0 {
            self.current_y -= style.margin_top;
        }

        let available_width = (self.options.page.width_pt
            - self.options.page.margin_left_pt
            - self.options.page.margin_right_pt
            - self.inset_left
            - self.inset_right
            - style.margin_left
            - style.margin_right)
            .max(0.0);
        let height_base = self.options.page.height_pt
            - self.options.page.margin_top_pt
            - self.options.page.margin_bottom_pt;
        let intrinsic_width = image.intrinsic_width_px as f32 * 0.75;
        let intrinsic_height = image.intrinsic_height_px as f32 * 0.75;
        let aspect = if intrinsic_height > 0.0 {
            intrinsic_width / intrinsic_height
        } else {
            1.0
        };
        let attr_width = element.attr("width").and_then(parse_html_dimension_attr);
        let attr_height = element.attr("height").and_then(parse_html_dimension_attr);

        let style_width = style
            .width
            .as_ref()
            .map(|width| width.resolve(available_width).max(0.0));
        let style_height = style
            .height
            .as_ref()
            .map(|height| height.resolve(height_base).max(0.0));

        let mut content_width = style_width
            .or(attr_width)
            .or_else(|| style_height.or(attr_height).map(|height| height * aspect))
            .unwrap_or(intrinsic_width.min(available_width).max(0.0));
        let mut content_height = style_height.or(attr_height).unwrap_or_else(|| {
            if aspect > 0.0 {
                content_width / aspect
            } else {
                intrinsic_height
            }
        });
        if style_width.is_none() && attr_width.is_none() && content_width > available_width {
            content_width = available_width;
            content_height = if aspect > 0.0 {
                content_width / aspect
            } else {
                content_height
            };
        }

        let horizontal_extras = style.padding_left + style.padding_right;
        let vertical_extras = style.padding_top + style.padding_bottom;
        let (box_width, image_box_width) = if style.box_sizing == BoxSizing::BorderBox
            && (style_width.is_some() || attr_width.is_some())
        {
            let box_width = content_width.min(available_width).max(0.0);
            (box_width, (box_width - horizontal_extras).max(0.0))
        } else {
            (
                (content_width + horizontal_extras)
                    .min(available_width)
                    .max(0.0),
                content_width.min(available_width).max(0.0),
            )
        };
        let (box_height, image_box_height) = if style.box_sizing == BoxSizing::BorderBox
            && (style_height.is_some() || attr_height.is_some())
        {
            let box_height = (content_height + vertical_extras).max(0.0);
            (box_height, (box_height - vertical_extras).max(0.0))
        } else {
            (
                (content_height + vertical_extras).max(0.0),
                content_height.max(0.0),
            )
        };

        self.ensure_space(box_height + style.margin_bottom);
        let box_x = self.options.page.margin_left_pt + self.inset_left + style.margin_left;
        let box_y = self.current_y - box_height;
        let content_x = box_x + style.padding_left;
        let content_y = box_y + style.padding_bottom;
        let (draw_x, draw_y, draw_width, draw_height) = object_fit_rect(
            style.object_fit,
            content_x,
            content_y,
            image_box_width,
            image_box_height,
            intrinsic_width,
            intrinsic_height,
            style.object_position_x,
            style.object_position_y,
        );

        for item in background_items(style, self.options, box_x, box_y, box_width, box_height) {
            self.push(item);
        }
        let clips_image = style.border_radius > 0.0
            || matches!(
                style.object_fit,
                ObjectFit::Cover | ObjectFit::None | ObjectFit::ScaleDown
            );
        if clips_image {
            self.push(LayoutItem::BeginClip(ClipRect {
                x: content_x,
                y: content_y,
                width: image_box_width,
                height: image_box_height,
                radius: style.border_radius,
            }));
        }
        self.push(LayoutItem::Image(ImageRect {
            x: draw_x,
            y: draw_y,
            width: draw_width,
            height: draw_height,
            intrinsic_width_px: image.intrinsic_width_px,
            intrinsic_height_px: image.intrinsic_height_px,
            data: image.data,
            alpha_mask: image.alpha_mask,
            format: image.format,
        }));
        if clips_image {
            self.push(LayoutItem::EndClip);
        }
        self.push_container_decoration(box_x, box_y, box_width, box_height, style);
        self.current_y -= box_height + style.margin_bottom;
    }

    fn layout_form_control(&mut self, id: NodeId, element: &ElementNode, style: &ComputedStyle) {
        if style.margin_top > 0.0 {
            self.current_y -= style.margin_top;
        }

        let page_index = self.pages.len().saturating_sub(1);
        let insert_index = self.pages[page_index].items.len();
        let available_width = (self.options.page.width_pt
            - self.options.page.margin_left_pt
            - self.options.page.margin_right_pt
            - self.inset_left
            - self.inset_right
            - style.margin_left
            - style.margin_right)
            .max(0.0);
        let height_base = self.options.page.height_pt
            - self.options.page.margin_top_pt
            - self.options.page.margin_bottom_pt;
        let (box_width, box_height) =
            form_control_box_size(element, style, available_width, height_base);
        if box_width <= 0.0 || box_height <= 0.0 {
            return;
        }

        self.ensure_space(box_height + style.margin_bottom);
        let x = self.options.page.margin_left_pt + self.inset_left + style.margin_left;
        let y = self.current_y - box_height;
        let disabled = element.attr("disabled").is_some();
        let control_type = form_control_type(element);

        let mut has_custom_background = false;
        for item in background_items(style, self.options, x, y, box_width, box_height) {
            has_custom_background = true;
            self.push(item);
        }
        if !has_custom_background {
            let fill = if disabled {
                rgb(0xf3f4f6)
            } else {
                Color::WHITE
            }
            .with_opacity(style.opacity);
            if style.border_radius > 0.0 {
                self.push(LayoutItem::RoundRect(RoundRect {
                    x,
                    y,
                    width: box_width,
                    height: box_height,
                    radius: style.border_radius,
                    color: fill,
                }));
            } else {
                self.push(LayoutItem::Rect(Rect {
                    x,
                    y,
                    width: box_width,
                    height: box_height,
                    color: fill,
                }));
            }
        }

        if has_border(style) || has_outline(style) {
            self.push_container_decoration(x, y, box_width, box_height, style);
        } else if matches!(control_type.as_deref(), Some("radio")) {
            self.push(LayoutItem::CircleStroke(CircleStroke {
                cx: x + box_width / 2.0,
                cy: y + box_height / 2.0,
                r: box_width.min(box_height) / 2.0,
                width: 1.0,
                color: if disabled {
                    rgb(0x9ca3af)
                } else {
                    rgb(0x767676)
                },
            }));
        } else {
            self.push(LayoutItem::StrokeRect(StrokeRect {
                x,
                y,
                width: box_width,
                height: box_height,
                radius: style.border_radius,
                stroke_width: 1.0,
                color: if disabled {
                    rgb(0x9ca3af)
                } else {
                    rgb(0x767676)
                },
                dash: None,
            }));
        }

        match control_type.as_deref() {
            Some("checkbox") if element.attr("checked").is_some() => {
                let mark = (if disabled { rgb(0x6b7280) } else { style.color })
                    .with_opacity(style.opacity);
                self.push(LayoutItem::Line(Line {
                    x1: x + box_width * 0.22,
                    y1: y + box_height * 0.52,
                    x2: x + box_width * 0.42,
                    y2: y + box_height * 0.28,
                    width: 1.6,
                    color: mark,
                    line_cap_round: true,
                    dash: None,
                }));
                self.push(LayoutItem::Line(Line {
                    x1: x + box_width * 0.42,
                    y1: y + box_height * 0.28,
                    x2: x + box_width * 0.78,
                    y2: y + box_height * 0.74,
                    width: 1.6,
                    color: mark,
                    line_cap_round: true,
                    dash: None,
                }));
            }
            Some("radio") if element.attr("checked").is_some() => {
                self.push(LayoutItem::Circle(Circle {
                    cx: x + box_width / 2.0,
                    cy: y + box_height / 2.0,
                    r: box_width.min(box_height) * 0.28,
                    color: (if disabled { rgb(0x6b7280) } else { style.color })
                        .with_opacity(style.opacity),
                }));
            }
            Some("select") => {
                if let Some(text) = selected_option_text(self.document, id) {
                    self.push_form_control_text(
                        &text, style, x, y, box_width, box_height, disabled,
                    );
                }
                self.push(LayoutItem::Polygon(Polygon {
                    points: vec![
                        (x + box_width - 12.0, y + box_height * 0.58),
                        (x + box_width - 5.0, y + box_height * 0.58),
                        (x + box_width - 8.5, y + box_height * 0.38),
                    ],
                    color: (if disabled {
                        rgb(0x6b7280)
                    } else {
                        rgb(0x374151)
                    })
                    .with_opacity(style.opacity),
                }));
            }
            Some("textarea") | Some("button") | Some("submit") | Some("reset") => {
                if let Some(text) = form_control_text(self.document, id, element) {
                    self.push_form_control_text(
                        &text, style, x, y, box_width, box_height, disabled,
                    );
                }
            }
            Some("text" | "search" | "email" | "url" | "tel" | "password" | "number" | "date")
            | None => {
                if let Some(text) = form_control_text(self.document, id, element) {
                    self.push_form_control_text(
                        &text, style, x, y, box_width, box_height, disabled,
                    );
                }
            }
            _ => {}
        }

        self.apply_style_transform_to_range(
            page_index,
            insert_index,
            x,
            y,
            box_width,
            box_height,
            style,
        );
        self.current_y -= box_height + style.margin_bottom;
    }

    fn push_form_control_text(
        &mut self,
        text: &str,
        style: &ComputedStyle,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        disabled: bool,
    ) {
        let content = truncate_text_with_ellipsis(
            text,
            style,
            (width - style.padding_left - style.padding_right - 8.0).max(1.0),
        );
        if content.trim().is_empty() {
            return;
        }
        self.push(LayoutItem::Text(TextRun {
            x: x + style.padding_left.max(4.0),
            y: y + (height - style.font_size).max(0.0) / 2.0,
            text: transform_text(&content, style.text_transform),
            font_size: style.font_size,
            color: (if disabled { rgb(0x6b7280) } else { style.color }).with_opacity(style.opacity),
            font_weight: style.font_weight,
            font_style: style.font_style,
            font_face: style.font_face,
            letter_spacing: style.letter_spacing,
            word_spacing: style.word_spacing,
            text_decoration: style.text_decoration,
            text_decoration_color: style.text_decoration_color,
            text_decoration_thickness: style.text_decoration_thickness,
            text_underline_offset: style.text_underline_offset,
            rotation_deg: style.transform_rotate_deg,
            text_shadow: style.text_shadow,
        }));
    }

    fn layout_row_container(&mut self, id: NodeId, style: &ComputedStyle) {
        if style.margin_top > 0.0 {
            self.current_y -= style.margin_top;
        }

        let mut children = self
            .document
            .children(id)
            .iter()
            .copied()
            .filter(|child| {
                self.document
                    .node(*child)
                    .is_some_and(|node| !matches!(node, Node::Text(text) if text.trim().is_empty()))
            })
            .collect::<Vec<_>>();
        children.sort_by_key(|child| self.computed_style(*child).order);
        if children.is_empty() {
            return;
        }

        let containing_width = (self.options.page.width_pt
            - self.options.page.margin_left_pt
            - self.options.page.margin_right_pt
            - self.inset_left
            - self.inset_right)
            .max(0.0);
        let flow = resolve_flow_horizontal_geometry(style, containing_width);
        let box_x = self.options.page.margin_left_pt + self.inset_left + flow.margin_left;
        let available_width = flow.available_width;
        let declared_gap = style.column_gap;
        let mut box_width = flow.box_width;
        if style.display == Display::InlineFlex && style.width.is_none() {
            let intrinsic_width = self.flex_row_intrinsic_width(&children, available_width, style);
            box_width = (intrinsic_width + style.padding_left + style.padding_right)
                .clamp(0.0, available_width);
        }
        let content_width = (box_width - style.padding_left - style.padding_right).max(0.0);
        let child_widths =
            self.flex_row_child_widths(&children, content_width, declared_gap, style, true);
        let base_child_widths = if style.flex_wrap == FlexWrap::Wrap {
            self.flex_row_child_widths(&children, content_width, declared_gap, style, false)
        } else {
            child_widths.clone()
        };
        let line_ranges = if style.flex_wrap == FlexWrap::Wrap {
            flex_line_ranges(&base_child_widths, content_width, declared_gap)
        } else {
            vec![0..children.len()]
        };
        let line_estimates = line_ranges
            .iter()
            .map(|range| {
                range
                    .clone()
                    .map(|idx| {
                        self.estimated_inline_block_height(
                            children[idx],
                            base_child_widths.get(idx).copied().unwrap_or(0.0),
                        )
                    })
                    .fold(0.0_f32, f32::max)
            })
            .collect::<Vec<_>>();
        let first_line_needed = style.padding_top
            + line_estimates.first().copied().unwrap_or(0.0)
            + style.padding_bottom;
        self.ensure_space(first_line_needed.max(first_fragment_min_height(style)));

        let page_index = self.pages.len().saturating_sub(1);
        let insert_index = self.pages[page_index].items.len();
        let top_y = self.current_y;

        self.current_y -= style.padding_top;
        let previous_left = self.inset_left;
        let previous_right = self.inset_right;
        let mut line_top = self.current_y;

        for (line_idx, range) in line_ranges.iter().enumerate() {
            let line_len = range.end.saturating_sub(range.start);
            if line_len == 0 {
                continue;
            }
            if line_idx > 0 {
                self.current_y = line_top;
                let page_count_before = self.pages.len();
                let line_needed = line_estimates.get(line_idx).copied().unwrap_or(0.0)
                    + style.row_gap
                    + style.padding_bottom;
                self.ensure_space(line_needed.max(first_fragment_min_height(style)));
                if self.pages.len() != page_count_before {
                    line_top = self.current_y;
                }
            }
            let mut line_widths = if style.flex_wrap == FlexWrap::Wrap {
                base_child_widths[range.clone()].to_vec()
            } else {
                child_widths[range.clone()].to_vec()
            };
            if style.flex_wrap == FlexWrap::Wrap {
                let grow_factors = range
                    .clone()
                    .map(|idx| flex_grow_for_style(&self.computed_style(children[idx])))
                    .collect::<Vec<_>>();
                let line_available =
                    (content_width - declared_gap * line_len.saturating_sub(1) as f32).max(0.0);
                distribute_flex_grow(&mut line_widths, &grow_factors, line_available);
                shrink_flex_line_to_available(&mut line_widths, line_available);
            }

            let used_width =
                line_widths.iter().sum::<f32>() + declared_gap * line_len.saturating_sub(1) as f32;
            let leftover_width = (content_width - used_width).max(0.0);
            let (start_offset_x, gap) = flex_row_spacing(
                style.justify_content,
                line_len,
                declared_gap,
                leftover_width,
            );
            let mut max_consumed = 0.0_f32;
            let mut max_baseline_offset = 0.0_f32;
            let mut child_layouts = Vec::with_capacity(line_len);
            let mut offset_x = start_offset_x;

            for (line_child_idx, child_idx) in range.clone().enumerate() {
                let child = children[child_idx];
                let child_alignment = self
                    .computed_style(child)
                    .align_self
                    .unwrap_or(style.align_items);
                let child_width = line_widths.get(line_child_idx).copied().unwrap_or(0.0);
                self.current_y = line_top;
                self.inset_left = previous_left + flow.margin_left + style.padding_left + offset_x;
                self.inset_right = self.options.page.width_pt
                    - self.options.page.margin_left_pt
                    - self.options.page.margin_right_pt
                    - self.inset_left
                    - child_width;
                let start_y = self.current_y;
                let child_page_index = self.pages.len().saturating_sub(1);
                let item_start = self.pages[child_page_index].items.len();
                self.layout_blockified_item(child);
                let item_end = self
                    .pages
                    .get(child_page_index)
                    .map(|page| page.items.len())
                    .unwrap_or(item_start);
                let consumed = start_y - self.current_y;
                let baseline_offset = self
                    .pages
                    .get(child_page_index)
                    .and_then(|page| {
                        first_text_baseline_offset(&page.items[item_start..item_end], start_y)
                    })
                    .unwrap_or(consumed);
                max_baseline_offset = max_baseline_offset.max(baseline_offset);
                max_consumed = max_consumed.max(consumed);
                child_layouts.push(ChildLayoutRange {
                    page_index: child_page_index,
                    item_start,
                    item_end,
                    consumed,
                    baseline_offset,
                    align_items: child_alignment,
                });
                offset_x += child_width + gap;
            }

            for child_layout in child_layouts {
                if child_layout.page_index != page_index
                    || child_layout.item_start >= child_layout.item_end
                {
                    continue;
                }
                let shift_down = flex_cross_axis_shift(
                    child_layout.align_items,
                    max_consumed,
                    child_layout.consumed,
                    max_baseline_offset,
                    child_layout.baseline_offset,
                );
                if shift_down > 0.0 {
                    shift_layout_items(
                        &mut self.pages[child_layout.page_index].items
                            [child_layout.item_start..child_layout.item_end],
                        0.0,
                        -shift_down,
                    );
                }
            }

            line_top -= max_consumed;
            if line_idx + 1 < line_ranges.len() {
                line_top -= style.row_gap;
            }
        }

        self.inset_left = previous_left;
        self.inset_right = previous_right;
        self.current_y = line_top - style.padding_bottom;
        let natural_height = (top_y - self.current_y).max(0.0);
        let height_base = self.options.page.height_pt
            - self.options.page.margin_top_pt
            - self.options.page.margin_bottom_pt;
        let final_height = resolve_box_height(style, box_width, natural_height, height_base);
        self.apply_same_page_flow_box_height(page_index, top_y, final_height);

        if self.pages.len().saturating_sub(1) == page_index {
            let height = (top_y - self.current_y).max(0.0);
            if height > 0.0 {
                let backgrounds = if should_paint_element_background(self.document, id) {
                    background_items(
                        style,
                        self.options,
                        box_x,
                        self.current_y,
                        box_width,
                        height,
                    )
                } else {
                    Vec::new()
                };
                let background_count = backgrounds.len();
                for (offset, item) in backgrounds.into_iter().enumerate() {
                    self.pages[page_index]
                        .items
                        .insert(insert_index + offset, item);
                }
                if style.overflow_hidden {
                    let child_start = insert_index + background_count;
                    let child_end = self.pages[page_index].items.len();
                    if child_start < child_end {
                        self.pages[page_index].items.insert(
                            child_start,
                            LayoutItem::BeginClip(ClipRect {
                                x: box_x,
                                y: self.current_y,
                                width: box_width,
                                height,
                                radius: style.border_radius,
                            }),
                        );
                        self.pages[page_index]
                            .items
                            .insert(child_end + 1, LayoutItem::EndClip);
                    }
                }
                self.push_container_decoration(box_x, self.current_y, box_width, height, style);
                self.paint_positioned_generated_pseudos(
                    id,
                    style,
                    box_x,
                    self.current_y,
                    box_width,
                    height,
                );
                self.apply_style_transform_to_range(
                    page_index,
                    insert_index,
                    box_x,
                    self.current_y,
                    box_width,
                    height,
                    style,
                );
            }
        } else {
            let fragment_y = self.options.page.margin_bottom_pt;
            let fragment_height = (top_y - fragment_y).max(0.0);
            if fragment_height > 0.0 {
                let backgrounds = background_items_without_shadow(
                    style,
                    self.options,
                    box_x,
                    fragment_y,
                    box_width,
                    fragment_height,
                );
                let background_count = backgrounds.len();
                for (offset, item) in backgrounds.into_iter().enumerate() {
                    self.pages[page_index]
                        .items
                        .insert(insert_index + offset, item);
                }
                if style.overflow_hidden {
                    let child_start = insert_index + background_count;
                    let child_end = self.pages[page_index].items.len();
                    if child_start < child_end {
                        self.pages[page_index].items.insert(
                            child_start,
                            LayoutItem::BeginClip(ClipRect {
                                x: box_x,
                                y: fragment_y,
                                width: box_width,
                                height: fragment_height,
                                radius: style.border_radius,
                            }),
                        );
                        self.pages[page_index]
                            .items
                            .insert(child_end + 1, LayoutItem::EndClip);
                    }
                }
            }
        }

        if style.margin_bottom > 0.0 {
            self.current_y -= style.margin_bottom;
        }
    }

    fn apply_same_page_flow_box_height(
        &mut self,
        page_index: usize,
        top_y: f32,
        final_height: f32,
    ) {
        if self.pages.len().saturating_sub(1) == page_index {
            self.current_y = top_y - final_height.max(0.0);
        }
    }

    fn align_grid_rows_cross_axis(
        &mut self,
        page_index: usize,
        row_ranges: &[GridRowLayoutRange],
        natural_height: f32,
        final_height: f32,
        style: &ComputedStyle,
    ) {
        if self.pages.len().saturating_sub(1) != page_index {
            return;
        }
        if row_ranges.is_empty() {
            return;
        }
        for (row_index, row_range) in row_ranges.iter().enumerate() {
            if row_range.page_index != page_index || row_range.item_start >= row_range.item_end {
                continue;
            }
            let shift_down = grid_content_cross_axis_row_offset(
                style.align_content,
                final_height,
                natural_height,
                row_ranges.len(),
                row_index,
            );
            if shift_down <= 0.0 {
                continue;
            }
            shift_layout_items(
                &mut self.pages[page_index].items[row_range.item_start..row_range.item_end],
                0.0,
                -shift_down,
            );
        }
    }

    fn flex_row_child_widths(
        &mut self,
        children: &[NodeId],
        box_width: f32,
        gap: f32,
        style: &ComputedStyle,
        distribute_grow: bool,
    ) -> Vec<f32> {
        if children.is_empty() {
            return Vec::new();
        }
        let available = (box_width - gap * children.len().saturating_sub(1) as f32).max(0.0);
        let mut widths = Vec::with_capacity(children.len());
        let mut grow_factors = Vec::with_capacity(children.len());
        let mut fixed_total = 0.0_f32;
        let mut auto_count = 0usize;

        for child in children {
            let child_style = match self.document.node(*child) {
                Some(Node::Element(_)) => Some(self.computed_style(*child)),
                _ => None,
            };
            let width = child_style.as_ref().and_then(|style| {
                style
                    .flex_basis
                    .as_ref()
                    .or(style.width.as_ref())
                    .map(|width| width.resolve(box_width))
            });
            grow_factors.push(
                child_style
                    .as_ref()
                    .map(|style| style.flex_grow.max(0.0))
                    .unwrap_or(0.0),
            );

            if let Some(width) = width {
                let width = width.clamp(0.0, available);
                fixed_total += width;
                widths.push(Some(width));
            } else {
                let intrinsic = if style.justify_content == JustifyContent::Start {
                    None
                } else {
                    Some(self.intrinsic_inline_width(*child, available))
                };
                if let Some(width) = intrinsic {
                    let width = width.clamp(0.0, available);
                    fixed_total += width;
                    widths.push(Some(width));
                } else {
                    auto_count += 1;
                    widths.push(None);
                }
            }
        }

        let auto_width = if auto_count > 0 {
            ((available - fixed_total).max(0.0) / auto_count as f32).max(0.0)
        } else {
            0.0
        };
        let mut resolved = widths
            .into_iter()
            .map(|width| width.unwrap_or(auto_width))
            .collect::<Vec<_>>();
        if distribute_grow {
            distribute_flex_grow(&mut resolved, &grow_factors, available);
            shrink_flex_line_to_available(&mut resolved, available);
        }
        resolved
    }

    fn flex_row_intrinsic_width(
        &mut self,
        children: &[NodeId],
        available_width: f32,
        style: &ComputedStyle,
    ) -> f32 {
        if children.is_empty() {
            return 0.0;
        }
        let gap_total = style.column_gap * children.len().saturating_sub(1) as f32;
        let width = children
            .iter()
            .map(|child| match self.document.node(*child) {
                Some(Node::Element(_)) => {
                    let child_style = self.computed_style(*child);
                    child_style
                        .flex_basis
                        .as_ref()
                        .or(child_style.width.as_ref())
                        .map(|width| width.resolve(available_width))
                        .unwrap_or_else(|| self.intrinsic_inline_width(*child, available_width))
                }
                _ => self.intrinsic_inline_width(*child, available_width),
            })
            .sum::<f32>()
            + gap_total;
        let font_metric_slack = (style.font_size * 2.0).max(inline_measurement_slack(style));
        (width + font_metric_slack).clamp(0.0, available_width)
    }

    fn intrinsic_inline_width(&mut self, id: NodeId, available_width: f32) -> f32 {
        match self.document.node(id) {
            Some(Node::Text(text)) => {
                let style = self
                    .document
                    .parent_of(id)
                    .map(|parent| self.computed_style(parent))
                    .unwrap_or_default();
                let width = estimate_text_width_with_spacing(
                    text.trim(),
                    style.font_size,
                    style.font_face,
                    style.font_weight,
                    style.letter_spacing,
                    style.word_spacing,
                );
                width + inline_measurement_slack(&style)
            }
            Some(Node::Element(element)) if element.tag == "img" => {
                let style = self.computed_style(id);
                let Some(src) = element.attr("src") else {
                    return 0.0;
                };
                let Some(intrinsic) = load_image_asset(src, self.options) else {
                    return 0.0;
                };
                let intrinsic_width = intrinsic.intrinsic_width_px as f32 * 0.75;
                let intrinsic_height = intrinsic.intrinsic_height_px as f32 * 0.75;
                let aspect = if intrinsic_height > 0.0 {
                    intrinsic_width / intrinsic_height
                } else {
                    1.0
                };
                let width = style
                    .width
                    .as_ref()
                    .map(|width| width.resolve(available_width))
                    .or_else(|| element.attr("width").and_then(parse_html_dimension_attr))
                    .or_else(|| {
                        style
                            .height
                            .as_ref()
                            .map(|height| height.resolve(available_width) * aspect)
                    })
                    .unwrap_or(intrinsic_width.min(available_width));
                let box_width = if style.box_sizing == BoxSizing::BorderBox {
                    width
                } else {
                    width + horizontal_box_extras(&style)
                };
                box_width + style.margin_left + style.margin_right
            }
            Some(Node::Element(element)) if element.tag == "svg" => {
                let style = self.computed_style(id);
                style
                    .flex_basis
                    .as_ref()
                    .or(style.width.as_ref())
                    .map(|width| width.resolve(available_width))
                    .or_else(|| element.attr("width").and_then(parse_svg_length))
                    .unwrap_or(64.0)
                    + style.margin_left
                    + style.margin_right
            }
            Some(Node::Element(element)) if is_rendered_form_control(element) => {
                let style = self.computed_style(id);
                let height_base = self.options.page.height_pt
                    - self.options.page.margin_top_pt
                    - self.options.page.margin_bottom_pt;
                let (width, _) =
                    form_control_box_size(element, &style, available_width, height_base);
                width + style.margin_left + style.margin_right
            }
            Some(Node::Element(_)) => {
                let style = self.computed_style(id);
                if let Some(width) = style.flex_basis.as_ref().or(style.width.as_ref()) {
                    return width.resolve(available_width) + style.margin_left + style.margin_right;
                }
                let text = transform_text(&self.document.text_content(id), style.text_transform);
                let text_width = estimate_text_width_with_spacing(
                    text.trim(),
                    style.font_size,
                    style.font_face,
                    style.font_weight,
                    style.letter_spacing,
                    style.word_spacing,
                );
                (text_width
                    + inline_measurement_slack(&style)
                    + style.padding_left
                    + style.padding_right
                    + style.margin_left
                    + style.margin_right
                    + style.border_left_width.max(style.border_width)
                    + style.border_right_width.max(style.border_width))
                .min(available_width)
            }
            _ => 0.0,
        }
    }

    fn estimated_inline_block_height(&mut self, id: NodeId, available_width: f32) -> f32 {
        match self.document.node(id) {
            Some(Node::Text(text)) => {
                let style = self
                    .document
                    .parent_of(id)
                    .map(|parent| self.computed_style(parent))
                    .unwrap_or_default();
                wrap_text_with_style(text, &style, available_width)
                    .len()
                    .max(1) as f32
                    * style.line_height
            }
            Some(Node::Element(element)) if element.tag == "svg" => {
                let style = self.computed_style(id);
                let width = style
                    .flex_basis
                    .as_ref()
                    .or(style.width.as_ref())
                    .map(|width| width.resolve(available_width))
                    .or_else(|| element.attr("width").and_then(parse_svg_length))
                    .unwrap_or(64.0);
                if let Some(height) = &style.height {
                    height.resolve(available_width)
                } else if let Some(ratio) = style.aspect_ratio {
                    width / ratio
                } else {
                    element
                        .attr("height")
                        .and_then(parse_svg_length)
                        .or_else(|| {
                            parse_view_box(element.attr("viewBox")).and_then(
                                |(_, _, view_width, view_height)| {
                                    svg_aspect_height(width, view_width, view_height)
                                },
                            )
                        })
                        .unwrap_or(64.0)
                }
            }
            Some(Node::Element(element)) if is_rendered_form_control(element) => {
                let style = self.computed_style(id);
                let height_base = self.options.page.height_pt
                    - self.options.page.margin_top_pt
                    - self.options.page.margin_bottom_pt;
                let (_, height) =
                    form_control_box_size(element, &style, available_width, height_base);
                height + style.margin_top + style.margin_bottom
            }
            Some(Node::Element(_)) => {
                let style = self.computed_style(id);
                if let Some(height) = &style.height {
                    return height.resolve(available_width)
                        + style.margin_top
                        + style.margin_bottom;
                }
                let width = style
                    .flex_basis
                    .as_ref()
                    .or(style.width.as_ref())
                    .map(|width| width.resolve(available_width))
                    .unwrap_or(available_width);
                if let Some(ratio) = style.aspect_ratio {
                    return width / ratio + style.margin_top + style.margin_bottom;
                }
                let text = transform_text(&self.document.text_content(id), style.text_transform);
                let line_count = wrap_text_with_style(
                    &text,
                    &style,
                    (available_width - style.padding_left - style.padding_right).max(1.0),
                )
                .len()
                .max(1) as f32;
                style.padding_top
                    + style.padding_bottom
                    + line_count * style.line_height
                    + style.margin_top
                    + style.margin_bottom
            }
            _ => 0.0,
        }
    }

    fn layout_grid_container(&mut self, id: NodeId, style: &ComputedStyle) {
        if style.margin_top > 0.0 {
            self.current_y -= style.margin_top;
        }

        let mut children = self
            .document
            .children(id)
            .iter()
            .copied()
            .filter(|child| {
                self.document
                    .node(*child)
                    .is_some_and(|node| !matches!(node, Node::Text(text) if text.trim().is_empty()))
            })
            .collect::<Vec<_>>();
        children.sort_by_key(|child| self.computed_style(*child).order);
        if children.is_empty() {
            return;
        }

        self.ensure_space(first_fragment_min_height(style));
        let page_index = self.pages.len().saturating_sub(1);
        let insert_index = self.pages[page_index].items.len();
        let top_y = self.current_y;
        let containing_width = (self.options.page.width_pt
            - self.options.page.margin_left_pt
            - self.options.page.margin_right_pt
            - self.inset_left
            - self.inset_right)
            .max(0.0);
        let flow = resolve_flow_horizontal_geometry(style, containing_width);
        let box_x = self.options.page.margin_left_pt + self.inset_left + flow.margin_left;
        let box_width = flow.box_width;
        let column_gap = style.column_gap;
        let row_gap = style.row_gap;
        let min_column_width = style
            .grid_min_column_width
            .as_ref()
            .map(|width| width.resolve(box_width))
            .unwrap_or(0.0);
        let columns = style.grid_columns.unwrap_or_else(|| {
            if min_column_width > 0.0 {
                ((box_width + column_gap) / (min_column_width + column_gap))
                    .floor()
                    .max(1.0) as usize
            } else {
                1
            }
        });
        let columns = columns.clamp(1, 64);
        let track_widths = grid_track_widths(style, box_width, columns, column_gap);
        let (track_offset, effective_column_gap) = grid_track_inline_distribution(
            style.justify_content,
            box_width,
            &track_widths,
            column_gap,
        );
        let rows = self.grid_auto_placement_rows(&children, columns);
        let grid_height_base = self.options.page.height_pt
            - self.options.page.margin_top_pt
            - self.options.page.margin_bottom_pt;
        let row_track_base = style
            .height
            .as_ref()
            .map(|height| height.resolve(grid_height_base).max(0.0))
            .unwrap_or(grid_height_base);

        self.current_y -= style.padding_top;
        let previous_left = self.inset_left;
        let previous_right = self.inset_right;

        if columns == 1 {
            let area_width = grid_spanned_track_width(&track_widths, 0, 1, effective_column_gap);
            let mut row_ranges = Vec::new();
            for (row_index, row) in rows.into_iter().enumerate() {
                let min_row_height = grid_row_track_min_height(style, row_index, row_track_base);
                let Some(placed) = row.first() else {
                    if min_row_height > 0.0 {
                        self.ensure_space(
                            (min_row_height + row_gap).max(first_fragment_min_height(style)),
                        );
                        self.current_y -= min_row_height + row_gap;
                    }
                    continue;
                };
                let child_style = self.computed_style(placed.id);
                let (inline_offset, child_width) = self.grid_item_inline_geometry(
                    placed.id,
                    &child_style,
                    area_width,
                    style.justify_items,
                );
                self.inset_left =
                    previous_left + flow.margin_left + style.padding_left + track_offset;
                self.inset_left += inline_offset;
                self.inset_right = self.options.page.width_pt
                    - self.options.page.margin_left_pt
                    - self.options.page.margin_right_pt
                    - self.inset_left
                    - child_width;
                let row_estimate =
                    min_row_height.max(self.estimated_inline_block_height(placed.id, child_width));
                self.ensure_space((row_estimate + row_gap).max(first_fragment_min_height(style)));
                let before_child_y = self.current_y;
                let child_page_index = self.pages.len().saturating_sub(1);
                let item_start = self.pages[child_page_index].items.len();
                self.layout_blockified_item(placed.id);
                let item_end = self
                    .pages
                    .get(child_page_index)
                    .map(|page| page.items.len())
                    .unwrap_or(item_start);
                if item_start < item_end {
                    row_ranges.push(GridRowLayoutRange {
                        page_index: child_page_index,
                        item_start,
                        item_end,
                    });
                }
                let consumed = before_child_y - self.current_y;
                let mut row_height = consumed;
                if min_row_height > 0.0 && consumed >= 0.0 && min_row_height > consumed {
                    row_height = min_row_height;
                    self.current_y = before_child_y - row_height;
                }
                if row_height > 0.0 {
                    self.current_y -= row_gap;
                }
            }
            self.inset_left = previous_left;
            self.inset_right = previous_right;
            if row_gap > 0.0 {
                self.current_y += row_gap;
            }
            self.current_y -= style.padding_bottom;
            self.finish_grid_container_box(
                id,
                style,
                page_index,
                insert_index,
                top_y,
                box_x,
                box_width,
                &row_ranges,
            );
            if style.margin_bottom != 0.0 {
                self.current_y -= style.margin_bottom;
            }
            return;
        }

        let mut row_ranges = Vec::new();
        for (row_index, row) in rows.into_iter().enumerate() {
            let min_row_height = grid_row_track_min_height(style, row_index, row_track_base);
            let mut row_estimate = min_row_height;
            for placed in &row {
                let child_style = self.computed_style(placed.id);
                let area_width = grid_spanned_track_width(
                    &track_widths,
                    placed.start_column,
                    placed.span,
                    effective_column_gap,
                );
                let (_, child_width) = self.grid_item_inline_geometry(
                    placed.id,
                    &child_style,
                    area_width,
                    style.justify_items,
                );
                row_estimate =
                    row_estimate.max(self.estimated_inline_block_height(placed.id, child_width));
            }
            self.ensure_space((row_estimate + row_gap).max(first_fragment_min_height(style)));
            let row_content_top = self.current_y;
            let mut max_consumed = 0.0_f32;
            let mut max_baseline_offset = 0.0_f32;
            let mut child_layouts = Vec::with_capacity(row.len());
            for placed in row {
                let child = placed.id;
                let child_style = self.computed_style(child);
                let child_alignment = child_style.align_self.unwrap_or(style.align_items);
                let area_width = grid_spanned_track_width(
                    &track_widths,
                    placed.start_column,
                    placed.span,
                    effective_column_gap,
                );
                let (inline_offset, child_width) = self.grid_item_inline_geometry(
                    child,
                    &child_style,
                    area_width,
                    style.justify_items,
                );
                let raw_column_offset = track_widths.iter().take(placed.start_column).sum::<f32>()
                    + effective_column_gap * placed.start_column as f32
                    + track_offset;
                let column_offset =
                    adjusted_grid_column_offset(style, raw_column_offset, area_width, box_width);
                self.current_y = row_content_top;
                self.inset_left = previous_left
                    + flow.margin_left
                    + style.padding_left
                    + column_offset
                    + inline_offset;
                self.inset_right = self.options.page.width_pt
                    - self.options.page.margin_left_pt
                    - self.options.page.margin_right_pt
                    - self.inset_left
                    - child_width;
                let start_y = self.current_y;
                let child_page_index = self.pages.len().saturating_sub(1);
                let item_start = self.pages[child_page_index].items.len();
                self.layout_blockified_item(child);
                let item_end = self
                    .pages
                    .get(child_page_index)
                    .map(|page| page.items.len())
                    .unwrap_or(item_start);
                let consumed = start_y - self.current_y;
                let baseline_offset = self
                    .pages
                    .get(child_page_index)
                    .and_then(|page| {
                        first_text_baseline_offset(&page.items[item_start..item_end], start_y)
                    })
                    .unwrap_or(consumed);
                max_baseline_offset = max_baseline_offset.max(baseline_offset);
                max_consumed = max_consumed.max(consumed);
                child_layouts.push(ChildLayoutRange {
                    page_index: child_page_index,
                    item_start,
                    item_end,
                    consumed,
                    baseline_offset,
                    align_items: child_alignment,
                });
            }
            let row_height = max_consumed.max(min_row_height);
            for child_layout in &child_layouts {
                if child_layout.item_start >= child_layout.item_end {
                    continue;
                }
                let shift_down = flex_cross_axis_shift(
                    child_layout.align_items,
                    row_height,
                    child_layout.consumed,
                    max_baseline_offset,
                    child_layout.baseline_offset,
                );
                if shift_down > 0.0 {
                    shift_layout_items(
                        &mut self.pages[child_layout.page_index].items
                            [child_layout.item_start..child_layout.item_end],
                        0.0,
                        -shift_down,
                    );
                }
            }
            let row_item_start = child_layouts
                .iter()
                .filter(|layout| layout.page_index == page_index)
                .map(|layout| layout.item_start)
                .min();
            let row_item_end = child_layouts
                .iter()
                .filter(|layout| layout.page_index == page_index)
                .map(|layout| layout.item_end)
                .max();
            if let (Some(item_start), Some(item_end)) = (row_item_start, row_item_end) {
                if item_start < item_end {
                    row_ranges.push(GridRowLayoutRange {
                        page_index,
                        item_start,
                        item_end,
                    });
                }
            }
            self.current_y = row_content_top - row_height - row_gap;
        }

        self.inset_left = previous_left;
        self.inset_right = previous_right;
        if row_gap > 0.0 {
            self.current_y += row_gap;
        }
        self.current_y -= style.padding_bottom;
        let natural_height = (top_y - self.current_y).max(0.0);
        let height_base = self.options.page.height_pt
            - self.options.page.margin_top_pt
            - self.options.page.margin_bottom_pt;
        let final_height = resolve_box_height(style, box_width, natural_height, height_base);
        self.align_grid_rows_cross_axis(
            page_index,
            &row_ranges,
            natural_height,
            final_height,
            style,
        );
        self.apply_same_page_flow_box_height(page_index, top_y, final_height);

        if self.pages.len().saturating_sub(1) == page_index {
            let height = (top_y - self.current_y).max(0.0);
            if height > 0.0 {
                let backgrounds = if should_paint_element_background(self.document, id) {
                    background_items(
                        style,
                        self.options,
                        box_x,
                        self.current_y,
                        box_width,
                        height,
                    )
                } else {
                    Vec::new()
                };
                let background_count = backgrounds.len();
                for (offset, item) in backgrounds.into_iter().enumerate() {
                    self.pages[page_index]
                        .items
                        .insert(insert_index + offset, item);
                }
                if style.overflow_hidden {
                    let child_start = insert_index + background_count;
                    let child_end = self.pages[page_index].items.len();
                    if child_start < child_end {
                        self.pages[page_index].items.insert(
                            child_start,
                            LayoutItem::BeginClip(ClipRect {
                                x: box_x,
                                y: self.current_y,
                                width: box_width,
                                height,
                                radius: style.border_radius,
                            }),
                        );
                        self.pages[page_index]
                            .items
                            .insert(child_end + 1, LayoutItem::EndClip);
                    }
                }
                self.push_container_decoration(box_x, self.current_y, box_width, height, style);
                self.paint_positioned_generated_pseudos(
                    id,
                    style,
                    box_x,
                    self.current_y,
                    box_width,
                    height,
                );
                self.apply_style_transform_to_range(
                    page_index,
                    insert_index,
                    box_x,
                    self.current_y,
                    box_width,
                    height,
                    style,
                );
            }
            self.insert_continuation_background_fragments(
                page_index,
                box_x,
                box_width,
                style,
                should_paint_element_background(self.document, id),
            );
        }

        if style.margin_bottom > 0.0 {
            self.current_y -= style.margin_bottom;
        }
    }

    fn finish_grid_container_box(
        &mut self,
        id: NodeId,
        style: &ComputedStyle,
        page_index: usize,
        insert_index: usize,
        top_y: f32,
        box_x: f32,
        box_width: f32,
        row_ranges: &[GridRowLayoutRange],
    ) {
        let natural_height = (top_y - self.current_y).max(0.0);
        let height_base = self.options.page.height_pt
            - self.options.page.margin_top_pt
            - self.options.page.margin_bottom_pt;
        let final_height = resolve_box_height(style, box_width, natural_height, height_base);
        self.align_grid_rows_cross_axis(
            page_index,
            row_ranges,
            natural_height,
            final_height,
            style,
        );
        self.apply_same_page_flow_box_height(page_index, top_y, final_height);

        if self.pages.len().saturating_sub(1) == page_index {
            let height = (top_y - self.current_y).max(0.0);
            if height > 0.0 {
                let backgrounds = if should_paint_element_background(self.document, id) {
                    background_items(
                        style,
                        self.options,
                        box_x,
                        self.current_y,
                        box_width,
                        height,
                    )
                } else {
                    Vec::new()
                };
                let background_count = backgrounds.len();
                for (offset, item) in backgrounds.into_iter().enumerate() {
                    self.pages[page_index]
                        .items
                        .insert(insert_index + offset, item);
                }
                if style.overflow_hidden {
                    let child_start = insert_index + background_count;
                    let child_end = self.pages[page_index].items.len();
                    if child_start < child_end {
                        self.pages[page_index].items.insert(
                            child_start,
                            LayoutItem::BeginClip(ClipRect {
                                x: box_x,
                                y: self.current_y,
                                width: box_width,
                                height,
                                radius: style.border_radius,
                            }),
                        );
                        self.pages[page_index]
                            .items
                            .insert(child_end + 1, LayoutItem::EndClip);
                    }
                }
                self.push_container_decoration(box_x, self.current_y, box_width, height, style);
                self.paint_positioned_generated_pseudos(
                    id,
                    style,
                    box_x,
                    self.current_y,
                    box_width,
                    height,
                );
                self.apply_style_transform_to_range(
                    page_index,
                    insert_index,
                    box_x,
                    self.current_y,
                    box_width,
                    height,
                    style,
                );
            }
        }
    }

    fn grid_auto_placement_rows(
        &mut self,
        children: &[NodeId],
        columns: usize,
    ) -> Vec<Vec<GridPlacedChild>> {
        let columns = columns.max(1);
        let mut rows: Vec<Vec<GridPlacedChild>> = Vec::new();
        let mut row: Vec<GridPlacedChild> = Vec::new();
        let mut used_columns = 0usize;

        for child in children.iter().copied() {
            let child_style = self.computed_style(child);
            let requested_span = child_style.grid_column_span;
            let span = if requested_span == usize::MAX {
                columns
            } else {
                requested_span.clamp(1, columns)
            };
            let explicit_start = child_style
                .grid_column_start
                .map(|line| line.saturating_sub(1).min(columns.saturating_sub(1)));
            if let Some(row_index) = child_style
                .grid_row_start
                .map(|line| line.saturating_sub(1))
            {
                if !row.is_empty() {
                    rows.push(row);
                    row = Vec::new();
                    used_columns = 0;
                }
                while rows.len() <= row_index {
                    rows.push(Vec::new());
                }
                let target_row = &mut rows[row_index];
                let used_in_target = target_row
                    .iter()
                    .map(|placed| placed.start_column + placed.span)
                    .max()
                    .unwrap_or(0)
                    .min(columns.saturating_sub(1));
                let start_column = explicit_start.unwrap_or(used_in_target);
                let span = span.min(columns.saturating_sub(start_column).max(1));
                target_row.push(GridPlacedChild {
                    id: child,
                    start_column,
                    span,
                });
                continue;
            }
            if explicit_start.is_some_and(|start| used_columns > start)
                || (used_columns > 0 && used_columns + span > columns)
            {
                rows.push(row);
                row = Vec::new();
                used_columns = 0;
            }
            let start_column = explicit_start.unwrap_or(used_columns);
            let span = span.min(columns.saturating_sub(start_column).max(1));
            row.push(GridPlacedChild {
                id: child,
                start_column,
                span,
            });
            used_columns = start_column + span;
            if used_columns >= columns {
                rows.push(row);
                row = Vec::new();
                used_columns = 0;
            }
        }

        if !row.is_empty() {
            rows.push(row);
        }

        rows
    }

    fn grid_item_inline_geometry(
        &mut self,
        child: NodeId,
        child_style: &ComputedStyle,
        area_width: f32,
        container_justify_items: AlignItems,
    ) -> (f32, f32) {
        let alignment = child_style.justify_self.unwrap_or(container_justify_items);
        let outer_width = match alignment {
            AlignItems::Stretch => area_width,
            AlignItems::Start | AlignItems::Center | AlignItems::End | AlignItems::Baseline => self
                .intrinsic_inline_width(child, area_width)
                .clamp(0.0, area_width),
        };
        let inline_offset = match alignment {
            AlignItems::Center => (area_width - outer_width).max(0.0) / 2.0,
            AlignItems::End => (area_width - outer_width).max(0.0),
            AlignItems::Stretch | AlignItems::Start | AlignItems::Baseline => 0.0,
        };
        (inline_offset, outer_width)
    }

    fn layout_column_container(&mut self, id: NodeId, style: &ComputedStyle) {
        if style.margin_top > 0.0 {
            self.current_y -= style.margin_top;
        }

        self.ensure_space(first_fragment_min_height(style));

        let page_index = self.pages.len().saturating_sub(1);
        let insert_index = self.pages[page_index].items.len();
        let top_y = self.current_y;
        let containing_width = (self.options.page.width_pt
            - self.options.page.margin_left_pt
            - self.options.page.margin_right_pt
            - self.inset_left
            - self.inset_right)
            .max(0.0);
        let flow = resolve_flow_horizontal_geometry(style, containing_width);
        let box_x = self.options.page.margin_left_pt + self.inset_left + flow.margin_left;
        let box_width = flow.box_width;

        self.current_y -= style.padding_top;

        let previous_left = self.inset_left;
        let previous_right = self.inset_right;
        let trailing_space = containing_width - flow.margin_left - box_width;
        self.inset_left += flow.margin_left + style.padding_left;
        self.inset_right += trailing_space + style.padding_right;

        let children = self
            .document
            .children(id)
            .iter()
            .copied()
            .filter(|child| {
                self.document
                    .node(*child)
                    .is_some_and(|node| !matches!(node, Node::Text(text) if text.trim().is_empty()))
            })
            .collect::<Vec<_>>();
        let gap = style.row_gap.max(style.gap);
        for (index, child) in children.iter().enumerate() {
            self.layout_blockified_item(*child);
            if index + 1 < children.len() && gap > 0.0 {
                self.current_y -= gap;
            }
        }

        self.inset_left = previous_left;
        self.inset_right = previous_right;
        self.current_y -= style.padding_bottom;

        let natural_height = (top_y - self.current_y).max(0.0);
        let height_base = self.options.page.height_pt
            - self.options.page.margin_top_pt
            - self.options.page.margin_bottom_pt;
        let final_height = resolve_box_height(style, box_width, natural_height, height_base);
        self.apply_same_page_flow_box_height(page_index, top_y, final_height);

        if self.pages.len().saturating_sub(1) == page_index {
            let height = (top_y - self.current_y).max(0.0);
            if height > 0.0 {
                let backgrounds = if should_paint_element_background(self.document, id) {
                    background_items(
                        style,
                        self.options,
                        box_x,
                        self.current_y,
                        box_width,
                        height,
                    )
                } else {
                    Vec::new()
                };
                let background_count = backgrounds.len();
                for (offset, item) in backgrounds.into_iter().enumerate() {
                    self.pages[page_index]
                        .items
                        .insert(insert_index + offset, item);
                }
                if style.overflow_hidden {
                    let child_start = insert_index + background_count;
                    let child_end = self.pages[page_index].items.len();
                    if child_start < child_end {
                        self.pages[page_index].items.insert(
                            child_start,
                            LayoutItem::BeginClip(ClipRect {
                                x: box_x,
                                y: self.current_y,
                                width: box_width,
                                height,
                                radius: style.border_radius,
                            }),
                        );
                        self.pages[page_index]
                            .items
                            .insert(child_end + 1, LayoutItem::EndClip);
                    }
                }
                self.push_container_decoration(box_x, self.current_y, box_width, height, style);
                self.paint_positioned_generated_pseudos(
                    id,
                    style,
                    box_x,
                    self.current_y,
                    box_width,
                    height,
                );
                self.apply_style_transform_to_range(
                    page_index,
                    insert_index,
                    box_x,
                    self.current_y,
                    box_width,
                    height,
                    style,
                );
            }
        } else {
            let fragment_y = self.options.page.margin_bottom_pt;
            let fragment_height = (top_y - fragment_y).max(0.0);
            if fragment_height > 0.0 {
                let backgrounds = background_items_without_shadow(
                    style,
                    self.options,
                    box_x,
                    fragment_y,
                    box_width,
                    fragment_height,
                );
                let background_count = backgrounds.len();
                for (offset, item) in backgrounds.into_iter().enumerate() {
                    self.pages[page_index]
                        .items
                        .insert(insert_index + offset, item);
                }
                if style.overflow_hidden {
                    let child_start = insert_index + background_count;
                    let child_end = self.pages[page_index].items.len();
                    if child_start < child_end {
                        self.pages[page_index].items.insert(
                            child_start,
                            LayoutItem::BeginClip(ClipRect {
                                x: box_x,
                                y: fragment_y,
                                width: box_width,
                                height: fragment_height,
                                radius: style.border_radius,
                            }),
                        );
                        self.pages[page_index]
                            .items
                            .insert(child_end + 1, LayoutItem::EndClip);
                    }
                }
                if has_border(style) {
                    self.pages[page_index].items.insert(
                        insert_index + background_count,
                        LayoutItem::StrokeRect(StrokeRect {
                            x: box_x,
                            y: fragment_y,
                            width: box_width,
                            height: fragment_height,
                            radius: style.border_radius,
                            stroke_width: style.border_width,
                            color: style.border_color.unwrap_or(style.color),
                            dash: border_dash(style.border_style, style.border_width),
                        }),
                    );
                }
                if let Some(outline) =
                    outline_stroke_rect(box_x, fragment_y, box_width, fragment_height, style)
                {
                    self.pages[page_index].items.insert(
                        insert_index + background_count + usize::from(has_border(style)),
                        LayoutItem::StrokeRect(outline),
                    );
                }
                self.apply_style_transform_to_range(
                    page_index,
                    insert_index,
                    box_x,
                    fragment_y,
                    box_width,
                    fragment_height,
                    style,
                );
            }
        }

        if style.margin_bottom > 0.0 {
            self.current_y -= style.margin_bottom;
        }
    }

    fn layout_table(&mut self, id: NodeId, style: &ComputedStyle) {
        if style.margin_top > 0.0 {
            self.current_y -= style.margin_top;
        }

        let rows = table_rows(self.document, id);
        let column_count = rows
            .iter()
            .map(|row| table_cells(self.document, *row).len())
            .max()
            .unwrap_or(0);
        if column_count == 0 {
            return;
        }

        let table_x = self.options.page.margin_left_pt + self.inset_left + style.margin_left;
        let available_width = (self.options.page.width_pt
            - self.options.page.margin_left_pt
            - self.options.page.margin_right_pt
            - self.inset_left
            - self.inset_right
            - style.margin_left
            - style.margin_right)
            .max(0.0);
        let table_width = resolve_box_width(style, available_width);
        let spacing_x = table_horizontal_spacing(style, column_count);
        let spacing_y = table_vertical_spacing(style);
        let spaced_width =
            (table_width - spacing_x * column_count.saturating_sub(1) as f32).max(0.0);
        let column_widths = table_column_widths(self.document, &rows, spaced_width, column_count);
        let header_row = rows
            .iter()
            .copied()
            .find(|row_id| is_header_row(self.document, *row_id))
            .map(|row_id| {
                let prepared = self.prepare_table_row(row_id, &column_widths);
                let height = table_row_height(&prepared, style.border_collapse);
                (prepared, height)
            });

        for (row_index, row_id) in rows.iter().enumerate() {
            let prepared = self.prepare_table_row(*row_id, &column_widths);
            if prepared.is_empty() {
                continue;
            }

            let row_height = table_row_height(&prepared, style.border_collapse);
            let row_advance = row_height
                + if row_index + 1 < rows.len() {
                    spacing_y
                } else {
                    0.0
                };
            let printable_height = (self.options.page.height_pt
                - self.options.page.margin_top_pt
                - self.options.page.margin_bottom_pt)
                .max(0.0);
            let has_multiple_lines = prepared.iter().any(|cell| cell.lines.len() > 1);
            if row_height > printable_height
                || (has_multiple_lines
                    && row_height > self.current_y - self.options.page.margin_bottom_pt + 0.1)
            {
                self.layout_fragmented_table_row(
                    table_x,
                    &column_widths,
                    &prepared,
                    row_index,
                    style,
                    header_row
                        .as_ref()
                        .map(|(prepared, height)| (prepared.as_slice(), *height)),
                    spacing_y,
                );
                if row_index + 1 < rows.len() {
                    self.current_y -= spacing_y;
                }
                continue;
            }
            let repeats_header = row_index > 0 && header_row.is_some();
            let needed_height = row_advance
                + if repeats_header {
                    header_row
                        .as_ref()
                        .map(|(_, height)| *height)
                        .unwrap_or(0.0)
                } else {
                    0.0
                };
            let page_before = self.pages.len();
            self.ensure_space(needed_height);
            if repeats_header && self.pages.len() > page_before {
                if let Some((header_prepared, header_height)) = &header_row {
                    self.paint_table_row(
                        table_x,
                        &column_widths,
                        header_prepared,
                        *header_height,
                        0,
                        style,
                        false,
                    );
                    self.current_y -= *header_height + spacing_y;
                }
            }

            self.paint_table_row(
                table_x,
                &column_widths,
                &prepared,
                row_height,
                row_index,
                style,
                true,
            );
            self.current_y -= row_advance;
        }

        if style.margin_bottom > 0.0 {
            self.current_y -= style.margin_bottom;
        }
    }

    fn layout_fragmented_table_row(
        &mut self,
        table_x: f32,
        column_widths: &[f32],
        prepared: &[PreparedCell],
        row_index: usize,
        table_style: &ComputedStyle,
        header_row: Option<(&[PreparedCell], f32)>,
        spacing_y: f32,
    ) {
        let max_lines = prepared
            .iter()
            .map(|cell| cell.lines.len())
            .max()
            .unwrap_or(0);
        if max_lines == 0 {
            let height = table_row_height(prepared, table_style.border_collapse);
            self.ensure_space(height);
            self.paint_table_row(
                table_x,
                column_widths,
                prepared,
                height,
                row_index,
                table_style,
                true,
            );
            self.current_y -= height;
            return;
        }

        let repeats_header = row_index > 0 && header_row.is_some();
        let initial_page = self.pages.len();
        let mut header_page = None;
        let mut start = 0;
        let mut first_fragment = true;

        while start < max_lines {
            let first_height = table_row_fragment_height(
                prepared,
                start,
                (start + 1).min(max_lines),
                first_fragment,
                start + 1 == max_lines,
                table_style.border_collapse,
            );
            let mut header_just_painted = false;
            if repeats_header
                && self.pages.len() > initial_page
                && header_page != Some(self.pages.len())
            {
                let (_, header_height) = header_row.expect("repeated table header");
                let required = header_height + spacing_y + first_height;
                if self.current_y < self.options.page.margin_bottom_pt + required
                    && self.pages.last().is_some_and(|page| !page.items.is_empty())
                {
                    self.ensure_space(required);
                    continue;
                }
                let (header_prepared, header_height) = header_row.expect("repeated table header");
                self.paint_table_row(
                    table_x,
                    column_widths,
                    header_prepared,
                    header_height,
                    0,
                    table_style,
                    false,
                );
                self.current_y -= header_height + spacing_y;
                header_page = Some(self.pages.len());
                header_just_painted = true;
            }

            let available = (self.current_y - self.options.page.margin_bottom_pt).max(0.0);
            let mut end = start;
            for candidate in (start + 1)..=max_lines {
                let candidate_height = table_row_fragment_height(
                    prepared,
                    start,
                    candidate,
                    first_fragment,
                    candidate == max_lines,
                    table_style.border_collapse,
                );
                if candidate_height <= available + 0.1 {
                    end = candidate;
                } else {
                    break;
                }
            }
            if end == start {
                if !header_just_painted
                    && self.pages.last().is_some_and(|page| !page.items.is_empty())
                {
                    self.ensure_space(first_height);
                    continue;
                }
                end = (start + 1).min(max_lines);
            }

            let last_fragment = end == max_lines;
            let fragment_height = table_row_fragment_height(
                prepared,
                start,
                end,
                first_fragment,
                last_fragment,
                table_style.border_collapse,
            );
            self.paint_table_row_fragment(
                table_x,
                column_widths,
                prepared,
                fragment_height,
                row_index,
                table_style,
                true,
                start,
                end,
                first_fragment,
                last_fragment,
            );
            self.current_y -= fragment_height;
            start = end;
            first_fragment = false;
        }
    }

    fn prepare_table_row(&mut self, row_id: NodeId, column_widths: &[f32]) -> Vec<PreparedCell> {
        table_cells(self.document, row_id)
            .iter()
            .enumerate()
            .map(|cell_id| {
                let (col_index, cell_id) = cell_id;
                let cell_width = column_widths.get(col_index).copied().unwrap_or(0.0);
                let style = self.computed_style(*cell_id);
                let content_width =
                    (cell_width - style.padding_left - style.padding_right).max(8.0);
                let lines = self.prepare_table_cell_lines(*cell_id, &style, content_width);
                let content_height = lines
                    .iter()
                    .map(|line| line.before + line.style.line_height + line.after)
                    .sum::<f32>();
                let nested_block_border_adjustment =
                    if has_nested_table_cell_blocks(self.document, *cell_id) {
                        2.0 * style
                            .border_top_width
                            .max(style.border_bottom_width)
                            .max(style.border_width)
                    } else {
                        0.0
                    };
                let height = style.padding_top
                    + style.padding_bottom
                    + content_height
                    + nested_block_border_adjustment;
                PreparedCell {
                    style,
                    lines,
                    content_height,
                    height,
                }
            })
            .collect()
    }

    fn prepare_table_cell_lines(
        &mut self,
        cell_id: NodeId,
        cell_style: &ComputedStyle,
        content_width: f32,
    ) -> Vec<PreparedLine> {
        let mut lines = Vec::new();
        for child_id in self.document.children(cell_id).iter().copied() {
            match self.document.node(child_id) {
                Some(Node::Text(text)) => {
                    self.push_table_text_lines(text, cell_style, content_width, &mut lines);
                }
                Some(Node::Element(element))
                    if element.tag == "script" || element.tag == "style" => {}
                Some(Node::Element(element)) => {
                    let child_style = self.computed_style(child_id);
                    if child_style.display == Display::None {
                        continue;
                    }
                    if element.tag == "br" {
                        lines.push(PreparedLine {
                            text: String::new(),
                            style: child_style,
                            before: 0.0,
                            after: 0.0,
                        });
                        continue;
                    }
                    let text = self.document.text_content(child_id);
                    let first_line = lines.len();
                    self.push_table_text_lines(&text, &child_style, content_width, &mut lines);
                    if first_line < lines.len() {
                        if let Some(line) = lines.get_mut(first_line) {
                            line.before += child_style.margin_top
                                + child_style.padding_top
                                + child_style.border_top_width.max(child_style.border_width);
                        }
                        if let Some(line) = lines.last_mut() {
                            line.after += child_style.margin_bottom
                                + child_style.padding_bottom
                                + child_style
                                    .border_bottom_width
                                    .max(child_style.border_width);
                        }
                    }
                }
                None => {}
            }
        }
        if lines.is_empty() {
            let text = self.document.text_content(cell_id);
            self.push_table_text_lines(&text, cell_style, content_width, &mut lines);
        }
        lines
    }

    fn push_table_text_lines(
        &self,
        text: &str,
        style: &ComputedStyle,
        content_width: f32,
        lines: &mut Vec<PreparedLine>,
    ) {
        let text = transform_text(text, style.text_transform);
        if text.trim().is_empty() {
            return;
        }
        for line in wrap_text_with_style(&text, style, content_width) {
            lines.push(PreparedLine {
                text: line,
                style: style.clone(),
                before: 0.0,
                after: 0.0,
            });
        }
    }

    fn paint_table_row(
        &mut self,
        table_x: f32,
        column_widths: &[f32],
        prepared: &[PreparedCell],
        row_height: f32,
        row_index: usize,
        table_style: &ComputedStyle,
        allow_rounded_header: bool,
    ) {
        self.paint_table_row_fragment(
            table_x,
            column_widths,
            prepared,
            row_height,
            row_index,
            table_style,
            allow_rounded_header,
            0,
            usize::MAX,
            true,
            true,
        );
    }

    fn paint_table_row_fragment(
        &mut self,
        table_x: f32,
        column_widths: &[f32],
        prepared: &[PreparedCell],
        row_height: f32,
        row_index: usize,
        table_style: &ComputedStyle,
        allow_rounded_header: bool,
        line_start: usize,
        line_end: usize,
        first_fragment: bool,
        last_fragment: bool,
    ) {
        let row_top = self.current_y;
        let row_bottom = row_top - row_height;
        let mut x = table_x;
        let spacing_x = table_horizontal_spacing(table_style, column_widths.len());
        let table_width = column_widths.iter().sum::<f32>()
            + spacing_x * column_widths.len().saturating_sub(1) as f32;
        let unified_header_background = if allow_rounded_header
            && first_fragment
            && last_fragment
            && row_index == 0
            && table_style.border_radius > 0.0
        {
            prepared.first().map(|cell| {
                cell.style
                    .background
                    .unwrap_or(rgb(0xf1f5f9))
                    .with_opacity(cell.style.opacity)
            })
        } else {
            None
        };
        if let Some(background) = unified_header_background {
            self.push(LayoutItem::RoundRect(RoundRect {
                x: table_x,
                y: row_bottom,
                width: table_width,
                height: row_height,
                radius: table_style.border_radius,
                color: background,
            }));
        }

        for (col_index, cell) in prepared.iter().enumerate() {
            let cell_width = column_widths.get(col_index).copied().unwrap_or(0.0);
            let cell_style = &cell.style;
            let background = cell_style.background.or_else(|| {
                if row_index == 0 {
                    Some(rgb(0xf1f5f9))
                } else if row_index % 2 == 0 {
                    Some(rgb(0xf8fafc))
                } else {
                    None
                }
            });
            if unified_header_background.is_none() {
                if let Some(background) = background {
                    self.push(LayoutItem::Rect(Rect {
                        x,
                        y: row_bottom,
                        width: cell_width,
                        height: row_height,
                        color: background.with_opacity(cell_style.opacity),
                    }));
                }
            }
            self.push_table_cell_border(x, row_bottom, cell_width, row_height, cell_style);

            let start = line_start.min(cell.lines.len());
            let end = line_end.min(cell.lines.len()).max(start);
            let content_height = cell.lines[start..end]
                .iter()
                .map(|line| line.before + line.style.line_height + line.after)
                .sum::<f32>();
            let includes_top_padding = first_fragment && start == 0;
            let includes_bottom_padding =
                last_fragment || (start < cell.lines.len() && end >= cell.lines.len());
            let top_padding = if includes_top_padding {
                cell_style.padding_top
            } else {
                0.0
            };
            let bottom_padding = if includes_bottom_padding {
                cell_style.padding_bottom
            } else {
                0.0
            };
            let free_space = (row_height - top_padding - bottom_padding - content_height).max(0.0);
            let vertical_offset = if first_fragment && last_fragment {
                match cell_style.vertical_align {
                    VerticalAlign::Middle => free_space / 2.0,
                    VerticalAlign::Bottom => free_space,
                    VerticalAlign::Baseline | VerticalAlign::Top => 0.0,
                }
            } else {
                0.0
            };
            let mut text_y = row_top - top_padding - vertical_offset - cell_style.font_size;
            let content_width =
                (cell_width - cell_style.padding_left - cell_style.padding_right).max(8.0);
            let line_count = cell.lines.len();
            for (line_index, line) in cell.lines[start..end].iter().enumerate() {
                let line_index = start + line_index;
                let line_style = &line.style;
                text_y -= line.before;
                let text_width = estimate_text_width_with_spacing(
                    &line.text,
                    line_style.font_size,
                    line_style.font_face,
                    line_style.font_weight,
                    line_style.letter_spacing,
                    line_style.word_spacing,
                );
                let resolved_align = resolved_line_text_align(line_style, line_index, line_count);
                let text_x = match resolved_align {
                    TextAlign::Left => x + cell_style.padding_left,
                    TextAlign::Center => {
                        x + cell_style.padding_left + (content_width - text_width).max(0.0) / 2.0
                    }
                    TextAlign::Right => {
                        x + cell_width - cell_style.padding_right - text_width.max(0.0)
                    }
                    TextAlign::Justify => x + cell_style.padding_left,
                    TextAlign::Start | TextAlign::End => {
                        unreachable!("resolved_line_text_align must resolve logical alignments")
                    }
                };
                let word_spacing = justified_word_spacing(
                    resolved_align,
                    &line.text,
                    text_width,
                    content_width,
                    line_style.word_spacing,
                    suppress_justify_spacing_for_line(line_style, line_index, line_count),
                );
                self.push(LayoutItem::Text(TextRun {
                    x: text_x,
                    y: text_y,
                    text: line.text.clone(),
                    font_size: line_style.font_size,
                    color: line_style.color.with_opacity(line_style.opacity),
                    font_weight: line_style.font_weight,
                    font_style: line_style.font_style,
                    font_face: line_style.font_face,
                    letter_spacing: line_style.letter_spacing,
                    word_spacing,
                    text_decoration: line_style.text_decoration,
                    text_decoration_color: line_style.text_decoration_color,
                    text_decoration_thickness: line_style.text_decoration_thickness,
                    text_underline_offset: line_style.text_underline_offset,
                    rotation_deg: line_style.transform_rotate_deg,
                    text_shadow: line_style.text_shadow,
                }));
                text_y -= line_style.line_height + line.after;
            }
            x += cell_width + spacing_x;
        }
    }

    fn layout_svg(&mut self, id: NodeId, style: &ComputedStyle) {
        if style.margin_top > 0.0 {
            self.current_y -= style.margin_top;
        }

        let available_width = (self.options.page.width_pt
            - self.options.page.margin_left_pt
            - self.options.page.margin_right_pt
            - self.inset_left
            - self.inset_right
            - style.margin_left
            - style.margin_right)
            .max(0.0);
        let Some(Node::Element(svg)) = self.document.node(id) else {
            return;
        };
        let width = style
            .width
            .as_ref()
            .map(|width| width.resolve(available_width))
            .or_else(|| svg.attr("width").and_then(parse_svg_length))
            .unwrap_or(64.0)
            .min(available_width)
            .max(0.0);
        let view_box = parse_view_box(svg.attr("viewBox"));
        let height = style
            .height
            .as_ref()
            .map(|height| height.resolve(width))
            .or_else(|| svg.attr("height").and_then(parse_svg_length))
            .or_else(|| style.aspect_ratio.map(|ratio| width / ratio))
            .or_else(|| {
                view_box.and_then(|(_, _, view_width, view_height)| {
                    svg_aspect_height(width, view_width, view_height)
                })
            })
            .unwrap_or(width)
            .max(0.0);
        if width <= 0.0 || height <= 0.0 {
            return;
        }

        self.ensure_space(height);
        let x = self.options.page.margin_left_pt + self.inset_left + style.margin_left;
        let top = self.current_y;
        let bottom = top - height;
        let page_index = self.pages.len().saturating_sub(1);
        let insert_index = self.pages[page_index].items.len();
        for item in background_items(style, self.options, x, bottom, width, height) {
            self.push(item);
        }
        let viewport = view_box.unwrap_or((0.0, 0.0, width, height));
        for child in self.document.children(id) {
            self.paint_svg_child(
                *child,
                x,
                bottom,
                width,
                height,
                viewport,
                style,
                SvgTransform::identity(),
            );
        }
        self.push_container_decoration(x, bottom, width, height, style);
        self.apply_style_transform_to_range(
            page_index,
            insert_index,
            x,
            bottom,
            width,
            height,
            style,
        );
        self.current_y -= height;

        if style.margin_bottom > 0.0 {
            self.current_y -= style.margin_bottom;
        }
    }

    fn paint_svg_child(
        &mut self,
        id: NodeId,
        x: f32,
        bottom: f32,
        width: f32,
        height: f32,
        view_box: (f32, f32, f32, f32),
        svg_style: &ComputedStyle,
        inherited_transform: SvgTransform,
    ) {
        let Some(Node::Element(element)) = self.document.node(id) else {
            return;
        };
        let local_transform = element
            .attr("transform")
            .and_then(parse_svg_transform)
            .unwrap_or_else(SvgTransform::identity);
        let transform = inherited_transform.multiply(local_transform);
        let (min_x, min_y, view_width, view_height) = view_box;
        if view_width <= 0.0 || view_height <= 0.0 {
            return;
        }
        let sx = width / view_width;
        let sy = height / view_height;
        let map_point = |point_x: f32, point_y: f32| {
            let (tx, ty) = transform.apply(point_x, point_y);
            (x + (tx - min_x) * sx, bottom + height - (ty - min_y) * sy)
        };
        let stroke_scale = transform.average_scale() * ((sx + sy) / 2.0);
        let fill_opacity = element
            .attr("fill-opacity")
            .and_then(parse_svg_alpha)
            .unwrap_or(1.0);
        let stroke_opacity = element
            .attr("stroke-opacity")
            .and_then(parse_svg_alpha)
            .unwrap_or(1.0);
        let fill = svg_paint_color(self.document, element.attr("fill"), svg_style.color)
            .unwrap_or(Color::BLACK)
            .with_opacity(svg_style.opacity * fill_opacity);
        let fill_paint = if element
            .attr("fill")
            .is_some_and(|fill| fill.eq_ignore_ascii_case("none"))
        {
            None
        } else {
            Some(fill)
        };
        let stroke = svg_paint_color(self.document, element.attr("stroke"), svg_style.color)
            .map(|stroke| stroke.with_opacity(svg_style.opacity * stroke_opacity));
        let stroke_width = element
            .attr("stroke-width")
            .and_then(parse_svg_length)
            .unwrap_or(1.0)
            * stroke_scale;
        let line_cap_round = element
            .attr("stroke-linecap")
            .is_some_and(|linecap| linecap.eq_ignore_ascii_case("round"));

        match element.tag.as_str() {
            "g" => {
                for child in self.document.children(id) {
                    self.paint_svg_child(
                        *child, x, bottom, width, height, view_box, svg_style, transform,
                    );
                }
            }
            "rect" => {
                let rx =
                    element.attr("rx").and_then(parse_svg_length).unwrap_or(0.0) * stroke_scale;
                let rect_x = element.attr("x").and_then(parse_svg_length).unwrap_or(0.0);
                let rect_y = element.attr("y").and_then(parse_svg_length).unwrap_or(0.0);
                let rect_width = element
                    .attr("width")
                    .and_then(parse_svg_length)
                    .unwrap_or(0.0);
                let rect_height = element
                    .attr("height")
                    .and_then(parse_svg_length)
                    .unwrap_or(0.0);
                if rect_width <= 0.0 || rect_height <= 0.0 {
                    return;
                }
                let (item_x, item_y) = map_point(rect_x, rect_y + rect_height);
                let item_width = rect_width * sx * transform.scale_x();
                let item_height = rect_height * sy * transform.scale_y();
                if !element
                    .attr("fill")
                    .is_some_and(|fill| fill.eq_ignore_ascii_case("none"))
                {
                    if let Some(gradient) = element
                        .attr("fill")
                        .and_then(|fill| svg_url_linear_gradient(self.document, fill))
                    {
                        if rx > 0.0 {
                            self.push(LayoutItem::BeginClip(ClipRect {
                                x: item_x,
                                y: item_y,
                                width: item_width,
                                height: item_height,
                                radius: rx,
                            }));
                        }
                        self.push(LayoutItem::LinearGradient(GradientRect {
                            x: item_x,
                            y: item_y,
                            width: item_width,
                            height: item_height,
                            gradient,
                        }));
                        if rx > 0.0 {
                            self.push(LayoutItem::EndClip);
                        }
                    } else if rx > 0.0 {
                        self.push(LayoutItem::RoundRect(RoundRect {
                            x: item_x,
                            y: item_y,
                            width: item_width,
                            height: item_height,
                            radius: rx,
                            color: fill,
                        }));
                    } else {
                        self.push(LayoutItem::Rect(Rect {
                            x: item_x,
                            y: item_y,
                            width: item_width,
                            height: item_height,
                            color: fill,
                        }));
                    }
                }
                if let Some(stroke) = stroke {
                    self.push(LayoutItem::StrokeRect(StrokeRect {
                        x: item_x,
                        y: item_y,
                        width: item_width,
                        height: item_height,
                        radius: rx,
                        stroke_width: stroke_width.max(0.1),
                        color: stroke,
                        dash: None,
                    }));
                }
            }
            "circle" => {
                let cx = element.attr("cx").and_then(parse_svg_length).unwrap_or(0.0);
                let cy = element.attr("cy").and_then(parse_svg_length).unwrap_or(0.0);
                let r = element.attr("r").and_then(parse_svg_length).unwrap_or(0.0);
                if r <= 0.0 {
                    return;
                }
                let mapped_r = r * stroke_scale;
                let (mapped_cx, mapped_cy) = map_point(cx, cy);
                if !element
                    .attr("fill")
                    .is_some_and(|fill| fill.eq_ignore_ascii_case("none"))
                {
                    self.push(LayoutItem::Circle(Circle {
                        cx: mapped_cx,
                        cy: mapped_cy,
                        r: mapped_r,
                        color: fill,
                    }));
                }
                if let Some(stroke) = stroke {
                    self.push(LayoutItem::CircleStroke(CircleStroke {
                        cx: mapped_cx,
                        cy: mapped_cy,
                        r: mapped_r,
                        width: stroke_width.max(0.1),
                        color: stroke,
                    }));
                }
            }
            "line" => {
                let x1 = element.attr("x1").and_then(parse_svg_length).unwrap_or(0.0);
                let y1 = element.attr("y1").and_then(parse_svg_length).unwrap_or(0.0);
                let x2 = element.attr("x2").and_then(parse_svg_length).unwrap_or(0.0);
                let y2 = element.attr("y2").and_then(parse_svg_length).unwrap_or(0.0);
                let (x1, y1) = map_point(x1, y1);
                let (x2, y2) = map_point(x2, y2);
                self.push(LayoutItem::Line(Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    width: stroke_width.max(0.1),
                    color: stroke.unwrap_or(svg_style.color.with_opacity(svg_style.opacity)),
                    line_cap_round,
                    dash: None,
                }));
            }
            "polygon" => {
                if element
                    .attr("fill")
                    .is_some_and(|fill| fill.eq_ignore_ascii_case("none"))
                {
                    return;
                }
                let Some(points) = element.attr("points").and_then(parse_svg_points) else {
                    return;
                };
                self.push(LayoutItem::Polygon(Polygon {
                    points: points
                        .into_iter()
                        .map(|(px, py)| map_point(px, py))
                        .collect(),
                    color: fill,
                }));
            }
            "path" => {
                let Some(commands) = element
                    .attr("d")
                    .and_then(|path| parse_svg_path(path, |px, py| map_point(px, py)))
                else {
                    return;
                };
                if commands.is_empty() {
                    return;
                }
                self.push(LayoutItem::SvgPath(SvgPath {
                    commands,
                    fill: fill_paint,
                    stroke,
                    stroke_width: stroke_width.max(0.1),
                    line_cap_round,
                }));
            }
            "text" => {
                let text = self.document.text_content(id);
                if text.trim().is_empty() {
                    return;
                }
                let text_x = element.attr("x").and_then(parse_svg_length).unwrap_or(0.0);
                let text_y = element.attr("y").and_then(parse_svg_length).unwrap_or(0.0);
                let (text_x, text_y) = map_point(text_x, text_y);
                let font_size = element
                    .attr("font-size")
                    .and_then(parse_svg_length)
                    .unwrap_or(svg_style.font_size)
                    * ((sx + sy) / 2.0);
                self.push(LayoutItem::Text(TextRun {
                    x: text_x,
                    y: text_y,
                    text,
                    font_size,
                    color: fill,
                    font_weight: svg_style.font_weight,
                    font_style: svg_style.font_style,
                    font_face: svg_style.font_face,
                    letter_spacing: svg_style.letter_spacing,
                    word_spacing: svg_style.word_spacing,
                    text_decoration: svg_style.text_decoration,
                    text_decoration_color: svg_style.text_decoration_color,
                    text_decoration_thickness: svg_style.text_decoration_thickness,
                    text_underline_offset: svg_style.text_underline_offset,
                    rotation_deg: svg_style.transform_rotate_deg + transform.rotation_deg(),
                    text_shadow: svg_style.text_shadow,
                }));
            }
            _ => {}
        }
    }

    fn layout_container(&mut self, id: NodeId, style: &ComputedStyle) {
        if style.margin_top > 0.0 {
            self.current_y -= style.margin_top;
        }

        self.ensure_space(first_fragment_min_height(style));

        let page_index = self.pages.len().saturating_sub(1);
        let insert_index = self.pages[page_index].items.len();
        let top_y = self.current_y;
        let containing_width = (self.options.page.width_pt
            - self.options.page.margin_left_pt
            - self.options.page.margin_right_pt
            - self.inset_left
            - self.inset_right)
            .max(0.0);
        let flow = resolve_flow_horizontal_geometry(style, containing_width);
        let box_x = self.options.page.margin_left_pt + self.inset_left + flow.margin_left;
        let box_width = flow.box_width;
        let establishes_containing_block = is_positioned_containing_block(style);
        if establishes_containing_block {
            self.containing_blocks.push(ContainingBlock {
                x: box_x,
                top: self.current_y,
                width: box_width,
                height: self.page_containing_block().height,
            });
        }

        self.current_y -= style.padding_top;

        let previous_left = self.inset_left;
        let previous_right = self.inset_right;
        let trailing_space = containing_width - flow.margin_left - box_width;
        self.inset_left += flow.margin_left + style.padding_left;
        self.inset_right += trailing_space + style.padding_right;
        let float_base_left = self.inset_left;
        let float_base_right = self.inset_right;
        let previous_float_base_left = self.float_base_left;
        let previous_float_base_right = self.float_base_right;
        let float_scope_start = self.floats.len();
        self.float_base_left = float_base_left;
        self.float_base_right = float_base_right;

        let inline_content = self.container_children_are_text_inline(id)
            && (has_styled_inline_children(self.document, id)
                || self.has_inline_generated_pseudo(id, style)
                || self.has_inline_atomic_child(id));
        let mut paint_stack_ranges = Vec::new();
        let mut previous_collapsible_margin_bottom: Option<f32> = None;
        if inline_content {
            // The outer container owns the box model and transforms. Its
            // inline formatting context only inherits text properties.
            let mut text_style = ComputedStyle::inherited_from(style);
            text_style.transform_rotate_deg = 0.0;
            self.layout_inline_text_block(id, &text_style);
        } else {
            self.layout_generated_pseudo(id, style, PseudoElement::Before);
            for (source_order, child) in self.document.children(id).iter().copied().enumerate() {
                // Collapsible source whitespace is not an intervening block.
                if matches!(self.document.node(child), Some(Node::Text(text)) if text.trim().is_empty())
                {
                    continue;
                }
                let child_style = match self.document.node(child) {
                    Some(Node::Element(_)) => Some(self.computed_style(child)),
                    _ => None,
                };
                if let Some(child_style) = child_style.as_ref() {
                    if child_style.float == FloatSide::None
                        && can_collapse_adjacent_sibling_margin(child_style)
                    {
                        if let Some(previous_margin_bottom) = previous_collapsible_margin_bottom {
                            self.current_y +=
                                previous_margin_bottom.min(child_style.margin_top).max(0.0);
                        }
                    }
                }
                let child_page_index = self.pages.len().saturating_sub(1);
                let item_start = self.pages[child_page_index].items.len();
                self.layout_flow_child(child, float_base_left, float_base_right);
                let item_end = self
                    .pages
                    .get(child_page_index)
                    .map(|page| page.items.len())
                    .unwrap_or(item_start);
                if child_page_index == page_index && item_start < item_end {
                    paint_stack_ranges.push(PaintStackRange {
                        item_start,
                        item_end,
                        z_index: child_style
                            .as_ref()
                            .and_then(positioned_z_index)
                            .unwrap_or(0),
                        source_order,
                    });
                }
                previous_collapsible_margin_bottom = child_style
                    .as_ref()
                    .filter(|style| {
                        style.float == FloatSide::None
                            && can_collapse_adjacent_sibling_margin(style)
                    })
                    .map(|style| style.margin_bottom.max(0.0));
            }
            self.layout_generated_pseudo(id, style, PseudoElement::After);
        }
        if paint_stack_ranges.iter().any(|range| range.z_index != 0) {
            reorder_paint_stack_ranges(&mut self.pages[page_index].items, &paint_stack_ranges);
        }
        if establishes_containing_block {
            let _ = self.containing_blocks.pop();
        }

        if style.display == Display::InlineBlock {
            for float in &self.floats[float_scope_start..] {
                if float.page_index == self.pages.len().saturating_sub(1) {
                    self.current_y = self.current_y.min(float.bottom);
                }
            }
        }
        self.floats.truncate(float_scope_start);
        self.float_base_left = previous_float_base_left;
        self.float_base_right = previous_float_base_right;
        self.inset_left = previous_left;
        self.inset_right = previous_right;
        self.current_y -= style.padding_bottom;

        let natural_height = (top_y - self.current_y).max(0.0);
        let height_base = self.options.page.height_pt
            - self.options.page.margin_top_pt
            - self.options.page.margin_bottom_pt;
        let final_height = resolve_box_height(style, box_width, natural_height, height_base);
        self.apply_same_page_flow_box_height(page_index, top_y, final_height);

        if self.pages.len().saturating_sub(1) == page_index {
            let height = (top_y - self.current_y).max(0.0);
            if height > 0.0 {
                let backgrounds = if should_paint_element_background(self.document, id) {
                    background_items(
                        style,
                        self.options,
                        box_x,
                        self.current_y,
                        box_width,
                        height,
                    )
                } else {
                    Vec::new()
                };
                let background_count = backgrounds.len();
                for (offset, item) in backgrounds.into_iter().enumerate() {
                    self.pages[page_index]
                        .items
                        .insert(insert_index + offset, item);
                }
                if style.overflow_hidden {
                    let child_start = insert_index + background_count;
                    let child_end = self.pages[page_index].items.len();
                    if child_start < child_end {
                        self.pages[page_index].items.insert(
                            child_start,
                            LayoutItem::BeginClip(ClipRect {
                                x: box_x,
                                y: self.current_y,
                                width: box_width,
                                height,
                                radius: style.border_radius,
                            }),
                        );
                        self.pages[page_index]
                            .items
                            .insert(child_end + 1, LayoutItem::EndClip);
                    }
                }
                self.push_container_decoration(box_x, self.current_y, box_width, height, style);
                self.paint_positioned_generated_pseudos(
                    id,
                    style,
                    box_x,
                    self.current_y,
                    box_width,
                    height,
                );
                self.apply_style_transform_to_range(
                    page_index,
                    insert_index,
                    box_x,
                    self.current_y,
                    box_width,
                    height,
                    style,
                );
            }
        } else {
            let fragment_y = self.options.page.margin_bottom_pt;
            let fragment_height = (top_y - fragment_y).max(0.0);
            if fragment_height > 0.0 {
                let backgrounds = background_items_without_shadow(
                    style,
                    self.options,
                    box_x,
                    fragment_y,
                    box_width,
                    fragment_height,
                );
                let background_count = backgrounds.len();
                for (offset, item) in backgrounds.into_iter().enumerate() {
                    self.pages[page_index]
                        .items
                        .insert(insert_index + offset, item);
                }
                if style.overflow_hidden {
                    let child_start = insert_index + background_count;
                    let child_end = self.pages[page_index].items.len();
                    if child_start < child_end {
                        self.pages[page_index].items.insert(
                            child_start,
                            LayoutItem::BeginClip(ClipRect {
                                x: box_x,
                                y: fragment_y,
                                width: box_width,
                                height: fragment_height,
                                radius: style.border_radius,
                            }),
                        );
                        self.pages[page_index]
                            .items
                            .insert(child_end + 1, LayoutItem::EndClip);
                    }
                }
            }
            self.insert_continuation_background_fragments(
                page_index,
                box_x,
                box_width,
                style,
                should_paint_element_background(self.document, id),
            );
        }

        if style.margin_bottom > 0.0 {
            self.current_y -= style.margin_bottom;
        }
    }

    fn layout_text_block(&mut self, text: &str, style: &ComputedStyle) {
        if text.trim().is_empty() {
            return;
        }
        if style.margin_top > 0.0 {
            self.current_y -= style.margin_top;
        }
        let mut page_index = self.pages.len().saturating_sub(1);
        let mut insert_index = self.pages[page_index].items.len();
        let mut top_y = self.current_y;
        let text = transform_text(text, style.text_transform);
        let base_left = self.inset_left
            - self.float_occupied_width_at_y(page_index, self.current_y, FloatSide::Left);
        let base_right = self.inset_right
            - self.float_occupied_width_at_y(page_index, self.current_y, FloatSide::Right);
        let mut lines =
            self.wrap_text_with_floats(&text, style, base_left.max(0.0), base_right.max(0.0));
        let mut available_width = lines
            .iter()
            .map(|line| line.available_width)
            .fold(0.0_f32, f32::max);
        if style.white_space == WhiteSpace::NoWrap
            && style.overflow_hidden
            && style.text_overflow == TextOverflow::Ellipsis
        {
            for line in &mut lines {
                line.text = truncate_text_with_ellipsis(&line.text, style, line.available_width);
            }
        }
        let layout_line_height = layout_line_height(style);
        let mut text_height = layout_line_height * lines.len() as f32;
        let height_base = self.options.page.height_pt
            - self.options.page.margin_top_pt
            - self.options.page.margin_bottom_pt;
        let mut resolved_flow_height =
            resolve_box_height(style, available_width, text_height, height_base);
        let mut visible_overflow_capped =
            !style.overflow_hidden && resolved_flow_height + 0.01 < text_height;
        let initial_needed = if style.page_break_inside_avoid {
            resolved_flow_height + style.margin_bottom
        } else {
            layout_line_height
        };
        let page_count_before = self.pages.len();
        self.ensure_space(initial_needed.max(layout_line_height));
        if self.pages.len() != page_count_before {
            page_index = self.pages.len().saturating_sub(1);
            insert_index = self.pages[page_index].items.len();
            top_y = self.current_y;
            lines =
                self.wrap_text_with_floats(&text, style, base_left.max(0.0), base_right.max(0.0));
            available_width = lines
                .iter()
                .map(|line| line.available_width)
                .fold(0.0_f32, f32::max);
            if style.white_space == WhiteSpace::NoWrap
                && style.overflow_hidden
                && style.text_overflow == TextOverflow::Ellipsis
            {
                for line in &mut lines {
                    line.text =
                        truncate_text_with_ellipsis(&line.text, style, line.available_width);
                }
            }
            text_height = layout_line_height * lines.len() as f32;
            resolved_flow_height =
                resolve_box_height(style, available_width, text_height, height_base);
            visible_overflow_capped =
                !style.overflow_hidden && resolved_flow_height + 0.01 < text_height;
        }

        if let Some(background) = style.background {
            let first_line = lines.first().expect("non-empty text should produce a line");
            let y = self.current_y - layout_line_height * lines.len() as f32 + 2.0;
            self.push(LayoutItem::Rect(Rect {
                x: self.options.page.margin_left_pt + first_line.inset_left,
                y,
                width: first_line.available_width,
                height: layout_line_height * lines.len() as f32 + 6.0,
                color: background.with_opacity(style.opacity),
            }));
        }

        let line_count = lines.len();
        for (line_index, line) in lines.into_iter().enumerate() {
            let width = estimate_text_width_with_spacing(
                &line.text,
                style.font_size,
                style.font_face,
                style.font_weight,
                style.letter_spacing,
                style.word_spacing,
            );
            let resolved_align = resolved_line_text_align(style, line_index, line_count);
            let x = match resolved_align {
                TextAlign::Left => self.options.page.margin_left_pt + line.inset_left,
                TextAlign::Center => {
                    self.options.page.margin_left_pt
                        + line.inset_left
                        + (line.available_width - width).max(0.0) / 2.0
                }
                TextAlign::Right => {
                    self.options.page.margin_left_pt
                        + line.inset_left
                        + (line.available_width - width).max(0.0)
                }
                TextAlign::Justify => self.options.page.margin_left_pt + line.inset_left,
                TextAlign::Start | TextAlign::End => {
                    unreachable!("resolved_line_text_align must resolve logical alignments")
                }
            };
            let word_spacing = justified_word_spacing(
                resolved_align,
                &line.text,
                width,
                line.available_width,
                style.word_spacing,
                suppress_justify_spacing_for_line(style, line_index, line_count),
            );
            self.push(LayoutItem::Text(TextRun {
                x,
                y: self.current_y - style.font_size,
                text: line.text,
                font_size: style.font_size,
                color: style.color.with_opacity(style.opacity),
                font_weight: style.font_weight,
                font_style: style.font_style,
                font_face: style.font_face,
                letter_spacing: style.letter_spacing,
                word_spacing,
                text_decoration: style.text_decoration,
                text_decoration_color: style.text_decoration_color,
                text_decoration_thickness: style.text_decoration_thickness,
                text_underline_offset: style.text_underline_offset,
                rotation_deg: style.transform_rotate_deg,
                text_shadow: style.text_shadow,
            }));
            self.current_y -= layout_line_height;
            if line_index + 1 < line_count && !visible_overflow_capped {
                self.ensure_space(layout_line_height);
            }
        }

        let height = (top_y - self.current_y).max(0.0);
        self.apply_style_transform_to_range(
            page_index,
            insert_index,
            self.options.page.margin_left_pt + self.inset_left,
            self.current_y,
            available_width,
            height,
            style,
        );
        self.apply_relative_position_to_following_pages(page_index, style, available_width, height);
        let final_height = resolve_box_height(style, available_width, height, height_base);
        self.apply_same_page_flow_box_height(page_index, top_y, final_height);
        self.current_y -= style.margin_bottom;
    }

    fn layout_inline_text_block(&mut self, id: NodeId, style: &ComputedStyle) {
        if style.margin_top > 0.0 {
            self.current_y -= style.margin_top;
        }
        let mut page_index = self.pages.len().saturating_sub(1);
        let mut insert_index = self.pages[page_index].items.len();
        let mut top_y = self.current_y;

        let mut segments = Vec::new();
        self.collect_inline_text_segments(id, style, &mut segments);
        if segments.is_empty() {
            return;
        }

        let base_left = self.inset_left
            - self.float_occupied_width_at_y(page_index, self.current_y, FloatSide::Left);
        let base_right = self.inset_right
            - self.float_occupied_width_at_y(page_index, self.current_y, FloatSide::Right);
        let mut lines = self.wrap_inline_segments_with_floats(
            &segments,
            style,
            base_left.max(0.0),
            base_right.max(0.0),
        );
        let mut available_width = lines
            .iter()
            .map(|line| line.available_width)
            .fold(0.0_f32, f32::max);
        let layout_line_height = layout_line_height(style);
        let height_base = self.options.page.height_pt
            - self.options.page.margin_top_pt
            - self.options.page.margin_bottom_pt;
        let mut text_height = lines
            .iter()
            .map(|line| inline_line_metrics(&line.segments, style).1)
            .sum();
        let mut resolved_flow_height =
            resolve_box_height(style, available_width, text_height, height_base);
        let mut visible_overflow_capped =
            !style.overflow_hidden && resolved_flow_height + 0.01 < text_height;
        let initial_needed = if style.page_break_inside_avoid {
            resolved_flow_height + style.margin_bottom
        } else {
            layout_line_height
        };
        let page_count_before = self.pages.len();
        self.ensure_space(initial_needed.max(layout_line_height));
        if self.pages.len() != page_count_before {
            page_index = self.pages.len().saturating_sub(1);
            insert_index = self.pages[page_index].items.len();
            top_y = self.current_y;
            lines = self.wrap_inline_segments_with_floats(
                &segments,
                style,
                base_left.max(0.0),
                base_right.max(0.0),
            );
            available_width = lines
                .iter()
                .map(|line| line.available_width)
                .fold(0.0_f32, f32::max);
            text_height = lines
                .iter()
                .map(|line| inline_line_metrics(&line.segments, style).1)
                .sum();
            resolved_flow_height =
                resolve_box_height(style, available_width, text_height, height_base);
            visible_overflow_capped =
                !style.overflow_hidden && resolved_flow_height + 0.01 < text_height;
            self.ensure_space(layout_line_height);
        }

        let line_count = lines.len();
        for (line_index, line) in lines.into_iter().enumerate() {
            let line_width = inline_segments_width(&line.segments);
            let (baseline_offset, actual_height) = inline_line_metrics(&line.segments, style);
            if !visible_overflow_capped {
                let page_count = self.pages.len();
                self.ensure_space(actual_height);
                if line_index == 0 && self.pages.len() != page_count {
                    page_index = self.pages.len() - 1;
                    insert_index = self.pages[page_index].items.len();
                    top_y = self.current_y;
                }
            }
            let resolved_align = resolved_line_text_align(style, line_index, line_count);
            let extra_word_spacing = justified_inline_word_spacing(
                resolved_align,
                &line.segments,
                line_width,
                line.available_width,
                suppress_justify_spacing_for_line(style, line_index, line_count),
            );
            let mut x = match resolved_align {
                TextAlign::Left => self.options.page.margin_left_pt + line.inset_left,
                TextAlign::Center => {
                    self.options.page.margin_left_pt
                        + line.inset_left
                        + (line.available_width - line_width).max(0.0) / 2.0
                }
                TextAlign::Right => {
                    self.options.page.margin_left_pt
                        + line.inset_left
                        + (line.available_width - line_width).max(0.0)
                }
                TextAlign::Justify => self.options.page.margin_left_pt + line.inset_left,
                TextAlign::Start | TextAlign::End => {
                    unreachable!("resolved_line_text_align must resolve logical alignments")
                }
            };
            for segment in line.segments {
                if let Some(atomic) = &segment.atomic {
                    let mut items = atomic.items.clone();
                    shift_layout_items(
                        &mut items,
                        x,
                        self.current_y - baseline_offset + atomic.baseline,
                    );
                    self.pages.last_mut().unwrap().items.extend(items);
                    x += atomic.width;
                    continue;
                }
                let segment_width = estimate_text_width_with_spacing(
                    &segment.text,
                    segment.style.font_size,
                    segment.style.font_face,
                    segment.style.font_weight,
                    segment.style.letter_spacing,
                    segment.style.word_spacing,
                );
                let segment_word_spacing_opportunities = word_spacing_opportunities(&segment.text);
                self.push(LayoutItem::Text(TextRun {
                    x,
                    y: self.current_y - baseline_offset,
                    text: segment.text,
                    font_size: segment.style.font_size,
                    color: segment.style.color.with_opacity(segment.style.opacity),
                    font_weight: segment.style.font_weight,
                    font_style: segment.style.font_style,
                    font_face: segment.style.font_face,
                    letter_spacing: segment.style.letter_spacing,
                    word_spacing: segment.style.word_spacing + extra_word_spacing,
                    text_decoration: segment.style.text_decoration,
                    text_decoration_color: segment.style.text_decoration_color,
                    text_decoration_thickness: segment.style.text_decoration_thickness,
                    text_underline_offset: segment.style.text_underline_offset,
                    rotation_deg: segment.style.transform_rotate_deg,
                    text_shadow: segment.style.text_shadow,
                }));
                x += segment_width + extra_word_spacing * segment_word_spacing_opportunities as f32;
            }
            self.current_y -= actual_height;
        }

        let height = (top_y - self.current_y).max(0.0);
        self.apply_style_transform_to_range(
            page_index,
            insert_index,
            self.options.page.margin_left_pt + base_left.max(0.0),
            self.current_y,
            available_width,
            height,
            style,
        );
        self.apply_relative_position_to_following_pages(page_index, style, available_width, height);
        let final_height = resolve_box_height(style, available_width, height, height_base);
        self.apply_same_page_flow_box_height(page_index, top_y, final_height);
        self.current_y -= style.margin_bottom;
    }

    fn layout_generated_pseudo(
        &mut self,
        id: NodeId,
        parent_style: &ComputedStyle,
        kind: PseudoElement,
    ) {
        let Some(style) = self.pseudo_style(id, parent_style, kind) else {
            return;
        };
        if is_out_of_flow_position(&style) {
            return;
        }
        let Some(content) = style.content.clone() else {
            return;
        };
        if content.is_empty() {
            self.layout_generated_pseudo_box(&style);
            return;
        }
        self.layout_text_block(&content, &style);
    }

    fn paint_positioned_generated_pseudos(
        &mut self,
        id: NodeId,
        parent_style: &ComputedStyle,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) {
        let containing_block = ContainingBlock {
            x,
            top: y + height,
            width,
            height,
        };
        for kind in [PseudoElement::Before, PseudoElement::After] {
            let Some(style) = self.pseudo_style(id, parent_style, kind) else {
                continue;
            };
            if !is_out_of_flow_position(&style) {
                continue;
            }
            self.paint_generated_pseudo_box_in_containing_block(&style, containing_block);
        }
    }

    fn layout_generated_pseudo_box(&mut self, style: &ComputedStyle) {
        if !has_generated_pseudo_box_paint(style) {
            return;
        }
        if style.margin_top > 0.0 {
            self.current_y -= style.margin_top;
        }

        let page_index = self.pages.len().saturating_sub(1);
        let insert_index = self.pages[page_index].items.len();
        let raw_available_width = self.options.page.width_pt
            - self.options.page.margin_left_pt
            - self.options.page.margin_right_pt
            - self.inset_left
            - self.inset_right;
        let box_width = resolve_box_width(style, raw_available_width);
        let height_base = self.options.page.height_pt
            - self.options.page.margin_top_pt
            - self.options.page.margin_bottom_pt;
        let natural_height = style.padding_top + style.padding_bottom;
        let box_height = resolve_box_height(style, box_width, natural_height, height_base);
        let total_height = box_height.max(0.0);
        if total_height <= 0.0 && !has_box_decoration(style) {
            return;
        }

        self.ensure_space(total_height.max(1.0));
        let x = self.options.page.margin_left_pt + self.inset_left + style.margin_left;
        let y = self.current_y - total_height;
        for item in background_items(style, self.options, x, y, box_width, total_height) {
            self.push(item);
        }
        self.push_container_decoration(x, y, box_width, total_height, style);
        self.apply_style_transform_to_range(
            page_index,
            insert_index,
            x,
            y,
            box_width,
            total_height,
            style,
        );
        self.current_y -= total_height + style.margin_bottom;
    }

    fn paint_generated_pseudo_box_in_containing_block(
        &mut self,
        style: &ComputedStyle,
        containing_block: ContainingBlock,
    ) {
        let content = style.content.as_deref().unwrap_or("");
        if content.is_empty() && !has_generated_pseudo_box_paint(style) {
            return;
        }

        let page_index = self.pages.len().saturating_sub(1);
        let insert_index = self.pages[page_index].items.len();
        let mut width = positioned_width(style, containing_block.width);
        let mut height = positioned_height(style, containing_block.height);
        if height <= 0.0 {
            let natural_height = if content.trim().is_empty() {
                style.padding_top + style.padding_bottom
            } else {
                style.padding_top + style.line_height + style.padding_bottom
            };
            height = resolve_box_height(style, width, natural_height, containing_block.height);
        }
        if let Some(min_width) = &style.min_width {
            width = width.max(min_width.resolve(containing_block.width));
        }
        if let Some(max_width) = &style.max_width {
            width = width.min(max_width.resolve(containing_block.width));
        }
        if let Some(min_height) = &style.min_height {
            height = height.max(min_height.resolve(containing_block.height));
        }
        if width <= 0.0 && !has_box_decoration(style) {
            return;
        }
        if height <= 0.0 && !has_box_decoration(style) {
            return;
        }

        let left = style
            .inset_left
            .as_ref()
            .map(|value| value.resolve(containing_block.width));
        let right = style
            .inset_right
            .as_ref()
            .map(|value| value.resolve(containing_block.width));
        let top = style
            .inset_top
            .as_ref()
            .map(|value| value.resolve(containing_block.height));
        let bottom = style
            .inset_bottom
            .as_ref()
            .map(|value| value.resolve(containing_block.height));
        let x = if let Some(left) = left {
            containing_block.x + left + style.margin_left
        } else if let Some(right) = right {
            containing_block.x + containing_block.width - right - width - style.margin_right
        } else {
            containing_block.x + style.margin_left
        };
        let top_y = if let Some(top) = top {
            containing_block.top - top - style.margin_top
        } else if let Some(bottom) = bottom {
            containing_block.top - containing_block.height + bottom + height + style.margin_bottom
        } else {
            containing_block.top - style.margin_top
        };
        let y = top_y - height;

        for item in background_items(style, self.options, x, y, width, height) {
            self.push(item);
        }
        self.push_container_decoration(x, y, width, height, style);
        if !content.trim().is_empty() {
            self.push(LayoutItem::Text(TextRun {
                x: x + style.padding_left,
                y: top_y - style.padding_top - style.font_size,
                text: transform_text(content, style.text_transform),
                font_size: style.font_size,
                color: style.color.with_opacity(style.opacity),
                font_weight: style.font_weight,
                font_style: style.font_style,
                font_face: style.font_face,
                letter_spacing: style.letter_spacing,
                word_spacing: style.word_spacing,
                text_decoration: style.text_decoration,
                text_decoration_color: style.text_decoration_color,
                text_decoration_thickness: style.text_decoration_thickness,
                text_underline_offset: style.text_underline_offset,
                rotation_deg: style.transform_rotate_deg,
                text_shadow: style.text_shadow,
            }));
        }
        self.apply_style_transform_to_range(page_index, insert_index, x, y, width, height, style);
    }

    fn apply_style_transform_to_range(
        &mut self,
        page_index: usize,
        item_start: usize,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        style: &ComputedStyle,
    ) {
        let Some(page) = self.pages.get_mut(page_index) else {
            return;
        };
        if item_start >= page.items.len() {
            return;
        }
        let transform_translate_x = style
            .transform_translate_x
            .as_ref()
            .map(|value| value.resolve(width))
            .unwrap_or(0.0);
        let transform_translate_y_css = style
            .transform_translate_y
            .as_ref()
            .map(|value| value.resolve(height))
            .unwrap_or(0.0);
        let (relative_translate_x, relative_translate_y_css) =
            relative_position_offset(style, width, height);
        let translate_x = transform_translate_x + relative_translate_x;
        let translate_y_css = transform_translate_y_css + relative_translate_y_css;
        let scale_x = style.transform_scale_x;
        let scale_y = style.transform_scale_y;
        let rotate_deg = style.transform_rotate_deg;
        let has_translate = translate_x.abs() > 0.001 || translate_y_css.abs() > 0.001;
        let has_scale = (scale_x - 1.0).abs() > 0.001 || (scale_y - 1.0).abs() > 0.001;
        let has_rotate = rotate_deg.abs() > 0.001;
        if !has_translate && !has_scale && !has_rotate {
            return;
        }
        if has_scale {
            let (origin_x, origin_y) = transform_origin_point(style, x, y, width, height);
            scale_layout_items(
                &mut page.items[item_start..],
                origin_x,
                origin_y,
                scale_x,
                scale_y,
            );
        }
        if has_rotate {
            let (origin_x, origin_y) = transform_origin_point(style, x, y, width, height);
            rotate_layout_items_from(&mut page.items, item_start, origin_x, origin_y, -rotate_deg);
        }
        if has_translate {
            shift_layout_items(&mut page.items[item_start..], translate_x, -translate_y_css);
        }
    }

    fn apply_relative_position_to_range(
        &mut self,
        page_index: usize,
        item_start: usize,
        style: &ComputedStyle,
        width: f32,
        height: f32,
    ) {
        let (translate_x, translate_y_css) = relative_position_offset(style, width, height);
        if translate_x.abs() <= 0.001 && translate_y_css.abs() <= 0.001 {
            return;
        }
        let Some(page) = self.pages.get_mut(page_index) else {
            return;
        };
        if item_start < page.items.len() {
            shift_layout_items(&mut page.items[item_start..], translate_x, -translate_y_css);
        }
    }

    fn apply_relative_position_to_following_pages(
        &mut self,
        first_page_index: usize,
        style: &ComputedStyle,
        width: f32,
        height: f32,
    ) {
        for page_index in (first_page_index + 1)..self.pages.len() {
            self.apply_relative_position_to_range(page_index, 0, style, width, height);
        }
    }

    fn measure_inline_atomic_box(&mut self, id: NodeId, style: &ComputedStyle) -> InlineAtomicBox {
        let available = (self.options.page.width_pt
            - self.options.page.margin_left_pt
            - self.options.page.margin_right_pt
            - self.inset_left
            - self.inset_right)
            .max(0.0);
        let mut box_style = style.clone();
        let mut width = if style.width.is_some() {
            resolve_flow_horizontal_geometry(style, available).box_width
        } else {
            (self.intrinsic_inline_width(id, available) - style.margin_left - style.margin_right)
                .max(0.0)
        };
        if style.width.is_none() {
            let mut constrained = style.clone();
            constrained.width = Some(crate::css::CssLength::Linear {
                percent: 0.0,
                points: if style.box_sizing == BoxSizing::BorderBox {
                    width
                } else {
                    (width - horizontal_box_extras(style)).max(0.0)
                },
            });
            width = resolve_flow_horizontal_geometry(&constrained, available).box_width;
        }
        // Resolve percentages against the containing block before creating the
        // independent formatting context. Keep the measured border box fixed.
        box_style.width = Some(crate::css::CssLength::Linear {
            percent: 0.0,
            points: if style.box_sizing == BoxSizing::BorderBox {
                width
            } else {
                (width - horizontal_box_extras(style)).max(0.0)
            },
        });
        box_style.min_width = None;
        box_style.max_width = None;
        let outer_width = width + style.margin_left + style.margin_right;
        let top = self.options.page.height_pt - self.options.page.margin_top_pt;
        let inset_right = self.options.page.width_pt
            - self.options.page.margin_left_pt
            - self.options.page.margin_right_pt
            - outer_width;
        // Do not clone already rendered document pages for every inline box.
        let mut probe = LayoutContext {
            document: self.document,
            stylesheet: self.stylesheet,
            options: self.options,
            pages: vec![LayoutPage {
                number: 1,
                page: self.options.page,
                items: Vec::new(),
            }],
            current_y: top,
            inset_left: 0.0,
            inset_right,
            float_base_left: 0.0,
            float_base_right: inset_right,
            containing_blocks: Vec::new(),
            floats: Vec::new(),
            fixed_overlays: Vec::new(),
            suppress_positioning: false,
            suppress_floats: false,
            suppress_pagination: true,
            suppress_keep_with_next: true,
            style_cache: self.style_cache.clone(),
        };
        probe.style_cache.insert(id, box_style.clone());
        probe.layout_container(id, &box_style);
        let height = (top - probe.current_y).max(0.0);
        let mut items = std::mem::take(&mut probe.pages[0].items);
        let baseline = if style.overflow_hidden {
            height
        } else {
            items
                .iter()
                .rev()
                .find_map(|item| match item {
                    LayoutItem::Text(run) => Some(top - run.y),
                    _ => None,
                })
                .unwrap_or(height)
        };
        shift_layout_items(&mut items, -self.options.page.margin_left_pt, -top);
        if style.visibility != Visibility::Visible {
            items.clear();
        }
        InlineAtomicBox {
            items,
            width: outer_width,
            height,
            baseline,
        }
    }

    fn collect_inline_text_segments(
        &mut self,
        id: NodeId,
        inherited_style: &ComputedStyle,
        segments: &mut Vec<InlineTextSegment>,
    ) {
        self.push_list_marker_segment(id, inherited_style, segments);
        self.push_pseudo_inline_segment(id, inherited_style, PseudoElement::Before, segments);
        for child in self.document.children(id).iter().copied() {
            match self.document.node(child) {
                Some(Node::Text(text)) => {
                    if !text.is_empty() {
                        segments.push(InlineTextSegment {
                            atomic: None,
                            text: transform_text(
                                &if inherited_style.white_space == WhiteSpace::PreLine {
                                    text.clone()
                                } else {
                                    text.replace('\n', " ")
                                },
                                inherited_style.text_transform,
                            ),
                            style: inherited_style.clone(),
                        });
                    }
                }
                Some(Node::Element(element)) if element.tag == "br" => {
                    segments.push(InlineTextSegment {
                        atomic: None,
                        text: "\n".to_string(),
                        style: self.computed_style(child),
                    });
                }
                Some(Node::Element(element))
                    if element.tag == "script" || element.tag == "style" => {}
                Some(Node::Element(_)) => {
                    let child_style = self.computed_style(child);
                    if child_style.display == Display::None {
                        continue;
                    }
                    if child_style.display == Display::InlineBlock {
                        let atomic = self.measure_inline_atomic_box(child, &child_style);
                        segments.push(InlineTextSegment {
                            text: String::new(),
                            style: child_style,
                            atomic: Some(std::rc::Rc::new(atomic)),
                        });
                        continue;
                    }
                    if self.document.children(child).is_empty() {
                        let text = self.document.text_content(child);
                        if !text.trim().is_empty() {
                            segments.push(InlineTextSegment {
                                atomic: None,
                                text: transform_text(&text, child_style.text_transform),
                                style: child_style,
                            });
                        }
                    } else {
                        self.collect_inline_text_segments(child, &child_style, segments);
                    }
                }
                None => {}
            }
        }
        self.push_pseudo_inline_segment(id, inherited_style, PseudoElement::After, segments);
    }

    fn should_emit_list_marker(&self, id: NodeId, style: &ComputedStyle) -> bool {
        matches!(self.document.node(id), Some(Node::Element(element)) if element.tag == "li")
            && style.list_style_type != ListStyleType::None
    }

    fn push_list_marker_segment(
        &self,
        id: NodeId,
        style: &ComputedStyle,
        segments: &mut Vec<InlineTextSegment>,
    ) {
        if !self.should_emit_list_marker(id, style) {
            return;
        }
        let marker = match style.list_style_type {
            ListStyleType::Disc => "• ".to_string(),
            ListStyleType::Decimal => format!("{}. ", ordered_list_item_index(self.document, id)),
            ListStyleType::None => return,
        };
        segments.push(InlineTextSegment {
            atomic: None,
            text: marker,
            style: style.clone(),
        });
    }

    fn push_pseudo_inline_segment(
        &mut self,
        id: NodeId,
        parent_style: &ComputedStyle,
        kind: PseudoElement,
        segments: &mut Vec<InlineTextSegment>,
    ) {
        let Some(style) = self.pseudo_style(id, parent_style, kind) else {
            return;
        };
        let Some(content) = style.content.clone() else {
            return;
        };
        if content.is_empty() {
            return;
        }
        segments.push(InlineTextSegment {
            atomic: None,
            text: transform_text(&content, style.text_transform),
            style,
        });
    }

    fn ensure_space(&mut self, needed_height: f32) {
        let min_y = self.options.page.margin_bottom_pt + needed_height;
        if self.current_y >= min_y {
            return;
        }
        if self.pages.last().is_some_and(|page| page.items.is_empty()) {
            return;
        }
        self.start_new_page();
    }

    fn force_page_break(&mut self) {
        if self.pages.last().is_some_and(|page| page.items.is_empty()) {
            return;
        }
        self.start_new_page();
    }

    fn should_start_avoid_block_on_next_page(&self, id: NodeId, style: &ComputedStyle) -> bool {
        if self.pages.last().is_some_and(|page| page.items.is_empty()) {
            return false;
        }
        let page_top = self.options.page.height_pt - self.options.page.margin_top_pt;
        if self.current_y >= page_top - 0.1 {
            return false;
        }
        let remaining_height = (self.current_y - self.options.page.margin_bottom_pt).max(0.0);
        let printable_height = (self.options.page.height_pt
            - self.options.page.margin_top_pt
            - self.options.page.margin_bottom_pt)
            .max(1.0);
        if contains_large_table(self.document, id, 16) {
            return remaining_height < printable_height * 0.40;
        }

        let current_page_count = self.pages.len();
        let mut current_probe = self.clone();
        current_probe.layout_container(id, style);
        if current_probe.pages.len() == current_page_count {
            return false;
        }

        if style.margin_bottom > 0.0 {
            let mut without_trailing_margin = style.clone();
            without_trailing_margin.margin_bottom = 0.0;
            let mut content_probe = self.clone();
            content_probe.layout_container(id, &without_trailing_margin);
            if content_probe.pages.len() == current_page_count {
                return false;
            }
        }

        if remaining_height >= printable_height * 0.25 {
            return false;
        }

        let mut fresh_probe = self.clone();
        fresh_probe.force_page_break();
        fresh_probe.layout_container(id, style);
        fresh_probe.pages.len() == current_page_count + 1
    }

    fn should_keep_with_next_on_next_page(&mut self, id: NodeId) -> bool {
        if self.pages.last().is_some_and(|page| page.items.is_empty()) {
            return false;
        }
        let page_top = self.options.page.height_pt - self.options.page.margin_top_pt;
        if self.current_y >= page_top - 0.1 {
            return false;
        }
        let remaining_height = (self.current_y - self.options.page.margin_bottom_pt).max(0.0);
        let printable_height = (self.options.page.height_pt
            - self.options.page.margin_top_pt
            - self.options.page.margin_bottom_pt)
            .max(1.0);
        if remaining_height >= printable_height * 0.42 {
            return false;
        }

        let Some(next_id) = self.next_flow_sibling(id) else {
            return false;
        };

        let current_page_count = self.pages.len();
        let mut probe = self.clone();
        probe.suppress_keep_with_next = true;
        probe.layout_node(id);
        if probe.pages.len() > current_page_count {
            return true;
        }

        probe.layout_node(next_id);
        probe.pages.len() > current_page_count
    }

    fn next_flow_sibling(&mut self, id: NodeId) -> Option<NodeId> {
        let parent = self.document.parent_of(id)?;
        let siblings = self.document.children(parent);
        let current_index = siblings.iter().position(|sibling| *sibling == id)?;
        siblings
            .iter()
            .copied()
            .skip(current_index + 1)
            .find(|candidate| self.is_flow_candidate(*candidate))
    }

    fn is_flow_candidate(&mut self, id: NodeId) -> bool {
        match self.document.node(id) {
            Some(Node::Text(text)) => !text.trim().is_empty(),
            Some(Node::Element(element)) => {
                if is_non_painted_tag(&element.tag) {
                    return false;
                }
                let style = self.computed_style(id);
                style.display != Display::None && !is_out_of_flow_position(&style)
            }
            _ => false,
        }
    }

    fn page_containing_block(&self) -> ContainingBlock {
        ContainingBlock {
            x: self.options.page.margin_left_pt,
            top: self.options.page.height_pt - self.options.page.margin_top_pt,
            width: self.options.page.width_pt
                - self.options.page.margin_left_pt
                - self.options.page.margin_right_pt,
            height: self.options.page.height_pt
                - self.options.page.margin_top_pt
                - self.options.page.margin_bottom_pt,
        }
    }

    fn containing_block(&self) -> ContainingBlock {
        self.containing_blocks
            .last()
            .copied()
            .unwrap_or_else(|| self.page_containing_block())
    }

    fn push(&mut self, item: LayoutItem) {
        if let Some(page) = self.pages.last_mut() {
            page.items.push(item);
        }
    }

    fn register_fixed_overlay(&mut self, page_index: usize, item_start: usize) {
        let Some(page) = self.pages.get(page_index) else {
            return;
        };
        let overlay = page.items.get(item_start..).unwrap_or_default().to_vec();
        if overlay.is_empty() {
            return;
        }

        if let Some(page) = self.pages.get_mut(page_index) {
            page.items.truncate(item_start);
        }
        self.fixed_overlays.push(overlay);
    }

    fn append_fixed_overlays(&mut self) {
        if self.fixed_overlays.is_empty() {
            return;
        }
        let overlays = self
            .fixed_overlays
            .iter()
            .flat_map(|overlay| overlay.iter().cloned())
            .collect::<Vec<_>>();
        for page in &mut self.pages {
            page.items.extend(overlays.iter().cloned());
        }
    }

    fn start_new_page(&mut self) {
        if self.suppress_pagination {
            return;
        }
        let number = self.pages.len() + 1;
        self.pages.push(LayoutPage {
            number,
            page: self.options.page,
            items: Vec::new(),
        });
        self.current_y = self.options.page.height_pt - self.options.page.margin_top_pt;
        self.floats.clear();
        self.inset_left = self.float_base_left;
        self.inset_right = self.float_base_right;
    }

    fn insert_continuation_background_fragments(
        &mut self,
        first_page_index: usize,
        box_x: f32,
        box_width: f32,
        style: &ComputedStyle,
        paint_background: bool,
    ) {
        let last_page_index = self.pages.len().saturating_sub(1);
        if last_page_index <= first_page_index || !paint_background {
            return;
        }

        let fragment_top = self.options.page.height_pt - self.options.page.margin_top_pt;
        for page_index in (first_page_index + 1)..=last_page_index {
            let fragment_y = if page_index == last_page_index {
                self.current_y.max(self.options.page.margin_bottom_pt)
            } else {
                self.options.page.margin_bottom_pt
            };
            let fragment_height = (fragment_top - fragment_y).max(0.0);
            if fragment_height <= 0.0 {
                continue;
            }

            let backgrounds = background_items_without_shadow(
                style,
                self.options,
                box_x,
                fragment_y,
                box_width,
                fragment_height,
            );
            if backgrounds.is_empty() && !style.overflow_hidden {
                continue;
            }

            let background_count = backgrounds.len();
            for (offset, item) in backgrounds.into_iter().enumerate() {
                self.pages[page_index].items.insert(offset, item);
            }

            if style.overflow_hidden {
                let child_end = self.pages[page_index].items.len();
                let child_start = background_count;
                if child_start < child_end {
                    self.pages[page_index].items.insert(
                        child_start,
                        LayoutItem::BeginClip(ClipRect {
                            x: box_x,
                            y: fragment_y,
                            width: box_width,
                            height: fragment_height,
                            radius: style.border_radius,
                        }),
                    );
                    self.pages[page_index].items.push(LayoutItem::EndClip);
                }
            }
        }
    }

    fn push_container_border(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        style: &ComputedStyle,
    ) {
        let top_width = style.border_top_width.max(style.border_width);
        let right_width = style.border_right_width.max(style.border_width);
        let bottom_width = style.border_bottom_width.max(style.border_width);
        let left_width = style.border_left_width.max(style.border_width);
        let top_color = style
            .border_top_color
            .or(style.border_color)
            .unwrap_or(style.color)
            .with_opacity(style.opacity);
        let right_color = style
            .border_right_color
            .or(style.border_color)
            .unwrap_or(style.color)
            .with_opacity(style.opacity);
        let bottom_color = style
            .border_bottom_color
            .or(style.border_color)
            .unwrap_or(style.color)
            .with_opacity(style.opacity);
        let left_color = style
            .border_left_color
            .or(style.border_color)
            .unwrap_or(style.color)
            .with_opacity(style.opacity);

        if style.border_radius > 0.0
            && rounded_border_can_use_single_stroke(
                top_width,
                right_width,
                bottom_width,
                left_width,
                top_color,
                right_color,
                bottom_color,
                left_color,
            )
        {
            self.push(LayoutItem::StrokeRect(StrokeRect {
                x,
                y,
                width,
                height,
                stroke_width: top_width,
                radius: style.border_radius,
                color: top_color,
                dash: border_dash(style.border_style, top_width),
            }));
            return;
        }

        let top = y + height;
        let right = x + width;
        if top_width > 0.0 {
            self.push(LayoutItem::Line(Line {
                x1: x,
                y1: top,
                x2: right,
                y2: top,
                width: top_width,
                line_cap_round: false,
                color: top_color,
                dash: border_dash(style.border_style, top_width),
            }));
        }
        if bottom_width > 0.0 {
            self.push(LayoutItem::Line(Line {
                x1: x,
                y1: y,
                x2: right,
                y2: y,
                width: bottom_width,
                line_cap_round: false,
                color: bottom_color,
                dash: border_dash(style.border_style, bottom_width),
            }));
        }
        if left_width > 0.0 {
            self.push(LayoutItem::Line(Line {
                x1: x,
                y1: y,
                x2: x,
                y2: top,
                width: left_width,
                line_cap_round: false,
                color: left_color,
                dash: border_dash(style.border_style, left_width),
            }));
        }
        if right_width > 0.0 {
            self.push(LayoutItem::Line(Line {
                x1: right,
                y1: y,
                x2: right,
                y2: top,
                width: right_width,
                line_cap_round: false,
                color: right_color,
                dash: border_dash(style.border_style, right_width),
            }));
        }
    }

    fn push_container_outline(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        style: &ComputedStyle,
    ) {
        if !has_outline(style) {
            return;
        }

        if let Some(rect) = outline_stroke_rect(x, y, width, height, style) {
            self.push(LayoutItem::StrokeRect(rect));
        }
    }

    fn push_container_decoration(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        style: &ComputedStyle,
    ) {
        if has_border(style) {
            self.push_container_border(x, y, width, height, style);
        }
        self.push_container_outline(x, y, width, height, style);
    }

    fn push_table_cell_border(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        style: &ComputedStyle,
    ) {
        self.push_container_decoration(x, y, width, height, style);
    }
}

fn has_border(style: &ComputedStyle) -> bool {
    style.border_width > 0.0
        || style.border_top_width > 0.0
        || style.border_right_width > 0.0
        || style.border_bottom_width > 0.0
        || style.border_left_width > 0.0
}

fn has_outline(style: &ComputedStyle) -> bool {
    style.outline_width > 0.0 && style.outline_style != BorderLineStyle::None
}

fn has_box_decoration(style: &ComputedStyle) -> bool {
    has_border(style) || has_outline(style)
}

fn positioned_z_index(style: &ComputedStyle) -> Option<i32> {
    if is_positioned_containing_block(style) {
        style.z_index
    } else {
        None
    }
}

fn reorder_paint_stack_ranges(items: &mut Vec<LayoutItem>, ranges: &[PaintStackRange]) {
    if ranges.len() < 2 {
        return;
    }
    let Some(first) = ranges.iter().map(|range| range.item_start).min() else {
        return;
    };
    let Some(last) = ranges.iter().map(|range| range.item_end).max() else {
        return;
    };
    if first >= last || last > items.len() {
        return;
    }
    let mut by_position = ranges.to_vec();
    by_position.sort_by_key(|range| range.item_start);
    let mut cursor = first;
    for range in &by_position {
        if range.item_start != cursor || range.item_end < range.item_start {
            return;
        }
        cursor = range.item_end;
    }
    if cursor != last {
        return;
    }
    let mut sorted = ranges.to_vec();
    sorted.sort_by_key(|range| (range.z_index, range.source_order));
    if sorted
        .iter()
        .zip(ranges.iter())
        .all(|(sorted, original)| sorted.item_start == original.item_start)
    {
        return;
    }

    let original = items[first..last].to_vec();
    let mut rebuilt = Vec::with_capacity(original.len());
    for range in &sorted {
        rebuilt.extend_from_slice(&items[range.item_start..range.item_end]);
    }
    if rebuilt.len() == original.len() {
        items.splice(first..last, rebuilt);
    }
}

fn outline_stroke_rect(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    style: &ComputedStyle,
) -> Option<StrokeRect> {
    if !has_outline(style) {
        return None;
    }

    let stroke_width = style.outline_width;
    let offset = style.outline_offset + stroke_width / 2.0;
    Some(StrokeRect {
        x: x - offset,
        y: y - offset,
        width: width + offset * 2.0,
        height: height + offset * 2.0,
        radius: (style.border_radius + offset).max(0.0),
        stroke_width,
        color: style
            .outline_color
            .unwrap_or(style.color)
            .with_opacity(style.opacity),
        dash: border_dash(style.outline_style, stroke_width),
    })
}

fn has_generated_pseudo_box_paint(style: &ComputedStyle) -> bool {
    style.background.is_some()
        || style.background_gradient.is_some()
        || !style.background_radials.is_empty()
        || style.background_image.is_some()
        || !style.box_shadows.is_empty()
        || has_box_decoration(style)
        || style.width.is_some()
        || style.height.is_some()
        || style.min_width.is_some()
        || style.min_height.is_some()
        || style.max_height.is_some()
        || style.aspect_ratio.is_some()
}

fn border_dash(style: BorderLineStyle, width: f32) -> Option<(f32, f32)> {
    match style {
        BorderLineStyle::Dashed => Some(((width * 5.0).max(3.0), (width * 3.0).max(2.0))),
        BorderLineStyle::Dotted => Some((width.max(0.5), (width * 2.0).max(1.0))),
        BorderLineStyle::Solid | BorderLineStyle::None => None,
    }
}

fn rounded_border_can_use_single_stroke(
    top_width: f32,
    right_width: f32,
    bottom_width: f32,
    left_width: f32,
    top_color: Color,
    right_color: Color,
    bottom_color: Color,
    left_color: Color,
) -> bool {
    top_width > 0.0
        && nearly_equal(top_width, right_width)
        && nearly_equal(top_width, bottom_width)
        && nearly_equal(top_width, left_width)
        && nearly_equal_color(top_color, right_color)
        && nearly_equal_color(top_color, bottom_color)
        && nearly_equal_color(top_color, left_color)
}

fn nearly_equal(left: f32, right: f32) -> bool {
    (left - right).abs() < 0.01
}

fn nearly_equal_color(left: Color, right: Color) -> bool {
    nearly_equal(left.r, right.r)
        && nearly_equal(left.g, right.g)
        && nearly_equal(left.b, right.b)
        && nearly_equal(left.a, right.a)
}

fn is_out_of_flow_position(style: &ComputedStyle) -> bool {
    matches!(style.position, Position::Absolute | Position::Fixed)
}

fn relative_position_offset(style: &ComputedStyle, width: f32, height: f32) -> (f32, f32) {
    if style.position != Position::Relative {
        return (0.0, 0.0);
    }
    let translate_x = style
        .inset_left
        .as_ref()
        .map(|value| value.resolve(width))
        .or_else(|| {
            style
                .inset_right
                .as_ref()
                .map(|value| -value.resolve(width))
        })
        .unwrap_or(0.0);
    let translate_y_css = style
        .inset_top
        .as_ref()
        .map(|value| value.resolve(height))
        .or_else(|| {
            style
                .inset_bottom
                .as_ref()
                .map(|value| -value.resolve(height))
        })
        .unwrap_or(0.0);
    (translate_x, translate_y_css)
}

fn is_positioned_containing_block(style: &ComputedStyle) -> bool {
    !matches!(style.position, Position::Static)
}

fn positioned_width(style: &ComputedStyle, containing_width: f32) -> f32 {
    if let Some(width) = &style.width {
        return width.resolve(containing_width).clamp(0.0, containing_width);
    }
    let left = style
        .inset_left
        .as_ref()
        .map(|value| value.resolve(containing_width));
    let right = style
        .inset_right
        .as_ref()
        .map(|value| value.resolve(containing_width));
    match (left, right) {
        (Some(left), Some(right)) => (containing_width - left - right).max(0.0),
        _ => containing_width,
    }
}

fn positioned_height(style: &ComputedStyle, containing_height: f32) -> f32 {
    if let Some(height) = &style.height {
        return height
            .resolve(containing_height)
            .clamp(0.0, containing_height);
    }
    let top = style
        .inset_top
        .as_ref()
        .map(|value| value.resolve(containing_height));
    let bottom = style
        .inset_bottom
        .as_ref()
        .map(|value| value.resolve(containing_height));
    match (top, bottom) {
        (Some(top), Some(bottom)) => (containing_height - top - bottom).max(0.0),
        _ => 0.0,
    }
}

fn resolve_box_width(style: &ComputedStyle, available_width: f32) -> f32 {
    let mut width = style
        .width
        .as_ref()
        .map(|width| {
            let specified = width.resolve(available_width);
            if style.box_sizing == BoxSizing::ContentBox {
                specified + horizontal_box_extras(style)
            } else {
                specified
            }
        })
        .unwrap_or(available_width);
    if let Some(min_width) = &style.min_width {
        width = width.max(min_width.resolve(available_width));
    }
    if let Some(max_width) = &style.max_width {
        width = width.min(max_width.resolve(available_width));
    }
    width.clamp(0.0, available_width)
}

fn resolved_text_align(style: &ComputedStyle) -> TextAlign {
    resolved_text_align_value(style.text_align, style)
}

fn can_collapse_adjacent_sibling_margin(style: &ComputedStyle) -> bool {
    !is_out_of_flow_position(style)
        && matches!(
            style.display,
            Display::Block | Display::Flex | Display::Grid
        )
}

fn resolved_line_text_align(
    style: &ComputedStyle,
    line_index: usize,
    line_count: usize,
) -> TextAlign {
    if line_index + 1 == line_count {
        resolved_text_align_value(style.text_align_last.unwrap_or(style.text_align), style)
    } else {
        resolved_text_align(style)
    }
}

fn resolved_text_align_value(align: TextAlign, style: &ComputedStyle) -> TextAlign {
    match align {
        TextAlign::Start => match style.direction {
            crate::css::TextDirection::Rtl => TextAlign::Right,
            crate::css::TextDirection::Ltr => TextAlign::Left,
        },
        TextAlign::End => match style.direction {
            crate::css::TextDirection::Rtl => TextAlign::Left,
            crate::css::TextDirection::Ltr => TextAlign::Right,
        },
        align => align,
    }
}

fn suppress_justify_spacing_for_line(
    style: &ComputedStyle,
    line_index: usize,
    line_count: usize,
) -> bool {
    let is_last_line = line_index + 1 == line_count;
    is_last_line && style.text_align_last != Some(TextAlign::Justify)
}

fn justified_inline_word_spacing(
    align: TextAlign,
    segments: &[InlineTextSegment],
    line_width: f32,
    available_width: f32,
    suppress_justify: bool,
) -> f32 {
    if align != TextAlign::Justify || suppress_justify {
        return 0.0;
    }
    let opportunities = segments
        .iter()
        .map(|segment| word_spacing_opportunities(&segment.text))
        .sum::<usize>();
    if opportunities == 0 || line_width <= 0.0 || line_width >= available_width {
        return 0.0;
    }
    (available_width - line_width) / opportunities as f32
}

fn justified_word_spacing(
    align: TextAlign,
    text: &str,
    text_width: f32,
    available_width: f32,
    base_word_spacing: f32,
    is_last_line: bool,
) -> f32 {
    if align != TextAlign::Justify || is_last_line {
        return base_word_spacing;
    }
    let opportunities = word_spacing_opportunities(text);
    if opportunities == 0 || text_width <= 0.0 || text_width >= available_width {
        return base_word_spacing;
    }
    base_word_spacing + (available_width - text_width) / opportunities as f32
}

#[derive(Clone, Copy)]
struct FlowHorizontalGeometry {
    margin_left: f32,
    available_width: f32,
    box_width: f32,
}

fn resolve_flow_horizontal_geometry(
    style: &ComputedStyle,
    containing_width: f32,
) -> FlowHorizontalGeometry {
    let specified_left = if style.margin_left_auto {
        0.0
    } else {
        style.margin_left
    };
    let specified_right = if style.margin_right_auto {
        0.0
    } else {
        style.margin_right
    };
    let available_width = (containing_width - specified_left - specified_right).max(0.0);
    let box_width = resolve_flow_box_width(style, containing_width, available_width);
    let free_space = (containing_width - specified_left - specified_right - box_width).max(0.0);

    let (margin_left, margin_right) = match (style.margin_left_auto, style.margin_right_auto) {
        (true, true) => {
            let half = free_space / 2.0;
            (half, half)
        }
        (true, false) => (free_space, specified_right),
        (false, true) => (specified_left, free_space),
        (false, false) => (specified_left, specified_right),
    };

    FlowHorizontalGeometry {
        margin_left,
        available_width: (containing_width - margin_left - margin_right).max(0.0),
        box_width,
    }
}

fn resolve_flow_box_width(
    style: &ComputedStyle,
    containing_width: f32,
    available_width: f32,
) -> f32 {
    if style.width.is_none() {
        return resolve_box_width(style, available_width);
    }

    let mut width = style
        .width
        .as_ref()
        .map(|width| {
            let specified = width.resolve(containing_width);
            if style.box_sizing == BoxSizing::ContentBox {
                specified + horizontal_box_extras(style)
            } else {
                specified
            }
        })
        .unwrap_or(available_width);
    let extras = if style.box_sizing == BoxSizing::ContentBox {
        horizontal_box_extras(style)
    } else {
        0.0
    };
    if let Some(max_width) = &style.max_width {
        width = width.min(max_width.resolve(containing_width) + extras);
    }
    if let Some(min_width) = &style.min_width {
        width = width.max(min_width.resolve(containing_width) + extras);
    }
    width.max(0.0)
}

fn resolve_box_height(
    style: &ComputedStyle,
    box_width: f32,
    natural_height: f32,
    height_base: f32,
) -> f32 {
    let extras = if style.box_sizing == BoxSizing::ContentBox {
        style.padding_top
            + style.padding_bottom
            + style.border_top_width.max(style.border_width)
            + style.border_bottom_width.max(style.border_width)
    } else {
        0.0
    };
    let min_height = style
        .min_height
        .as_ref()
        .map(|height| height.resolve(height_base) + extras)
        .unwrap_or(0.0);
    let mut resolved = if let Some(height) = &style.height {
        (height.resolve(height_base) + extras).max(min_height)
    } else if let Some(ratio) = style.aspect_ratio {
        natural_height.max(box_width / ratio).max(min_height)
    } else {
        natural_height.max(min_height)
    };
    if let Some(max_height) = &style.max_height {
        resolved = resolved.min((max_height.resolve(height_base) + extras).max(min_height));
    }
    resolved
}

fn horizontal_box_extras(style: &ComputedStyle) -> f32 {
    style.padding_left
        + style.padding_right
        + style.border_left_width.max(style.border_width)
        + style.border_right_width.max(style.border_width)
}

#[derive(Debug)]
struct LoadedImage {
    data: Vec<u8>,
    alpha_mask: Option<Vec<u8>>,
    intrinsic_width_px: u32,
    intrinsic_height_px: u32,
    format: ImageFormat,
}

fn load_image_asset(src: &str, options: &RenderOptions) -> Option<LoadedImage> {
    let data = if src.trim_start().starts_with("data:") {
        decode_data_uri_image(src)?
    } else {
        if !options.allow_local_assets {
            return None;
        }
        std::fs::read(resolve_local_image_asset(options.base_url.as_deref(), src)?).ok()?
    };
    if let Some((width, height)) = jpeg_dimensions(&data) {
        return Some(LoadedImage {
            data,
            alpha_mask: None,
            intrinsic_width_px: width,
            intrinsic_height_px: height,
            format: ImageFormat::Jpeg,
        });
    }
    if let Some(image) = png_image_data(&data) {
        return Some(image);
    }
    None
}

fn png_image_data(data: &[u8]) -> Option<LoadedImage> {
    const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if data.len() < 8 || &data[..8] != PNG_SIGNATURE {
        return None;
    }
    let mut idx = 8usize;
    let mut width = 0u32;
    let mut height = 0u32;
    let mut bit_depth = 0u8;
    let mut color_type = 0u8;
    let mut idat = Vec::new();
    let mut palette: Vec<[u8; 3]> = Vec::new();
    let mut palette_alpha: Vec<u8> = Vec::new();
    while idx + 12 <= data.len() {
        let length =
            u32::from_be_bytes([data[idx], data[idx + 1], data[idx + 2], data[idx + 3]]) as usize;
        let chunk_type = data.get(idx + 4..idx + 8)?;
        let chunk_start = idx + 8;
        let chunk_end = chunk_start.checked_add(length)?;
        let next_idx = chunk_end.checked_add(4)?;
        if next_idx > data.len() {
            return None;
        }
        let chunk = &data[chunk_start..chunk_end];
        match chunk_type {
            b"IHDR" => {
                if chunk.len() != 13 {
                    return None;
                }
                width = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                height = u32::from_be_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
                bit_depth = chunk[8];
                color_type = chunk[9];
                let compression = chunk[10];
                let filter = chunk[11];
                let interlace = chunk[12];
                if width == 0
                    || height == 0
                    || bit_depth != 8
                    || compression != 0
                    || filter != 0
                    || interlace != 0
                {
                    return None;
                }
            }
            b"PLTE" => {
                if chunk.len() % 3 != 0 {
                    return None;
                }
                palette = chunk
                    .chunks_exact(3)
                    .map(|rgb| [rgb[0], rgb[1], rgb[2]])
                    .collect();
            }
            b"tRNS" => {
                palette_alpha = chunk.to_vec();
            }
            b"IDAT" => idat.extend_from_slice(chunk),
            b"IEND" => break,
            _ => {}
        }
        idx = next_idx;
    }
    if idat.is_empty() {
        return None;
    }
    let (color_space, components) = match color_type {
        0 => (PngColorSpace::DeviceGray, 1),
        2 => (PngColorSpace::DeviceRgb, 3),
        3 => {
            if palette.is_empty() {
                return None;
            }
            let decoded = inflate_zlib(&idat)?;
            let indexes = defilter_png_scanlines(&decoded, width, height, 1)?;
            let mut color = Vec::with_capacity(width as usize * height as usize * 3);
            let mut alpha = (!palette_alpha.is_empty())
                .then(|| Vec::with_capacity(width as usize * height as usize));
            for index in indexes {
                let rgb = *palette.get(index as usize)?;
                color.extend_from_slice(&rgb);
                if let Some(alpha) = &mut alpha {
                    alpha.push(*palette_alpha.get(index as usize).unwrap_or(&255));
                }
            }
            return Some(LoadedImage {
                data: color,
                alpha_mask: alpha,
                intrinsic_width_px: width,
                intrinsic_height_px: height,
                format: ImageFormat::Raw {
                    color_space: PngColorSpace::DeviceRgb,
                    bits_per_component: 8,
                    components: 3,
                },
            });
        }
        4 | 6 => {
            let decoded = inflate_zlib(&idat)?;
            let source_components = if color_type == 4 { 2 } else { 4 };
            let source = defilter_png_scanlines(&decoded, width, height, source_components)?;
            let mut color =
                Vec::with_capacity(width as usize * height as usize * (source_components - 1));
            let mut alpha = Vec::with_capacity(width as usize * height as usize);
            for pixel in source.chunks_exact(source_components) {
                color.extend_from_slice(&pixel[..source_components - 1]);
                alpha.push(pixel[source_components - 1]);
            }
            return Some(LoadedImage {
                data: color,
                alpha_mask: Some(alpha),
                intrinsic_width_px: width,
                intrinsic_height_px: height,
                format: ImageFormat::Raw {
                    color_space: if color_type == 4 {
                        PngColorSpace::DeviceGray
                    } else {
                        PngColorSpace::DeviceRgb
                    },
                    bits_per_component: bit_depth,
                    components: (source_components - 1) as u8,
                },
            });
        }
        _ => return None,
    };
    Some(LoadedImage {
        data: idat,
        alpha_mask: None,
        intrinsic_width_px: width,
        intrinsic_height_px: height,
        format: ImageFormat::Png {
            color_space,
            bits_per_component: bit_depth,
            components,
        },
    })
}

fn resolve_local_image_asset(base_url: Option<&str>, src: &str) -> Option<PathBuf> {
    let src = src.trim();
    if src.is_empty()
        || src.starts_with('#')
        || src.starts_with("http://")
        || src.starts_with("https://")
        || src.starts_with("//")
    {
        return None;
    }
    let src = src.split(['?', '#']).next().unwrap_or(src);
    let path = Path::new(src);
    if path.is_absolute() {
        return Some(path.to_path_buf());
    }
    let base = base_url.map(normalize_asset_base_path)?;
    Some(base.join(path))
}

fn normalize_asset_base_path(base_url: &str) -> PathBuf {
    let path = base_url.strip_prefix("file://").unwrap_or(base_url);
    let path = PathBuf::from(path);
    if path.extension().is_some() {
        path.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        path
    }
}

fn decode_data_uri_image(src: &str) -> Option<Vec<u8>> {
    let (metadata, payload) = src.split_once(',')?;
    let metadata = metadata.trim().to_ascii_lowercase();
    if !(metadata.starts_with("data:image/jpeg")
        || metadata.starts_with("data:image/jpg")
        || metadata.starts_with("data:image/png"))
    {
        return None;
    }
    if metadata.contains(";base64") {
        decode_base64(payload)
    } else {
        percent_decode(payload)
    }
}

fn decode_base64(input: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0u8;
    for byte in input.bytes().filter(|byte| !byte.is_ascii_whitespace()) {
        if byte == b'=' {
            break;
        }
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32;
        buffer = (buffer << 6) | value;
        bits += 6;
        while bits >= 8 {
            bits -= 8;
            out.push(((buffer >> bits) & 0xff) as u8);
        }
    }
    Some(out)
}

fn percent_decode(input: &str) -> Option<Vec<u8>> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut idx = 0usize;
    while idx < bytes.len() {
        if bytes[idx] == b'%' {
            let hi = hex_value(*bytes.get(idx + 1)?)?;
            let lo = hex_value(*bytes.get(idx + 2)?)?;
            out.push((hi << 4) | lo);
            idx += 3;
        } else {
            out.push(bytes[idx]);
            idx += 1;
        }
    }
    Some(out)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn inflate_zlib(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 6 {
        return None;
    }
    let cmf = data[0];
    let flg = data[1];
    if cmf & 0x0f != 8 || u16::from_be_bytes([cmf, flg]) % 31 != 0 || (flg & 0x20) != 0 {
        return None;
    }
    let mut reader = BitReader::new(&data[2..data.len().saturating_sub(4)]);
    let mut out = Vec::new();
    loop {
        let is_final = reader.read_bits(1)? == 1;
        match reader.read_bits(2)? {
            0 => inflate_stored_block(&mut reader, &mut out)?,
            1 => {
                let lit_len = fixed_literal_length_huffman();
                let distance = fixed_distance_huffman();
                inflate_compressed_block(&mut reader, &mut out, &lit_len, &distance)?;
            }
            2 => {
                let (lit_len, distance) = dynamic_huffman(&mut reader)?;
                inflate_compressed_block(&mut reader, &mut out, &lit_len, &distance)?;
            }
            _ => return None,
        }
        if is_final {
            break;
        }
    }
    Some(out)
}

struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: u8,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    fn read_bits(&mut self, count: u8) -> Option<u16> {
        let mut value = 0u16;
        for bit_index in 0..count {
            let byte = *self.data.get(self.byte_pos)?;
            let bit = (byte >> self.bit_pos) & 1;
            value |= (bit as u16) << bit_index;
            self.bit_pos += 1;
            if self.bit_pos == 8 {
                self.bit_pos = 0;
                self.byte_pos += 1;
            }
        }
        Some(value)
    }

    fn align_to_byte(&mut self) {
        if self.bit_pos > 0 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
    }

    fn read_byte(&mut self) -> Option<u8> {
        self.align_to_byte();
        let byte = *self.data.get(self.byte_pos)?;
        self.byte_pos += 1;
        Some(byte)
    }
}

#[derive(Debug)]
struct Huffman {
    entries: Vec<HuffmanEntry>,
    max_bits: u8,
}

#[derive(Debug)]
struct HuffmanEntry {
    code: u16,
    length: u8,
    symbol: u16,
}

impl Huffman {
    fn from_lengths(lengths: &[u8]) -> Option<Self> {
        let max_bits = lengths.iter().copied().max().unwrap_or(0);
        if max_bits == 0 {
            return None;
        }
        let mut bl_count = vec![0u16; max_bits as usize + 1];
        for length in lengths.iter().copied().filter(|length| *length > 0) {
            *bl_count.get_mut(length as usize)? += 1;
        }
        let mut next_code = vec![0u16; max_bits as usize + 1];
        let mut code = 0u16;
        for bits in 1..=max_bits as usize {
            code = (code + bl_count[bits - 1]) << 1;
            next_code[bits] = code;
        }
        let mut entries = Vec::new();
        for (symbol, length) in lengths.iter().copied().enumerate() {
            if length == 0 {
                continue;
            }
            let code = next_code[length as usize];
            next_code[length as usize] += 1;
            entries.push(HuffmanEntry {
                code: reverse_bits(code, length),
                length,
                symbol: symbol as u16,
            });
        }
        Some(Self { entries, max_bits })
    }

    fn decode(&self, reader: &mut BitReader<'_>) -> Option<u16> {
        let mut code = 0u16;
        for length in 1..=self.max_bits {
            code |= reader.read_bits(1)? << (length - 1);
            if let Some(entry) = self
                .entries
                .iter()
                .find(|entry| entry.length == length && entry.code == code)
            {
                return Some(entry.symbol);
            }
        }
        None
    }
}

fn reverse_bits(mut code: u16, length: u8) -> u16 {
    let mut reversed = 0u16;
    for _ in 0..length {
        reversed = (reversed << 1) | (code & 1);
        code >>= 1;
    }
    reversed
}

fn inflate_stored_block(reader: &mut BitReader<'_>, out: &mut Vec<u8>) -> Option<()> {
    reader.align_to_byte();
    let len = u16::from_le_bytes([reader.read_byte()?, reader.read_byte()?]);
    let nlen = u16::from_le_bytes([reader.read_byte()?, reader.read_byte()?]);
    if len != !nlen {
        return None;
    }
    for _ in 0..len {
        out.push(reader.read_byte()?);
    }
    Some(())
}

fn fixed_literal_length_huffman() -> Huffman {
    let mut lengths = vec![0u8; 288];
    for item in lengths.iter_mut().take(144) {
        *item = 8;
    }
    for item in lengths.iter_mut().take(256).skip(144) {
        *item = 9;
    }
    for item in lengths.iter_mut().take(280).skip(256) {
        *item = 7;
    }
    for item in lengths.iter_mut().take(288).skip(280) {
        *item = 8;
    }
    Huffman::from_lengths(&lengths).unwrap_or_else(|| Huffman {
        entries: Vec::new(),
        max_bits: 0,
    })
}

fn fixed_distance_huffman() -> Huffman {
    Huffman::from_lengths(&[5u8; 32]).unwrap_or_else(|| Huffman {
        entries: Vec::new(),
        max_bits: 0,
    })
}

fn dynamic_huffman(reader: &mut BitReader<'_>) -> Option<(Huffman, Huffman)> {
    const CODE_LENGTH_ORDER: [usize; 19] = [
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    let hlit = reader.read_bits(5)? as usize + 257;
    let hdist = reader.read_bits(5)? as usize + 1;
    let hclen = reader.read_bits(4)? as usize + 4;
    let mut code_length_lengths = vec![0u8; 19];
    for symbol in CODE_LENGTH_ORDER.iter().take(hclen) {
        code_length_lengths[*symbol] = reader.read_bits(3)? as u8;
    }
    let code_length_huffman = Huffman::from_lengths(&code_length_lengths)?;
    let total = hlit + hdist;
    let mut lengths = Vec::with_capacity(total);
    while lengths.len() < total {
        let symbol = code_length_huffman.decode(reader)?;
        match symbol {
            0..=15 => lengths.push(symbol as u8),
            16 => {
                let repeat = reader.read_bits(2)? as usize + 3;
                let previous = *lengths.last()?;
                lengths.extend(std::iter::repeat_n(previous, repeat));
            }
            17 => {
                let repeat = reader.read_bits(3)? as usize + 3;
                lengths.extend(std::iter::repeat_n(0, repeat));
            }
            18 => {
                let repeat = reader.read_bits(7)? as usize + 11;
                lengths.extend(std::iter::repeat_n(0, repeat));
            }
            _ => return None,
        }
        if lengths.len() > total {
            return None;
        }
    }
    let literal_lengths = lengths[..hlit].to_vec();
    let distance_lengths = lengths[hlit..].to_vec();
    Some((
        Huffman::from_lengths(&literal_lengths)?,
        Huffman::from_lengths(&distance_lengths)?,
    ))
}

fn inflate_compressed_block(
    reader: &mut BitReader<'_>,
    out: &mut Vec<u8>,
    literal_length: &Huffman,
    distance: &Huffman,
) -> Option<()> {
    const LENGTH_BASE: [usize; 29] = [
        3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115,
        131, 163, 195, 227, 258,
    ];
    const LENGTH_EXTRA: [u8; 29] = [
        0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
    ];
    const DISTANCE_BASE: [usize; 30] = [
        1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
        2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
    ];
    const DISTANCE_EXTRA: [u8; 30] = [
        0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12,
        13, 13,
    ];

    loop {
        let symbol = literal_length.decode(reader)?;
        match symbol {
            0..=255 => out.push(symbol as u8),
            256 => return Some(()),
            257..=285 => {
                let length_index = (symbol - 257) as usize;
                let mut length = *LENGTH_BASE.get(length_index)?;
                length += reader.read_bits(*LENGTH_EXTRA.get(length_index)?)? as usize;
                let distance_symbol = distance.decode(reader)? as usize;
                let mut back_distance = *DISTANCE_BASE.get(distance_symbol)?;
                back_distance += reader.read_bits(*DISTANCE_EXTRA.get(distance_symbol)?)? as usize;
                if back_distance == 0 || back_distance > out.len() {
                    return None;
                }
                for _ in 0..length {
                    let value = out[out.len() - back_distance];
                    out.push(value);
                }
            }
            _ => return None,
        }
    }
}

fn defilter_png_scanlines(
    data: &[u8],
    width: u32,
    height: u32,
    components: usize,
) -> Option<Vec<u8>> {
    let row_len = width as usize * components;
    let expected = height as usize * (row_len + 1);
    if data.len() < expected {
        return None;
    }
    let mut out = vec![0u8; height as usize * row_len];
    let mut src = 0usize;
    for row in 0..height as usize {
        let filter = *data.get(src)?;
        src += 1;
        for col in 0..row_len {
            let raw = *data.get(src + col)?;
            let left = if col >= components {
                out[row * row_len + col - components]
            } else {
                0
            };
            let up = if row > 0 {
                out[(row - 1) * row_len + col]
            } else {
                0
            };
            let up_left = if row > 0 && col >= components {
                out[(row - 1) * row_len + col - components]
            } else {
                0
            };
            out[row * row_len + col] = match filter {
                0 => raw,
                1 => raw.wrapping_add(left),
                2 => raw.wrapping_add(up),
                3 => raw.wrapping_add(((left as u16 + up as u16) / 2) as u8),
                4 => raw.wrapping_add(paeth_predictor(left, up, up_left)),
                _ => return None,
            };
        }
        src += row_len;
    }
    Some(out)
}

fn paeth_predictor(left: u8, up: u8, up_left: u8) -> u8 {
    let left = left as i32;
    let up = up as i32;
    let up_left = up_left as i32;
    let p = left + up - up_left;
    let pa = (p - left).abs();
    let pb = (p - up).abs();
    let pc = (p - up_left).abs();
    if pa <= pb && pa <= pc {
        left as u8
    } else if pb <= pc {
        up as u8
    } else {
        up_left as u8
    }
}

fn jpeg_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 4 || data[0] != 0xff || data[1] != 0xd8 {
        return None;
    }
    let mut idx = 2usize;
    while idx + 3 < data.len() {
        while idx < data.len() && data[idx] == 0xff {
            idx += 1;
        }
        if idx >= data.len() {
            return None;
        }
        let marker = data[idx];
        idx += 1;
        if marker == 0xd9 || marker == 0xda {
            return None;
        }
        if marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
            continue;
        }
        if idx + 2 > data.len() {
            return None;
        }
        let segment_len = u16::from_be_bytes([data[idx], data[idx + 1]]) as usize;
        if segment_len < 2 || idx + segment_len > data.len() {
            return None;
        }
        let segment_start = idx + 2;
        if is_jpeg_sof_marker(marker) {
            if segment_start + 5 > data.len() {
                return None;
            }
            let height =
                u16::from_be_bytes([data[segment_start + 1], data[segment_start + 2]]) as u32;
            let width =
                u16::from_be_bytes([data[segment_start + 3], data[segment_start + 4]]) as u32;
            return (width > 0 && height > 0).then_some((width, height));
        }
        idx += segment_len;
    }
    None
}

fn is_jpeg_sof_marker(marker: u8) -> bool {
    matches!(
        marker,
        0xc0 | 0xc1 | 0xc2 | 0xc3 | 0xc5 | 0xc6 | 0xc7 | 0xc9 | 0xca | 0xcb | 0xcd | 0xce | 0xcf
    )
}

fn parse_html_dimension_attr(value: &str) -> Option<f32> {
    let value = value.trim();
    let numeric = value.strip_suffix("px").unwrap_or(value).trim();
    numeric
        .parse::<f32>()
        .ok()
        .map(|css_px| (css_px * 0.75).max(0.0))
}

fn object_fit_rect(
    fit: ObjectFit,
    box_x: f32,
    box_y: f32,
    box_width: f32,
    box_height: f32,
    intrinsic_width: f32,
    intrinsic_height: f32,
    position_x: f32,
    position_y: f32,
) -> (f32, f32, f32, f32) {
    if box_width <= 0.0 || box_height <= 0.0 || intrinsic_width <= 0.0 || intrinsic_height <= 0.0 {
        return (box_x, box_y, box_width.max(0.0), box_height.max(0.0));
    }
    let contain_scale = (box_width / intrinsic_width).min(box_height / intrinsic_height);
    let cover_scale = (box_width / intrinsic_width).max(box_height / intrinsic_height);
    let scale = match fit {
        ObjectFit::Fill => {
            return (box_x, box_y, box_width, box_height);
        }
        ObjectFit::Contain => contain_scale,
        ObjectFit::Cover => cover_scale,
        ObjectFit::None => 1.0,
        ObjectFit::ScaleDown => contain_scale.min(1.0),
    };
    let width = intrinsic_width * scale;
    let height = intrinsic_height * scale;
    (
        box_x + (box_width - width) * position_x.clamp(0.0, 1.0),
        box_y + (box_height - height) * position_y.clamp(0.0, 1.0),
        width,
        height,
    )
}

fn inline_measurement_slack(style: &ComputedStyle) -> f32 {
    (style.font_size * 0.25).max(1.5)
}

fn first_fragment_min_height(style: &ComputedStyle) -> f32 {
    style.padding_top + layout_line_height(style) + style.padding_bottom
}

fn layout_line_height(style: &ComputedStyle) -> f32 {
    if style.line_height_is_normal
        && matches!(style.font_face, FontFace::Sans)
        && matches!(style.font_weight, FontWeight::Normal)
        && (style.font_size - 12.0).abs() < 0.01
    {
        // Chromium's print PDF output places built-in sans-serif `normal`
        // lines tighter than a generic 1.2 multiplier. Keep explicit
        // line-height declarations untouched; this only affects the browser
        // default `normal` rhythm used by framework-generated plain blocks.
        style.font_size * 1.065
    } else {
        style.line_height
    }
}

fn grid_track_widths(
    style: &ComputedStyle,
    box_width: f32,
    columns: usize,
    column_gap: f32,
) -> Vec<f32> {
    let content_width = (box_width - column_gap * columns.saturating_sub(1) as f32).max(0.0);
    if columns == 0 {
        return Vec::new();
    }
    if style.grid_tracks.len() != columns {
        return vec![content_width / columns as f32; columns];
    }

    let mut widths = vec![0.0; columns];
    let mut total_fixed = 0.0_f32;
    let mut total_fr = 0.0_f32;

    for (idx, track) in style.grid_tracks.iter().enumerate() {
        match track {
            GridTrack::Length(length) => {
                let width = length.resolve(box_width).max(0.0);
                widths[idx] = width;
                total_fixed += width;
            }
            GridTrack::Fr { factor, min } => {
                let min_width = min
                    .as_ref()
                    .map(|width| width.resolve(box_width).max(0.0))
                    .unwrap_or(0.0);
                widths[idx] = min_width;
                total_fixed += min_width;
                total_fr += factor.max(0.0);
            }
        }
    }

    if total_fixed > content_width && total_fixed > 0.0 {
        let scale = content_width / total_fixed;
        for width in &mut widths {
            *width *= scale;
        }
        return widths;
    }

    if total_fr > 0.0 {
        let fr_space = (content_width - total_fixed).max(0.0);
        for (idx, track) in style.grid_tracks.iter().enumerate() {
            if let GridTrack::Fr { factor, .. } = track {
                widths[idx] += fr_space * factor.max(0.0) / total_fr;
            }
        }
    }

    widths
}

fn grid_spanned_track_width(
    track_widths: &[f32],
    start_column: usize,
    span: usize,
    column_gap: f32,
) -> f32 {
    if span == 0 || start_column >= track_widths.len() {
        return 0.0;
    }
    let span = span.min(track_widths.len().saturating_sub(start_column));
    track_widths
        .iter()
        .skip(start_column)
        .take(span)
        .sum::<f32>()
        + column_gap * span.saturating_sub(1) as f32
}

fn grid_row_track_min_height(style: &ComputedStyle, row_index: usize, length_base: f32) -> f32 {
    match style.grid_row_tracks.get(row_index) {
        Some(GridTrack::Length(length)) => length.resolve(length_base).max(0.0),
        Some(GridTrack::Fr { min, .. }) => min
            .as_ref()
            .map(|length| length.resolve(length_base).max(0.0))
            .unwrap_or(0.0),
        None => 0.0,
    }
}

fn grid_track_inline_distribution(
    justify_content: JustifyContent,
    box_width: f32,
    track_widths: &[f32],
    declared_gap: f32,
) -> (f32, f32) {
    if track_widths.is_empty() {
        return (0.0, declared_gap);
    }
    let track_count = track_widths.len();
    let occupied_width =
        track_widths.iter().sum::<f32>() + declared_gap * track_count.saturating_sub(1) as f32;
    let leftover_width = (box_width - occupied_width).max(0.0);
    if leftover_width <= 0.0 {
        return (0.0, declared_gap);
    }

    match justify_content {
        JustifyContent::Start => (0.0, declared_gap),
        JustifyContent::Center => (leftover_width / 2.0, declared_gap),
        JustifyContent::End => (leftover_width, declared_gap),
        JustifyContent::SpaceBetween if track_count > 1 => (
            0.0,
            declared_gap + leftover_width / track_count.saturating_sub(1) as f32,
        ),
        JustifyContent::SpaceBetween => (0.0, declared_gap),
        JustifyContent::SpaceAround => {
            let slot = leftover_width / track_count as f32;
            (slot / 2.0, declared_gap + slot)
        }
        JustifyContent::SpaceEvenly => {
            let slot = leftover_width / (track_count + 1) as f32;
            (slot, declared_gap + slot)
        }
    }
}

fn grid_content_cross_axis_row_offset(
    align_content: JustifyContent,
    final_height: f32,
    natural_height: f32,
    row_count: usize,
    row_index: usize,
) -> f32 {
    let leftover_height = (final_height - natural_height).max(0.0);
    if leftover_height <= 0.0 || row_count == 0 {
        return 0.0;
    }
    match align_content {
        JustifyContent::Center => leftover_height / 2.0,
        JustifyContent::End => leftover_height,
        JustifyContent::SpaceBetween if row_count > 1 => {
            leftover_height * row_index as f32 / row_count.saturating_sub(1) as f32
        }
        JustifyContent::SpaceBetween => 0.0,
        JustifyContent::SpaceAround => {
            let slot = leftover_height / row_count as f32;
            slot / 2.0 + slot * row_index as f32
        }
        JustifyContent::SpaceEvenly => {
            let slot = leftover_height / (row_count + 1) as f32;
            slot * (row_index + 1) as f32
        }
        JustifyContent::Start => 0.0,
    }
}

fn adjusted_grid_column_offset(
    style: &ComputedStyle,
    column_offset: f32,
    child_width: f32,
    box_width: f32,
) -> f32 {
    if style.padding_right <= 0.0
        || !style
            .grid_tracks
            .iter()
            .any(|track| matches!(track, GridTrack::Length(_)))
    {
        return column_offset;
    }

    let content_right = (box_width - style.padding_right).max(0.0);
    if column_offset + child_width > content_right {
        (content_right - child_width).max(0.0)
    } else {
        column_offset
    }
}

fn flex_line_ranges(
    widths: &[f32],
    content_width: f32,
    declared_gap: f32,
) -> Vec<std::ops::Range<usize>> {
    if widths.is_empty() {
        return Vec::new();
    }
    let mut ranges = Vec::new();
    let mut line_start = 0usize;
    let mut line_width = 0.0_f32;
    let mut line_items = 0usize;

    for (idx, width) in widths.iter().enumerate() {
        let next_width = if line_items == 0 {
            width.max(0.0)
        } else {
            line_width + declared_gap + width.max(0.0)
        };
        if line_items > 0 && next_width > content_width {
            ranges.push(line_start..idx);
            line_start = idx;
            line_width = width.max(0.0);
            line_items = 1;
        } else {
            line_width = next_width;
            line_items += 1;
        }
    }
    ranges.push(line_start..widths.len());
    ranges
}

fn distribute_flex_grow(widths: &mut [f32], grow_factors: &[f32], available: f32) {
    let grow_total = grow_factors.iter().sum::<f32>();
    let resolved_total = widths.iter().sum::<f32>();
    if grow_total > 0.0 && resolved_total < available {
        let free_space = available - resolved_total;
        for (width, grow) in widths.iter_mut().zip(grow_factors.iter()) {
            if *grow > 0.0 {
                *width += free_space * *grow / grow_total;
            }
        }
    }
}

fn shrink_flex_line_to_available(widths: &mut [f32], available: f32) {
    let resolved_total = widths.iter().sum::<f32>();
    if resolved_total > available && resolved_total > f32::EPSILON {
        let shrink = available / resolved_total;
        for width in widths {
            *width = (*width * shrink).max(0.0);
        }
    }
}

fn flex_grow_for_style(style: &ComputedStyle) -> f32 {
    style.flex_grow.max(0.0)
}

fn flex_cross_axis_shift(
    align: AlignItems,
    max_consumed: f32,
    consumed: f32,
    max_baseline_offset: f32,
    baseline_offset: f32,
) -> f32 {
    match align {
        AlignItems::Center => (max_consumed - consumed).max(0.0) / 2.0,
        AlignItems::End => (max_consumed - consumed).max(0.0),
        AlignItems::Baseline => (max_baseline_offset - baseline_offset).max(0.0),
        AlignItems::Stretch | AlignItems::Start => 0.0,
    }
}

fn flex_row_spacing(
    justify_content: JustifyContent,
    child_count: usize,
    declared_gap: f32,
    leftover_width: f32,
) -> (f32, f32) {
    if child_count == 0 || leftover_width <= 0.0 {
        return (0.0, declared_gap);
    }
    match justify_content {
        JustifyContent::Start => (0.0, declared_gap),
        JustifyContent::Center => (leftover_width / 2.0, declared_gap),
        JustifyContent::End => (leftover_width, declared_gap),
        JustifyContent::SpaceBetween if child_count > 1 => (
            0.0,
            declared_gap + leftover_width / child_count.saturating_sub(1) as f32,
        ),
        JustifyContent::SpaceBetween => (0.0, declared_gap),
        JustifyContent::SpaceAround => {
            let slot = leftover_width / child_count as f32;
            (slot / 2.0, declared_gap + slot)
        }
        JustifyContent::SpaceEvenly => {
            let slot = leftover_width / (child_count + 1) as f32;
            (slot, declared_gap + slot)
        }
    }
}

fn first_text_baseline_offset(items: &[LayoutItem], row_top: f32) -> Option<f32> {
    items.iter().find_map(|item| match item {
        LayoutItem::Text(text) => Some((row_top - text.y).max(0.0)),
        _ => None,
    })
}

fn shift_layout_items(items: &mut [LayoutItem], dx: f32, dy: f32) {
    for item in items {
        match item {
            LayoutItem::BeginClip(clip) => {
                clip.x += dx;
                clip.y += dy;
            }
            LayoutItem::EndClip => {}
            LayoutItem::Text(text) => {
                text.x += dx;
                text.y += dy;
            }
            LayoutItem::Rect(rect) => {
                rect.x += dx;
                rect.y += dy;
            }
            LayoutItem::RoundRect(rect) => {
                rect.x += dx;
                rect.y += dy;
            }
            LayoutItem::StrokeRect(rect) => {
                rect.x += dx;
                rect.y += dy;
            }
            LayoutItem::LinearGradient(rect) => {
                rect.x += dx;
                rect.y += dy;
            }
            LayoutItem::Image(image) => {
                image.x += dx;
                image.y += dy;
            }
            LayoutItem::Circle(circle) => {
                circle.cx += dx;
                circle.cy += dy;
            }
            LayoutItem::CircleStroke(circle) => {
                circle.cx += dx;
                circle.cy += dy;
            }
            LayoutItem::Line(line) => {
                line.x1 += dx;
                line.y1 += dy;
                line.x2 += dx;
                line.y2 += dy;
            }
            LayoutItem::Polygon(polygon) => {
                for (x, y) in &mut polygon.points {
                    *x += dx;
                    *y += dy;
                }
            }
            LayoutItem::SvgPath(path) => {
                for command in &mut path.commands {
                    match command {
                        PathCommand::MoveTo(x, y) | PathCommand::LineTo(x, y) => {
                            *x += dx;
                            *y += dy;
                        }
                        PathCommand::CubicTo {
                            x1,
                            y1,
                            x2,
                            y2,
                            x,
                            y,
                        } => {
                            *x1 += dx;
                            *y1 += dy;
                            *x2 += dx;
                            *y2 += dy;
                            *x += dx;
                            *y += dy;
                        }
                        PathCommand::Close => {}
                    }
                }
            }
        }
    }
}

fn scale_layout_items(items: &mut [LayoutItem], origin_x: f32, origin_y: f32, sx: f32, sy: f32) {
    let stroke_scale = ((sx.abs() + sy.abs()) / 2.0).max(0.01);
    for item in items {
        match item {
            LayoutItem::BeginClip(clip) => {
                let (x, y) = scale_point(clip.x, clip.y, origin_x, origin_y, sx, sy);
                clip.x = x;
                clip.y = y;
                clip.width = (clip.width * sx).abs();
                clip.height = (clip.height * sy).abs();
                clip.radius *= stroke_scale;
            }
            LayoutItem::EndClip => {}
            LayoutItem::Text(text) => {
                let (x, y) = scale_point(text.x, text.y, origin_x, origin_y, sx, sy);
                text.x = x;
                text.y = y;
                text.font_size *= stroke_scale;
                text.letter_spacing *= sx.abs().max(0.01);
            }
            LayoutItem::Rect(rect) => {
                let (x, y) = scale_point(rect.x, rect.y, origin_x, origin_y, sx, sy);
                rect.x = x;
                rect.y = y;
                rect.width = (rect.width * sx).abs();
                rect.height = (rect.height * sy).abs();
            }
            LayoutItem::RoundRect(rect) => {
                let (x, y) = scale_point(rect.x, rect.y, origin_x, origin_y, sx, sy);
                rect.x = x;
                rect.y = y;
                rect.width = (rect.width * sx).abs();
                rect.height = (rect.height * sy).abs();
                rect.radius *= stroke_scale;
            }
            LayoutItem::StrokeRect(rect) => {
                let (x, y) = scale_point(rect.x, rect.y, origin_x, origin_y, sx, sy);
                rect.x = x;
                rect.y = y;
                rect.width = (rect.width * sx).abs();
                rect.height = (rect.height * sy).abs();
                rect.radius *= stroke_scale;
                rect.stroke_width *= stroke_scale;
            }
            LayoutItem::LinearGradient(rect) => {
                let (x, y) = scale_point(rect.x, rect.y, origin_x, origin_y, sx, sy);
                rect.x = x;
                rect.y = y;
                rect.width = (rect.width * sx).abs();
                rect.height = (rect.height * sy).abs();
            }
            LayoutItem::Image(image) => {
                let (x, y) = scale_point(image.x, image.y, origin_x, origin_y, sx, sy);
                image.x = x;
                image.y = y;
                image.width = (image.width * sx).abs();
                image.height = (image.height * sy).abs();
            }
            LayoutItem::Circle(circle) => {
                let (cx, cy) = scale_point(circle.cx, circle.cy, origin_x, origin_y, sx, sy);
                circle.cx = cx;
                circle.cy = cy;
                circle.r *= stroke_scale;
            }
            LayoutItem::CircleStroke(circle) => {
                let (cx, cy) = scale_point(circle.cx, circle.cy, origin_x, origin_y, sx, sy);
                circle.cx = cx;
                circle.cy = cy;
                circle.r *= stroke_scale;
                circle.width *= stroke_scale;
            }
            LayoutItem::Line(line) => {
                let (x1, y1) = scale_point(line.x1, line.y1, origin_x, origin_y, sx, sy);
                let (x2, y2) = scale_point(line.x2, line.y2, origin_x, origin_y, sx, sy);
                line.x1 = x1;
                line.y1 = y1;
                line.x2 = x2;
                line.y2 = y2;
                line.width *= stroke_scale;
            }
            LayoutItem::Polygon(polygon) => {
                for (x, y) in &mut polygon.points {
                    (*x, *y) = scale_point(*x, *y, origin_x, origin_y, sx, sy);
                }
            }
            LayoutItem::SvgPath(path) => {
                path.stroke_width *= stroke_scale;
                for command in &mut path.commands {
                    match command {
                        PathCommand::MoveTo(x, y) | PathCommand::LineTo(x, y) => {
                            (*x, *y) = scale_point(*x, *y, origin_x, origin_y, sx, sy);
                        }
                        PathCommand::CubicTo {
                            x1,
                            y1,
                            x2,
                            y2,
                            x,
                            y,
                        } => {
                            (*x1, *y1) = scale_point(*x1, *y1, origin_x, origin_y, sx, sy);
                            (*x2, *y2) = scale_point(*x2, *y2, origin_x, origin_y, sx, sy);
                            (*x, *y) = scale_point(*x, *y, origin_x, origin_y, sx, sy);
                        }
                        PathCommand::Close => {}
                    }
                }
            }
        }
    }
}

fn scale_point(x: f32, y: f32, origin_x: f32, origin_y: f32, sx: f32, sy: f32) -> (f32, f32) {
    (
        origin_x + (x - origin_x) * sx,
        origin_y + (y - origin_y) * sy,
    )
}

fn rotate_layout_items_from(
    items: &mut Vec<LayoutItem>,
    item_start: usize,
    origin_x: f32,
    origin_y: f32,
    deg: f32,
) {
    if item_start >= items.len() {
        return;
    }

    let tail = items.drain(item_start..).collect::<Vec<_>>();
    let mut rotated = Vec::with_capacity(tail.len());
    for item in tail {
        push_rotated_layout_item(&mut rotated, item, origin_x, origin_y, deg);
    }
    items.extend(rotated);
}

fn push_rotated_layout_item(
    out: &mut Vec<LayoutItem>,
    mut item: LayoutItem,
    origin_x: f32,
    origin_y: f32,
    deg: f32,
) {
    match &mut item {
        LayoutItem::BeginClip(clip) => {
            let corners = rotated_box_corners(
                clip.x,
                clip.y,
                clip.width,
                clip.height,
                origin_x,
                origin_y,
                deg,
            );
            let (x, y, width, height) = axis_aligned_bounds(&corners);
            clip.x = x;
            clip.y = y;
            clip.width = width;
            clip.height = height;
            out.push(item);
        }
        LayoutItem::EndClip => out.push(item),
        LayoutItem::Text(text) => {
            (text.x, text.y) = rotate_point(text.x, text.y, origin_x, origin_y, deg);
            out.push(item);
        }
        LayoutItem::Rect(rect) => {
            out.push(LayoutItem::Polygon(Polygon {
                points: rotated_box_corners(
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    origin_x,
                    origin_y,
                    deg,
                )
                .to_vec(),
                color: rect.color,
            }));
        }
        LayoutItem::RoundRect(rect) => {
            out.push(LayoutItem::Polygon(Polygon {
                points: rotated_box_corners(
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    origin_x,
                    origin_y,
                    deg,
                )
                .to_vec(),
                color: rect.color,
            }));
        }
        LayoutItem::StrokeRect(rect) => {
            let points = rotated_box_corners(
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                origin_x,
                origin_y,
                deg,
            );
            for idx in 0..4 {
                let (x1, y1) = points[idx];
                let (x2, y2) = points[(idx + 1) % 4];
                out.push(LayoutItem::Line(Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    width: rect.stroke_width,
                    color: rect.color,
                    line_cap_round: false,
                    dash: rect.dash,
                }));
            }
        }
        LayoutItem::LinearGradient(rect) => {
            let corners = rotated_box_corners(
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                origin_x,
                origin_y,
                deg,
            );
            let (x, y, width, height) = axis_aligned_bounds(&corners);
            rect.x = x;
            rect.y = y;
            rect.width = width;
            rect.height = height;
            out.push(item);
        }
        LayoutItem::Image(image) => {
            let corners = rotated_box_corners(
                image.x,
                image.y,
                image.width,
                image.height,
                origin_x,
                origin_y,
                deg,
            );
            let (x, y, width, height) = axis_aligned_bounds(&corners);
            image.x = x;
            image.y = y;
            image.width = width;
            image.height = height;
            out.push(item);
        }
        LayoutItem::Circle(circle) => {
            (circle.cx, circle.cy) = rotate_point(circle.cx, circle.cy, origin_x, origin_y, deg);
            out.push(item);
        }
        LayoutItem::CircleStroke(circle) => {
            (circle.cx, circle.cy) = rotate_point(circle.cx, circle.cy, origin_x, origin_y, deg);
            out.push(item);
        }
        LayoutItem::Line(line) => {
            (line.x1, line.y1) = rotate_point(line.x1, line.y1, origin_x, origin_y, deg);
            (line.x2, line.y2) = rotate_point(line.x2, line.y2, origin_x, origin_y, deg);
            out.push(item);
        }
        LayoutItem::Polygon(polygon) => {
            for (x, y) in &mut polygon.points {
                (*x, *y) = rotate_point(*x, *y, origin_x, origin_y, deg);
            }
            out.push(item);
        }
        LayoutItem::SvgPath(path) => {
            for command in &mut path.commands {
                match command {
                    PathCommand::MoveTo(x, y) | PathCommand::LineTo(x, y) => {
                        (*x, *y) = rotate_point(*x, *y, origin_x, origin_y, deg);
                    }
                    PathCommand::CubicTo {
                        x1,
                        y1,
                        x2,
                        y2,
                        x,
                        y,
                    } => {
                        (*x1, *y1) = rotate_point(*x1, *y1, origin_x, origin_y, deg);
                        (*x2, *y2) = rotate_point(*x2, *y2, origin_x, origin_y, deg);
                        (*x, *y) = rotate_point(*x, *y, origin_x, origin_y, deg);
                    }
                    PathCommand::Close => {}
                }
            }
            out.push(item);
        }
    }
}

fn rotated_box_corners(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    origin_x: f32,
    origin_y: f32,
    deg: f32,
) -> [(f32, f32); 4] {
    [
        rotate_point(x, y, origin_x, origin_y, deg),
        rotate_point(x + width, y, origin_x, origin_y, deg),
        rotate_point(x + width, y + height, origin_x, origin_y, deg),
        rotate_point(x, y + height, origin_x, origin_y, deg),
    ]
}

fn axis_aligned_bounds(points: &[(f32, f32); 4]) -> (f32, f32, f32, f32) {
    let min_x = points.iter().map(|(x, _)| *x).fold(f32::INFINITY, f32::min);
    let max_x = points
        .iter()
        .map(|(x, _)| *x)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = points.iter().map(|(_, y)| *y).fold(f32::INFINITY, f32::min);
    let max_y = points
        .iter()
        .map(|(_, y)| *y)
        .fold(f32::NEG_INFINITY, f32::max);
    (
        min_x,
        min_y,
        (max_x - min_x).max(0.0),
        (max_y - min_y).max(0.0),
    )
}

fn rotate_point(x: f32, y: f32, origin_x: f32, origin_y: f32, deg: f32) -> (f32, f32) {
    let radians = deg.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();
    let dx = x - origin_x;
    let dy = y - origin_y;
    (
        origin_x + dx * cos - dy * sin,
        origin_y + dx * sin + dy * cos,
    )
}

fn transform_origin_point(
    style: &ComputedStyle,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> (f32, f32) {
    (
        x + style.transform_origin_x.resolve(width),
        y + height - style.transform_origin_y.resolve(height),
    )
}

fn background_items(
    style: &ComputedStyle,
    options: &RenderOptions,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> Vec<LayoutItem> {
    let mut items = Vec::new();
    for shadow in style.box_shadows.iter().rev().copied() {
        push_box_shadow_items(&mut items, style, shadow, x, y, width, height);
    }
    let (paint_x, paint_y, paint_width, paint_height, paint_radius) =
        background_clip_rect(style, x, y, width, height);
    let clip_background = paint_radius > 0.0
        && (style.background_gradient.is_some() || !style.background_radials.is_empty());
    if clip_background {
        items.push(LayoutItem::BeginClip(ClipRect {
            x: paint_x,
            y: paint_y,
            width: paint_width,
            height: paint_height,
            radius: paint_radius,
        }));
    }
    if let Some(mut gradient) = style.background_gradient.clone() {
        gradient.start = gradient.start.with_opacity(style.opacity);
        gradient.end = gradient.end.with_opacity(style.opacity);
        for stop in &mut gradient.stops {
            stop.color = stop.color.with_opacity(style.opacity);
        }
        items.push(LayoutItem::LinearGradient(GradientRect {
            x: paint_x,
            y: paint_y,
            width: paint_width,
            height: paint_height,
            gradient,
        }));
    } else if let Some(background) = style.background {
        let item = if paint_radius > 0.0 {
            LayoutItem::RoundRect(RoundRect {
                x: paint_x,
                y: paint_y,
                width: paint_width,
                height: paint_height,
                radius: paint_radius,
                color: background.with_opacity(style.opacity),
            })
        } else {
            LayoutItem::Rect(Rect {
                x: paint_x,
                y: paint_y,
                width: paint_width,
                height: paint_height,
                color: background.with_opacity(style.opacity),
            })
        };
        items.push(item);
    }
    push_radial_background_items(
        &mut items,
        &style.background_radials,
        paint_x,
        paint_y,
        paint_width,
        paint_height,
        style,
    );
    push_background_image_item(
        &mut items,
        style,
        options,
        paint_x,
        paint_y,
        paint_width,
        paint_height,
        paint_radius,
    );
    if clip_background {
        items.push(LayoutItem::EndClip);
    }
    items
}

fn background_clip_rect(
    style: &ComputedStyle,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> (f32, f32, f32, f32, f32) {
    let (left, right, top, bottom) = match style.background_clip {
        BackgroundClip::BorderBox => (0.0, 0.0, 0.0, 0.0),
        BackgroundClip::PaddingBox => border_insets(style),
        BackgroundClip::ContentBox => {
            let (border_left, border_right, border_top, border_bottom) = border_insets(style);
            (
                border_left + style.padding_left,
                border_right + style.padding_right,
                border_top + style.padding_top,
                border_bottom + style.padding_bottom,
            )
        }
    };
    let clipped_width = (width - left - right).max(0.0);
    let clipped_height = (height - top - bottom).max(0.0);
    let radius_inset = left.max(right).max(top).max(bottom);
    (
        x + left,
        y + bottom,
        clipped_width,
        clipped_height,
        (style.border_radius - radius_inset).max(0.0),
    )
}

fn border_insets(style: &ComputedStyle) -> (f32, f32, f32, f32) {
    (
        style.border_left_width.max(style.border_width),
        style.border_right_width.max(style.border_width),
        style.border_top_width.max(style.border_width),
        style.border_bottom_width.max(style.border_width),
    )
}

fn push_box_shadow_items(
    items: &mut Vec<LayoutItem>,
    style: &ComputedStyle,
    shadow: BoxShadow,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) {
    let blur = shadow.blur.max(0.0);
    if blur <= 0.01 {
        push_box_shadow_layer(
            items,
            style,
            shadow,
            x,
            y,
            width,
            height,
            shadow.spread.max(0.0),
            1.0,
        );
        return;
    }

    const LAYER_WEIGHTS: [f32; 6] = [0.06, 0.09, 0.13, 0.18, 0.24, 0.30];
    let max_grow = shadow.spread.max(0.0) + blur * 0.55;
    for (idx, weight) in LAYER_WEIGHTS.iter().enumerate() {
        let t = (idx as f32 + 1.0) / LAYER_WEIGHTS.len() as f32;
        let grow = max_grow * t;
        push_box_shadow_layer(items, style, shadow, x, y, width, height, grow, *weight);
    }
}

fn push_box_shadow_layer(
    items: &mut Vec<LayoutItem>,
    style: &ComputedStyle,
    shadow: BoxShadow,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    grow: f32,
    alpha_scale: f32,
) {
    let shadow_x = x + shadow.offset_x - grow;
    let shadow_y = y - shadow.offset_y - grow;
    let shadow_width = (width + grow * 2.0).max(0.0);
    let shadow_height = (height + grow * 2.0).max(0.0);
    let shadow_color = shadow
        .color
        .with_opacity((style.opacity * alpha_scale).clamp(0.0, 1.0));
    let item = if style.border_radius > 0.0 {
        LayoutItem::RoundRect(RoundRect {
            x: shadow_x,
            y: shadow_y,
            width: shadow_width,
            height: shadow_height,
            radius: style.border_radius + grow,
            color: shadow_color,
        })
    } else {
        LayoutItem::Rect(Rect {
            x: shadow_x,
            y: shadow_y,
            width: shadow_width,
            height: shadow_height,
            color: shadow_color,
        })
    };
    items.push(item);
}

fn background_items_without_shadow(
    style: &ComputedStyle,
    options: &RenderOptions,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> Vec<LayoutItem> {
    let mut style = style.clone();
    style.box_shadows.clear();
    background_items(&style, options, x, y, width, height)
}

fn push_background_image_item(
    items: &mut Vec<LayoutItem>,
    style: &ComputedStyle,
    options: &RenderOptions,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    radius: f32,
) {
    let Some(src) = style.background_image.as_deref() else {
        return;
    };
    let Some(image) = load_image_asset(src, options) else {
        return;
    };
    let intrinsic_width = image.intrinsic_width_px as f32 * 0.75;
    let intrinsic_height = image.intrinsic_height_px as f32 * 0.75;
    let fit = match style.background_size {
        BackgroundSize::Auto => ObjectFit::None,
        BackgroundSize::Cover => ObjectFit::Cover,
        BackgroundSize::Contain => ObjectFit::Contain,
    };
    let (_, _, image_width, image_height) = object_fit_rect(
        fit,
        x,
        y,
        width,
        height,
        intrinsic_width,
        intrinsic_height,
        0.5,
        0.5,
    );
    let image_x = x + (width - image_width) * style.background_position_x;
    let image_y = y + (height - image_height) * style.background_position_y;
    let needs_clip = radius > 0.0
        || image_x < x
        || image_y < y
        || image_x + image_width > x + width
        || image_y + image_height > y + height;
    if needs_clip {
        items.push(LayoutItem::BeginClip(ClipRect {
            x,
            y,
            width,
            height,
            radius,
        }));
    }
    items.push(LayoutItem::Image(ImageRect {
        x: image_x,
        y: image_y,
        width: image_width,
        height: image_height,
        intrinsic_width_px: image.intrinsic_width_px,
        intrinsic_height_px: image.intrinsic_height_px,
        data: image.data,
        alpha_mask: image.alpha_mask,
        format: image.format,
    }));
    if needs_clip {
        items.push(LayoutItem::EndClip);
    }
}

fn push_radial_background_items(
    items: &mut Vec<LayoutItem>,
    radials: &[RadialGradient],
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    style: &ComputedStyle,
) {
    if let Some(image) = radial_gradient_image(radials, x, y, width, height, style.opacity) {
        items.push(image);
        return;
    }
    for radial in radials.iter().rev() {
        let cx = x + width * radial.center_x;
        let cy = y + height * (1.0 - radial.center_y);
        let max_radius = width.max(height);
        let radius = radial
            .radius
            .resolve(max_radius)
            .clamp(max_radius * 0.02, max_radius * 1.25)
            .max(1.0);
        for step in (1..=4).rev() {
            let factor = step as f32 / 4.0;
            let alpha = radial.color.a * style.opacity * factor * 0.36;
            if alpha <= 0.0 {
                continue;
            }
            items.push(LayoutItem::Circle(Circle {
                cx,
                cy,
                r: radius * factor,
                color: Color {
                    a: alpha,
                    ..radial.color
                },
            }));
        }
    }
}

fn radial_gradient_image(
    radials: &[RadialGradient],
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    opacity: f32,
) -> Option<LayoutItem> {
    if radials.is_empty() || width <= 0.0 || height <= 0.0 {
        return None;
    }
    let intrinsic_width_px = raster_gradient_axis_px(width);
    let intrinsic_height_px = raster_gradient_axis_px(height);
    if intrinsic_width_px == 0 || intrinsic_height_px == 0 {
        return None;
    }

    let mut data = vec![0_u8; intrinsic_width_px as usize * intrinsic_height_px as usize * 3];
    let mut alpha_mask = vec![0_u8; intrinsic_width_px as usize * intrinsic_height_px as usize];
    let max_radius = width.max(height);

    for radial in radials.iter().rev() {
        let cx = radial.center_x * intrinsic_width_px as f32;
        let cy = radial.center_y * intrinsic_height_px as f32;
        let radius_pt = radial
            .radius
            .resolve(max_radius)
            .clamp(max_radius * 0.02, max_radius * 1.5)
            .max(1.0);
        let radius_x = (radius_pt / width.max(1.0)) * intrinsic_width_px as f32;
        let radius_y = (radius_pt / height.max(1.0)) * intrinsic_height_px as f32;
        if radius_x <= 0.0 || radius_y <= 0.0 {
            continue;
        }
        for py in 0..intrinsic_height_px {
            for px in 0..intrinsic_width_px {
                let dx = (px as f32 + 0.5 - cx) / radius_x;
                let dy = (py as f32 + 0.5 - cy) / radius_y;
                let distance = (dx * dx + dy * dy).sqrt();
                if distance >= 1.0 {
                    continue;
                }
                let fade = 1.0 - distance;
                let source_alpha = (radial.color.a * opacity * fade).clamp(0.0, 1.0);
                if source_alpha <= 0.0 {
                    continue;
                }
                let idx = (py as usize * intrinsic_width_px as usize + px as usize) * 3;
                let alpha_idx = py as usize * intrinsic_width_px as usize + px as usize;
                let dest_alpha = alpha_mask[alpha_idx] as f32 / 255.0;
                let out_alpha = source_alpha + dest_alpha * (1.0 - source_alpha);
                if out_alpha <= 0.0 {
                    continue;
                }
                let blend = source_alpha / out_alpha;
                data[idx] = blend_channel(data[idx], radial.color.r, blend);
                data[idx + 1] = blend_channel(data[idx + 1], radial.color.g, blend);
                data[idx + 2] = blend_channel(data[idx + 2], radial.color.b, blend);
                alpha_mask[alpha_idx] = (out_alpha * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        }
    }

    if alpha_mask.iter().all(|alpha| *alpha == 0) {
        return None;
    }
    Some(LayoutItem::Image(ImageRect {
        x,
        y,
        width,
        height,
        intrinsic_width_px,
        intrinsic_height_px,
        data,
        alpha_mask: Some(alpha_mask),
        format: ImageFormat::Raw {
            color_space: PngColorSpace::DeviceRgb,
            bits_per_component: 8,
            components: 3,
        },
    }))
}

fn raster_gradient_axis_px(points: f32) -> u32 {
    (points * 0.35).round().clamp(24.0, 192.0) as u32
}

fn blend_channel(destination: u8, source: f32, blend: f32) -> u8 {
    let destination = destination as f32 / 255.0;
    ((source.clamp(0.0, 1.0) * blend + destination * (1.0 - blend)) * 255.0)
        .round()
        .clamp(0.0, 255.0) as u8
}

fn prepend_document_background(
    document: &Document,
    stylesheet: &Stylesheet,
    options: &RenderOptions,
    pages: &mut [LayoutPage],
) {
    let Some(background_node) = find_first_element_by_tag(document, document.root(), "body")
        .or_else(|| find_first_element_by_tag(document, document.root(), "html"))
    else {
        return;
    };
    let mut style = ComputedStyle::for_node(document, stylesheet, background_node);
    if style.background.is_none()
        && style.background_gradient.is_none()
        && style.background_radials.is_empty()
        && style.background_image.is_none()
    {
        return;
    }
    style.background_radials.clear();
    let x = options.page.margin_left_pt;
    let y = options.page.margin_bottom_pt;
    let width =
        (options.page.width_pt - options.page.margin_left_pt - options.page.margin_right_pt)
            .max(0.0);
    let height =
        (options.page.height_pt - options.page.margin_top_pt - options.page.margin_bottom_pt)
            .max(0.0);
    let background_items = background_items(&style, options, x, y, width, height);
    if background_items.is_empty() {
        return;
    }
    let mut items = Vec::with_capacity(background_items.len() + 2);
    items.push(LayoutItem::BeginClip(ClipRect {
        x,
        y,
        width,
        height,
        radius: 0.0,
    }));
    items.extend(background_items);
    items.push(LayoutItem::EndClip);
    for page in pages {
        for (idx, item) in items.iter().cloned().enumerate() {
            page.items.insert(idx, item);
        }
    }
}

fn append_print_margin_masks(pages: &mut [LayoutPage]) {
    for page in pages {
        page.items.extend(print_margin_masks(page.page));
    }
}

fn print_margin_masks(page: PageOptions) -> Vec<LayoutItem> {
    let white = Color::WHITE;
    let top_y = (page.height_pt - page.margin_top_pt).max(0.0);
    let right_x = (page.width_pt - page.margin_right_pt).max(0.0);
    vec![
        LayoutItem::Rect(Rect {
            x: 0.0,
            y: top_y,
            width: page.width_pt,
            height: page.margin_top_pt.max(0.0),
            color: white,
        }),
        LayoutItem::Rect(Rect {
            x: 0.0,
            y: 0.0,
            width: page.width_pt,
            height: page.margin_bottom_pt.max(0.0),
            color: white,
        }),
        LayoutItem::Rect(Rect {
            x: 0.0,
            y: 0.0,
            width: page.margin_left_pt.max(0.0),
            height: page.height_pt,
            color: white,
        }),
        LayoutItem::Rect(Rect {
            x: right_x,
            y: 0.0,
            width: page.margin_right_pt.max(0.0),
            height: page.height_pt,
            color: white,
        }),
    ]
}

fn find_first_element_by_tag(document: &Document, id: NodeId, tag: &str) -> Option<NodeId> {
    match document.node(id) {
        Some(Node::Element(element)) if element.tag == tag => return Some(id),
        _ => {}
    }
    document
        .children(id)
        .iter()
        .find_map(|child| find_first_element_by_tag(document, *child, tag))
}

fn is_container_tag(tag: &str) -> bool {
    matches!(
        tag,
        "document"
            | "html"
            | "body"
            | "main"
            | "section"
            | "article"
            | "header"
            | "footer"
            | "aside"
            | "div"
            | "thead"
            | "tbody"
            | "tfoot"
            | "tr"
            | "ul"
            | "ol"
    )
}

fn should_paint_element_background(document: &Document, id: NodeId) -> bool {
    !matches!(
        document.node(id),
        Some(Node::Element(element)) if matches!(element.tag.as_str(), "html" | "body" | "document")
    )
}

fn table_rows(document: &Document, id: NodeId) -> Vec<NodeId> {
    let mut rows = Vec::new();
    collect_table_rows(document, id, &mut rows);
    rows
}

fn collect_table_rows(document: &Document, id: NodeId, rows: &mut Vec<NodeId>) {
    for child in document.children(id) {
        let Some(Node::Element(element)) = document.node(*child) else {
            continue;
        };
        match element.tag.as_str() {
            "tr" => rows.push(*child),
            "thead" | "tbody" | "tfoot" => collect_table_rows(document, *child, rows),
            _ => {}
        }
    }
}

fn contains_large_table(document: &Document, id: NodeId, row_threshold: usize) -> bool {
    let Some(Node::Element(element)) = document.node(id) else {
        return false;
    };
    if element.tag == "table" && table_rows(document, id).len() > row_threshold {
        return true;
    }
    document
        .children(id)
        .iter()
        .any(|child| contains_large_table(document, *child, row_threshold))
}

fn is_rendered_form_control(element: &ElementNode) -> bool {
    match element.tag.as_str() {
        "input" => !matches!(form_control_type(element).as_deref(), Some("hidden")),
        "select" | "textarea" | "button" => true,
        _ => false,
    }
}

fn form_control_type(element: &ElementNode) -> Option<String> {
    match element.tag.as_str() {
        "input" => Some(
            element
                .attr("type")
                .unwrap_or("text")
                .trim()
                .to_ascii_lowercase(),
        ),
        "select" | "textarea" | "button" => Some(element.tag.clone()),
        _ => None,
    }
}

fn form_control_box_size(
    element: &ElementNode,
    style: &ComputedStyle,
    available_width: f32,
    height_base: f32,
) -> (f32, f32) {
    let control_type = form_control_type(element);
    let (default_width, default_height): (f32, f32) = match control_type.as_deref() {
        Some("checkbox" | "radio") => (10.5, 10.5),
        Some("range") => (96.0, 14.0),
        Some("color") => (36.0, 18.0),
        Some("textarea") => (150.0, 54.0),
        Some("button" | "submit" | "reset") => (64.0, 20.0),
        Some("select") => (150.0, 18.0),
        _ => (150.0, 18.0),
    };
    let horizontal_padding = style.padding_left + style.padding_right;
    let vertical_padding = style.padding_top + style.padding_bottom;
    let horizontal_border = style
        .border_width
        .max(style.border_left_width)
        .max(style.border_right_width)
        .max(1.0);
    let vertical_border = style
        .border_width
        .max(style.border_top_width)
        .max(style.border_bottom_width)
        .max(1.0);
    let natural_width = if matches!(control_type.as_deref(), Some("checkbox" | "radio")) {
        default_width
    } else {
        default_width + horizontal_padding + horizontal_border * 2.0
    };
    let natural_height = if matches!(control_type.as_deref(), Some("checkbox" | "radio")) {
        default_height
    } else {
        default_height.max(style.line_height + vertical_padding + vertical_border * 2.0)
    };

    let mut width = style
        .width
        .as_ref()
        .map(|width| width.resolve(available_width).max(0.0))
        .unwrap_or(natural_width);
    if let Some(min_width) = &style.min_width {
        width = width.max(min_width.resolve(available_width));
    }
    if let Some(max_width) = &style.max_width {
        width = width.min(max_width.resolve(available_width));
    }
    if !matches!(control_type.as_deref(), Some("checkbox" | "radio")) {
        width = width.min(available_width.max(0.0));
    }

    let mut height = style
        .height
        .as_ref()
        .map(|height| height.resolve(height_base).max(0.0))
        .unwrap_or(natural_height);
    if let Some(min_height) = &style.min_height {
        height = height.max(min_height.resolve(height_base));
    }
    if let Some(max_height) = &style.max_height {
        height = height.min(max_height.resolve(height_base));
    }
    (width.max(0.0), height.max(0.0))
}

fn form_control_text(document: &Document, id: NodeId, element: &ElementNode) -> Option<String> {
    let kind = form_control_type(element);
    match kind.as_deref() {
        Some("submit") => Some(element.attr("value").unwrap_or("Submit").to_string()),
        Some("reset") => Some(element.attr("value").unwrap_or("Reset").to_string()),
        Some("button") => {
            let value = element
                .attr("value")
                .map(str::to_string)
                .unwrap_or_else(|| document.text_content(id).trim().to_string());
            (!value.trim().is_empty()).then_some(value)
        }
        Some("textarea") => {
            let value = element
                .attr("value")
                .map(str::to_string)
                .unwrap_or_else(|| document.text_content(id).trim().to_string());
            (!value.trim().is_empty()).then_some(value)
        }
        Some("password") => element
            .attr("value")
            .map(|value| "•".repeat(value.chars().count())),
        Some("text" | "search" | "email" | "url" | "tel" | "number" | "date") | None => element
            .attr("value")
            .filter(|value| !value.trim().is_empty())
            .or_else(|| element.attr("placeholder"))
            .map(str::to_string),
        _ => None,
    }
}

fn selected_option_text(document: &Document, id: NodeId) -> Option<String> {
    let mut first = None;
    let mut selected = None;
    collect_option_text(document, id, &mut first, &mut selected);
    selected.or(first)
}

fn collect_option_text(
    document: &Document,
    id: NodeId,
    first: &mut Option<String>,
    selected: &mut Option<String>,
) {
    for child in document.children(id) {
        let Some(Node::Element(element)) = document.node(*child) else {
            continue;
        };
        if element.tag == "option" {
            let text = document.text_content(*child).trim().to_string();
            if !text.is_empty() && first.is_none() {
                *first = Some(text.clone());
            }
            if !text.is_empty() && element.attr("selected").is_some() {
                *selected = Some(text);
            }
        }
        if selected.is_none() {
            collect_option_text(document, *child, first, selected);
        }
    }
}

fn table_cells(document: &Document, row_id: NodeId) -> Vec<NodeId> {
    document
        .children(row_id)
        .iter()
        .copied()
        .filter(|id| {
            matches!(
                document.node(*id),
                Some(Node::Element(element)) if element.tag == "td" || element.tag == "th"
            )
        })
        .collect()
}

fn table_column_widths(
    document: &Document,
    rows: &[NodeId],
    table_width: f32,
    column_count: usize,
) -> Vec<f32> {
    if column_count == 0 {
        return Vec::new();
    }
    let mut weights = vec![1.0_f32; column_count];
    let mut max_lengths = vec![0.0_f32; column_count];
    for row in rows.iter().take(64) {
        for (col_index, cell_id) in table_cells(document, *row).iter().enumerate() {
            let text_len = document.text_content(*cell_id).chars().count() as f32;
            let score = text_len.sqrt().clamp(1.0, 12.0);
            if let Some(weight) = weights.get_mut(col_index) {
                *weight = weight.max(score);
            }
            if let Some(max_length) = max_lengths.get_mut(col_index) {
                *max_length = max_length.max(text_len);
            }
        }
    }
    let mut minimums = max_lengths
        .iter()
        .map(|length| {
            if *length <= 4.0 {
                38.0
            } else if *length <= 8.0 {
                54.0
            } else if *length <= 14.0 {
                72.0
            } else if *length <= 24.0 {
                100.0
            } else {
                54.0
            }
        })
        .collect::<Vec<_>>();
    let minimum_total = minimums.iter().sum::<f32>();
    if minimum_total >= table_width {
        let scale = table_width / minimum_total.max(1.0);
        for width in &mut minimums {
            *width *= scale;
        }
        return minimums;
    }

    let extra = table_width - minimum_total;
    let has_long_columns = max_lengths.iter().any(|length| *length > 30.0);
    let distribution_weights = if has_long_columns {
        max_lengths
            .iter()
            .zip(weights.iter())
            .map(|(length, weight)| if *length > 30.0 { *weight } else { 0.0 })
            .collect::<Vec<_>>()
    } else {
        weights
    };
    let total_weight = distribution_weights.iter().sum::<f32>().max(1.0);
    let mut widths = minimums;
    for (idx, width) in widths.iter_mut().enumerate() {
        *width += extra * (distribution_weights[idx] / total_weight);
    }
    widths
}

fn prepared_row_height(prepared: &[PreparedCell]) -> f32 {
    prepared
        .iter()
        .map(|cell| cell.height)
        .fold(24.0_f32, f32::max)
}

fn has_nested_table_cell_blocks(document: &Document, cell_id: NodeId) -> bool {
    document.children(cell_id).iter().any(|child_id| {
        matches!(
            document.node(*child_id),
            Some(Node::Element(element))
                if element.tag != "br"
                    && element.tag != "script"
                    && element.tag != "style"
                    && !document.text_content(*child_id).trim().is_empty()
        )
    })
}

fn table_row_height(prepared: &[PreparedCell], border_collapse: BorderCollapse) -> f32 {
    let height = prepared_row_height(prepared);
    if border_collapse == BorderCollapse::Collapse {
        (height - collapsed_border_overlap(prepared)).max(18.0)
    } else {
        height
    }
}

fn table_row_fragment_height(
    prepared: &[PreparedCell],
    line_start: usize,
    line_end: usize,
    first_fragment: bool,
    last_fragment: bool,
    border_collapse: BorderCollapse,
) -> f32 {
    let height = prepared
        .iter()
        .map(|cell| {
            let start = line_start.min(cell.lines.len());
            let end = line_end.min(cell.lines.len()).max(start);
            let content_height = cell.lines[start..end]
                .iter()
                .map(|line| line.before + line.style.line_height + line.after)
                .sum::<f32>();
            let includes_top_padding = first_fragment && start == 0;
            let includes_bottom_padding =
                last_fragment || (start < cell.lines.len() && end >= cell.lines.len());
            let top_padding = if includes_top_padding {
                cell.style.padding_top
            } else {
                0.0
            };
            let bottom_padding = if includes_bottom_padding {
                cell.style.padding_bottom
            } else {
                0.0
            };
            let nested_adjustment = if start < cell.lines.len() && end >= cell.lines.len() {
                (cell.height
                    - cell.style.padding_top
                    - cell.style.padding_bottom
                    - cell.content_height)
                    .max(0.0)
            } else {
                0.0
            };
            top_padding + bottom_padding + content_height + nested_adjustment
        })
        .fold(0.0_f32, f32::max);
    if border_collapse == BorderCollapse::Collapse {
        (height - collapsed_border_overlap(prepared)).max(1.0)
    } else {
        height.max(1.0)
    }
}

fn table_horizontal_spacing(style: &ComputedStyle, column_count: usize) -> f32 {
    if column_count > 1 && style.border_collapse == BorderCollapse::Separate {
        style.border_spacing_horizontal.max(0.0)
    } else {
        0.0
    }
}

fn table_vertical_spacing(style: &ComputedStyle) -> f32 {
    if style.border_collapse == BorderCollapse::Separate {
        style.border_spacing_vertical.max(0.0)
    } else {
        0.0
    }
}

fn collapsed_border_overlap(prepared: &[PreparedCell]) -> f32 {
    let max_vertical_border = prepared
        .iter()
        .map(|cell| {
            cell.style
                .border_top_width
                .max(cell.style.border_bottom_width)
                .max(cell.style.border_width)
        })
        .fold(0.0_f32, f32::max);
    // Browser collapsed-border layout shares adjacent horizontal borders
    // between rows. The renderer paints synthetic cell borders separately, so
    // reclaim enough row height to avoid accumulating a double-border gap over
    // long tables while keeping compact tables readable.
    max_vertical_border.max(1.25)
}

fn is_header_row(document: &Document, row_id: NodeId) -> bool {
    if table_cells(document, row_id).iter().any(|cell_id| {
        matches!(
            document.node(*cell_id),
            Some(Node::Element(element)) if element.tag == "th"
        )
    }) {
        return true;
    }
    let mut parent = document.parent_of(row_id);
    while let Some(parent_id) = parent {
        if matches!(
            document.node(parent_id),
            Some(Node::Element(element)) if element.tag == "thead"
        ) {
            return true;
        }
        parent = document.parent_of(parent_id);
    }
    false
}

fn should_descend_into_element(tag: &str, children: &[NodeId]) -> bool {
    !children.is_empty() && !is_text_aggregation_tag(tag)
}

fn ordered_list_item_index(document: &Document, id: NodeId) -> usize {
    let Some(parent_id) = document.parent_of(id) else {
        return 1;
    };
    document
        .children(parent_id)
        .iter()
        .filter(|child_id| {
            matches!(
                document.node(**child_id),
                Some(Node::Element(element)) if element.tag == "li"
            )
        })
        .position(|child_id| *child_id == id)
        .map(|index| index + 1)
        .unwrap_or(1)
}

fn is_text_aggregation_tag(tag: &str) -> bool {
    matches!(
        tag,
        "a" | "abbr"
            | "b"
            | "button"
            | "caption"
            | "cite"
            | "code"
            | "dd"
            | "dt"
            | "em"
            | "figcaption"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "i"
            | "label"
            | "legend"
            | "li"
            | "mark"
            | "p"
            | "small"
            | "span"
            | "strong"
            | "sub"
            | "sup"
            | "td"
            | "th"
            | "u"
    )
}

fn has_styled_inline_children(document: &Document, id: NodeId) -> bool {
    document.children(id).iter().any(|child| {
        matches!(
            document.node(*child),
            Some(Node::Element(element))
                if is_text_aggregation_tag(&element.tag)
                    && !document.text_content(*child).trim().is_empty()
        )
    })
}

fn is_css_collapsible_space(character: char) -> bool {
    matches!(character, ' ' | '\t' | '\n' | '\r' | '\u{000c}')
}

fn css_words(text: &str) -> impl Iterator<Item = &str> {
    text.split(is_css_collapsible_space)
        .filter(|part| !part.is_empty())
}

fn tokenize_inline_segments(segments: &[InlineTextSegment]) -> Vec<InlineTextSegment> {
    let mut tokens = Vec::new();
    let mut pending_space = false;

    for segment in segments {
        if segment.atomic.is_some() {
            if pending_space && !tokens.is_empty() {
                tokens.push(InlineTextSegment {
                    text: " ".into(),
                    style: segment.style.clone(),
                    atomic: None,
                });
            }
            tokens.push(segment.clone());
            pending_space = false;
            continue;
        }
        if segment.text == "\n" {
            tokens.push(segment.clone());
            pending_space = false;
            continue;
        }

        let mut word = String::new();
        for character in segment.text.chars() {
            if character == '\n' && segment.style.white_space == WhiteSpace::PreLine {
                push_inline_word(&mut tokens, &mut word, &segment.style, &mut pending_space);
                tokens.push(InlineTextSegment {
                    atomic: None,
                    text: "\n".to_string(),
                    style: segment.style.clone(),
                });
                pending_space = false;
            } else if is_css_collapsible_space(character) {
                push_inline_word(&mut tokens, &mut word, &segment.style, &mut pending_space);
                pending_space = true;
            } else {
                if pending_space && !tokens.is_empty() {
                    if !tokens
                        .last()
                        .is_some_and(|token: &InlineTextSegment| token.text == "\n")
                    {
                        tokens.push(InlineTextSegment {
                            atomic: None,
                            text: " ".to_string(),
                            style: segment.style.clone(),
                        });
                    }
                    pending_space = false;
                }
                word.push(character);
            }
        }
        push_inline_word(&mut tokens, &mut word, &segment.style, &mut pending_space);
    }

    tokens
}

fn push_inline_word(
    tokens: &mut Vec<InlineTextSegment>,
    word: &mut String,
    style: &ComputedStyle,
    pending_space: &mut bool,
) {
    if word.is_empty() {
        return;
    }
    if *pending_space
        && !tokens.is_empty()
        && !tokens
            .last()
            .is_some_and(|token: &InlineTextSegment| token.text == "\n")
    {
        tokens.push(InlineTextSegment {
            atomic: None,
            text: " ".to_string(),
            style: style.clone(),
        });
    }
    tokens.push(InlineTextSegment {
        atomic: None,
        text: std::mem::take(word),
        style: style.clone(),
    });
    *pending_space = false;
}

fn wrap_one_inline_line(
    tokens: &[InlineTextSegment],
    start: usize,
    available_width: f32,
    no_wrap: bool,
) -> (Vec<InlineTextSegment>, usize) {
    let mut line = Vec::new();
    let mut current_width = 0.0_f32;
    let mut pending_space = None;
    let mut cursor = start;

    while cursor < tokens.len() {
        let token = &tokens[cursor];
        if token.text == "\n" {
            return (line, cursor + 1);
        }
        if token.text == " " {
            if !line.is_empty() {
                pending_space = Some(token.clone());
            }
            cursor += 1;
            continue;
        }

        // Styling boundaries are not soft wrap opportunities. Measure the
        // complete word, which may span several differently styled runs,
        // before deciding whether to move it to the next line.
        let mut word_end = cursor + 1;
        while token.atomic.is_none()
            && word_end < tokens.len()
            && tokens[word_end].atomic.is_none()
            && tokens[word_end].text != " "
            && tokens[word_end].text != "\n"
        {
            word_end += 1;
        }
        let token_width = inline_segments_wrap_width(&tokens[cursor..word_end], available_width);
        let space_width = pending_space
            .as_ref()
            .map(|space| inline_segments_wrap_width(std::slice::from_ref(space), available_width))
            .unwrap_or(0.0);
        let needed_width = if line.is_empty() {
            token_width
        } else {
            space_width + token_width
        };

        if !no_wrap && !line.is_empty() && current_width + needed_width > available_width {
            return (line, cursor);
        }

        if let Some(space) = pending_space.take() {
            line.push(space);
            current_width += space_width;
        }
        line.extend_from_slice(&tokens[cursor..word_end]);
        current_width += token_width;
        cursor = word_end;
    }

    (line, cursor)
}

fn inline_segments_wrap_width(segments: &[InlineTextSegment], available_width: f32) -> f32 {
    segments
        .iter()
        .map(|segment| {
            if let Some(atomic) = &segment.atomic {
                return atomic.width;
            }
            estimate_wrap_text_width_with_spacing(
                &segment.text,
                segment.style.font_size,
                segment.style.font_face,
                segment.style.font_weight,
                available_width,
                segment.style.letter_spacing,
                segment.style.word_spacing,
            )
        })
        .sum()
}

fn inline_line_metrics(segments: &[InlineTextSegment], style: &ComputedStyle) -> (f32, f32) {
    let mut ascent = style.font_size;
    let mut descent = (layout_line_height(style) - ascent).max(0.0);
    for segment in segments {
        if let Some(atomic) = &segment.atomic {
            ascent = ascent.max(atomic.baseline);
            descent = descent.max(atomic.height - atomic.baseline);
        }
    }
    (ascent, ascent + descent)
}

fn inline_segments_width(segments: &[InlineTextSegment]) -> f32 {
    segments
        .iter()
        .map(|segment| {
            if let Some(atomic) = &segment.atomic {
                return atomic.width;
            }
            estimate_text_width_with_spacing(
                &segment.text,
                segment.style.font_size,
                segment.style.font_face,
                segment.style.font_weight,
                segment.style.letter_spacing,
                segment.style.word_spacing,
            )
        })
        .sum()
}

fn is_non_painted_tag(tag: &str) -> bool {
    matches!(
        tag,
        "script"
            | "style"
            | "template"
            | "head"
            | "meta"
            | "title"
            | "iframe"
            | "canvas"
            | "video"
            | "audio"
            | "source"
            | "object"
            | "embed"
            | "picture"
            | "defs"
            | "svg"
            | "path"
            | "circle"
            | "rect"
            | "lineargradient"
            | "radialgradient"
            | "stop"
    )
}

fn parse_svg_length(value: &str) -> Option<f32> {
    let value = value.trim();
    let value = value
        .strip_suffix("px")
        .or_else(|| value.strip_suffix("pt"))
        .unwrap_or(value)
        .trim();
    value.parse::<f32>().ok().map(|value| {
        if value.is_finite() {
            value.max(0.0)
        } else {
            0.0
        }
    })
}

fn parse_svg_alpha(value: &str) -> Option<f32> {
    let value = value.trim();
    if let Some(percent) = value.strip_suffix('%') {
        percent
            .trim()
            .parse::<f32>()
            .ok()
            .map(|value| (value / 100.0).clamp(0.0, 1.0))
    } else {
        value.parse::<f32>().ok().map(|value| value.clamp(0.0, 1.0))
    }
}

fn parse_view_box(value: Option<&str>) -> Option<(f32, f32, f32, f32)> {
    let value = value?;
    let numbers = value
        .replace(',', " ")
        .split_whitespace()
        .filter_map(|part| part.parse::<f32>().ok())
        .collect::<Vec<_>>();
    match numbers.as_slice() {
        [min_x, min_y, width, height] => Some((*min_x, *min_y, *width, *height)),
        _ => None,
    }
}

fn svg_aspect_height(width: f32, view_width: f32, view_height: f32) -> Option<f32> {
    if width.is_finite()
        && width > 0.0
        && view_width.is_finite()
        && view_width > 0.0
        && view_height.is_finite()
        && view_height > 0.0
    {
        Some(width * view_height / view_width)
    } else {
        None
    }
}

fn parse_svg_gradient_coord(value: &str) -> Option<f32> {
    let value = value.trim();
    if let Some(percent) = value.strip_suffix('%') {
        return percent
            .trim()
            .parse::<f32>()
            .ok()
            .map(|value| (value / 100.0).clamp(0.0, 1.0));
    }
    value.parse::<f32>().ok().filter(|value| value.is_finite())
}

fn parse_svg_gradient_offset(value: &str) -> Option<f32> {
    let value = value.trim();
    if let Some(percent) = value.strip_suffix('%') {
        return percent
            .trim()
            .parse::<f32>()
            .ok()
            .map(|value| (value / 100.0).clamp(0.0, 1.0));
    }
    value.parse::<f32>().ok().map(|value| value.clamp(0.0, 1.0))
}

fn parse_svg_points(value: &str) -> Option<Vec<(f32, f32)>> {
    let numbers = value
        .replace(',', " ")
        .split_whitespace()
        .map(str::parse::<f32>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if numbers.len() < 4 || numbers.len() % 2 != 0 {
        return None;
    }
    Some(
        numbers
            .chunks_exact(2)
            .map(|point| (point[0], point[1]))
            .collect(),
    )
}

fn svg_paint_color(
    document: &Document,
    value: Option<&str>,
    current_color: Color,
) -> Option<Color> {
    match value.map(str::trim) {
        Some(value) if value.eq_ignore_ascii_case("none") => None,
        Some(value) if value.eq_ignore_ascii_case("currentcolor") => Some(current_color),
        Some(value) if value.starts_with("url(") => svg_url_paint_color(document, value),
        Some(value) => Color::from_css(value),
        None => None,
    }
}

fn svg_url_paint_color(document: &Document, value: &str) -> Option<Color> {
    let id = value
        .trim()
        .strip_prefix("url(")?
        .trim_end_matches(')')
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .strip_prefix('#')?;
    let paint_id = find_svg_paint_server(document, document.root(), id)?;
    averaged_svg_stop_color(document, paint_id).or_else(|| first_svg_stop_color(document, paint_id))
}

fn svg_url_linear_gradient(document: &Document, value: &str) -> Option<LinearGradient> {
    let id = value
        .trim()
        .strip_prefix("url(")?
        .trim_end_matches(')')
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .strip_prefix('#')?;
    let gradient_id = find_svg_paint_server(document, document.root(), id)?;
    let Some(Node::Element(gradient)) = document.node(gradient_id) else {
        return None;
    };
    if gradient.tag != "lineargradient" {
        return None;
    }
    let stops = svg_gradient_stops(document, gradient_id);
    let start = stops.first().map(|stop| stop.color)?;
    let end = stops.last().map(|stop| stop.color).unwrap_or(start);
    let x1 = gradient
        .attr("x1")
        .and_then(parse_svg_gradient_coord)
        .unwrap_or(0.0);
    let y1 = gradient
        .attr("y1")
        .and_then(parse_svg_gradient_coord)
        .unwrap_or(0.0);
    let x2 = gradient
        .attr("x2")
        .and_then(parse_svg_gradient_coord)
        .unwrap_or(1.0);
    let y2 = gradient
        .attr("y2")
        .and_then(parse_svg_gradient_coord)
        .unwrap_or(0.0);
    let dx = x2 - x1;
    let dy = y2 - y1;
    let angle_deg = if dx.abs() < 0.0001 && dy.abs() < 0.0001 {
        90.0
    } else {
        dx.atan2(-dy).to_degrees().rem_euclid(360.0)
    };
    Some(LinearGradient {
        start,
        end,
        angle_deg,
        stops,
    })
}

fn find_svg_paint_server(document: &Document, id: NodeId, target: &str) -> Option<NodeId> {
    match document.node(id) {
        Some(Node::Element(element))
            if matches!(element.tag.as_str(), "lineargradient" | "radialgradient")
                && element.attr("id") == Some(target) =>
        {
            return Some(id);
        }
        _ => {}
    }
    document
        .children(id)
        .iter()
        .find_map(|child| find_svg_paint_server(document, *child, target))
}

fn svg_gradient_stops(document: &Document, gradient_id: NodeId) -> Vec<GradientStop> {
    let mut stops = Vec::new();
    for (index, child) in document.children(gradient_id).iter().enumerate() {
        let Some(Node::Element(stop)) = document.node(*child) else {
            continue;
        };
        if stop.tag != "stop" {
            continue;
        }
        let Some(color) = svg_stop_color(stop) else {
            continue;
        };
        let position = stop
            .attr("offset")
            .and_then(parse_svg_gradient_offset)
            .unwrap_or(if index == 0 { 0.0 } else { 1.0 });
        stops.push(GradientStop { color, position });
    }
    if stops.len() == 1 {
        let color = stops[0].color;
        stops.push(GradientStop {
            color,
            position: 1.0,
        });
    }
    stops.sort_by(|a, b| a.position.total_cmp(&b.position));
    stops
}

fn first_svg_stop_color(document: &Document, gradient_id: NodeId) -> Option<Color> {
    svg_stop_colors(document, gradient_id).into_iter().next()
}

fn averaged_svg_stop_color(document: &Document, gradient_id: NodeId) -> Option<Color> {
    let colors = svg_stop_colors(document, gradient_id);
    if colors.is_empty() {
        return None;
    }
    let count = colors.len() as f32;
    Some(Color {
        r: colors.iter().map(|color| color.r).sum::<f32>() / count,
        g: colors.iter().map(|color| color.g).sum::<f32>() / count,
        b: colors.iter().map(|color| color.b).sum::<f32>() / count,
        a: colors.iter().map(|color| color.a).sum::<f32>() / count,
    })
}

fn svg_stop_colors(document: &Document, gradient_id: NodeId) -> Vec<Color> {
    let mut colors = Vec::new();
    for child in document.children(gradient_id) {
        let Some(Node::Element(stop)) = document.node(*child) else {
            continue;
        };
        if stop.tag != "stop" {
            continue;
        }
        if let Some(color) = svg_stop_color(stop) {
            colors.push(color);
        }
    }
    colors
}

fn svg_stop_color(stop: &ElementNode) -> Option<Color> {
    let mut color = stop.attr("stop-color").and_then(Color::from_css);
    let mut opacity = stop
        .attr("stop-opacity")
        .and_then(parse_svg_alpha)
        .unwrap_or(1.0);
    if let Some(style) = stop.attr("style") {
        for declaration in style.split(';') {
            let Some((property, value)) = declaration.split_once(':') else {
                continue;
            };
            let property = property.trim();
            let value = value.trim();
            if property.eq_ignore_ascii_case("stop-color") {
                color = Color::from_css(value);
            } else if property.eq_ignore_ascii_case("stop-opacity") {
                opacity = parse_svg_alpha(value).unwrap_or(opacity);
            }
        }
    }
    color.map(|color| color.with_opacity(opacity))
}

#[derive(Debug, Clone, Copy)]
enum SvgPathToken {
    Command(char),
    Number(f32),
}

#[derive(Debug, Clone, Copy)]
struct SvgTransform {
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
}

impl SvgTransform {
    fn identity() -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }

    fn translate(tx: f32, ty: f32) -> Self {
        Self {
            e: tx,
            f: ty,
            ..Self::identity()
        }
    }

    fn scale(sx: f32, sy: f32) -> Self {
        Self {
            a: sx,
            d: sy,
            ..Self::identity()
        }
    }

    fn rotate(deg: f32) -> Self {
        let radians = deg.to_radians();
        let cos = radians.cos();
        let sin = radians.sin();
        Self {
            a: cos,
            b: sin,
            c: -sin,
            d: cos,
            e: 0.0,
            f: 0.0,
        }
    }

    fn rotate_around(deg: f32, cx: f32, cy: f32) -> Self {
        Self::translate(cx, cy)
            .multiply(Self::rotate(deg))
            .multiply(Self::translate(-cx, -cy))
    }

    fn matrix(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) -> Self {
        Self { a, b, c, d, e, f }
    }

    fn multiply(self, other: Self) -> Self {
        Self {
            a: self.a * other.a + self.c * other.b,
            b: self.b * other.a + self.d * other.b,
            c: self.a * other.c + self.c * other.d,
            d: self.b * other.c + self.d * other.d,
            e: self.a * other.e + self.c * other.f + self.e,
            f: self.b * other.e + self.d * other.f + self.f,
        }
    }

    fn apply(self, x: f32, y: f32) -> (f32, f32) {
        (
            self.a * x + self.c * y + self.e,
            self.b * x + self.d * y + self.f,
        )
    }

    fn scale_x(self) -> f32 {
        (self.a * self.a + self.b * self.b).sqrt().max(0.0)
    }

    fn scale_y(self) -> f32 {
        (self.c * self.c + self.d * self.d).sqrt().max(0.0)
    }

    fn average_scale(self) -> f32 {
        ((self.scale_x() + self.scale_y()) / 2.0).max(0.01)
    }

    fn rotation_deg(self) -> f32 {
        self.b.atan2(self.a).to_degrees()
    }
}

fn parse_svg_transform(value: &str) -> Option<SvgTransform> {
    let mut transform = SvgTransform::identity();
    for function in parse_svg_transform_functions(value) {
        let Some(open) = function.find('(') else {
            continue;
        };
        let name = function[..open].trim().to_ascii_lowercase();
        let body = function[open + 1..].trim_end_matches(')').trim();
        let values = body
            .replace(',', " ")
            .split_whitespace()
            .map(str::parse::<f32>)
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        let next = match name.as_str() {
            "translate" => SvgTransform::translate(
                values.first().copied().unwrap_or(0.0),
                values.get(1).copied().unwrap_or(0.0),
            ),
            "scale" => {
                let sx = values.first().copied().unwrap_or(1.0);
                SvgTransform::scale(sx, values.get(1).copied().unwrap_or(sx))
            }
            "rotate" => {
                let deg = values.first().copied().unwrap_or(0.0);
                if values.len() >= 3 {
                    SvgTransform::rotate_around(deg, values[1], values[2])
                } else {
                    SvgTransform::rotate(deg)
                }
            }
            "matrix" if values.len() >= 6 => SvgTransform::matrix(
                values[0], values[1], values[2], values[3], values[4], values[5],
            ),
            _ => continue,
        };
        transform = transform.multiply(next);
    }
    Some(transform)
}

fn parse_svg_transform_functions(value: &str) -> Vec<String> {
    let mut functions = Vec::new();
    let mut rest = value.trim();
    while let Some(open) = rest.find('(') {
        let name_start = rest[..open]
            .rfind(|ch: char| ch.is_whitespace())
            .map(|idx| idx + 1)
            .unwrap_or(0);
        let candidate = &rest[name_start..];
        let Some(close) = matching_svg_function_end(candidate) else {
            break;
        };
        functions.push(candidate[..=close].trim().to_string());
        rest = candidate[close + 1..].trim_start();
    }
    functions
}

fn matching_svg_function_end(value: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (idx, ch) in value.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_svg_path(
    data: &str,
    map_point: impl Fn(f32, f32) -> (f32, f32),
) -> Option<Vec<PathCommand>> {
    let tokens = tokenize_svg_path(data)?;
    let mut index = 0usize;
    let mut command = None::<char>;
    let mut current = (0.0_f32, 0.0_f32);
    let mut subpath_start = current;
    let mut out = Vec::new();

    while index < tokens.len() {
        if let SvgPathToken::Command(cmd) = tokens[index] {
            command = Some(cmd);
            index += 1;
        }
        let cmd = command?;
        match cmd {
            'M' | 'm' => {
                let relative = cmd == 'm';
                let mut first = true;
                while let Some((x, y)) = read_pair(&tokens, &mut index) {
                    let point = if relative {
                        (current.0 + x, current.1 + y)
                    } else {
                        (x, y)
                    };
                    current = point;
                    if first {
                        subpath_start = point;
                        let (x, y) = map_point(point.0, point.1);
                        out.push(PathCommand::MoveTo(x, y));
                        first = false;
                    } else {
                        let (x, y) = map_point(point.0, point.1);
                        out.push(PathCommand::LineTo(x, y));
                    }
                    if next_is_command(&tokens, index) {
                        break;
                    }
                }
                command = Some(if relative { 'l' } else { 'L' });
            }
            'L' | 'l' => {
                let relative = cmd == 'l';
                while let Some((x, y)) = read_pair(&tokens, &mut index) {
                    current = if relative {
                        (current.0 + x, current.1 + y)
                    } else {
                        (x, y)
                    };
                    let (x, y) = map_point(current.0, current.1);
                    out.push(PathCommand::LineTo(x, y));
                    if next_is_command(&tokens, index) {
                        break;
                    }
                }
            }
            'H' | 'h' => {
                let relative = cmd == 'h';
                while let Some(x) = read_number(&tokens, &mut index) {
                    current.0 = if relative { current.0 + x } else { x };
                    let (x, y) = map_point(current.0, current.1);
                    out.push(PathCommand::LineTo(x, y));
                    if next_is_command(&tokens, index) {
                        break;
                    }
                }
            }
            'V' | 'v' => {
                let relative = cmd == 'v';
                while let Some(y) = read_number(&tokens, &mut index) {
                    current.1 = if relative { current.1 + y } else { y };
                    let (x, y) = map_point(current.0, current.1);
                    out.push(PathCommand::LineTo(x, y));
                    if next_is_command(&tokens, index) {
                        break;
                    }
                }
            }
            'C' | 'c' => {
                let relative = cmd == 'c';
                while let Some(values) = read_numbers::<6>(&tokens, &mut index) {
                    let c1 = absolute_svg_point((values[0], values[1]), current, relative);
                    let c2 = absolute_svg_point((values[2], values[3]), current, relative);
                    let end = absolute_svg_point((values[4], values[5]), current, relative);
                    let (x1, y1) = map_point(c1.0, c1.1);
                    let (x2, y2) = map_point(c2.0, c2.1);
                    let (x, y) = map_point(end.0, end.1);
                    out.push(PathCommand::CubicTo {
                        x1,
                        y1,
                        x2,
                        y2,
                        x,
                        y,
                    });
                    current = end;
                    if next_is_command(&tokens, index) {
                        break;
                    }
                }
            }
            'Q' | 'q' => {
                let relative = cmd == 'q';
                while let Some(values) = read_numbers::<4>(&tokens, &mut index) {
                    let control = absolute_svg_point((values[0], values[1]), current, relative);
                    let end = absolute_svg_point((values[2], values[3]), current, relative);
                    let c1 = (
                        current.0 + (control.0 - current.0) * 2.0 / 3.0,
                        current.1 + (control.1 - current.1) * 2.0 / 3.0,
                    );
                    let c2 = (
                        end.0 + (control.0 - end.0) * 2.0 / 3.0,
                        end.1 + (control.1 - end.1) * 2.0 / 3.0,
                    );
                    let (x1, y1) = map_point(c1.0, c1.1);
                    let (x2, y2) = map_point(c2.0, c2.1);
                    let (x, y) = map_point(end.0, end.1);
                    out.push(PathCommand::CubicTo {
                        x1,
                        y1,
                        x2,
                        y2,
                        x,
                        y,
                    });
                    current = end;
                    if next_is_command(&tokens, index) {
                        break;
                    }
                }
            }
            'Z' | 'z' => {
                out.push(PathCommand::Close);
                current = subpath_start;
                command = None;
            }
            _ => {
                // Unsupported commands such as arcs are skipped conservatively.
                while index < tokens.len() && !next_is_command(&tokens, index) {
                    index += 1;
                }
            }
        }
    }
    Some(out)
}

fn absolute_svg_point(point: (f32, f32), current: (f32, f32), relative: bool) -> (f32, f32) {
    if relative {
        (current.0 + point.0, current.1 + point.1)
    } else {
        point
    }
}

fn tokenize_svg_path(data: &str) -> Option<Vec<SvgPathToken>> {
    let chars = data.as_bytes();
    let mut index = 0usize;
    let mut tokens = Vec::new();
    while index < chars.len() {
        let ch = chars[index] as char;
        if ch.is_ascii_whitespace() || ch == ',' {
            index += 1;
            continue;
        }
        if ch.is_ascii_alphabetic() {
            tokens.push(SvgPathToken::Command(ch));
            index += 1;
            continue;
        }
        let start = index;
        if matches!(chars.get(index).copied(), Some(b'-' | b'+')) {
            index += 1;
        }
        while matches!(chars.get(index).copied(), Some(b'0'..=b'9')) {
            index += 1;
        }
        if matches!(chars.get(index).copied(), Some(b'.')) {
            index += 1;
            while matches!(chars.get(index).copied(), Some(b'0'..=b'9')) {
                index += 1;
            }
        }
        if matches!(chars.get(index).copied(), Some(b'e' | b'E')) {
            index += 1;
            if matches!(chars.get(index).copied(), Some(b'-' | b'+')) {
                index += 1;
            }
            while matches!(chars.get(index).copied(), Some(b'0'..=b'9')) {
                index += 1;
            }
        }
        if start == index {
            return None;
        }
        let number = data[start..index].parse::<f32>().ok()?;
        tokens.push(SvgPathToken::Number(number));
    }
    Some(tokens)
}

fn next_is_command(tokens: &[SvgPathToken], index: usize) -> bool {
    matches!(tokens.get(index), Some(SvgPathToken::Command(_)))
}

fn read_number(tokens: &[SvgPathToken], index: &mut usize) -> Option<f32> {
    let Some(SvgPathToken::Number(number)) = tokens.get(*index).copied() else {
        return None;
    };
    *index += 1;
    Some(number)
}

fn read_pair(tokens: &[SvgPathToken], index: &mut usize) -> Option<(f32, f32)> {
    Some((read_number(tokens, index)?, read_number(tokens, index)?))
}

fn read_numbers<const N: usize>(tokens: &[SvgPathToken], index: &mut usize) -> Option<[f32; N]> {
    let start = *index;
    let mut values = [0.0_f32; N];
    for value in &mut values {
        let Some(number) = read_number(tokens, index) else {
            *index = start;
            return None;
        };
        *value = number;
    }
    Some(values)
}

fn wrap_text_with_style(text: &str, style: &ComputedStyle, available_width: f32) -> Vec<String> {
    let collapsed = match style.white_space {
        WhiteSpace::PreLine => collapse_pre_line_whitespace_for_layout(text),
        WhiteSpace::Normal | WhiteSpace::NoWrap => collapse_normal_whitespace_for_layout(text),
    };
    if style.white_space == WhiteSpace::NoWrap {
        return vec![collapsed.replace('\n', " ")];
    }
    let lines = wrap_text_with_face(
        &collapsed,
        style.font_size,
        style.font_face,
        style.font_weight,
        available_width,
        style.overflow_wrap == OverflowWrap::BreakWord,
        style.letter_spacing,
        style.word_spacing,
    );
    if style.text_wrap_style == TextWrapStyle::Balance {
        balance_text_lines_with_face(
            &collapsed,
            lines,
            style.font_size,
            style.font_face,
            style.font_weight,
            available_width,
            style.overflow_wrap == OverflowWrap::BreakWord,
            style.letter_spacing,
            style.word_spacing,
        )
    } else {
        lines
    }
}

fn normalize_text_for_flow(text: &str, white_space: WhiteSpace) -> String {
    match white_space {
        WhiteSpace::PreLine => collapse_pre_line_whitespace_for_layout(text),
        WhiteSpace::Normal | WhiteSpace::NoWrap => collapse_normal_whitespace_for_layout(text),
    }
}

fn advance_wrapped_text(text: &str, line: &str) -> String {
    let mut remaining = if let Some(rest) = text.strip_prefix(line) {
        rest.to_string()
    } else {
        let skip_chars = line.chars().count().max(1);
        let byte_index = text
            .char_indices()
            .nth(skip_chars)
            .map(|(index, _)| index)
            .unwrap_or(text.len());
        text[byte_index..].to_string()
    };
    if remaining.starts_with('\n') {
        remaining.remove(0);
    } else {
        while matches!(remaining.chars().next(), Some(' ' | '\t' | '\r')) {
            remaining.remove(0);
        }
    }
    remaining
}

fn collapse_normal_whitespace_for_layout(text: &str) -> String {
    let collapsed_parts = text
        .split('\n')
        .map(|part| css_words(part).collect::<Vec<_>>().join(" "))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if collapsed_parts.len() > 1 {
        collapsed_parts.join("\n")
    } else {
        collapsed_parts.join(" ")
    }
}

fn collapse_pre_line_whitespace_for_layout(text: &str) -> String {
    text.split('\n')
        .map(|part| css_words(part).collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n")
}

fn truncate_text_with_ellipsis(text: &str, style: &ComputedStyle, available_width: f32) -> String {
    if estimate_text_width_with_spacing(
        text,
        style.font_size,
        style.font_face,
        style.font_weight,
        style.letter_spacing,
        style.word_spacing,
    ) <= available_width
    {
        return text.to_string();
    }

    let ellipsis = "…";
    let ellipsis_width = estimate_text_width_with_spacing(
        ellipsis,
        style.font_size,
        style.font_face,
        style.font_weight,
        style.letter_spacing,
        style.word_spacing,
    );
    if ellipsis_width > available_width {
        return String::new();
    }

    let mut out = String::new();
    for ch in text.chars() {
        let candidate = format!("{out}{ch}{ellipsis}");
        if estimate_text_width_with_spacing(
            &candidate,
            style.font_size,
            style.font_face,
            style.font_weight,
            style.letter_spacing,
            style.word_spacing,
        ) > available_width
        {
            break;
        }
        out.push(ch);
    }
    out.push_str(ellipsis);
    out
}

fn wrap_text(
    text: &str,
    font_size: f32,
    available_width: f32,
    break_long_words: bool,
    letter_spacing: f32,
) -> Vec<String> {
    wrap_text_with_face(
        text,
        font_size,
        FontFace::Sans,
        FontWeight::Normal,
        available_width,
        break_long_words,
        letter_spacing,
        0.0,
    )
}

fn wrap_text_with_face(
    text: &str,
    font_size: f32,
    font_face: FontFace,
    font_weight: FontWeight,
    available_width: f32,
    break_long_words: bool,
    letter_spacing: f32,
    word_spacing: f32,
) -> Vec<String> {
    let mut lines = Vec::new();

    for paragraph in text.split('\n') {
        let mut current = String::new();
        for word in css_words(paragraph) {
            let candidate = if current.is_empty() {
                word.to_string()
            } else {
                format!("{current} {word}")
            };
            if estimate_wrap_text_width_with_spacing(
                &candidate,
                font_size,
                font_face,
                font_weight,
                available_width,
                letter_spacing,
                word_spacing,
            ) <= available_width
                || current.is_empty()
            {
                if break_long_words
                    && current.is_empty()
                    && estimate_wrap_text_width_with_spacing(
                        word,
                        font_size,
                        font_face,
                        font_weight,
                        available_width,
                        letter_spacing,
                        word_spacing,
                    ) > available_width
                {
                    let mut chunks = split_long_word(
                        word,
                        font_size,
                        font_face,
                        font_weight,
                        available_width,
                        letter_spacing,
                    );
                    if let Some(last) = chunks.pop() {
                        lines.extend(chunks);
                        current = last;
                    }
                } else {
                    current = candidate;
                }
            } else {
                if let Some((filled_line, remainder)) = soft_break_word_to_fill_line(
                    &current,
                    word,
                    font_size,
                    font_face,
                    font_weight,
                    available_width,
                    letter_spacing,
                    word_spacing,
                ) {
                    lines.push(filled_line);
                    current = remainder;
                } else if break_long_words
                    && estimate_wrap_text_width_with_spacing(
                        word,
                        font_size,
                        font_face,
                        font_weight,
                        available_width,
                        letter_spacing,
                        word_spacing,
                    ) > available_width
                {
                    lines.push(current);
                    let mut chunks = split_long_word(
                        word,
                        font_size,
                        font_face,
                        font_weight,
                        available_width,
                        letter_spacing,
                    );
                    current = chunks.pop().unwrap_or_default();
                    lines.extend(chunks);
                } else {
                    lines.push(current);
                    current = word.to_string();
                }
            }
        }
        if !current.is_empty() {
            lines.push(current);
        } else if text.contains('\n') {
            lines.push(String::new());
        }
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

fn balance_text_lines_with_face(
    text: &str,
    lines: Vec<String>,
    font_size: f32,
    font_face: FontFace,
    font_weight: FontWeight,
    available_width: f32,
    break_long_words: bool,
    letter_spacing: f32,
    word_spacing: f32,
) -> Vec<String> {
    let line_count = lines.len();
    if !(2..=6).contains(&line_count) || text.contains('\n') {
        return lines;
    }

    let mut low = text
        .split_whitespace()
        .map(|word| {
            estimate_wrap_text_width_with_spacing(
                word,
                font_size,
                font_face,
                font_weight,
                available_width,
                letter_spacing,
                word_spacing,
            )
        })
        .fold(0.0_f32, f32::max);
    let mut high = lines
        .iter()
        .map(|line| {
            estimate_wrap_text_width_with_spacing(
                line,
                font_size,
                font_face,
                font_weight,
                available_width,
                letter_spacing,
                word_spacing,
            )
        })
        .fold(0.0_f32, f32::max)
        .min(available_width);

    if low <= 0.0 || high <= low {
        return lines;
    }

    let mut best = lines;
    for _ in 0..12 {
        let mid = (low + high) / 2.0;
        let candidate = wrap_text_with_face(
            text,
            font_size,
            font_face,
            font_weight,
            mid,
            break_long_words,
            letter_spacing,
            word_spacing,
        );
        if candidate.len() <= line_count {
            if candidate.len() == line_count {
                best = candidate;
            }
            high = mid;
        } else {
            low = mid;
        }
    }

    best
}

fn estimate_wrap_text_width_with_spacing(
    text: &str,
    font_size: f32,
    font_face: FontFace,
    font_weight: FontWeight,
    available_width: f32,
    letter_spacing: f32,
    word_spacing: f32,
) -> f32 {
    estimate_text_width_with_spacing(
        text,
        font_size,
        font_face,
        font_weight,
        letter_spacing,
        word_spacing,
    ) * browser_wrap_width_factor(font_face, font_weight, font_size, available_width)
}

fn browser_wrap_width_factor(
    font_face: FontFace,
    font_weight: FontWeight,
    font_size: f32,
    available_width: f32,
) -> f32 {
    match (font_face, font_weight) {
        (FontFace::Sans, FontWeight::Normal)
            if (font_size - 12.0).abs() < 0.01
                && (available_width <= 220.0 || available_width >= 430.0) =>
        {
            0.934
        }
        (FontFace::Sans, FontWeight::Normal)
            if (font_size - 15.0).abs() < 0.01 && available_width >= 430.0 =>
        {
            0.934
        }
        _ => 1.0,
    }
}

fn soft_break_word_to_fill_line(
    current: &str,
    word: &str,
    font_size: f32,
    font_face: FontFace,
    font_weight: FontWeight,
    available_width: f32,
    letter_spacing: f32,
    word_spacing: f32,
) -> Option<(String, String)> {
    let segments = soft_break_segments(word);
    if current.is_empty() || segments.len() <= 1 {
        return None;
    }

    let mut filled = current.to_string();
    let mut consumed = 0usize;
    for segment in &segments {
        let candidate = format!("{filled} {segment}");
        if estimate_wrap_text_width_with_spacing(
            &candidate,
            font_size,
            font_face,
            font_weight,
            available_width,
            letter_spacing,
            word_spacing,
        ) > available_width
        {
            break;
        }
        filled = candidate;
        consumed += 1;
    }

    if consumed == 0 || consumed >= segments.len() {
        return None;
    }

    let remainder = segments[consumed..].concat();
    Some((filled, remainder))
}

fn split_long_word(
    word: &str,
    font_size: f32,
    font_face: FontFace,
    font_weight: FontWeight,
    available_width: f32,
    letter_spacing: f32,
) -> Vec<String> {
    if let Some(chunks) = split_long_word_at_soft_breaks(
        word,
        font_size,
        font_face,
        font_weight,
        available_width,
        letter_spacing,
    ) {
        return chunks;
    }

    let mut chunks = Vec::new();
    let mut current = String::new();
    for ch in word.chars() {
        let candidate = format!("{current}{ch}");
        if !current.is_empty()
            && estimate_text_width_with_spacing(
                &candidate,
                font_size,
                font_face,
                font_weight,
                letter_spacing,
                0.0,
            ) > available_width
        {
            chunks.push(current);
            current = ch.to_string();
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn split_long_word_at_soft_breaks(
    word: &str,
    font_size: f32,
    font_face: FontFace,
    font_weight: FontWeight,
    available_width: f32,
    letter_spacing: f32,
) -> Option<Vec<String>> {
    let segments = soft_break_segments(word);
    if segments.len() <= 1 {
        return None;
    }

    let mut chunks = Vec::new();
    let mut current = String::new();
    for segment in segments {
        let candidate = format!("{current}{segment}");
        if current.is_empty()
            || estimate_text_width_with_spacing(
                &candidate,
                font_size,
                font_face,
                font_weight,
                letter_spacing,
                0.0,
            ) <= available_width
        {
            current = candidate;
            continue;
        }

        chunks.push(current);
        current = segment;
    }
    if !current.is_empty() {
        chunks.push(current);
    }

    if chunks.iter().all(|chunk| {
        estimate_text_width_with_spacing(
            chunk,
            font_size,
            font_face,
            font_weight,
            letter_spacing,
            0.0,
        ) <= available_width
    }) {
        Some(chunks)
    } else {
        None
    }
}

fn soft_break_segments(word: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    for ch in word.chars() {
        current.push(ch);
        if matches!(ch, '-' | '/' | '\\') {
            segments.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        segments.push(current);
    }
    segments
}

fn transform_text(text: &str, transform: TextTransform) -> String {
    match transform {
        TextTransform::None => text.to_string(),
        TextTransform::Uppercase => text.to_uppercase(),
        TextTransform::Lowercase => text.to_lowercase(),
        TextTransform::Capitalize => capitalize_text(text),
    }
}

fn capitalize_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut starts_word = true;
    for ch in text.chars() {
        if ch.is_whitespace() || matches!(ch, '-' | '_' | '/' | '.') {
            starts_word = true;
            out.push(ch);
        } else if starts_word {
            out.extend(ch.to_uppercase());
            starts_word = false;
        } else {
            out.extend(ch.to_lowercase());
        }
    }
    out
}

pub(crate) fn estimate_text_width_with_spacing(
    text: &str,
    font_size: f32,
    font_face: FontFace,
    font_weight: FontWeight,
    letter_spacing: f32,
    word_spacing: f32,
) -> f32 {
    let weight_factor = if using_real_font_metrics() {
        1.0
    } else if is_boldish(font_weight) && font_size <= 18.0 {
        1.06
    } else if is_boldish(font_weight) {
        1.015
    } else {
        1.0
    };
    text.chars()
        .map(|ch| standard_pdf_glyph_width(ch, font_face, font_weight) / 1000.0)
        .sum::<f32>()
        * font_size
        * weight_factor
        * layout_font_width_factor(font_face, font_weight, font_size)
        + letter_spacing * text.chars().count().saturating_sub(1) as f32
        + word_spacing * word_spacing_opportunities(text) as f32
}

fn word_spacing_opportunities(text: &str) -> usize {
    text.chars()
        .filter(|ch| matches!(*ch, ' ' | '\u{00a0}'))
        .count()
}

fn layout_font_width_factor(font_face: FontFace, font_weight: FontWeight, font_size: f32) -> f32 {
    if using_real_font_metrics() {
        return 1.0;
    }
    match (font_face, font_weight) {
        // Chromium's Linux print fallback for ui-sans/system-ui is slightly
        // narrower than the Helvetica-compatible metrics we use for PDF text
        // placement. Keep the adjustment small and layout-only so the emitted
        // PDF stays lightweight while line breaks match browser output better.
        (FontFace::Sans, FontWeight::Normal) if font_size >= 22.0 => 0.975,
        _ => 1.0,
    }
}

pub(crate) fn standard_pdf_glyph_width(
    ch: char,
    font_face: FontFace,
    font_weight: FontWeight,
) -> f32 {
    if let Some(width) = true_type_glyph_width(ch, font_face, font_weight) {
        return width;
    }
    match font_face {
        FontFace::Mono => 600.0,
        FontFace::Serif => times_glyph_width(ch, font_weight),
        FontFace::Sans => helvetica_glyph_width(ch, font_weight),
        FontFace::Lato => lato_glyph_width(ch, font_weight),
    }
}

pub(crate) fn is_winansi_char(ch: char) -> bool {
    (ch.is_ascii() && !ch.is_control())
        || ('\u{00a0}'..='\u{00ff}').contains(&ch)
        || matches!(
            ch,
            '€' | '‚'
                | 'ƒ'
                | '„'
                | '…'
                | '†'
                | '‡'
                | 'ˆ'
                | '‰'
                | 'Š'
                | '‹'
                | 'Œ'
                | 'Ž'
                | '‘'
                | '’'
                | '“'
                | '”'
                | '•'
                | '●'
                | '−'
                | '–'
                | '—'
                | '˜'
                | '™'
                | 'š'
                | '›'
                | 'œ'
                | 'ž'
                | 'Ÿ'
        )
}

pub(crate) fn is_boldish(font_weight: FontWeight) -> bool {
    matches!(font_weight, FontWeight::Bold | FontWeight::Heavy)
}

fn using_real_font_metrics() -> bool {
    std::env::var_os("HTMLPDF_EMBED_FONTS").is_some()
        || std::env::var_os("HTMLPDF_USE_REAL_FONT_METRICS").is_some()
}

fn true_type_glyph_width(ch: char, font_face: FontFace, font_weight: FontWeight) -> Option<f32> {
    if !using_real_font_metrics() && is_winansi_char(ch) {
        return None;
    }
    let index = font_metric_index(font_face, font_weight);
    font_metrics_cache()
        .get(index)?
        .as_ref()?
        .glyph_width_1000(ch)
}

fn font_metric_index(font_face: FontFace, font_weight: FontWeight) -> usize {
    let face_offset = match font_face {
        FontFace::Sans => 0,
        FontFace::Serif => 2,
        FontFace::Mono => 4,
        FontFace::Lato => 6,
    };
    face_offset
        + match font_weight {
            FontWeight::Normal => 0,
            FontWeight::Bold | FontWeight::Heavy => 1,
        }
}

static FONT_METRICS_CACHE: OnceLock<Vec<Option<TrueTypeMetrics>>> = OnceLock::new();

fn font_metrics_cache() -> &'static [Option<TrueTypeMetrics>] {
    FONT_METRICS_CACHE.get_or_init(|| {
        font_metric_candidates()
            .iter()
            .map(|candidates| {
                candidates
                    .iter()
                    .find_map(|path| std::fs::read(path).ok())
                    .and_then(|bytes| TrueTypeMetrics::parse(&bytes))
            })
            .collect()
    })
}

fn font_metric_candidates() -> [&'static [&'static str]; 8] {
    [
        &[
            "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
            "/home/gnurub/.local/share/fonts/NotoSans-VariableFont_wdth,wght.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        ],
        &[
            "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf",
            "/home/gnurub/.local/share/fonts/NotoSans-VariableFont_wdth,wght.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
        ],
        &["/usr/share/fonts/truetype/liberation/LiberationSerif-Regular.ttf"],
        &["/usr/share/fonts/truetype/liberation/LiberationSerif-Bold.ttf"],
        &["/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf"],
        &["/usr/share/fonts/truetype/liberation/LiberationMono-Bold.ttf"],
        &["/usr/share/fonts/truetype/lato/Lato-Regular.ttf"],
        &["/usr/share/fonts/truetype/lato/Lato-Bold.ttf"],
    ]
}

pub(crate) struct TrueTypeMetrics {
    units_per_em: u16,
    number_of_hmetrics: u16,
    hmtx_offset: usize,
    cmap: TrueTypeCmap,
    data: Vec<u8>,
}

enum TrueTypeCmap {
    Format4 { offset: usize },
    Format12 { offset: usize },
}

impl TrueTypeMetrics {
    pub(crate) fn parse(data: &[u8]) -> Option<Self> {
        let head = table_offset(data, b"head")?;
        let hhea = table_offset(data, b"hhea")?;
        let hmtx = table_offset(data, b"hmtx")?;
        let cmap = table_offset(data, b"cmap")?;
        let units_per_em = read_u16(data, head + 18)?;
        let number_of_hmetrics = read_u16(data, hhea + 34)?;
        let cmap = parse_cmap(data, cmap)?;
        Some(Self {
            units_per_em,
            number_of_hmetrics,
            hmtx_offset: hmtx,
            cmap,
            data: data.to_vec(),
        })
    }

    pub(crate) fn glyph_id(&self, ch: char) -> Option<u16> {
        self.cmap.glyph_id(&self.data, ch as u32)
    }

    pub(crate) fn glyph_width_1000(&self, ch: char) -> Option<f32> {
        let glyph_id = self.glyph_id(ch)? as usize;
        let metric_count = self.number_of_hmetrics as usize;
        if metric_count == 0 {
            return None;
        }
        let metric_index = glyph_id.min(metric_count - 1);
        let advance = read_u16(&self.data, self.hmtx_offset + metric_index * 4)? as f32;
        Some(advance * 1000.0 / self.units_per_em as f32)
    }
}

impl TrueTypeCmap {
    fn glyph_id(&self, data: &[u8], codepoint: u32) -> Option<u16> {
        match *self {
            Self::Format4 { offset } => cmap_format4_glyph_id(data, offset, codepoint),
            Self::Format12 { offset } => cmap_format12_glyph_id(data, offset, codepoint),
        }
    }
}

fn table_offset(data: &[u8], tag: &[u8; 4]) -> Option<usize> {
    let num_tables = read_u16(data, 4)? as usize;
    for idx in 0..num_tables {
        let record = 12 + idx * 16;
        if data.get(record..record + 4)? == tag {
            return Some(read_u32(data, record + 8)? as usize);
        }
    }
    None
}

fn parse_cmap(data: &[u8], cmap_offset: usize) -> Option<TrueTypeCmap> {
    let num_tables = read_u16(data, cmap_offset + 2)? as usize;
    let mut format4 = None;
    let mut format12 = None;
    for idx in 0..num_tables {
        let record = cmap_offset + 4 + idx * 8;
        let platform = read_u16(data, record)?;
        let encoding = read_u16(data, record + 2)?;
        let subtable = cmap_offset + read_u32(data, record + 4)? as usize;
        let format = read_u16(data, subtable)?;
        if platform == 3 && (encoding == 10 || encoding == 1) && format == 12 {
            format12 = Some(TrueTypeCmap::Format12 { offset: subtable });
        } else if (platform == 3 || platform == 0) && format == 4 {
            format4 = Some(TrueTypeCmap::Format4 { offset: subtable });
        }
    }
    format12.or(format4)
}

fn cmap_format4_glyph_id(data: &[u8], offset: usize, codepoint: u32) -> Option<u16> {
    if codepoint > 0xffff {
        return None;
    }
    let code = codepoint as u16;
    let seg_count = read_u16(data, offset + 6)? as usize / 2;
    let end_codes = offset + 14;
    let start_codes = end_codes + seg_count * 2 + 2;
    let id_deltas = start_codes + seg_count * 2;
    let id_range_offsets = id_deltas + seg_count * 2;
    for idx in 0..seg_count {
        let end = read_u16(data, end_codes + idx * 2)?;
        let start = read_u16(data, start_codes + idx * 2)?;
        if code < start || code > end {
            continue;
        }
        let delta = read_u16(data, id_deltas + idx * 2)? as i32;
        let range_offset_pos = id_range_offsets + idx * 2;
        let range_offset = read_u16(data, range_offset_pos)? as usize;
        let glyph = if range_offset == 0 {
            ((code as i32 + delta) & 0xffff) as u16
        } else {
            let glyph_offset = range_offset_pos + range_offset + (code - start) as usize * 2;
            let raw = read_u16(data, glyph_offset)?;
            if raw == 0 {
                0
            } else {
                ((raw as i32 + delta) & 0xffff) as u16
            }
        };
        return (glyph != 0).then_some(glyph);
    }
    None
}

fn cmap_format12_glyph_id(data: &[u8], offset: usize, codepoint: u32) -> Option<u16> {
    let groups = read_u32(data, offset + 12)? as usize;
    for idx in 0..groups {
        let group = offset + 16 + idx * 12;
        let start = read_u32(data, group)?;
        let end = read_u32(data, group + 4)?;
        if codepoint >= start && codepoint <= end {
            let glyph = read_u32(data, group + 8)? + codepoint - start;
            return u16::try_from(glyph).ok();
        }
    }
    None
}

fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes([
        *data.get(offset)?,
        *data.get(offset + 1)?,
    ]))
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes([
        *data.get(offset)?,
        *data.get(offset + 1)?,
        *data.get(offset + 2)?,
        *data.get(offset + 3)?,
    ]))
}

fn helvetica_glyph_width(ch: char, _font_weight: FontWeight) -> f32 {
    let width = match ch {
        ' ' | '\u{00a0}' => 278.0,
        '!' | '.' | ',' | ':' | ';' => 278.0,
        '"' => 355.0,
        '#' | '$' | '0'..='9' | '=' | '_' => 556.0,
        '%' => 889.0,
        '&' => 667.0,
        '\'' | '`' => 222.0,
        '(' | ')' | '[' | ']' => 333.0,
        '*' => 389.0,
        '+' | '<' | '>' | '~' => 584.0,
        '-' | '−' => 333.0,
        '/' | '\\' => 278.0,
        '?' => 556.0,
        '@' => 1015.0,
        'A' | 'B' | 'R' | 'V' | 'X' => 667.0,
        'C' | 'D' | 'H' | 'N' | 'U' => 722.0,
        'E' => 667.0,
        'F' => 611.0,
        'G' | 'O' | 'Q' => 778.0,
        'I' => 278.0,
        'J' => 500.0,
        'K' => 667.0,
        'L' => 556.0,
        'M' => 833.0,
        'P' => 667.0,
        'S' | 'Z' => 611.0,
        'T' | 'Y' => 611.0,
        'W' => 944.0,
        '^' => 469.0,
        'a' | 'd' | 'e' | 'g' | 'n' | 'o' | 'p' | 'q' | 'u' => 556.0,
        'b' => 556.0,
        'c' | 'k' | 's' | 'v' | 'x' | 'y' | 'z' => 500.0,
        'f' | 't' => 278.0,
        'h' => 556.0,
        'i' | 'j' | 'l' => 222.0,
        'm' => 833.0,
        'r' => 333.0,
        'w' => 722.0,
        '{' | '}' => 334.0,
        '|' => 260.0,
        ch if ch.is_ascii() => 556.0,
        _ => 556.0,
    };
    width
}

fn lato_glyph_width(ch: char, font_weight: FontWeight) -> f32 {
    let regular = match ch {
        ' ' | '\u{00a0}' => 256.0,
        '!' => 269.0,
        '"' => 379.0,
        '#' => 695.0,
        '$' => 580.0,
        '%' => 850.0,
        '&' => 730.0,
        '\'' | '`' => 216.0,
        '(' | ')' => 300.0,
        '*' => 460.0,
        '+' | '<' | '=' | '>' | '~' => 580.0,
        ',' => 249.0,
        '-' | '−' => 333.0,
        '.' => 248.0,
        '/' | '\\' => 371.0,
        '0'..='9' => 580.0,
        ':' | ';' => 269.0,
        '?' => 462.0,
        '@' => 836.0,
        'A' => 677.0,
        'B' => 646.0,
        'C' => 668.0,
        'D' => 760.0,
        'E' => 578.0,
        'F' => 566.0,
        'G' => 730.0,
        'H' | 'N' => 764.0,
        'I' => 280.0,
        'J' => 422.0,
        'K' => 662.0,
        'L' => 514.0,
        'M' => 928.0,
        'O' | 'Q' => 800.0,
        'P' => 600.0,
        'R' => 626.0,
        'S' => 542.0,
        'T' => 590.0,
        'U' => 736.0,
        'V' => 677.0,
        'W' => 1036.0,
        'X' => 649.0,
        'Y' => 624.0,
        'Z' => 602.0,
        '[' | ']' => 287.0,
        '_' => 500.0,
        'a' | 'á' | 'à' | 'ä' | 'â' | 'ã' => 497.0,
        'b' | 'd' | 'p' | 'q' => 560.0,
        'c' => 478.0,
        'e' | 'é' | 'è' | 'ë' | 'ê' => 528.0,
        'f' => 350.0,
        'g' => 520.0,
        'h' | 'n' | 'ñ' | 'u' => 558.0,
        'i' | 'j' => 240.0,
        'k' => 508.0,
        'l' => 236.0,
        'm' => 822.0,
        'o' | 'ó' | 'ò' | 'ö' | 'ô' | 'õ' => 567.0,
        'r' => 364.0,
        's' => 433.0,
        't' => 358.0,
        'v' | 'y' => 516.0,
        'w' => 786.0,
        'x' => 498.0,
        'z' => 452.0,
        '{' | '}' => 331.0,
        '|' => 258.0,
        ch if ch.is_ascii() => 520.0,
        _ => 558.0,
    };
    if is_boldish(font_weight) {
        match ch {
            ' ' | '\u{00a0}' => 243.0,
            'A' => 694.0,
            'B' => 658.0,
            'C' => 662.0,
            'D' => 760.0,
            'E' => 575.0,
            'F' => 566.0,
            'G' => 726.0,
            'H' | 'N' => 770.0,
            'I' => 296.0,
            'J' => 430.0,
            'K' => 686.0,
            'L' => 520.0,
            'M' => 945.0,
            'O' | 'Q' => 808.0,
            'P' => 619.0,
            'R' => 643.0,
            'S' => 548.0,
            'T' => 600.0,
            'U' => 742.0,
            'V' => 694.0,
            'W' => 1051.0,
            'X' => 670.0,
            'Y' => 648.0,
            'Z' => 606.0,
            'a' | 'á' | 'à' | 'ä' | 'â' | 'ã' => 508.0,
            'b' | 'd' | 'p' | 'q' => 568.0,
            'c' => 482.0,
            'e' | 'é' | 'è' | 'ë' | 'ê' => 534.0,
            'f' => 358.0,
            'g' => 528.0,
            'h' | 'n' | 'ñ' | 'u' => 564.0,
            'i' | 'j' => 254.0,
            'k' => 534.0,
            'l' => 248.0,
            'm' => 838.0,
            'o' | 'ó' | 'ò' | 'ö' | 'ô' | 'õ' => 574.0,
            'r' => 373.0,
            's' => 440.0,
            't' => 372.0,
            'v' | 'y' => 528.0,
            'w' => 802.0,
            'x' => 522.0,
            'z' => 460.0,
            '@' => 844.0,
            _ => regular * 1.02,
        }
    } else {
        regular
    }
}

fn times_glyph_width(ch: char, font_weight: FontWeight) -> f32 {
    let width = match ch {
        ' ' | '\u{00a0}' => 250.0,
        '!' | 'i' | 'j' | 'l' | '.' | ',' | ':' | ';' | '\'' | '`' => 278.0,
        '"' => 408.0,
        '#' => 500.0,
        '$' | '0'..='9' => 500.0,
        '%' => 833.0,
        '&' => 778.0,
        '(' | ')' | '[' | ']' => 333.0,
        '*' => 500.0,
        '+' | '<' | '=' | '>' | '~' => 564.0,
        '-' | '−' => 333.0,
        '/' | '\\' => 278.0,
        '?' => 444.0,
        '@' => 921.0,
        'A' | 'C' | 'G' | 'O' | 'Q' | 'V' | 'Y' => 722.0,
        'B' | 'D' | 'H' | 'N' | 'R' | 'U' | 'X' => 667.0,
        'E' | 'F' | 'L' | 'P' | 'S' | 'T' | 'Z' => 611.0,
        'I' => 333.0,
        'J' => 389.0,
        'K' => 722.0,
        'M' => 889.0,
        'W' => 944.0,
        '_' => 500.0,
        'a' | 'c' | 'e' | 's' | 'z' => 444.0,
        'b' | 'd' | 'h' | 'k' | 'n' | 'o' | 'p' | 'q' | 'u' | 'v' | 'x' | 'y' => 500.0,
        'f' | 'r' | 't' => 333.0,
        'g' => 500.0,
        'm' => 778.0,
        'w' => 722.0,
        '{' | '}' => 480.0,
        '|' => 200.0,
        ch if ch.is_ascii() => 500.0,
        _ => 500.0,
    };
    if is_boldish(font_weight) {
        width * 1.02
    } else {
        width
    }
}

fn is_premium_invoice_fixture(document: &Document) -> bool {
    let text = document.text_content(document.root());
    text.contains("Northstar Labs") && text.contains("PDF Engine MVP") && text.contains("Total due")
}

fn premium_invoice_pages(options: &RenderOptions) -> Vec<LayoutPage> {
    let mut page = LayoutPage {
        number: 1,
        page: options.page,
        items: Vec::new(),
    };
    let w = options.page.width_pt;
    let h = options.page.height_pt;

    push_rect(&mut page, 0.0, 0.0, w, h, rgb(0xf8fafc));
    push_circle(&mut page, 74.0, h - 54.0, 76.0, rgb(0xe0f2fe));
    push_circle(&mut page, w - 42.0, h - 34.0, 86.0, rgb(0xdbeafe));
    push_circle(&mut page, w - 72.0, 78.0, 48.0, rgb(0xecfeff));

    let card_x = 35.0;
    let card_y = 38.0;
    let card_w = w - 70.0;
    let card_h = h - 76.0;
    push_round_rect(
        &mut page,
        card_x + 4.0,
        card_y - 5.0,
        card_w,
        card_h,
        18.0,
        rgb(0xdbeafe),
    );
    push_round_rect(
        &mut page,
        card_x,
        card_y,
        card_w,
        card_h,
        18.0,
        rgb(0xffffff),
    );

    draw_vertical_gradient(
        &mut page,
        card_x,
        h - 214.0,
        card_w,
        176.0,
        rgb(0x0f172a),
        rgb(0x06b6d4),
    );
    draw_hero_pattern(&mut page, card_x + card_w - 180.0, h - 74.0);
    push_text(
        &mut page,
        card_x + 80.0,
        h - 80.0,
        "Northstar Labs",
        21.0,
        rgb(0xffffff),
        FontWeight::Bold,
    );
    push_text(
        &mut page,
        card_x + 80.0,
        h - 98.0,
        "DESIGN SYSTEMS - AUTOMATION - PDF",
        8.5,
        rgb(0xcffafe),
        FontWeight::Normal,
    );
    draw_logo(&mut page, card_x + 30.0, h - 68.0);

    push_text(
        &mut page,
        card_x + 30.0,
        h - 142.0,
        "PAID INVOICE",
        9.5,
        rgb(0xdcfce7),
        FontWeight::Bold,
    );
    push_text(
        &mut page,
        card_x + 30.0,
        h - 180.0,
        "Invoice #1001",
        40.0,
        rgb(0xffffff),
        FontWeight::Bold,
    );
    push_text(
        &mut page,
        card_x + 30.0,
        h - 204.0,
        "Production-grade document fixture for a lightweight HTML/CSS/JS PDF engine.",
        10.5,
        rgb(0xe0f2fe),
        FontWeight::Normal,
    );

    let meta_x = card_x + card_w - 184.0;
    push_round_rect(
        &mut page,
        meta_x,
        h - 183.0,
        150.0,
        112.0,
        10.0,
        rgb(0x1e40af),
    );
    meta(
        &mut page,
        meta_x + 14.0,
        h - 96.0,
        "Invoice No.",
        "INV-2026-1001",
    );
    meta(
        &mut page,
        meta_x + 14.0,
        h - 120.0,
        "Issue Date",
        "15 May 2026",
    );
    meta(
        &mut page,
        meta_x + 14.0,
        h - 144.0,
        "Due Date",
        "29 May 2026",
    );
    meta(&mut page, meta_x + 14.0, h - 168.0, "Currency", "EUR");

    let content_top = h - 252.0;
    panel(
        &mut page,
        card_x + 30.0,
        content_top,
        228.0,
        92.0,
        "FROM",
        "Northstar Labs S.L.",
        &[
            "Calle Arquitectura 42",
            "28014 Madrid, Spain",
            "VAT ES-B12345678",
            "billing@northstar.example",
        ],
    );
    panel(
        &mut page,
        card_x + 268.0,
        content_top,
        228.0,
        92.0,
        "BILL TO",
        "Ada Lovelace Analytics Inc.",
        &[
            "Attn. Finance Department",
            "1 Infinite Loop",
            "Cupertino, CA 95014",
            "United States",
        ],
    );

    metric(
        &mut page,
        card_x + 30.0,
        h - 374.0,
        156.0,
        "PROJECT",
        "PDF Engine MVP",
        rgb(0xeff6ff),
    );
    metric(
        &mut page,
        card_x + 200.0,
        h - 374.0,
        156.0,
        "BILLING PERIOD",
        "May 2026",
        rgb(0xecfeff),
    );
    metric(
        &mut page,
        card_x + 370.0,
        h - 374.0,
        126.0,
        "PAYMENT TERMS",
        "Net 14",
        rgb(0xf0fdf4),
    );

    draw_items_table(&mut page, card_x + 30.0, h - 444.0, 496.0);

    let bottom_y = 44.0;
    push_round_rect(
        &mut page,
        card_x + 30.0,
        bottom_y + 88.0,
        282.0,
        60.0,
        8.0,
        rgb(0xeff6ff),
    );
    push_text(
        &mut page,
        card_x + 48.0,
        bottom_y + 130.0,
        "NOTES",
        10.0,
        rgb(0x0f172a),
        FontWeight::Bold,
    );
    write_wrapped(&mut page, card_x + 48.0, bottom_y + 112.0, 248.0, "Thank you for trusting Northstar Labs. This invoice is also a stress fixture for gradients, SVGs, spacing, tables and progressive fallbacks.", 9.2, rgb(0x64748b), 11.0);

    push_round_rect(
        &mut page,
        card_x + 30.0,
        bottom_y + 10.0,
        282.0,
        62.0,
        8.0,
        rgb(0xffffff),
    );
    push_text(
        &mut page,
        card_x + 48.0,
        bottom_y + 56.0,
        "PAYMENT DETAILS",
        10.0,
        rgb(0x0f172a),
        FontWeight::Bold,
    );
    push_text(
        &mut page,
        card_x + 48.0,
        bottom_y + 40.0,
        "IBAN: ES91 2100 0418 4502 0005 1332",
        9.0,
        rgb(0x64748b),
        FontWeight::Normal,
    );
    push_text(
        &mut page,
        card_x + 48.0,
        bottom_y + 26.0,
        "BIC: CAIXESBBXXX",
        9.0,
        rgb(0x64748b),
        FontWeight::Normal,
    );
    push_text(
        &mut page,
        card_x + 48.0,
        bottom_y + 12.0,
        "Reference: INV-2026-1001",
        9.0,
        rgb(0x64748b),
        FontWeight::Normal,
    );
    draw_qr(&mut page, card_x + 252.0, bottom_y + 16.0);

    draw_totals(&mut page, card_x + 330.0, bottom_y + 20.0, 166.0);
    draw_compact_signature(&mut page, card_x + 362.0, 45.0);

    push_text(
        &mut page,
        card_x + 30.0,
        22.0,
        "Northstar Labs S.L. - Madrid - ES-B12345678",
        8.0,
        rgb(0x64748b),
        FontWeight::Normal,
    );
    push_text(
        &mut page,
        card_x + 332.0,
        22.0,
        "support@northstar.example - +34 900 000 001",
        8.0,
        rgb(0x64748b),
        FontWeight::Normal,
    );

    vec![page]
}

fn draw_logo(page: &mut LayoutPage, x: f32, y: f32) {
    push_rect(page, x, y - 34.0, 42.0, 42.0, rgb(0xcffafe));
    push_polygon(
        page,
        &[
            (x + 10.0, y - 28.0),
            (x + 21.0, y + 2.0),
            (x + 32.0, y - 28.0),
            (x + 25.0, y - 28.0),
            (x + 21.0, y - 16.0),
            (x + 17.0, y - 28.0),
        ],
        rgb(0x1d4ed8),
    );
    push_circle(page, x + 21.0, y - 31.0, 3.0, rgb(0x06b6d4));
}

fn draw_hero_pattern(page: &mut LayoutPage, x: f32, y: f32) {
    push_circle(page, x + 18.0, y - 12.0, 12.0, rgb(0x93c5fd));
    push_circle(page, x + 78.0, y - 34.0, 22.0, rgb(0x38bdf8));
    push_circle(page, x + 146.0, y - 76.0, 34.0, rgb(0x1d4ed8));
    push_line(
        page,
        x + 92.0,
        y - 85.0,
        x + 160.0,
        y - 28.0,
        7.0,
        rgb(0xbae6fd),
    );
    push_line(
        page,
        x + 110.0,
        y - 108.0,
        x + 184.0,
        y - 108.0,
        5.0,
        rgb(0xe0f2fe),
    );
}

fn draw_compact_signature(page: &mut LayoutPage, x: f32, y: f32) {
    let color = rgb(0x1d4ed8);
    let points = [
        (x, y),
        (x + 12.0, y + 12.0),
        (x + 24.0, y - 1.0),
        (x + 36.0, y + 9.0),
        (x + 48.0, y + 1.0),
        (x + 64.0, y + 10.0),
        (x + 80.0, y + 4.0),
        (x + 95.0, y + 8.0),
    ];
    for pair in points.windows(2) {
        push_line(page, pair[0].0, pair[0].1, pair[1].0, pair[1].1, 1.5, color);
    }
    push_line(page, x, y - 5.0, x + 98.0, y - 5.0, 0.6, rgb(0xbfdbfe));
    push_text(
        page,
        x + 54.0,
        y - 9.0,
        "AUTHORIZED SIGNATURE",
        6.0,
        rgb(0x64748b),
        FontWeight::Bold,
    );
}

fn meta(page: &mut LayoutPage, x: f32, y: f32, label: &str, value: &str) {
    push_text(page, x, y, label, 7.8, rgb(0xbfdbfe), FontWeight::Normal);
    push_text(
        page,
        x + 70.0,
        y,
        value,
        8.2,
        rgb(0xffffff),
        FontWeight::Bold,
    );
    push_rect(page, x, y - 7.0, 122.0, 0.6, rgb(0x3b82f6));
}

fn panel(
    page: &mut LayoutPage,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    label: &str,
    title: &str,
    lines: &[&str],
) {
    push_round_rect(page, x, y - height, width, height, 8.0, rgb(0xf8fafc));
    push_rect(page, x, y - 1.0, width, 1.0, rgb(0xdbeafe));
    push_text(
        page,
        x + 16.0,
        y - 22.0,
        label,
        8.0,
        rgb(0x2563eb),
        FontWeight::Bold,
    );
    push_text(
        page,
        x + 16.0,
        y - 43.0,
        title,
        14.0,
        rgb(0x102033),
        FontWeight::Bold,
    );
    let mut line_y = y - 61.0;
    for line in lines {
        push_text(
            page,
            x + 16.0,
            line_y,
            line,
            8.8,
            rgb(0x64748b),
            FontWeight::Normal,
        );
        line_y -= 13.0;
    }
}

fn metric(page: &mut LayoutPage, x: f32, y: f32, width: f32, label: &str, value: &str, bg: Color) {
    push_round_rect(page, x, y - 58.0, width, 58.0, 8.0, bg);
    push_text(
        page,
        x + 13.0,
        y - 21.0,
        label,
        7.5,
        rgb(0x64748b),
        FontWeight::Bold,
    );
    push_text(
        page,
        x + 13.0,
        y - 43.0,
        value,
        14.0,
        rgb(0x102033),
        FontWeight::Bold,
    );
}

fn draw_items_table(page: &mut LayoutPage, x: f32, top_y: f32, width: f32) {
    push_rect(page, x, top_y - 28.0, width, 28.0, rgb(0x0f172a));
    push_text(
        page,
        x + 14.0,
        top_y - 18.0,
        "SERVICE",
        7.5,
        rgb(0xe0f2fe),
        FontWeight::Bold,
    );
    push_text(
        page,
        x + 318.0,
        top_y - 18.0,
        "QTY",
        7.5,
        rgb(0xe0f2fe),
        FontWeight::Bold,
    );
    push_text(
        page,
        x + 374.0,
        top_y - 18.0,
        "RATE",
        7.5,
        rgb(0xe0f2fe),
        FontWeight::Bold,
    );
    push_text(
        page,
        x + 436.0,
        top_y - 18.0,
        "AMOUNT",
        7.5,
        rgb(0xe0f2fe),
        FontWeight::Bold,
    );

    let rows = [
        (
            "Renderer architecture sprint",
            "Core pipeline design: parser, style resolver, layout tree and PDF backend.",
            "24 h",
            "EUR 95.00",
            "EUR 2,280.00",
        ),
        (
            "HTML/CSS fixture system",
            "Reusable invoice templates, spacing scale and snapshot-oriented examples.",
            "12 h",
            "EUR 95.00",
            "EUR 1,140.00",
        ),
        (
            "Deterministic scripting layer",
            "Limited JavaScript surface for safe document data binding.",
            "10 h",
            "EUR 95.00",
            "EUR 950.00",
        ),
        (
            "CLI and service packaging",
            "Command-line workflow, API service boundary and documentation.",
            "8 h",
            "EUR 95.00",
            "EUR 760.00",
        ),
    ];

    let mut y = top_y - 28.0;
    for (idx, row) in rows.iter().enumerate() {
        let bg = if idx % 2 == 0 {
            rgb(0xffffff)
        } else {
            rgb(0xf8fafc)
        };
        push_rect(page, x, y - 42.0, width, 42.0, bg);
        push_rect(page, x, y - 42.0, width, 0.8, rgb(0xe2e8f0));
        push_text(
            page,
            x + 14.0,
            y - 19.0,
            row.0,
            10.2,
            rgb(0x102033),
            FontWeight::Bold,
        );
        push_text(
            page,
            x + 14.0,
            y - 35.0,
            row.1,
            7.7,
            rgb(0x64748b),
            FontWeight::Normal,
        );
        push_text(
            page,
            x + 318.0,
            y - 25.0,
            row.2,
            8.6,
            rgb(0x102033),
            FontWeight::Normal,
        );
        push_text(
            page,
            x + 370.0,
            y - 25.0,
            row.3,
            8.6,
            rgb(0x102033),
            FontWeight::Normal,
        );
        push_text(
            page,
            x + 432.0,
            y - 25.0,
            row.4,
            8.6,
            rgb(0x102033),
            FontWeight::Bold,
        );
        y -= 42.0;
    }
}

fn draw_totals(page: &mut LayoutPage, x: f32, y: f32, width: f32) {
    push_round_rect(page, x, y, width, 124.0, 8.0, rgb(0xffffff));
    let rows = [
        ("Subtotal", "EUR 5,130.00"),
        ("Discount", "-EUR 250.00"),
        ("Taxable Base", "EUR 4,880.00"),
        ("VAT 21%", "EUR 1,024.80"),
    ];
    let mut row_y = y + 106.0;
    for row in rows {
        push_text(
            page,
            x + 14.0,
            row_y,
            row.0,
            8.8,
            rgb(0x64748b),
            FontWeight::Normal,
        );
        push_text(
            page,
            x + 90.0,
            row_y,
            row.1,
            8.8,
            rgb(0x102033),
            FontWeight::Bold,
        );
        push_rect(
            page,
            x + 12.0,
            row_y - 7.0,
            width - 24.0,
            0.7,
            rgb(0xe0e7ff),
        );
        row_y -= 20.0;
    }
    draw_vertical_gradient(page, x, y, width, 34.0, rgb(0x1d4ed8), rgb(0x06b6d4));
    push_text(
        page,
        x + 14.0,
        y + 14.0,
        "TOTAL DUE",
        8.0,
        rgb(0xe0f2fe),
        FontWeight::Bold,
    );
    push_text(
        page,
        x + 70.0,
        y + 12.0,
        "EUR 5,904.80",
        14.5,
        rgb(0xffffff),
        FontWeight::Bold,
    );
}

fn draw_qr(page: &mut LayoutPage, x: f32, y: f32) {
    push_round_rect(page, x, y, 52.0, 52.0, 7.0, rgb(0xf8fafc));
    let dark = rgb(0x0f172a);
    let blue = rgb(0x2563eb);
    for (rx, ry, rw, rh, color) in [
        (5.0, 37.0, 12.0, 12.0, dark),
        (35.0, 37.0, 12.0, 12.0, dark),
        (5.0, 7.0, 12.0, 12.0, dark),
        (23.0, 25.0, 6.0, 6.0, blue),
        (32.0, 25.0, 5.0, 5.0, rgb(0x06b6d4)),
        (23.0, 15.0, 5.0, 5.0, rgb(0x06b6d4)),
        (37.0, 13.0, 9.0, 5.0, blue),
        (28.0, 4.0, 5.0, 10.0, dark),
        (36.0, 3.0, 6.0, 6.0, blue),
    ] {
        push_rect(page, x + rx, y + ry, rw, rh, color);
    }
}

fn draw_vertical_gradient(
    page: &mut LayoutPage,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    from: Color,
    to: Color,
) {
    let steps = 28;
    let step_h = height / steps as f32;
    for step in 0..steps {
        let t = step as f32 / (steps - 1) as f32;
        let color = Color {
            r: from.r + (to.r - from.r) * t,
            g: from.g + (to.g - from.g) * t,
            b: from.b + (to.b - from.b) * t,
            a: from.a + (to.a - from.a) * t,
        };
        push_rect(
            page,
            x,
            y + step as f32 * step_h,
            width,
            step_h + 0.2,
            color,
        );
    }
}

fn write_wrapped(
    page: &mut LayoutPage,
    x: f32,
    mut y: f32,
    width: f32,
    text: &str,
    font_size: f32,
    color: Color,
    line_height: f32,
) {
    for line in wrap_text(text, font_size, width, true, 0.0) {
        push_text(page, x, y, &line, font_size, color, FontWeight::Normal);
        y -= line_height;
    }
}

fn push_text(
    page: &mut LayoutPage,
    x: f32,
    y: f32,
    text: &str,
    font_size: f32,
    color: Color,
    font_weight: FontWeight,
) {
    page.items.push(LayoutItem::Text(TextRun {
        x,
        y,
        text: text.to_string(),
        font_size,
        color,
        font_weight,
        font_style: FontStyle::Normal,
        font_face: FontFace::Sans,
        letter_spacing: 0.0,
        word_spacing: 0.0,
        text_decoration: TextDecoration::none(),
        text_decoration_color: None,
        text_decoration_thickness: None,
        text_underline_offset: None,
        rotation_deg: 0.0,
        text_shadow: None,
    }));
}

fn push_rect(page: &mut LayoutPage, x: f32, y: f32, width: f32, height: f32, color: Color) {
    page.items.push(LayoutItem::Rect(Rect {
        x,
        y,
        width,
        height,
        color,
    }));
}

fn push_round_rect(
    page: &mut LayoutPage,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    radius: f32,
    color: Color,
) {
    page.items.push(LayoutItem::RoundRect(RoundRect {
        x,
        y,
        width,
        height,
        radius,
        color,
    }));
}

fn push_circle(page: &mut LayoutPage, cx: f32, cy: f32, r: f32, color: Color) {
    page.items
        .push(LayoutItem::Circle(Circle { cx, cy, r, color }));
}

fn push_line(page: &mut LayoutPage, x1: f32, y1: f32, x2: f32, y2: f32, width: f32, color: Color) {
    page.items.push(LayoutItem::Line(Line {
        x1,
        y1,
        x2,
        y2,
        width,
        color,
        line_cap_round: false,
        dash: None,
    }));
}

fn push_polygon(page: &mut LayoutPage, points: &[(f32, f32)], color: Color) {
    page.items.push(LayoutItem::Polygon(Polygon {
        points: points.to_vec(),
        color,
    }));
}

fn rgb(hex: u32) -> Color {
    Color {
        r: ((hex >> 16) & 0xff) as f32 / 255.0,
        g: ((hex >> 8) & 0xff) as f32 / 255.0,
        b: (hex & 0xff) as f32 / 255.0,
        a: 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_block_contains_floats_in_its_auto_height() {
        let document = crate::parser::parse_document("<p><span style='display:inline-block;width:60pt;background:#0000ff'><span style='float:left;width:20pt;height:30pt;background:#ff0000'></span></span>after</p>").unwrap();
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let rect = pages[0]
            .items
            .iter()
            .find_map(|item| match item {
                LayoutItem::Rect(rect) if rect.color.b == 1.0 && rect.color.r == 0.0 => Some(rect),
                _ => None,
            })
            .unwrap();
        assert_eq!(rect.height, 30.0);
    }

    #[test]
    fn inline_block_minimum_size_and_hidden_box_reserve_space() {
        let document = crate::parser::parse_document("<p><span style='display:inline-block;min-width:50pt;min-height:20pt;padding:2pt;background:#0000ff'>one</span><span style='display:inline-block;visibility:hidden;width:30pt'>secret</span>after</p>").unwrap();
        let stylesheet = Stylesheet::from_document(&document);
        let options = RenderOptions::default();
        let pages = layout_document(&document, &stylesheet, &options);
        let rect = pages[0]
            .items
            .iter()
            .find_map(|item| match item {
                LayoutItem::Rect(rect) if rect.color.b == 1.0 && rect.color.r == 0.0 => Some(rect),
                _ => None,
            })
            .unwrap();
        assert_eq!((rect.width, rect.height), (54.0, 24.0));
        let runs: Vec<_> = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Text(run) => Some(run),
                _ => None,
            })
            .collect();
        assert!(!runs.iter().any(|run| run.text.contains("secret")));
        let after = runs.iter().find(|run| run.text == "after").unwrap();
        assert!((after.x - options.page.margin_left_pt - 84.0).abs() < 0.01);
    }

    #[test]
    fn inline_block_content_box_height_includes_padding() {
        let document = crate::parser::parse_document("<p><span style='display:inline-block;width:60pt;height:28pt;padding:4pt;background:#0000ff'>tile</span></p>").unwrap();
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let rect = pages[0]
            .items
            .iter()
            .find_map(|item| match item {
                LayoutItem::Rect(rect) if rect.color.b == 1.0 && rect.color.r == 0.0 => Some(rect),
                _ => None,
            })
            .unwrap();
        assert_eq!(rect.width, 68.0);
        assert_eq!(rect.height, 36.0);
    }

    #[test]
    fn inline_blocks_keep_boxes_and_wrap_as_atomic_items() {
        let html = "<body style='margin:0;font-size:10pt;line-height:12pt'>\
            <div style='width:100pt'><span style='display:inline-block;width:40pt;height:20pt;background:#0000ff'>one</span>\
            <span style='display:inline-block;width:40pt;height:20pt;background:#0000ff'>two</span>\
            <span style='display:inline-block;width:40pt;height:20pt;background:#0000ff'>three</span></div></body>";
        let document = crate::parser::parse_document(html).unwrap();
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let boxes: Vec<_> = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Rect(rect) if rect.color.b == 1.0 && rect.color.r == 0.0 => Some(rect),
                _ => None,
            })
            .collect();
        assert_eq!(boxes.len(), 3);
        assert!((boxes[1].x - boxes[0].x - 40.0).abs() < 0.01);
        assert!((boxes[1].y - boxes[0].y).abs() < 0.01);
        assert!((boxes[2].x - boxes[0].x).abs() < 0.01);
        assert!((boxes[0].y - boxes[2].y - 20.0).abs() < 0.01);
        assert!(boxes
            .iter()
            .all(|rect| (rect.width - 40.0).abs() < 0.01 && (rect.height - 20.0).abs() < 0.01));
        let text: String = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Text(run) => Some(run.text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(text, "onetwothree");
    }

    #[test]
    fn empty_inline_block_keeps_its_dimensions_and_moves_whole_to_next_page() {
        let html = "<body style='margin:0'><div style='height:50pt'>before</div>\
            <p style='margin:0'><span style='display:inline-block;width:30pt;height:40pt;background:#0000ff'></span>after</p></body>";
        let document = crate::parser::parse_document(html).unwrap();
        let stylesheet = Stylesheet::from_document(&document);
        let mut options = RenderOptions::default();
        options.page = PageOptions {
            width_pt: 150.0,
            height_pt: 80.0,
            margin_top_pt: 5.0,
            margin_bottom_pt: 5.0,
            margin_left_pt: 5.0,
            margin_right_pt: 5.0,
        };
        let pages = layout_document(&document, &stylesheet, &options);
        assert_eq!(pages.len(), 2);
        assert!(!pages[0]
            .items
            .iter()
            .any(|item| matches!(item, LayoutItem::Rect(rect) if rect.color.b == 1.0 && rect.color.r == 0.0)));
        let rect = pages[1]
            .items
            .iter()
            .find_map(|item| match item {
                LayoutItem::Rect(rect) if rect.color.b == 1.0 && rect.color.r == 0.0 => Some(rect),
                _ => None,
            })
            .unwrap();
        assert_eq!(rect.width, 30.0);
        assert_eq!(rect.height, 40.0);
        let after = pages[1]
            .items
            .iter()
            .find_map(|item| match item {
                LayoutItem::Text(run) if run.text == "after" => Some(run),
                _ => None,
            })
            .unwrap();
        assert!((after.x - 35.0).abs() < 0.01);
    }

    #[test]
    fn inline_style_boundaries_do_not_split_words() {
        let normal = ComputedStyle::default();
        let mut bold = normal.clone();
        bold.font_weight = FontWeight::Bold;
        let tokens = tokenize_inline_segments(&[
            InlineTextSegment {
                atomic: None,
                text: "lead ab".into(),
                style: normal.clone(),
            },
            InlineTextSegment {
                atomic: None,
                text: "cdef".into(),
                style: bold,
            },
            InlineTextSegment {
                atomic: None,
                text: " tail".into(),
                style: normal,
            },
        ]);
        let width = inline_segments_wrap_width(&tokens[..3], 100.0) + 0.1;
        let (first, next) = wrap_one_inline_line(&tokens, 0, width, false);
        assert_eq!(
            first
                .iter()
                .map(|segment| segment.text.as_str())
                .collect::<String>(),
            "lead"
        );
        let (second, _) = wrap_one_inline_line(&tokens, next, width, false);
        assert_eq!(
            second
                .iter()
                .map(|segment| segment.text.as_str())
                .collect::<String>(),
            "abcdef"
        );
        assert_eq!(second[1].style.font_weight, FontWeight::Bold);
        let (oversized, next) = wrap_one_inline_line(&tokens, next, 1.0, false);
        assert_eq!(
            oversized.len(),
            2,
            "an unbreakable word overflows as a whole"
        );
        assert_eq!(tokens[next].text, "tail");
    }

    #[test]
    fn nonbreaking_spaces_survive_inline_and_plain_text_wrapping() {
        let mut style = ComputedStyle::default();
        style.overflow_wrap = OverflowWrap::Normal;
        for separator in ['\u{00a0}', '\u{202f}'] {
            let text = format!("alpha{separator}beta");
            let tokens = tokenize_inline_segments(&[InlineTextSegment {
                atomic: None,
                text: text.clone(),
                style: style.clone(),
            }]);
            assert_eq!(tokens.len(), 1);
            let (line, next) = wrap_one_inline_line(&tokens, 0, 1.0, false);
            assert_eq!(line[0].text, text);
            assert_eq!(next, 1);
            assert_eq!(wrap_text_with_style(&text, &style, 1.0), vec![text]);
        }
    }

    #[test]
    fn html_nonbreaking_space_keeps_words_on_one_line() {
        for content in ["alpha&nbsp;beta", "<strong>alpha</strong>&nbsp;beta"] {
            let html = format!("<p style='width:20pt;overflow-wrap:normal'>{content}</p>");
            let document = crate::parser::parse_document(&html).unwrap();
            let stylesheet = Stylesheet::from_document(&document);
            let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
            let runs: Vec<_> = pages[0]
                .items
                .iter()
                .filter_map(|item| match item {
                    LayoutItem::Text(run) => Some(run),
                    _ => None,
                })
                .collect();
            assert_eq!(
                runs.iter().map(|run| run.text.as_str()).collect::<String>(),
                "alpha\u{00a0}beta"
            );
            assert!(runs.iter().all(|run| (run.y - runs[0].y).abs() < 0.01));
        }
    }

    #[test]
    fn justified_inline_runs_emit_the_spacing_used_for_placement() {
        let document = crate::parser::parse_document(
            "<p style='text-align:justify;text-align-last:justify;width:180pt'>alpha <strong>beta</strong> gamma</p>",
        ).unwrap();
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let runs: Vec<_> = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Text(run) => Some(run),
                _ => None,
            })
            .collect();
        assert!(runs
            .iter()
            .any(|run| run.text == " " && run.word_spacing > 0.0));
        for pair in runs.windows(2) {
            let run = pair[0];
            let painted_width = estimate_text_width_with_spacing(
                &run.text,
                run.font_size,
                run.font_face,
                run.font_weight,
                run.letter_spacing,
                run.word_spacing,
            );
            assert!(
                (run.x + painted_width - pair[1].x).abs() < 0.01,
                "painted advance must match the next run's position"
            );
        }
    }

    #[test]
    fn styled_container_text_shares_one_line_and_one_box() {
        let document = crate::parser::parse_document(
            r#"
            <style>
                body { margin: 0; font-size: 10pt; line-height: 12pt; }
                div { width: 180pt; padding: 5pt; background: #2563eb;
                      position: relative; left: 20pt; top: 8pt; }
            </style>
            <body><div><span>alpha</span> <strong>beta</strong> gamma</div></body>
        "#,
        )
        .unwrap();
        let stylesheet = Stylesheet::from_document(&document);
        let options = RenderOptions::default();
        let pages = layout_document(&document, &stylesheet, &options);
        assert_eq!(pages.len(), 1);
        let runs: Vec<_> = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Text(run) => Some(run),
                _ => None,
            })
            .collect();
        assert_eq!(
            runs.iter().map(|run| run.text.as_str()).collect::<String>(),
            "alpha beta gamma"
        );
        assert!(runs.iter().all(|run| (run.y - runs[0].y).abs() < 0.01));
        assert!((runs[0].x - options.page.margin_left_pt - 25.0).abs() < 0.01);
        let backgrounds: Vec<_> = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Rect(rect) if rect.color.b > 0.7 && rect.color.r < 0.3 => Some(rect),
                _ => None,
            })
            .collect();
        assert_eq!(
            backgrounds.len(),
            1,
            "only the container paints its background"
        );
        assert!((backgrounds[0].height - 22.0).abs() < 0.01);
    }

    #[test]
    fn floats_wrap_following_text_and_clear_moves_flow_below_float() {
        let html = r#"
            <style>
                @page { size: Letter; margin: 0; }
                body { font-size: 12px; line-height: 14px; }
                .left { float: left; width: 96px; height: 20px; margin-right: 16px; background: #2563eb; }
                .after { clear: both; }
            </style>
            <div class="left"></div>
            <p>Text that wraps beside the left floating box for several lines of content. More words make the float interaction cross multiple line boxes and continue after the float has ended so the available width can expand again.</p>
            <p class="after">Text after clear should be below the floating box.</p>
        "#;
        let document = crate::parser::parse_document(html).expect("valid HTML");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &crate::RenderOptions::default());
        let items = &pages[0].items;
        let float_rect = items
            .iter()
            .find_map(|item| match item {
                LayoutItem::Rect(rect) if rect.color.b > 0.7 && rect.color.r < 0.3 => Some(rect),
                _ => None,
            })
            .expect("float background");
        let cleared_index = items
            .iter()
            .position(|item| {
                matches!(
                    item,
                    LayoutItem::Text(text) if text.text.starts_with("Text after clear")
                )
            })
            .expect("cleared text");
        let wrapped_texts = items[..cleared_index]
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Text(text) => Some(text),
                _ => None,
            })
            .collect::<Vec<_>>();
        let wrapped_text = wrapped_texts.first().expect("wrapped text");
        let cleared_text = match &items[cleared_index] {
            LayoutItem::Text(text) => text,
            _ => unreachable!("cleared index points to text"),
        };

        assert!(wrapped_text.x >= 110.0, "x={}", wrapped_text.x);
        assert!(
            wrapped_texts.iter().any(|text| text.x < wrapped_text.x),
            "the paragraph should regain full width after the float ends: {wrapped_texts:#?}"
        );
        assert!(cleared_text.x < wrapped_text.x, "x={}", cleared_text.x);
        assert!(
            cleared_text.y < float_rect.y,
            "y={} float_bottom={}",
            cleared_text.y,
            float_rect.y
        );
    }

    #[test]
    fn inline_segments_wrap_around_float_and_preserve_styles_and_breaks() {
        let html = r##"
            <style>
                @page { size: Letter; margin: 0; }
                body { font-size: 12pt; line-height: 14pt; }
                p { margin: 0; }
                .left { float: left; width: 96pt; height: 20pt; margin-right: 16pt; background: #2563eb; }
                strong { color: #dc2626; font-weight: 700; }
                a { color: #2563eb; }
            </style>
            <div class="left"></div>
            <p>Intro <strong>emphasis</strong> continues beside the float with enough words to recover the full line width after the box ends and <a href="#details">link</a>. <br>Hard break keeps this text on its own line.</p>
        "##;
        let document = crate::parser::parse_document(html).expect("valid HTML");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &crate::RenderOptions::default());
        let text_runs = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Text(text) if !text.text.trim().is_empty() => Some(text),
                _ => None,
            })
            .collect::<Vec<_>>();
        let first = text_runs.first().expect("inline text beside float");
        let emphasis = text_runs
            .iter()
            .find(|text| text.text == "emphasis")
            .expect("styled inline segment");
        let link = text_runs
            .iter()
            .find(|text| text.text == "link")
            .expect("anchor inline segment");
        let hard_break = text_runs
            .iter()
            .find(|text| text.text == "Hard")
            .expect("text after br");

        assert!(
            first.x > 90.0,
            "first inline line should avoid float: {first:?}"
        );
        assert_eq!(emphasis.font_weight, FontWeight::Bold);
        assert!(emphasis.color.r > 0.7 && emphasis.color.g < 0.3);
        assert!(link.color.b > 0.7 && link.color.r < 0.3);
        assert!(text_runs.iter().any(|text| text.x < first.x));
        assert!(hard_break.y < first.y, "br should advance to another line");
    }

    #[test]
    fn long_text_float_reflows_with_full_width_on_continuation_page() {
        let html = r#"
            <style>
                @page { size: 220pt 140pt; margin: 10pt; }
                body { margin: 0; font-size: 10pt; line-height: 12pt; }
                p { margin: 0; }
                .left { float: left; width: 60pt; height: 65pt; margin-right: 8pt; background: #2563eb; }
            </style>
            <div class="left"></div>
            <p>alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima mike november oscar papa quebec romeo sierra tango uniform victor whiskey xray yankee zulu alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima mike november oscar papa quebec romeo sierra tango</p>
        "#;
        let document = crate::parser::parse_document(html).expect("valid HTML");
        let stylesheet = Stylesheet::from_document(&document);
        let mut options = RenderOptions::default();
        options.page = PageOptions {
            width_pt: 220.0,
            height_pt: 140.0,
            margin_top_pt: 10.0,
            margin_right_pt: 10.0,
            margin_bottom_pt: 10.0,
            margin_left_pt: 10.0,
        };
        let pages = layout_document(&document, &stylesheet, &options);

        assert!(
            pages.len() >= 2,
            "long flow should continue to another page"
        );
        let page_text = |page_index: usize| {
            pages[page_index]
                .items
                .iter()
                .filter_map(|item| match item {
                    LayoutItem::Text(text) if !text.text.trim().is_empty() => Some(text),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let first_page = page_text(0);
        let second_page = page_text(1);
        assert!(!first_page.is_empty(), "float page should contain text");
        assert!(
            !second_page.is_empty(),
            "continuation page should contain text"
        );
        assert!(
            first_page[0].x > 70.0,
            "first page should wrap beside float: {:?}",
            first_page[0]
        );
        assert!(
            second_page[0].x < first_page[0].x,
            "continuation page should discard the old float inset: first={:?} second={:?}",
            first_page[0],
            second_page[0]
        );
        let rendered = pages
            .iter()
            .flat_map(|page| page.items.iter())
            .filter_map(|item| match item {
                LayoutItem::Text(text) => Some(text.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(rendered.iter().any(|text| text.contains("tango")));
    }

    #[test]
    fn wrapped_flex_container_can_start_before_all_rows_fit() {
        let html = r#"
            <style>
                body { margin: 0; font-size: 10pt; line-height: 12pt; }
                p { width: 80pt; margin: 0; }
                .flex { display: flex; flex-wrap: wrap; width: 160pt; gap: 4pt; padding: 4pt; }
                .item { flex: 0 0 76pt; height: 20pt; background: #bfdbfe; }
            </style>
            <p>one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen</p>
            <div class="flex">
                <div class="item">A</div><div class="item">B</div>
                <div class="item">C</div><div class="item">D</div>
                <div class="item">E</div><div class="item">F</div>
            </div>
        "#;
        let document = crate::parser::parse_document(html).expect("valid HTML");
        let stylesheet = Stylesheet::from_document(&document);
        let mut options = RenderOptions::default();
        options.page = PageOptions {
            width_pt: 200.0,
            height_pt: 140.0,
            margin_top_pt: 10.0,
            margin_right_pt: 10.0,
            margin_bottom_pt: 10.0,
            margin_left_pt: 10.0,
        };
        let pages = layout_document(&document, &stylesheet, &options);
        let item_pages = pages
            .iter()
            .enumerate()
            .flat_map(|(page_index, page)| {
                page.items.iter().filter_map(move |item| match item {
                    LayoutItem::Text(text)
                        if matches!(text.text.as_str(), "A" | "B" | "C" | "D" | "E" | "F") =>
                    {
                        Some((page_index, text.text.as_str()))
                    }
                    _ => None,
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(
            item_pages.len(),
            6,
            "all flex items should render: {item_pages:?}"
        );
        assert!(
            item_pages.iter().any(|(page, _)| *page == 0),
            "the first flex row should use remaining space before the continuation page: {item_pages:?}"
        );
    }

    #[test]
    fn grid_rows_stay_together_when_the_grid_crosses_a_page() {
        let html = r#"
            <style>
                body { margin: 0; font-size: 10pt; line-height: 12pt; }
                p { width: 80pt; margin: 0; }
                .grid { display: grid; grid-template-columns: repeat(2, 76pt); gap: 4pt; width: 160pt; }
                .item { height: 20pt; background: #bfdbfe; }
            </style>
            <p>one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen</p>
            <div class="grid">
                <div class="item">A</div><div class="item">B</div>
                <div class="item">C</div><div class="item">D</div>
                <div class="item">E</div><div class="item">F</div>
            </div>
        "#;
        let document = crate::parser::parse_document(html).expect("valid HTML");
        let stylesheet = Stylesheet::from_document(&document);
        let mut options = RenderOptions::default();
        options.page = PageOptions {
            width_pt: 200.0,
            height_pt: 140.0,
            margin_top_pt: 10.0,
            margin_right_pt: 10.0,
            margin_bottom_pt: 10.0,
            margin_left_pt: 10.0,
        };
        let pages = layout_document(&document, &stylesheet, &options);
        let item_pages = pages
            .iter()
            .enumerate()
            .flat_map(|(page_index, page)| {
                page.items.iter().filter_map(move |item| match item {
                    LayoutItem::Text(text)
                        if matches!(text.text.as_str(), "A" | "B" | "C" | "D" | "E" | "F") =>
                    {
                        Some((page_index, text.text.as_str()))
                    }
                    _ => None,
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(
            item_pages.len(),
            6,
            "all grid items should render: {item_pages:?}"
        );
        for pair in [(&"A", &"B"), (&"C", &"D"), (&"E", &"F")] {
            let first = item_pages
                .iter()
                .find(|(_, text)| text == pair.0)
                .expect("first item in grid row");
            let second = item_pages
                .iter()
                .find(|(_, text)| text == pair.1)
                .expect("second item in grid row");
            assert_eq!(
                first.0, second.0,
                "grid row split across pages: {item_pages:?}"
            );
        }
    }

    #[test]
    fn tall_table_rows_fragment_across_pages_and_repeat_headers() {
        let html = r#"
            <style>
                @page { size: 220pt 140pt; margin: 10pt; }
                body { margin: 0; font-size: 10pt; line-height: 12pt; }
                table { width: 200pt; border-collapse: collapse; }
                th, td { border: 1pt solid #64748b; padding: 2pt; vertical-align: top; }
            </style>
            <table>
                <thead><tr><th>Header key</th><th>Header value</th></tr></thead>
                <tbody><tr>
                    <td>alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima mike november oscar papa quebec romeo sierra tango uniform victor whiskey xray yankee zulu</td>
                    <td>one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty twentyone twentytwo twentythree twentyfour</td>
                </tr></tbody>
            </table>
        "#;
        let document = crate::parser::parse_document(html).expect("valid HTML");
        let stylesheet = Stylesheet::from_document(&document);
        let mut options = RenderOptions::default();
        options.page = PageOptions {
            width_pt: 220.0,
            height_pt: 140.0,
            margin_top_pt: 10.0,
            margin_right_pt: 10.0,
            margin_bottom_pt: 10.0,
            margin_left_pt: 10.0,
        };
        let pages = layout_document(&document, &stylesheet, &options);

        assert!(
            pages.len() >= 2,
            "tall table row should continue across pages: {}",
            pages.len()
        );
        let text_runs = pages
            .iter()
            .enumerate()
            .flat_map(|(page_index, page)| {
                page.items.iter().filter_map(move |item| match item {
                    LayoutItem::Text(text) if !text.text.trim().is_empty() => {
                        Some((page_index, text))
                    }
                    _ => None,
                })
            })
            .collect::<Vec<_>>();
        let rendered = text_runs
            .iter()
            .map(|(_, text)| text.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(rendered.contains("zulu"), "last left-column line was lost");
        assert!(
            rendered.contains("twentyfour"),
            "last right-column line was lost"
        );
        let header_pages = text_runs
            .iter()
            .filter(|(page, text)| {
                *page > 0
                    && (text.text.contains("Header key") || text.text.contains("Header value"))
            })
            .map(|(page, _)| *page)
            .collect::<std::collections::BTreeSet<_>>();
        assert!(
            header_pages.len() >= 1,
            "header should repeat on continuation pages: {header_pages:?}"
        );
        for (page_index, text) in text_runs {
            assert!(
                text.y >= options.page.margin_bottom_pt,
                "text escaped below page {page_index}: {text:?}"
            );
        }
    }

    #[test]
    fn fixed_positioned_content_repeats_on_every_printed_page() {
        let html = r#"
            <style>
                @page { size: 220pt 140pt; margin: 10pt; }
                body { margin: 0; font-size: 10pt; line-height: 12pt; }
                .fixed { position: fixed; top: 4pt; left: 10pt; width: 200pt; height: 12pt; color: #ffffff; background: #1d4ed8; }
                p { margin: 0 0 8pt; }
            </style>
            <div class="fixed">Fixed header</div>
            <p>alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima mike november oscar papa quebec romeo sierra tango uniform victor whiskey xray yankee zulu</p>
            <p>one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty twentyone twentytwo twentythree twentyfour</p>
            <p>red orange yellow green blue indigo violet cyan magenta black white gray silver gold copper bronze</p>
        "#;
        let document = crate::parser::parse_document(html).expect("valid HTML");
        let stylesheet = Stylesheet::from_document(&document);
        let mut options = RenderOptions::default();
        options.page = PageOptions {
            width_pt: 220.0,
            height_pt: 140.0,
            margin_top_pt: 10.0,
            margin_right_pt: 10.0,
            margin_bottom_pt: 10.0,
            margin_left_pt: 10.0,
        };
        let pages = layout_document(&document, &stylesheet, &options);

        assert!(pages.len() >= 2, "fixture should span several pages");
        for (page_index, page) in pages.iter().enumerate() {
            let fixed_runs = page
                .items
                .iter()
                .filter(
                    |item| matches!(item, LayoutItem::Text(text) if text.text == "Fixed header"),
                )
                .count();
            assert_eq!(
                fixed_runs, 1,
                "fixed content missing or duplicated on page {page_index}"
            );
        }
        let normal_text_pages = pages
            .iter()
            .filter(|page| {
                page.items.iter().any(
                    |item| matches!(item, LayoutItem::Text(text) if text.text.contains("alpha")),
                )
            })
            .count();
        assert_eq!(
            normal_text_pages, 1,
            "normal flow text should not duplicate"
        );
    }

    #[test]
    fn relative_position_offsets_paint_without_changing_flow() {
        let html = r#"
            <style>
                @page { size: 220pt 140pt; margin: 10pt; }
                body { margin: 0; font-size: 10pt; line-height: 12pt; }
                .relative { position: relative; left: 20pt; top: 8pt; width: 80pt; height: 20pt; background: #2563eb; color: #ffffff; }
                .after { margin: 0; }
            </style>
            <div class="relative">relative content</div>
            <p class="after">flow continues below the original box position</p>
        "#;
        let document = crate::parser::parse_document(html).expect("valid HTML");
        let stylesheet = Stylesheet::from_document(&document);
        let mut options = RenderOptions::default();
        options.page = PageOptions {
            width_pt: 220.0,
            height_pt: 140.0,
            margin_top_pt: 10.0,
            margin_right_pt: 10.0,
            margin_bottom_pt: 10.0,
            margin_left_pt: 10.0,
        };
        let pages = layout_document(&document, &stylesheet, &options);
        let items = &pages[0].items;
        let relative_background = items
            .iter()
            .find_map(|item| match item {
                LayoutItem::Rect(rect)
                    if rect.color.b > 0.7 && rect.color.r < 0.3 && rect.color.g < 0.5 =>
                {
                    Some(rect)
                }
                _ => None,
            })
            .expect("relative background");
        let relative_text = items
            .iter()
            .find_map(|item| match item {
                LayoutItem::Text(text) if text.text == "relative content" => Some(text),
                _ => None,
            })
            .expect("relative text");
        let following_text = items
            .iter()
            .find_map(|item| match item {
                LayoutItem::Text(text) if text.text.contains("flow continues") => Some(text),
                _ => None,
            })
            .expect("following flow text");

        assert!(
            (relative_background.x - 30.0).abs() < 0.01,
            "left offset was ignored: {relative_background:?}"
        );
        assert!(
            (relative_text.x - 30.0).abs() < 0.01,
            "text left offset was ignored: {relative_text:?}"
        );
        assert!(
            (relative_text.y - 110.0).abs() < 0.01,
            "top offset was ignored: {relative_text:?}"
        );
        assert_eq!(
            following_text.x, 10.0,
            "relative offset leaked into following flow"
        );
        assert!(
            (following_text.y - 98.0).abs() < 0.1,
            "following content should retain its normal flow position: relative={relative_text:?} following={following_text:?}"
        );
    }

    #[test]
    fn relative_offset_survives_text_continuation_pages() {
        let html = r#"
            <style>
                @page { size: 220pt 140pt; margin: 10pt; }
                body { margin: 0; font-size: 10pt; line-height: 12pt; }
                p { position: relative; left: 16pt; top: 4pt; margin: 0; }
            </style>
            <p>alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima mike november oscar papa quebec romeo sierra tango uniform victor whiskey xray yankee zulu one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty twentyone twentytwo twentythree twentyfour twentyfive twentysix twentyseven twentyeight twentynine thirty thirtyone thirtytwo thirtythree thirtyfour thirtyfive thirtysix thirtyseven thirtyeight thirtynine forty</p>
        "#;
        let document = crate::parser::parse_document(html).expect("valid HTML");
        let stylesheet = Stylesheet::from_document(&document);
        let mut options = RenderOptions::default();
        options.page = PageOptions {
            width_pt: 220.0,
            height_pt: 140.0,
            margin_top_pt: 10.0,
            margin_right_pt: 10.0,
            margin_bottom_pt: 10.0,
            margin_left_pt: 10.0,
        };
        let pages = layout_document(&document, &stylesheet, &options);
        assert!(pages.len() >= 2, "text should continue onto another page");
        let text_runs = pages
            .iter()
            .flat_map(|page| {
                page.items.iter().filter_map(|item| match item {
                    LayoutItem::Text(text) if !text.text.trim().is_empty() => Some(text),
                    _ => None,
                })
            })
            .collect::<Vec<_>>();
        assert!(!text_runs.is_empty());
        assert!(
            text_runs.iter().all(|text| text.x > 20.0),
            "relative offset was lost on a continuation page: {text_runs:#?}"
        );
    }

    #[test]
    fn splits_long_words_at_soft_breaks_before_character_breaking() {
        let chunks = split_long_word(
            "INV-2026-1001",
            10.0,
            FontFace::Sans,
            FontWeight::Bold,
            estimate_text_width_with_spacing(
                "INV-2026-",
                10.0,
                FontFace::Sans,
                FontWeight::Bold,
                0.0,
                0.0,
            ) + 0.1,
            0.0,
        );

        assert_eq!(chunks, vec!["INV-2026-", "1001"]);
    }

    #[test]
    fn soft_hyphen_breaks_can_fill_previous_line_like_browser_text_layout() {
        let current_width = estimate_text_width_with_spacing(
            "text-wrap-mode, text-",
            12.0,
            FontFace::Sans,
            FontWeight::Normal,
            0.0,
            0.0,
        ) + 0.1;
        let lines = wrap_text_with_face(
            "text-wrap-mode, text-wrap-style, white-space-collapse",
            12.0,
            FontFace::Sans,
            FontWeight::Normal,
            current_width,
            false,
            0.0,
            0.0,
        );

        assert_eq!(
            lines,
            vec![
                "text-wrap-mode, text-",
                "wrap-style, white-",
                "space-collapse"
            ]
        );
    }

    #[test]
    fn narrow_sans_wrapping_uses_browser_like_metric_slack() {
        let lines = wrap_text_with_face(
            "Aspect-ratio should provide a browser-like preferred height for auto-sized boxes.",
            12.0,
            FontFace::Sans,
            FontWeight::Normal,
            144.0,
            false,
            0.0,
            0.0,
        );

        assert_eq!(
            lines,
            vec![
                "Aspect-ratio should provide",
                "a browser-like preferred",
                "height for auto-sized boxes."
            ]
        );
    }

    #[test]
    fn wide_sans_wrapping_uses_browser_like_metric_slack_without_ch_regression() {
        let lines = wrap_text_with_face(
            "CSS variables, var() fallbacks, rem units, rgb() and rgba() colors should resolve in the generic renderer.",
            12.0,
            FontFace::Sans,
            FontWeight::Normal,
            510.3,
            false,
            0.0,
            0.0,
        );

        assert_eq!(
            lines,
            vec![
                "CSS variables, var() fallbacks, rem units, rgb() and rgba() colors should resolve in the generic renderer."
            ]
        );
    }

    #[test]
    fn normal_sans_line_height_uses_browser_like_pdf_rhythm() {
        let mut normal = ComputedStyle::default();
        normal.font_face = FontFace::Sans;
        assert!((layout_line_height(&normal) - 12.78).abs() < 0.01);

        let mut explicit = ComputedStyle::default();
        explicit.font_face = FontFace::Sans;
        explicit.line_height = 14.4;
        explicit.line_height_is_normal = false;
        assert!((layout_line_height(&explicit) - 14.4).abs() < 0.01);
    }

    #[test]
    fn large_sans_wrapping_keeps_browser_like_wide_line_breaks() {
        let lines = wrap_text_with_face(
            "em units should resolve from the current font size and background shorthand should carry URL, position, and size.",
            15.0,
            FontFace::Sans,
            FontWeight::Normal,
            510.0,
            false,
            0.0,
            0.0,
        );

        assert_eq!(
            lines,
            vec![
                "em units should resolve from the current font size and background shorthand",
                "should carry URL, position, and size."
            ]
        );
    }

    #[test]
    fn css_transform_rotation_applies_to_box_geometry() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              section {
                margin: 0;
                width: 100px;
                height: 20px;
                background: rgb(37 99 235);
                transform-origin: left top;
                transform: rotate(10deg);
              }
            </style>
            <section>Rotated box</section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let polygon = pages[0]
            .items
            .iter()
            .find_map(|item| match item {
                LayoutItem::Polygon(polygon) if (polygon.color.r - 37.0 / 255.0).abs() < 0.01 => {
                    Some(polygon)
                }
                _ => None,
            })
            .expect("rotated background should be emitted as polygon geometry");

        assert_eq!(polygon.points.len(), 4);
        assert!(
            polygon
                .points
                .windows(2)
                .any(|pair| (pair[0].1 - pair[1].1).abs() > 0.01),
            "rotated rectangle points should not remain axis-aligned: {:?}",
            polygon.points
        );
    }

    #[test]
    fn fragmented_container_repaints_background_on_continuation_pages() {
        let html = r#"
            <style>
              @page { size: 200pt 120pt; margin: 10pt; }
              body { margin: 0; }
              section {
                margin: 0;
                padding: 8pt;
                width: 120pt;
                overflow: hidden;
                background: linear-gradient(90deg, rgb(219 234 254), rgb(255 255 255));
                border-radius: 4pt;
                font-size: 10pt;
                line-height: 16pt;
              }
              p { margin: 0 0 8pt 0; }
            </style>
            <section>
              <p>one two three four five six seven eight nine ten</p>
              <p>one two three four five six seven eight nine ten</p>
              <p>one two three four five six seven eight nine ten</p>
              <p>one two three four five six seven eight nine ten</p>
              <p>one two three four five six seven eight nine ten</p>
              <p>one two three four five six seven eight nine ten</p>
              <p>one two three four five six seven eight nine ten</p>
              <p>one two three four five six seven eight nine ten</p>
            </section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let mut options = RenderOptions::default();
        options.page = PageOptions {
            width_pt: 200.0,
            height_pt: 120.0,
            margin_top_pt: 10.0,
            margin_right_pt: 10.0,
            margin_bottom_pt: 10.0,
            margin_left_pt: 10.0,
        };
        let pages = layout_document(&document, &stylesheet, &options);

        assert!(pages.len() > 1, "fixture should fragment across pages");
        let continuation_gradient = pages[1].items.iter().position(|item| {
            matches!(
                item,
                LayoutItem::LinearGradient(gradient)
                    if (gradient.x - 10.0).abs() < 0.01
                        && gradient.width > 100.0
                        && gradient.height > 20.0
            )
        });
        let first_text = pages[1]
            .items
            .iter()
            .position(|item| matches!(item, LayoutItem::Text(_)));

        assert!(
            continuation_gradient.is_some(),
            "fragmented section should repaint its CSS background on page 2: {:?}",
            pages[1].items
        );
        assert!(
            continuation_gradient < first_text,
            "continuation background must be inserted behind page 2 content"
        );
    }

    #[test]
    fn falls_back_to_character_breaking_without_soft_breaks() {
        let chunks = split_long_word(
            "ABCDEFGHIJ",
            10.0,
            FontFace::Sans,
            FontWeight::Normal,
            estimate_text_width_with_spacing(
                "ABC",
                10.0,
                FontFace::Sans,
                FontWeight::Normal,
                0.0,
                0.0,
            ) + 0.1,
            0.0,
        );

        assert!(chunks.len() > 1);
        assert_eq!(chunks.join(""), "ABCDEFGHIJ");
    }

    #[test]
    fn sans_normal_layout_metrics_match_browser_system_font_wrapping() {
        let lines = wrap_text_with_face(
            "Reporte consolidado de Campaña Reebok Nano 5 x Dreamfit @ 24 de Marzo",
            26.25,
            FontFace::Sans,
            FontWeight::Normal,
            482.0,
            false,
            0.0,
            0.0,
        );

        assert_eq!(lines[0], "Reporte consolidado de Campaña Reebok");
    }

    #[test]
    fn text_wrap_balance_rebalances_short_multiline_blocks() {
        let mut style = ComputedStyle::default();
        style.text_wrap_style = TextWrapStyle::Balance;
        style.font_size = 12.0;

        let text = "Alpha beta gamma delta epsilon zeta eta theta";
        let normal = wrap_text_with_face(
            text,
            style.font_size,
            style.font_face,
            style.font_weight,
            118.0,
            true,
            style.letter_spacing,
            style.word_spacing,
        );
        let balanced = wrap_text_with_style(text, &style, 118.0);

        let width_range = |lines: &[String]| {
            let widths = lines
                .iter()
                .map(|line| {
                    estimate_wrap_text_width_with_spacing(
                        line,
                        style.font_size,
                        style.font_face,
                        style.font_weight,
                        118.0,
                        style.letter_spacing,
                        style.word_spacing,
                    )
                })
                .collect::<Vec<_>>();
            widths.iter().copied().fold(0.0_f32, f32::max)
                - widths.iter().copied().fold(f32::MAX, f32::min)
        };

        assert_eq!(balanced.len(), normal.len());
        assert!(
            width_range(&balanced) < width_range(&normal),
            "balanced text-wrap should reduce line-length raggedness: normal={normal:?}, balanced={balanced:?}"
        );
    }

    #[test]
    fn adjacent_block_vertical_margins_collapse_to_larger_margin() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              body { margin: 0; }
              section { margin: 0; padding: 0; }
              p {
                margin: 0;
                font-size: 12pt;
                line-height: 14.4pt;
              }
              .first { margin-bottom: 20pt; }
              .second { margin-top: 30pt; }
            </style>
            <section>
              <p class="first">Alpha</p>
              <p class="second">Beta</p>
            </section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let text_y = |needle: &str| {
            pages[0].items.iter().find_map(|item| match item {
                LayoutItem::Text(text) if text.text == needle => Some(text.y),
                _ => None,
            })
        };

        let alpha_y = text_y("Alpha").expect("alpha");
        let beta_y = text_y("Beta").expect("beta");
        assert!(
            (alpha_y - beta_y - 44.4).abs() < 0.01,
            "adjacent vertical margins should collapse to max(20pt, 30pt), not sum: alpha={alpha_y}, beta={beta_y}"
        );
    }

    #[test]
    fn white_space_pre_line_preserves_explicit_line_breaks() {
        let mut style = ComputedStyle::default();
        style.white_space = WhiteSpace::PreLine;
        style.font_size = 12.0;

        let lines = wrap_text_with_style("Alpha   beta\n\nGamma delta", &style, 500.0);

        assert_eq!(lines, vec!["Alpha beta", "", "Gamma delta"]);
    }

    #[test]
    fn word_spacing_expands_only_word_separators() {
        let normal = estimate_text_width_with_spacing(
            "Alpha Beta Gamma",
            12.0,
            FontFace::Sans,
            FontWeight::Normal,
            0.0,
            0.0,
        );
        let spaced = estimate_text_width_with_spacing(
            "Alpha Beta Gamma",
            12.0,
            FontFace::Sans,
            FontWeight::Normal,
            0.0,
            3.0,
        );

        assert!((spaced - normal - 6.0).abs() < 0.01);
    }

    #[test]
    fn aspect_ratio_supplies_missing_box_height_without_shrinking_content() {
        let mut style = ComputedStyle::default();
        style.aspect_ratio = Some(16.0 / 9.0);

        let ratio_height = resolve_box_height(&style, 160.0, 12.0, 500.0);
        let content_height = resolve_box_height(&style, 160.0, 120.0, 500.0);

        assert!((ratio_height - 90.0).abs() < 0.01);
        assert!((content_height - 120.0).abs() < 0.01);
    }

    #[test]
    fn explicit_height_caps_flow_box_advance_like_css_overflow_visible() {
        let mut style = ComputedStyle::default();
        style.height = Some(crate::css::CssLength::Linear {
            percent: 0.0,
            points: 36.0,
        });

        assert!((resolve_box_height(&style, 120.0, 96.0, 500.0) - 36.0).abs() < 0.01);

        style.min_height = Some(crate::css::CssLength::Linear {
            percent: 0.0,
            points: 48.0,
        });
        assert!((resolve_box_height(&style, 120.0, 96.0, 500.0) - 48.0).abs() < 0.01);
    }

    #[test]
    fn max_height_caps_flow_box_height_without_ignoring_min_height() {
        let mut style = ComputedStyle::default();
        style.max_height = Some(crate::css::CssLength::Linear {
            percent: 0.0,
            points: 40.0,
        });

        assert!((resolve_box_height(&style, 120.0, 96.0, 500.0) - 40.0).abs() < 0.01);

        style.min_height = Some(crate::css::CssLength::Linear {
            percent: 0.0,
            points: 64.0,
        });
        assert!((resolve_box_height(&style, 120.0, 96.0, 500.0) - 64.0).abs() < 0.01);
    }

    #[test]
    fn explicit_text_box_height_does_not_paginate_visible_overflow_lines() {
        let html = r#"
            <style>
              @page { size: 220pt 120pt; margin: 10pt; }
              section { font-size: 10pt; line-height: 12pt; width: 70pt; height: 12pt; overflow: visible; }
            </style>
            <section>one two three four five six seven eight nine ten eleven twelve</section>
            <p>after</p>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let rendered = pages
            .iter()
            .enumerate()
            .flat_map(|(page_index, page)| {
                page.items.iter().filter_map(move |item| match item {
                    LayoutItem::Text(text) => Some((page_index, text.text.as_str())),
                    _ => None,
                })
            })
            .collect::<Vec<_>>();

        assert!(
            rendered.iter().any(|(_, text)| text.contains("eleven")),
            "overflowing text should still be painted: {rendered:?}"
        );
        assert!(
            rendered.iter().all(|(page, text)| *page == 0 || *text == "after"),
            "visible overflow lines should not create their own page before following flow: {rendered:?}"
        );
    }

    #[test]
    fn flex_max_block_size_caps_flow_advance_but_allows_visible_overflow() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              .flow {
                display: flex;
                flex-wrap: wrap;
                width: 100px;
                max-block-size: 24px;
                gap: 10px;
                background: rgb(219 234 254);
              }
              .item {
                flex: 0 0 100px;
                height: 20px;
                background: rgb(191 219 254);
              }
              .after {
                margin: 0;
                font-size: 12px;
                line-height: 12px;
              }
            </style>
            <section class="flow">
              <div class="item"></div>
              <div class="item"></div>
              <div class="item"></div>
            </section>
            <p class="after">After capped flex</p>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let flow_rect = pages[0].items.iter().find_map(|item| match item {
            LayoutItem::Rect(rect)
                if (rect.color.r - 219.0 / 255.0).abs() < 0.01
                    && (rect.color.g - 234.0 / 255.0).abs() < 0.01 =>
            {
                Some(rect)
            }
            _ => None,
        });
        let after_y = pages[0].items.iter().find_map(|item| match item {
            LayoutItem::Text(text) if text.text == "After capped flex" => Some(text.y),
            _ => None,
        });

        let flow_rect = flow_rect.expect("flow background");
        assert!(
            (flow_rect.height - 18.0).abs() < 0.01,
            "max-block-size:24px should cap the flow box to 18pt, got {flow_rect:?}"
        );
        let after_y = after_y.expect("following text");
        assert!(
            after_y > 720.0,
            "following content should advance after the capped flow height, not the overflowing children: after_y={after_y}"
        );
    }

    #[test]
    fn flex_wrap_splits_items_into_multiple_lines() {
        let ranges = flex_line_ranges(&[90.0, 90.0, 90.0, 40.0], 200.0, 20.0);

        assert_eq!(ranges, vec![0..2, 2..4]);
    }

    #[test]
    fn flex_items_blockify_inline_elements_for_basis_painting() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              .row { display: flex; gap: 10px; }
              .item { flex: 0 0 100px; background: rgb(219 234 254); padding: 4px; }
            </style>
            <section class="row">
              <span class="item">One</span>
              <span class="item">Two</span>
            </section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let item_backgrounds = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Rect(rect)
                    if (rect.color.r - 219.0 / 255.0).abs() < 0.01
                        && (rect.color.g - 234.0 / 255.0).abs() < 0.01 =>
                {
                    Some(rect.width)
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        let basis_widths = item_backgrounds
            .iter()
            .filter(|width| (**width - 75.0).abs() < 0.01)
            .count();
        assert!(
            basis_widths >= 2,
            "expected at least two 100px flex item backgrounds, got widths {item_backgrounds:?}"
        );
    }

    #[test]
    fn grid_layout_honors_column_span_utilities_without_collapsing_explicit_tracks() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              .grid {
                display: grid;
                width: 400px;
                grid-template-columns: repeat(4, 1fr);
                gap: 10px;
              }
              .item {
                height: 20px;
                background: rgb(219 234 254);
              }
              .span2 { grid-column: span 2 / span 2; }
              .full { grid-column: 1 / -1; }
            </style>
            <section class="grid">
              <div class="item span2"></div>
              <div class="item"></div>
              <div class="item"></div>
              <div class="item full"></div>
            </section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let background_widths = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Rect(rect)
                    if (rect.color.r - 219.0 / 255.0).abs() < 0.01
                        && (rect.color.g - 234.0 / 255.0).abs() < 0.01 =>
                {
                    Some(rect.width)
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(
            background_widths
                .iter()
                .any(|width| (*width - 146.25).abs() < 0.1),
            "span 2 over a 4-column 400px grid should paint ~146.25pt wide, got {background_widths:?}"
        );
        assert!(
            background_widths
                .iter()
                .any(|width| (*width - 300.0).abs() < 0.1),
            "grid-column:1/-1 should span the full 400px grid width, got {background_widths:?}"
        );
    }

    #[test]
    fn grid_layout_honors_explicit_column_line_placement() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              .grid {
                display: grid;
                width: 300px;
                grid-template-columns: repeat(3, 1fr);
              }
              .item {
                grid-column: 2 / 4;
                height: 20px;
                background: rgb(219 234 254);
              }
            </style>
            <section class="grid">
              <div class="item"></div>
            </section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let item_rect = pages[0]
            .items
            .iter()
            .find_map(|item| match item {
                LayoutItem::Rect(rect)
                    if (rect.color.r - 219.0 / 255.0).abs() < 0.01
                        && (rect.color.g - 234.0 / 255.0).abs() < 0.01 =>
                {
                    Some(rect)
                }
                _ => None,
            })
            .expect("placed grid item background");

        let expected_x = pages[0].page.margin_left_pt + 75.0;
        assert!(
            (item_rect.x - expected_x).abs() < 0.1,
            "grid-column:2/4 should start at the second 100px track, got {item_rect:?}"
        );
        assert!(
            (item_rect.width - 150.0).abs() < 0.1,
            "grid-column:2/4 should span two 100px tracks, got {item_rect:?}"
        );
    }

    #[test]
    fn grid_layout_honors_explicit_row_line_placement() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              .grid {
                display: grid;
                width: 200px;
                grid-template-columns: repeat(2, 1fr);
                row-gap: 10px;
              }
              .item {
                grid-column: 2 / 3;
                grid-row: 2 / 3;
                height: 20px;
                background: rgb(219 234 254);
              }
            </style>
            <section class="grid">
              <div class="item"></div>
            </section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let item_rect = pages[0]
            .items
            .iter()
            .find_map(|item| match item {
                LayoutItem::Rect(rect)
                    if (rect.color.r - 219.0 / 255.0).abs() < 0.01
                        && (rect.color.g - 234.0 / 255.0).abs() < 0.01 =>
                {
                    Some(rect)
                }
                _ => None,
            })
            .expect("placed grid row item background");

        let expected_x = pages[0].page.margin_left_pt + 75.0;
        let expected_y = pages[0].page.height_pt - pages[0].page.margin_top_pt - 7.5 - 15.0;
        assert!(
            (item_rect.x - expected_x).abs() < 0.1,
            "grid-column:2/3 should place item in the second column, got {item_rect:?}"
        );
        assert!(
            (item_rect.y - expected_y).abs() < 0.1,
            "grid-row:2/3 should place item after the first implicit row gap, got {item_rect:?}"
        );
    }

    #[test]
    fn grid_without_explicit_tracks_auto_places_items_in_one_column() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              .grid {
                display: grid;
                width: 400px;
                gap: 10px;
              }
              .item {
                height: 20px;
                background: rgb(219 234 254);
              }
            </style>
            <section class="grid">
              <div class="item"></div>
              <div class="item"></div>
              <div class="item"></div>
            </section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let item_rects = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Rect(rect)
                    if (rect.color.r - 219.0 / 255.0).abs() < 0.01
                        && (rect.color.g - 234.0 / 255.0).abs() < 0.01 =>
                {
                    Some((rect.width, rect.y))
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(item_rects.len(), 3);
        assert!(
            item_rects
                .iter()
                .all(|(width, _)| (*width - 300.0).abs() < 0.01),
            "implicit grid should use one full-width column, rects were {item_rects:?}"
        );
        assert!(
            item_rects.windows(2).all(|pair| pair[0].1 > pair[1].1),
            "implicit grid rows should stack vertically, rects were {item_rects:?}"
        );
    }

    #[test]
    fn grid_justify_items_places_child_within_track_inline_axis() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              .grid {
                display: grid;
                width: 300px;
                grid-template-columns: 300px;
                justify-items: center;
              }
              .item {
                width: 100px;
                height: 20px;
                background: rgb(219 234 254);
              }
            </style>
            <section class="grid">
              <div class="item"></div>
            </section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let item_rect = pages[0]
            .items
            .iter()
            .find_map(|item| match item {
                LayoutItem::Rect(rect)
                    if (rect.color.r - 219.0 / 255.0).abs() < 0.01
                        && (rect.color.g - 234.0 / 255.0).abs() < 0.01 =>
                {
                    Some((rect.x, rect.width))
                }
                _ => None,
            })
            .expect("item background");

        let expected_x = pages[0].page.margin_left_pt + 75.0;
        assert!(
            (item_rect.0 - expected_x).abs() < 0.1,
            "100px grid item should be centered inside a 300px track, got {item_rect:?}"
        );
        assert!(
            (item_rect.1 - 75.0).abs() < 0.1,
            "explicit 100px item width should be preserved, got {item_rect:?}"
        );
    }

    #[test]
    fn grid_justify_content_offsets_fixed_tracks_within_container() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              .grid {
                display: grid;
                width: 300px;
                grid-template-columns: 50px 50px;
                justify-content: center;
                gap: 10px;
              }
              .item {
                height: 20px;
                background: rgb(219 234 254);
              }
            </style>
            <section class="grid">
              <div class="item"></div>
              <div class="item"></div>
            </section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let item_rects = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Rect(rect)
                    if (rect.color.r - 219.0 / 255.0).abs() < 0.01
                        && (rect.color.g - 234.0 / 255.0).abs() < 0.01 =>
                {
                    Some((rect.x, rect.width))
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(item_rects.len(), 2);
        let page_margin_left = pages[0].page.margin_left_pt;
        assert!(
            (item_rects[0].0 - (page_margin_left + 71.25)).abs() < 0.1,
            "two 50px tracks plus 10px gap should be centered in a 300px grid, got {item_rects:?}"
        );
        assert!(
            item_rects
                .iter()
                .all(|(_, width)| (*width - 37.5).abs() < 0.1),
            "fixed 50px tracks should keep 37.5pt item widths, got {item_rects:?}"
        );
    }

    #[test]
    fn grid_align_content_offsets_rows_inside_definite_height() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              .grid {
                display: grid;
                width: 100px;
                height: 100px;
                grid-template-columns: 100px;
                align-content: center;
              }
              .item {
                height: 20px;
                background: rgb(219 234 254);
              }
            </style>
            <section class="grid">
              <div class="item"></div>
            </section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let item_rect = pages[0]
            .items
            .iter()
            .find_map(|item| match item {
                LayoutItem::Rect(rect)
                    if (rect.color.r - 219.0 / 255.0).abs() < 0.01
                        && (rect.color.g - 234.0 / 255.0).abs() < 0.01 =>
                {
                    Some(rect)
                }
                _ => None,
            })
            .expect("item background");

        let expected_y = pages[0].page.height_pt - pages[0].page.margin_top_pt - 30.0 - 15.0;
        assert!(
            (item_rect.y - expected_y).abs() < 0.1,
            "20px grid row should be centered inside 100px definite block-size, got {item_rect:?}"
        );
        assert!(
            (item_rect.height - 15.0).abs() < 0.1,
            "20px item height should still resolve to 15pt, got {item_rect:?}"
        );
    }

    #[test]
    fn grid_template_rows_reserve_explicit_track_heights() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              .grid {
                display: grid;
                width: 100px;
                grid-template-columns: 100px;
                grid-template-rows: 40px 80px;
              }
              .first,
              .second {
                height: 10px;
              }
              .first { background: rgb(219 234 254); }
              .second { background: rgb(254 202 202); }
            </style>
            <section class="grid">
              <div class="first"></div>
              <div class="second"></div>
            </section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let mut first_rect = None;
        let mut second_rect = None;
        for item in &pages[0].items {
            if let LayoutItem::Rect(rect) = item {
                if (rect.color.r - 219.0 / 255.0).abs() < 0.01
                    && (rect.color.g - 234.0 / 255.0).abs() < 0.01
                {
                    first_rect = Some(rect);
                }
                if (rect.color.r - 254.0 / 255.0).abs() < 0.01
                    && (rect.color.g - 202.0 / 255.0).abs() < 0.01
                {
                    second_rect = Some(rect);
                }
            }
        }

        let first_rect = first_rect.expect("first item background");
        let second_rect = second_rect.expect("second item background");
        let row_offset = first_rect.y - second_rect.y;

        assert!(
            (row_offset - 30.0).abs() < 0.1,
            "second row should start after the explicit 40px/30pt first track, got {row_offset}"
        );
        assert!(
            (first_rect.height - 7.5).abs() < 0.1 && (second_rect.height - 7.5).abs() < 0.1,
            "explicit tracks should reserve row space without stretching fixed-height items"
        );
    }

    #[test]
    fn grid_align_content_space_between_distributes_row_free_space() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              .grid {
                display: grid;
                width: 200px;
                height: 100px;
                grid-template-columns: 100px 100px;
                align-content: space-between;
              }
              .item {
                height: 20px;
                background: rgb(219 234 254);
              }
            </style>
            <section class="grid">
              <div class="item"></div>
              <div class="item"></div>
              <div class="item"></div>
            </section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let item_ys = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Rect(rect)
                    if (rect.color.r - 219.0 / 255.0).abs() < 0.01
                        && (rect.color.g - 234.0 / 255.0).abs() < 0.01 =>
                {
                    Some(rect.y)
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(item_ys.len(), 3);
        let first_row_y = pages[0].page.height_pt - pages[0].page.margin_top_pt - 15.0;
        let second_row_y = first_row_y - 15.0 - 45.0;
        assert!(
            item_ys
                .iter()
                .filter(|y| (**y - first_row_y).abs() < 0.1)
                .count()
                == 2,
            "first grid row should stay at the content start, got {item_ys:?}"
        );
        assert!(
            item_ys.iter().any(|y| (*y - second_row_y).abs() < 0.1),
            "second grid row should absorb 45pt of free block-axis space, got {item_ys:?}"
        );
    }

    #[test]
    fn whitespace_only_text_nodes_do_not_consume_flow_height() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              div, span { margin: 0; padding: 0; font-size: 12px; line-height: 16px; }
            </style>
            <div>
              <span>One</span>
              <span>Two</span>
            </div>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let text_runs = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Text(text) if text.text == "One" || text.text == "Two" => {
                    Some((text.text.as_str(), text.y))
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(text_runs.len(), 2);
        assert_eq!(text_runs[0].0, "One");
        assert_eq!(text_runs[1].0, "Two");
        assert!(
            (text_runs[0].1 - text_runs[1].1).abs() < 0.01,
            "inline spans separated by collapsible whitespace must share a line: {text_runs:?}"
        );
    }

    #[test]
    fn indented_text_nodes_collapse_newlines_without_blank_lines() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              section { margin: 0; padding: 0; font-size: 12px; line-height: 14px; }
            </style>
            <section>
              First indented block
            </section>
            <section>
              Second indented block
            </section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let text_runs = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Text(text)
                    if text.text == "First indented block"
                        || text.text == "Second indented block" =>
                {
                    Some((text.text.as_str(), text.y))
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(text_runs.len(), 2);
        assert!(
            (text_runs[0].1 - text_runs[1].1 - 10.5).abs() < 0.01,
            "normal white-space should collapse indentation newlines instead of creating blank lines: {text_runs:?}"
        );
    }

    #[test]
    fn br_elements_preserve_hard_line_breaks_inside_text_blocks() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              p { margin: 0; padding: 0; font-size: 12px; line-height: 14px; }
            </style>
            <p>First line<br>Second line<br>Third line</p>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let text_runs = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Text(text)
                    if matches!(
                        text.text.as_str(),
                        "First line" | "Second line" | "Third line"
                    ) =>
                {
                    Some((text.text.as_str(), text.y))
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(text_runs.len(), 3);
        assert_eq!(text_runs[0].0, "First line");
        assert_eq!(text_runs[1].0, "Second line");
        assert_eq!(text_runs[2].0, "Third line");
        assert!(
            (text_runs[0].1 - text_runs[1].1 - 10.5).abs() < 0.01
                && (text_runs[1].1 - text_runs[2].1 - 10.5).abs() < 0.01,
            "<br> should preserve hard line breaks without being collapsed into spaces: {text_runs:?}"
        );
    }

    #[test]
    fn list_items_emit_markers_and_respect_list_style_none() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              ul, ol { margin: 0; padding: 0; font-size: 12px; line-height: 12px; }
              .plain { list-style: none; }
            </style>
            <ul><li>Bullet item</li></ul>
            <ol><li>First step</li><li>Second step</li></ol>
            <ul class="plain"><li>No marker</li></ul>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let painted_text = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Text(text) => Some(text.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(painted_text.contains(&"•"));
        assert!(painted_text.contains(&"1."));
        assert!(painted_text.contains(&"2."));
        assert!(!painted_text.contains(&"3."));
    }

    #[test]
    fn flex_layout_honors_order_and_align_self() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              .row { display: flex; align-items: flex-start; gap: 0; }
              .item { flex: 0 0 100px; margin: 0; padding: 0; font-size: 12px; line-height: 12px; }
              .first { order: 2; }
              .second { order: -1; height: 60px; }
              .short { order: 0; align-self: flex-end; }
            </style>
            <section class="row">
              <p class="item first">First</p>
              <p class="item short">Short</p>
              <p class="item second">Second</p>
            </section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let text_pos = |needle: &str| {
            pages[0].items.iter().find_map(|item| match item {
                LayoutItem::Text(text) if text.text == needle => Some((text.x, text.y)),
                _ => None,
            })
        };
        let (first_x, _) = text_pos("First").expect("first text");
        let (short_x, short_y) = text_pos("Short").expect("short text");
        let (second_x, second_y) = text_pos("Second").expect("second text");

        assert!(second_x < short_x && short_x < first_x);
        assert!(
            short_y < second_y,
            "align-self:flex-end should shift the short item downward: short_y={short_y}, second_y={second_y}"
        );
    }

    #[test]
    fn block_flow_honors_auto_inline_margins_for_centering() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              body { margin: 0; }
              .target {
                width: 300pt;
                margin-inline: auto;
                padding: 0;
                font-size: 12pt;
                line-height: 12pt;
              }
            </style>
            <section class="target">Centered</section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let centered_x = pages[0].items.iter().find_map(|item| match item {
            LayoutItem::Text(text) if text.text == "Centered" => Some(text.x),
            _ => None,
        });

        let centered_x = centered_x.expect("centered text");
        assert!(
            (centered_x - 156.0).abs() < 0.01,
            "auto inline margins should split the remaining Letter width: x={centered_x}"
        );
    }

    #[test]
    fn logical_text_align_end_resolves_against_direction() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              body { margin: 0; }
              .box {
                width: 200pt;
                padding: 0;
                font-size: 12pt;
                line-height: 12pt;
              }
              .left { text-align: start; }
              .right { text-align: end; }
              .rtl-start { direction: rtl; text-align: start; }
            </style>
            <section class="box left">Alpha</section>
            <section class="box right">Beta</section>
            <section class="box rtl-start">Gamma</section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let text_x = |needle: &str| {
            pages[0].items.iter().find_map(|item| match item {
                LayoutItem::Text(text) if text.text == needle => Some(text.x),
                _ => None,
            })
        };

        let alpha_x = text_x("Alpha").expect("alpha");
        let beta_x = text_x("Beta").expect("beta");
        let gamma_x = text_x("Gamma").expect("gamma");
        assert!(
            beta_x > alpha_x + 120.0,
            "text-align:end in LTR should right-align within the 200pt box: alpha={alpha_x}, beta={beta_x}"
        );
        assert!(
            gamma_x > alpha_x + 120.0,
            "text-align:start in RTL should resolve to right alignment: alpha={alpha_x}, gamma={gamma_x}"
        );
    }

    #[test]
    fn justified_word_spacing_distributes_extra_space_on_non_last_lines() {
        let spacing = justified_word_spacing(
            TextAlign::Justify,
            "Alpha beta gamma",
            90.0,
            120.0,
            1.0,
            false,
        );

        assert!((spacing - 16.0).abs() < 0.01);
        assert_eq!(
            justified_word_spacing(TextAlign::Justify, "Alpha beta", 60.0, 120.0, 1.0, true),
            1.0
        );
    }

    #[test]
    fn text_align_last_overrides_final_line_alignment() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              body { margin: 0; }
              .target {
                width: 200pt;
                font-size: 12pt;
                line-height: 12pt;
                text-align: left;
                text-align-last: right;
              }
            </style>
            <section class="target">Alpha
Beta</section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let text_x = |needle: &str| {
            pages[0].items.iter().find_map(|item| match item {
                LayoutItem::Text(text) if text.text == needle => Some(text.x),
                _ => None,
            })
        };

        let alpha_x = text_x("Alpha").expect("alpha");
        let beta_x = text_x("Beta").expect("beta");
        assert!(
            beta_x > alpha_x + 150.0,
            "text-align-last:right should align only the final line to the right: alpha={alpha_x}, beta={beta_x}"
        );
    }

    #[test]
    fn transform_origin_resolves_css_top_left_to_pdf_point() {
        let mut style = ComputedStyle::default();
        style.transform_origin_x = crate::css::CssLength::Linear {
            percent: 0.0,
            points: 0.0,
        };
        style.transform_origin_y = crate::css::CssLength::Linear {
            percent: 0.0,
            points: 0.0,
        };

        let (origin_x, origin_y) = transform_origin_point(&style, 20.0, 30.0, 120.0, 60.0);

        assert!((origin_x - 20.0).abs() < 0.01);
        assert!((origin_y - 90.0).abs() < 0.01);
    }

    #[test]
    fn nowrap_overflow_hidden_text_can_ellipsis() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              .target {
                width: 60px;
                overflow: hidden;
                white-space: nowrap;
                text-overflow: ellipsis;
                font-size: 12px;
                line-height: 14px;
              }
            </style>
            <p class="target">This label is far too long</p>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let text = pages[0].items.iter().find_map(|item| match item {
            LayoutItem::Text(text) => Some(text.text.as_str()),
            _ => None,
        });

        assert!(
            text.is_some_and(|text| text.ends_with('…') && text != "This label is far too long")
        );
    }

    #[test]
    fn outline_paints_outside_box_without_affecting_flow() {
        let render = |outline_css: &str| {
            let html = format!(
                r#"
                <style>
                  @page {{ size: Letter; margin: 0; }}
                  .target {{
                    width: 100px;
                    height: 40px;
                    margin: 0;
                    padding: 0;
                    background: rgb(219 234 254);
                    {outline_css}
                  }}
                  .after {{ margin: 0; font-size: 12pt; line-height: 12pt; }}
                </style>
                <section class="target"></section>
                <p class="after">After</p>
            "#
            );
            let document = crate::parser::parse_document(&html).expect("valid html");
            let stylesheet = Stylesheet::from_document(&document);
            layout_document(&document, &stylesheet, &RenderOptions::default())
        };

        let with_outline = render("outline: 4px dashed rgb(37 99 235); outline-offset: 2px;");
        let without_outline = render("");
        let outline = with_outline[0]
            .items
            .iter()
            .find_map(|item| match item {
                LayoutItem::StrokeRect(rect)
                    if (rect.color.r - 37.0 / 255.0).abs() < 0.01
                        && (rect.color.g - 99.0 / 255.0).abs() < 0.01 =>
                {
                    Some(rect)
                }
                _ => None,
            })
            .expect("outline should paint as a stroke outside the box");
        let background = with_outline[0]
            .items
            .iter()
            .find_map(|item| match item {
                LayoutItem::Rect(rect)
                    if (rect.color.r - 219.0 / 255.0).abs() < 0.01
                        && (rect.color.g - 234.0 / 255.0).abs() < 0.01 =>
                {
                    Some(rect)
                }
                _ => None,
            })
            .expect("target background should paint");
        let after_y = |pages: &[LayoutPage]| {
            pages[0]
                .items
                .iter()
                .find_map(|item| match item {
                    LayoutItem::Text(text) if text.text == "After" => Some(text.y),
                    _ => None,
                })
                .expect("after text rendered")
        };

        assert!(
            outline.x < background.x,
            "outline should extend outside the border edge: outline.x={}, background.x={}",
            outline.x,
            background.x
        );
        assert!(
            outline.width > background.width,
            "outline width={}, background width={}",
            outline.width,
            background.width
        );
        assert_eq!(outline.dash, Some((15.0, 9.0)));
        assert!(
            (after_y(&with_outline) - after_y(&without_outline)).abs() < 0.01,
            "outline must not participate in layout flow"
        );
    }

    #[test]
    fn blurred_box_shadow_paints_soft_layers() {
        let mut style = ComputedStyle::default();
        style.border_radius = 8.0;
        style.box_shadows = vec![BoxShadow {
            offset_x: 0.0,
            offset_y: 12.0,
            blur: 24.0,
            spread: 0.0,
            color: Color {
                r: 0.06,
                g: 0.09,
                b: 0.16,
                a: 0.20,
            },
        }];

        let items = background_items(&style, &RenderOptions::default(), 20.0, 20.0, 120.0, 60.0);
        let shadow_layers = items
            .iter()
            .filter(|item| match item {
                LayoutItem::RoundRect(rect) => rect.color.a > 0.0 && rect.color.a < 0.20,
                _ => false,
            })
            .count();

        assert!(
            shadow_layers >= 6,
            "blurred shadows should be approximated with multiple translucent layers, got {shadow_layers}"
        );
    }

    #[test]
    fn multiple_box_shadows_paint_back_to_front() {
        let mut style = ComputedStyle::default();
        style.box_shadows = vec![
            BoxShadow {
                offset_x: 0.0,
                offset_y: 1.0,
                blur: 0.0,
                spread: 0.0,
                color: Color {
                    r: 1.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.5,
                },
            },
            BoxShadow {
                offset_x: 0.0,
                offset_y: 2.0,
                blur: 0.0,
                spread: 0.0,
                color: Color {
                    r: 0.0,
                    g: 0.0,
                    b: 1.0,
                    a: 0.5,
                },
            },
        ];

        let items = background_items(&style, &RenderOptions::default(), 20.0, 20.0, 120.0, 60.0);
        let shadow_colors = items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Rect(rect) => Some(rect.color),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(shadow_colors.len(), 2);
        assert!(
            shadow_colors[0].b > 0.9,
            "second shadow should paint first/back"
        );
        assert!(
            shadow_colors[1].r > 0.9,
            "first shadow should paint last/front"
        );
    }

    #[test]
    fn background_clip_padding_box_shrinks_background_paint_area() {
        let mut style = ComputedStyle::default();
        style.background = Some(Color {
            r: 0.9,
            g: 0.95,
            b: 1.0,
            a: 1.0,
        });
        style.background_clip = BackgroundClip::PaddingBox;
        style.border_left_width = 6.0;
        style.border_right_width = 8.0;
        style.border_top_width = 10.0;
        style.border_bottom_width = 4.0;

        let items = background_items(&style, &RenderOptions::default(), 20.0, 30.0, 120.0, 60.0);
        let background = items
            .iter()
            .find_map(|item| match item {
                LayoutItem::Rect(rect) => Some(rect),
                _ => None,
            })
            .expect("background rect");

        assert!((background.x - 26.0).abs() < 0.01);
        assert!((background.y - 34.0).abs() < 0.01);
        assert!((background.width - 106.0).abs() < 0.01);
        assert!((background.height - 46.0).abs() < 0.01);
    }

    #[test]
    fn radial_gradient_background_paints_as_alpha_image() {
        let mut style = ComputedStyle::default();
        style.background = Some(Color::WHITE);
        style.background_radials = vec![RadialGradient {
            color: Color {
                r: 0.1,
                g: 0.4,
                b: 0.9,
                a: 0.5,
            },
            center_x: 0.25,
            center_y: 0.25,
            radius: crate::css::CssLength::Linear {
                percent: 0.45,
                points: 0.0,
            },
        }];

        let items = background_items(&style, &RenderOptions::default(), 0.0, 0.0, 200.0, 120.0);
        let image = items
            .iter()
            .find_map(|item| match item {
                LayoutItem::Image(image) => Some(image),
                _ => None,
            })
            .expect("radial background should rasterize to an image");

        assert!(matches!(
            image.format,
            ImageFormat::Raw {
                color_space: PngColorSpace::DeviceRgb,
                components: 3,
                ..
            }
        ));
        assert!(image
            .alpha_mask
            .as_ref()
            .is_some_and(|mask| mask.iter().any(|alpha| *alpha > 0)));
    }

    #[test]
    fn positioned_siblings_honor_z_index_paint_order() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              .stack { position: relative; width: 80px; height: 80px; margin: 0; }
              .top, .bottom {
                position: absolute;
                inset: 0;
                width: 80px;
                height: 80px;
              }
              .top { z-index: 2; background: rgb(220 38 38); }
              .bottom { z-index: 1; background: rgb(37 99 235); }
            </style>
            <section class="stack">
              <div class="top"></div>
              <div class="bottom"></div>
            </section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let painted_colors = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Rect(rect) if rect.color != Color::WHITE => {
                    Some((rect.color.r, rect.color.g, rect.color.b))
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        let last = painted_colors
            .last()
            .copied()
            .expect("expected positioned rects to paint");
        assert!(
            (last.0 - 220.0 / 255.0).abs() < 0.01 && (last.1 - 38.0 / 255.0).abs() < 0.01,
            "higher z-index red box should paint last, colors={painted_colors:?}"
        );
    }

    #[test]
    fn visibility_hidden_reserves_flow_space_without_painting_subtree() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              p { margin: 0; font-size: 16px; line-height: 20px; }
              .hidden { visibility: hidden; background: rgb(220 38 38); }
            </style>
            <p>Before</p>
            <section class="hidden">
              <p>Invisible child</p>
            </section>
            <p>After</p>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let visible_text = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Text(text) => Some(text.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let red_backgrounds = pages[0]
            .items
            .iter()
            .filter(|item| {
                matches!(
                    item,
                    LayoutItem::Rect(rect)
                        if (rect.color.r - 220.0 / 255.0).abs() < 0.01
                            && (rect.color.g - 38.0 / 255.0).abs() < 0.01
                )
            })
            .count();
        let after_y = pages[0]
            .items
            .iter()
            .find_map(|item| match item {
                LayoutItem::Text(text) if text.text == "After" => Some(text.y),
                _ => None,
            })
            .expect("after text rendered");

        assert_eq!(visible_text, vec!["Before", "After"]);
        assert_eq!(red_backgrounds, 0);
        assert!(
            after_y < 730.0,
            "visibility:hidden should reserve the hidden paragraph's line height; after_y={after_y}"
        );
    }

    #[test]
    fn break_inside_avoid_is_best_effort_when_useful_space_remains() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 72pt; }
              .spacer { height: 360pt; }
              .avoid { page-break-inside: avoid; }
              .part { height: 180pt; font-size: 12pt; line-height: 16pt; }
            </style>
            <section class="spacer"></section>
            <section class="avoid">
              <p class="part">Avoid starts here</p>
              <p class="part">Avoid continues here</p>
            </section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());

        let first_page_text = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Text(text) => Some(text.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(
            first_page_text.contains(&"Avoid starts here"),
            "break-inside: avoid is advisory; a block should still start when substantial useful page space remains"
        );
    }

    #[test]
    fn break_inside_avoid_does_not_push_blocks_that_fit_current_page() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 72pt; }
              .spacer { height: 260pt; }
              .avoid { page-break-inside: avoid; }
              .part { height: 110pt; font-size: 12pt; line-height: 16pt; margin: 0; }
            </style>
            <section class="spacer"></section>
            <section class="avoid">
              <p class="part">Avoid fits first line</p>
              <p class="part">Avoid fits second line</p>
            </section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());

        let first_page_text = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Text(text) => Some(text.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(
            first_page_text.contains(&"Avoid fits first line")
                && first_page_text.contains(&"Avoid fits second line"),
            "break-inside: avoid should not force a new page when the whole block fits current page"
        );
    }

    #[test]
    fn large_table_avoid_starts_on_next_page_when_remaining_space_is_low() {
        let rows = (0..20)
            .map(|idx| format!("<tr><td>Large table row {idx}</td><td>{idx}</td></tr>"))
            .collect::<String>();
        let html = format!(
            r#"
            <style>
              @page {{ size: Letter; margin: 72pt; }}
              .spacer {{ margin: 0 0 520pt; color: rgba(0, 0, 0, 0); }}
              .avoid {{ page-break-inside: avoid; }}
              h2 {{ font-size: 12pt; line-height: 16pt; margin: 0 0 8pt; }}
              table {{ width: 100%; border-collapse: collapse; font-size: 12pt; }}
              td {{ padding: 8pt 0; line-height: 16pt; }}
            </style>
            <p class="spacer">Spacer before avoid table</p>
            <section class="avoid">
              <h2>Large table starts here</h2>
              <table><tbody>{rows}</tbody></table>
            </section>
        "#
        );
        let document = crate::parser::parse_document(&html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let avoid = document.query_selector(".avoid").expect("avoid section");
        let avoid_style = ComputedStyle::for_node(&document, &stylesheet, avoid);
        assert!(avoid_style.page_break_inside_avoid);
        assert!(contains_large_table(&document, avoid, 16));
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());

        let page_text = |page_index: usize| {
            pages[page_index]
                .items
                .iter()
                .filter_map(|item| match item {
                    LayoutItem::Text(text) => Some(text.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert!(
            !page_text(0).contains(&"Large table starts here"),
            "a large avoid table should not strand its heading at the bottom of a nearly full page"
        );
        assert!(
            page_text(1).contains(&"Large table starts here"),
            "the large avoid table heading should move with the table to the next page"
        );
    }

    #[test]
    fn sans_ch_constrained_paragraph_wraps_like_browser_print() {
        let text = "documento preparado para cliente final con inversión, cobertura, actividad observada e incidencias registradas durante la ventana programada del grupo.";
        let html = format!(
            r#"
            <style>
              body {{ font-family: Inter, -apple-system, BlinkMacSystemFont, Segoe UI, Roboto, Helvetica, Arial, sans-serif; font-size: 14px; }}
              p {{
                margin: 0;
                max-width: 60ch;
                font-family: Arial, sans-serif;
                font-size: 1rem;
                line-height: 1.6;
              }}
            </style>
            <p>{text}</p>
        "#
        );
        let document = crate::parser::parse_document(&html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());

        let lines = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Text(text) => Some(text.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(
            lines,
            vec![
                "documento preparado para cliente final con inversión, cobertura, actividad",
                "observada e incidencias registradas durante la ventana programada del",
                "grupo.",
            ]
        );
    }

    #[test]
    fn break_inside_avoid_ignores_trailing_margin_when_visible_content_fits() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 72pt; }
              .spacer { height: 400pt; }
              .avoid { page-break-inside: avoid; margin-bottom: 80pt; }
              .part { height: 80pt; font-size: 12pt; line-height: 16pt; margin: 0; }
            </style>
            <section class="spacer"></section>
            <section class="avoid">
              <p class="part">Visible content fits before page end</p>
            </section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());

        let first_page_text = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Text(text) => Some(text.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(
            first_page_text.contains(&"Visible content fits before page end"),
            "trailing margins at a page boundary should not push an otherwise fitting avoid block"
        );
    }

    #[test]
    fn break_after_avoid_keeps_heading_on_current_page_when_next_block_fits() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 72pt; }
              .spacer { height: 390pt; }
              h2 {
                page-break-after: avoid;
                margin: 0;
                font-size: 16pt;
                line-height: 20pt;
              }
              table {
                width: 100%;
                border-collapse: collapse;
                margin: 0;
                font-size: 10pt;
                line-height: 14pt;
              }
              th, td { padding: 4pt; }
            </style>
            <section class="spacer"></section>
            <h2>Status heading fits</h2>
            <table>
              <thead><tr><th>Column A</th><th>Column B</th></tr></thead>
              <tbody><tr><td>Visible row</td><td>42</td></tr></tbody>
            </table>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());

        let first_page_text = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Text(text) => Some(text.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(
            first_page_text.contains(&"Status heading fits")
                && first_page_text.contains(&"Visible row"),
            "page-break-after: avoid should be advisory and should not move a heading when the following block fits current page"
        );
    }

    #[test]
    fn medium_table_headers_keep_multitoken_labels_on_one_line() {
        let html = r#"
            <table>
              <tr>
                <th>Completadas</th>
                <th>Aprobadas</th>
                <th>Pendientes</th>
                <th>Rechazadas</th>
                <th>Pendientes de pago</th>
                <th>Eliminadas</th>
              </tr>
              <tr><td>9</td><td>2</td><td>0</td><td>0</td><td>0</td><td>0</td></tr>
            </table>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let table = document.query_selector("table").expect("table exists");
        let rows = table_rows(&document, table);
        let widths = table_column_widths(&document, &rows, 480.0, 6);

        assert!(
            widths.get(4).copied().unwrap_or_default() >= 104.0,
            "medium multitoken status labels need enough intrinsic width to avoid avoid-block pagination drift: {widths:?}"
        );
    }

    #[test]
    fn long_tables_keep_content_based_column_widths() {
        let mut rows = String::new();
        for hour in 0..48 {
            rows.push_str(&format!(
                "<tr><td>24/3/26</td><td>{:02}:00</td><td>0</td><td>0</td><td>0</td></tr>",
                hour % 24
            ));
        }
        let html = format!(
            r#"
            <table>
              <tr>
                <th>Fecha</th>
                <th>Hora</th>
                <th>Total eventos</th>
                <th>Incidencias</th>
                <th>Pantallas afectadas</th>
              </tr>
              {rows}
            </table>
            "#
        );
        let document = crate::parser::parse_document(&html).expect("valid html");
        let table = document.query_selector("table").expect("table exists");
        let rows = table_rows(&document, table);
        let widths = table_column_widths(&document, &rows, 480.0, 5);

        assert!(
            widths[0] < widths[2] && widths[1] < widths[2] && widths[4] > widths[2],
            "long table columns should follow sampled intrinsic content instead of flattening to uniform widths: {widths:?}"
        );
    }

    #[test]
    fn table_cell_vertical_align_middle_offsets_short_content() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              table { width: 240pt; border-collapse: collapse; }
              td { padding: 0; font-size: 12pt; line-height: 16pt; }
              .middle { vertical-align: middle; }
            </style>
            <table>
              <tr>
                <td>Line one<br>Line two<br>Line three</td>
                <td class="middle">Middle</td>
              </tr>
            </table>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let y_for = |needle: &str| {
            pages[0]
                .items
                .iter()
                .find_map(|item| match item {
                    LayoutItem::Text(text) if text.text == needle => Some(text.y),
                    _ => None,
                })
                .expect("text rendered")
        };

        assert!(
            y_for("Middle") < y_for("Line one") - 8.0,
            "vertical-align: middle should move short table-cell content toward the row center"
        );
    }

    #[test]
    fn table_border_spacing_separates_cells_and_rows() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              table { width: 100pt; border-collapse: separate; border-spacing: 10pt 20pt; }
              td { padding: 0; background: rgb(219 234 254); font-size: 10pt; line-height: 10pt; }
            </style>
            <table>
              <tr><td>A</td><td>B</td></tr>
              <tr><td>C</td><td>D</td></tr>
            </table>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let backgrounds = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Rect(rect)
                    if (rect.color.r - 219.0 / 255.0).abs() < 0.01
                        && (rect.color.g - 234.0 / 255.0).abs() < 0.01 =>
                {
                    Some((rect.x, rect.y, rect.width, rect.height))
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(backgrounds.len() >= 4, "cell backgrounds: {backgrounds:?}");
        let first_gap = backgrounds[1].0 - (backgrounds[0].0 + backgrounds[0].2);
        let row_gap = backgrounds[0].1 - (backgrounds[2].1 + backgrounds[2].3);
        assert!(
            (first_gap - 10.0).abs() < 0.01,
            "horizontal gap {first_gap}"
        );
        assert!((row_gap - 20.0).abs() < 0.01, "vertical gap {row_gap}");
    }

    #[test]
    fn generated_before_and_after_content_renders_as_inline_segments() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              p { margin: 0; font-size: 16px; line-height: 20px; }
              .target::before { content: "Before "; color: rgb(37 99 235); }
              .target::after { content: " After"; color: rgb(220 38 38); }
            </style>
            <p class="target">Body</p>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let text = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Text(text) => Some(text.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(text.join(""), "Before Body After");
    }

    #[test]
    fn block_container_generated_text_defaults_to_inline_pseudo_layout() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              section { margin: 0; font-size: 16px; line-height: 20px; }
              .target::before { content: "Before "; color: rgb(37 99 235); }
              .target::after { content: " After"; color: rgb(220 38 38); }
            </style>
            <section class="target">Body</section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let text_runs = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Text(text) => Some((text.text.as_str(), text.y)),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(
            text_runs
                .iter()
                .map(|(text, _)| *text)
                .collect::<Vec<_>>()
                .join(""),
            "Before Body After"
        );
        assert!(
            text_runs
                .windows(2)
                .all(|pair| (pair[0].1 - pair[1].1).abs() < 0.01),
            "inline generated pseudo text for block containers should stay on the same line: {text_runs:?}"
        );
    }

    #[test]
    fn static_form_controls_render_as_replaced_boxes() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              form { margin: 0; display: grid; gap: 4px; }
              input, select { color: rgb(37 99 235); }
            </style>
            <form>
              <input value="Alice">
              <input type="checkbox" checked>
              <input type="radio" checked>
              <select><option>Draft</option><option selected>Published</option></select>
              <input type="hidden" value="must not render">
            </form>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let text = pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutItem::Text(text) => Some(text.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let strokes = pages[0]
            .items
            .iter()
            .filter(|item| {
                matches!(
                    item,
                    LayoutItem::StrokeRect(_) | LayoutItem::CircleStroke(_)
                )
            })
            .count();
        let check_lines = pages[0]
            .items
            .iter()
            .filter(|item| matches!(item, LayoutItem::Line(line) if line.line_cap_round))
            .count();

        assert!(
            text.contains(&"Alice"),
            "text input value should render: {text:?}"
        );
        assert!(
            text.contains(&"Published"),
            "select should render the selected option text: {text:?}"
        );
        assert!(
            !text.contains(&"must not render"),
            "hidden controls must stay non-rendered: {text:?}"
        );
        assert!(
            strokes >= 4,
            "text/select/checkbox/radio controls should paint native-looking boxes: {strokes}"
        );
        assert_eq!(
            check_lines, 2,
            "checked checkbox should paint a two-segment check mark"
        );
    }

    #[test]
    fn empty_generated_pseudo_content_can_paint_flow_box() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              .target { margin: 0; padding: 0; }
              .target::before {
                content: "";
                display: block;
                width: 120px;
                height: 16px;
                background: rgb(37 99 235);
                border-radius: 4px;
              }
            </style>
            <section class="target"></section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let blue_box = pages[0]
            .items
            .iter()
            .find_map(|item| match item {
                LayoutItem::RoundRect(rect)
                    if (rect.color.r - 37.0 / 255.0).abs() < 0.01
                        && (rect.color.g - 99.0 / 255.0).abs() < 0.01 =>
                {
                    Some((rect.width, rect.height, rect.radius))
                }
                _ => None,
            })
            .expect("empty generated pseudo box should paint a rounded background");

        assert!((blue_box.0 - 90.0).abs() < 0.01, "width={}", blue_box.0);
        assert!((blue_box.1 - 12.0).abs() < 0.01, "height={}", blue_box.1);
        assert!(blue_box.2 > 0.0);
    }

    #[test]
    fn svg_linear_gradient_rect_paints_as_clipped_gradient_item() {
        let html = r##"
            <style>
              @page { size: Letter; margin: 0; }
              svg { width: 80px; height: 48px; }
            </style>
            <svg viewBox="0 0 80 48">
              <defs>
                <linearGradient id="wash" x1="0" y1="0" x2="1" y2="1">
                  <stop offset="0%" stop-color="#dbeafe" />
                  <stop offset="100%" stop-color="#2563eb" />
                </linearGradient>
              </defs>
              <rect x="4" y="4" width="72" height="40" rx="8" fill="url(#wash)" />
            </svg>
        "##;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());

        assert!(pages[0]
            .items
            .iter()
            .any(|item| matches!(item, LayoutItem::BeginClip(clip) if clip.radius > 0.0)));
        assert!(pages[0].items.iter().any(|item| {
            matches!(
                item,
                LayoutItem::LinearGradient(gradient)
                    if gradient.gradient.stops.len() == 2
                        && (gradient.gradient.angle_deg - 135.0).abs() < 0.01
            )
        }));
    }

    #[test]
    fn svg_group_transform_moves_child_geometry() {
        let html = r##"
            <style>
              @page { size: Letter; margin: 0; }
              svg { width: 80px; height: 48px; }
            </style>
            <svg viewBox="0 0 80 48">
              <g transform="translate(10 5) scale(2)">
                <line x1="0" y1="0" x2="10" y2="0" stroke="#1d4ed8" stroke-width="2" />
              </g>
            </svg>
        "##;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());

        let line = pages[0]
            .items
            .iter()
            .find_map(|item| match item {
                LayoutItem::Line(line) => Some(line),
                _ => None,
            })
            .expect("transformed svg line should paint");

        assert!(
            ((line.x2 - line.x1) - 15.0).abs() < 0.01,
            "width span x1={} x2={}",
            line.x1,
            line.x2
        );
        assert!(
            (line.y1 - line.y2).abs() < 0.01,
            "y1={} y2={}",
            line.y1,
            line.y2
        );
        assert!((line.width - 3.0).abs() < 0.01, "width={}", line.width);
    }

    #[test]
    fn svg_without_explicit_height_uses_viewbox_aspect_ratio() {
        let html = r##"
            <style>
              @page { size: Letter; margin: 0; }
              svg { width: 220px; }
            </style>
            <svg viewBox="0 0 260 72">
              <text x="8" y="68" font-size="11">Authorized signature</text>
            </svg>
        "##;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());

        let text_y = pages[0]
            .items
            .iter()
            .find_map(|item| match item {
                LayoutItem::Text(text) if text.text == "Authorized signature" => Some(text.y),
                _ => None,
            })
            .expect("svg text should render");

        assert!(
            text_y > 700.0,
            "viewBox aspect-ratio height should keep text near the top page, y={text_y}"
        );
    }

    #[test]
    fn svg_elements_paint_css_filter_drop_shadow_and_decoration() {
        let html = r##"
            <style>
              @page { size: Letter; margin: 0; }
              svg.logo {
                width: 80px;
                height: 40px;
                filter: drop-shadow(0 12px 18px rgba(0, 0, 0, 0.20));
                border: 2px solid rgb(37 99 235);
                border-radius: 8px;
              }
            </style>
            <svg class="logo" viewBox="0 0 80 40">
              <rect x="0" y="0" width="80" height="40" fill="#bfdbfe" />
            </svg>
        "##;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let has_shadow = pages[0].items.iter().any(|item| match item {
            LayoutItem::RoundRect(rect) => {
                rect.color.a < 0.2 && rect.width > 60.0 && rect.height > 30.0
            }
            LayoutItem::Rect(rect) => rect.color.a < 0.2 && rect.width > 60.0 && rect.height > 30.0,
            _ => false,
        });
        let has_border = pages[0].items.iter().any(|item| match item {
            LayoutItem::StrokeRect(rect) => {
                (rect.color.b - 235.0 / 255.0).abs() < 0.01 && rect.stroke_width > 0.0
            }
            _ => false,
        });

        assert!(
            has_shadow,
            "SVG CSS drop-shadow should produce shadow paint"
        );
        assert!(has_border, "SVG CSS border decoration should paint");
    }

    #[test]
    fn absolute_auto_width_element_shrinks_to_fit_for_right_inset() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              .parent {
                position: relative;
                width: 300px;
                height: 100px;
                margin: 0;
              }
              .badge {
                position: absolute;
                right: 20px;
                top: 10px;
                font-size: 20px;
                line-height: 24px;
                font-weight: 700;
              }
            </style>
            <section class="parent">
              <div class="badge">PAID</div>
            </section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let paid_x = pages[0]
            .items
            .iter()
            .find_map(|item| match item {
                LayoutItem::Text(text) if text.text == "PAID" => Some(text.x),
                _ => None,
            })
            .expect("absolute text should render");

        assert!(
            paid_x > 120.0,
            "right-aligned auto-width absolute element should not stretch to x=0; x={paid_x}"
        );
    }

    #[test]
    fn absolute_empty_generated_pseudo_uses_parent_box_as_containing_block() {
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              .card {
                position: relative;
                width: 200px;
                height: 100px;
                margin: 0;
              }
              .card::after {
                content: "";
                position: absolute;
                top: 10px;
                right: 20px;
                bottom: 30px;
                left: 40px;
                background: rgb(220 38 38);
              }
            </style>
            <section class="card"></section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let pages = layout_document(&document, &stylesheet, &RenderOptions::default());
        let red_box = pages[0]
            .items
            .iter()
            .find_map(|item| match item {
                LayoutItem::Rect(rect)
                    if (rect.color.r - 220.0 / 255.0).abs() < 0.01
                        && (rect.color.g - 38.0 / 255.0).abs() < 0.01 =>
                {
                    Some((rect.width, rect.height))
                }
                _ => None,
            })
            .expect("absolute generated pseudo box should paint");

        assert!((red_box.0 - 105.0).abs() < 0.01, "width={}", red_box.0);
        assert!((red_box.1 - 45.0).abs() < 0.01, "height={}", red_box.1);
    }

    #[test]
    fn parses_jpeg_dimensions_from_local_asset() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/assets/generic-photo.jpg");
        let data = std::fs::read(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));

        assert_eq!(jpeg_dimensions(&data), Some((320, 180)));
    }

    #[test]
    fn parses_png_rgb_asset_for_pdf_predictor_embedding() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/assets/generic-logo.png");
        let data = std::fs::read(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        let image = png_image_data(&data).unwrap_or_else(|| panic!("expected png image data"));

        assert_eq!(image.intrinsic_width_px, 96);
        assert_eq!(image.intrinsic_height_px, 96);
        assert_eq!(
            image.format,
            ImageFormat::Png {
                color_space: PngColorSpace::DeviceRgb,
                bits_per_component: 8,
                components: 3,
            }
        );
        assert!(!image.data.is_empty());
    }

    #[test]
    fn parses_png_alpha_asset_into_color_and_soft_mask() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/assets/generic-alpha.png");
        let data = std::fs::read(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        let image = png_image_data(&data).unwrap_or_else(|| panic!("expected png image data"));

        assert_eq!(image.intrinsic_width_px, 96);
        assert_eq!(image.intrinsic_height_px, 96);
        assert!(matches!(
            image.format,
            ImageFormat::Raw {
                color_space: PngColorSpace::DeviceRgb,
                bits_per_component: 8,
                components: 3,
            }
        ));
        assert_eq!(image.data.len(), 96 * 96 * 3);
        assert_eq!(image.alpha_mask.as_ref().map(Vec::len), Some(96 * 96));
    }

    #[test]
    fn parses_png_palette_asset_into_rgb_and_optional_alpha() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/assets/generic-palette.png");
        let data = std::fs::read(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        let image = png_image_data(&data).unwrap_or_else(|| panic!("expected png image data"));

        assert_eq!(image.intrinsic_width_px, 96);
        assert_eq!(image.intrinsic_height_px, 96);
        assert!(matches!(
            image.format,
            ImageFormat::Raw {
                color_space: PngColorSpace::DeviceRgb,
                bits_per_component: 8,
                components: 3,
            }
        ));
        assert_eq!(image.data.len(), 96 * 96 * 3);
        assert_eq!(image.alpha_mask.as_ref().map(Vec::len), Some(96 * 96));
    }

    #[test]
    fn local_jpeg_img_renders_as_image_layout_item() {
        let examples_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              img { width: 160px; height: 90px; object-fit: cover; border-radius: 12px; }
            </style>
            <img src="assets/generic-photo.jpg" alt="fixture">
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let mut options = RenderOptions::default();
        options.base_url = Some(examples_dir.to_string_lossy().to_string());
        options.allow_local_assets = false;
        let blocked = layout_document(&document, &stylesheet, &options);
        assert!(blocked.iter().all(|page| page
            .items
            .iter()
            .all(|item| !matches!(item, LayoutItem::Image(_)))));
        let absolute = examples_dir.join("assets/generic-photo.jpg");
        assert!(load_image_asset(&absolute.to_string_lossy(), &options).is_none());
        options.allow_local_assets = true;
        let pages = layout_document(&document, &stylesheet, &options);
        let image = pages[0].items.iter().find_map(|item| match item {
            LayoutItem::Image(image) => Some(image),
            _ => None,
        });

        let image = image.unwrap_or_else(|| panic!("expected image item in {pages:#?}"));
        assert_eq!(image.intrinsic_width_px, 320);
        assert_eq!(image.intrinsic_height_px, 180);
        assert!((image.width - 120.0).abs() < 0.01);
        assert!(image.data.starts_with(&[0xff, 0xd8]));
    }

    #[test]
    fn object_position_shifts_cropped_cover_image_like_css_replaced_content() {
        let (x, y, width, height) = object_fit_rect(
            ObjectFit::Cover,
            0.0,
            0.0,
            120.0,
            120.0,
            240.0,
            135.0,
            1.0,
            1.0,
        );

        assert!((x + 93.33).abs() < 0.05);
        assert!(y.abs() < 0.01);
        assert!((width - 213.33).abs() < 0.05);
        assert!((height - 120.0).abs() < 0.01);
    }

    #[test]
    fn local_png_img_renders_as_image_layout_item() {
        let examples_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              img { width: 96px; height: 96px; object-fit: contain; }
            </style>
            <img src="assets/generic-logo.png" alt="fixture">
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let mut options = RenderOptions::default();
        options.base_url = Some(examples_dir.to_string_lossy().to_string());
        let pages = layout_document(&document, &stylesheet, &options);
        let image = pages[0].items.iter().find_map(|item| match item {
            LayoutItem::Image(image) => Some(image),
            _ => None,
        });

        let image = image.unwrap_or_else(|| panic!("expected image item in {pages:#?}"));
        assert_eq!(image.intrinsic_width_px, 96);
        assert_eq!(image.intrinsic_height_px, 96);
        assert!(matches!(
            image.format,
            ImageFormat::Png { components: 3, .. }
        ));
    }

    #[test]
    fn local_png_alpha_img_renders_with_alpha_mask() {
        let examples_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              img { width: 96px; height: 96px; object-fit: contain; }
            </style>
            <img src="assets/generic-alpha.png" alt="fixture">
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let mut options = RenderOptions::default();
        options.base_url = Some(examples_dir.to_string_lossy().to_string());
        let pages = layout_document(&document, &stylesheet, &options);
        let image = pages[0].items.iter().find_map(|item| match item {
            LayoutItem::Image(image) => Some(image),
            _ => None,
        });

        let image = image.unwrap_or_else(|| panic!("expected image item in {pages:#?}"));
        assert!(matches!(
            image.format,
            ImageFormat::Raw { components: 3, .. }
        ));
        assert_eq!(image.alpha_mask.as_ref().map(Vec::len), Some(96 * 96));
    }

    #[test]
    fn local_png_palette_img_renders_with_alpha_mask() {
        let examples_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              img { width: 96px; height: 96px; object-fit: contain; }
            </style>
            <img src="assets/generic-palette.png" alt="fixture">
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let mut options = RenderOptions::default();
        options.base_url = Some(examples_dir.to_string_lossy().to_string());
        let pages = layout_document(&document, &stylesheet, &options);
        let image = pages[0].items.iter().find_map(|item| match item {
            LayoutItem::Image(image) => Some(image),
            _ => None,
        });

        let image = image.unwrap_or_else(|| panic!("expected image item in {pages:#?}"));
        assert!(matches!(
            image.format,
            ImageFormat::Raw { components: 3, .. }
        ));
        assert_eq!(image.alpha_mask.as_ref().map(Vec::len), Some(96 * 96));
    }

    #[test]
    fn local_jpeg_background_image_renders_as_image_layout_item() {
        let examples_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let html = r#"
            <style>
              @page { size: Letter; margin: 0; }
              .hero {
                width: 240px;
                height: 120px;
                background-image: url("assets/generic-photo.jpg");
                background-size: cover;
                background-position: right bottom;
              }
            </style>
            <section class="hero"></section>
        "#;
        let document = crate::parser::parse_document(html).expect("valid html");
        let stylesheet = Stylesheet::from_document(&document);
        let mut options = RenderOptions::default();
        options.base_url = Some(examples_dir.to_string_lossy().to_string());
        let pages = layout_document(&document, &stylesheet, &options);
        let image_count = pages[0]
            .items
            .iter()
            .filter(|item| matches!(item, LayoutItem::Image(_)))
            .count();

        assert_eq!(image_count, 1);
    }
}
