use crate::dom::{Document, ElementNode, Node, NodeId};
use crate::PageOptions;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };

    pub fn with_opacity(self, opacity: f32) -> Self {
        Self {
            a: (self.a * opacity.clamp(0.0, 1.0)).clamp(0.0, 1.0),
            ..self
        }
    }

    pub(crate) fn from_css(value: &str) -> Option<Self> {
        let value = value.trim().to_ascii_lowercase();
        match value.as_str() {
            "black" => Some(Self::BLACK),
            "white" => Some(Self::WHITE),
            "transparent" => Some(Self {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }),
            "red" => Some(Self {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            }),
            "green" => Some(Self {
                r: 0.0,
                g: 128.0 / 255.0,
                b: 0.0,
                a: 1.0,
            }),
            "blue" => Some(Self {
                r: 0.0,
                g: 0.0,
                b: 1.0,
                a: 1.0,
            }),
            "gray" | "grey" => Some(Self {
                r: 128.0 / 255.0,
                g: 128.0 / 255.0,
                b: 128.0 / 255.0,
                a: 1.0,
            }),
            "slate" => Some(Self {
                r: 0.06,
                g: 0.09,
                b: 0.16,
                a: 1.0,
            }),
            _ if value.starts_with('#') => parse_hex_color(&value),
            _ if value.starts_with("color-mix(") => parse_color_mix(&value),
            _ if value.starts_with("rgb(") || value.starts_with("rgba(") => parse_rgb_color(&value),
            _ if value.starts_with("hsl(") || value.starts_with("hsla(") => parse_hsl_color(&value),
            _ if value.starts_with("hwb(") => parse_hwb_color(&value),
            _ if value.starts_with("color(") => parse_css_color_function(&value),
            _ if value.starts_with("lab(") => parse_lab_color(&value),
            _ if value.starts_with("lch(") => parse_lch_color(&value),
            _ if value.starts_with("oklch(") => parse_oklch_color(&value),
            _ if value.starts_with("oklab(") => parse_oklab_color(&value),
            _ if value.starts_with("light-dark(") => parse_light_dark_color(&value),
            _ => parse_named_color(&value),
        }
    }
}

fn parse_color_or_current(value: &str, current_color: Color) -> Option<Color> {
    if value.trim().eq_ignore_ascii_case("currentcolor") {
        Some(current_color)
    } else {
        Color::from_css(value)
    }
}

fn parse_css_url(value: &str) -> Option<String> {
    let lower = value.to_ascii_lowercase();
    let start = lower.find("url(")?;
    let body_start = start + "url(".len();
    let body_end = value[body_start..].find(')')? + body_start;
    let raw = value[body_start..body_end].trim();
    let unquoted = raw
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .or_else(|| {
            raw.strip_prefix('\'')
                .and_then(|inner| inner.strip_suffix('\''))
        })
        .unwrap_or(raw)
        .trim();
    (!unquoted.is_empty()).then(|| unquoted.to_string())
}

fn parse_background_position(value: &str) -> (f32, f32) {
    let mut x = None;
    let mut y = None;
    let mut numeric_tokens = Vec::new();
    for part in split_css_whitespace(value) {
        match part.as_str() {
            "left" => x = Some(0.0),
            "center" => {}
            "right" => x = Some(1.0),
            "top" => y = Some(1.0),
            "bottom" => y = Some(0.0),
            _ => {
                if let Some(percent) = parse_position_percentage(&part) {
                    numeric_tokens.push(percent);
                }
            }
        }
    }
    let had_x_keyword = x.is_some();
    if x.is_none() {
        x = numeric_tokens.first().copied();
    }
    if y.is_none() {
        let y_percent = if had_x_keyword {
            numeric_tokens.first().copied()
        } else {
            numeric_tokens.get(1).copied()
        };
        y = y_percent.map(|percent| 1.0 - percent);
    }
    (x.unwrap_or(0.5), y.unwrap_or(0.5))
}

fn parse_position_percentage(value: &str) -> Option<f32> {
    let percent = value.strip_suffix('%')?.trim().parse::<f32>().ok()?;
    Some((percent / 100.0).clamp(0.0, 1.0))
}

fn parse_background_shorthand_geometry(
    value: &str,
) -> (Option<BackgroundSize>, Option<(f32, f32)>) {
    let normalized = normalize_background_slash_outside_functions(value);
    let tokens = split_css_whitespace(&normalized);
    let mut position_tokens = Vec::new();
    let mut size = None;
    let mut after_slash = false;

    for token in tokens {
        if token == "/" {
            after_slash = true;
            continue;
        }
        if after_slash {
            if size.is_none() {
                size = parse_background_size_keyword(&token);
            }
            continue;
        }
        match token.as_str() {
            "left" | "right" | "center" | "top" | "bottom" => position_tokens.push(token),
            _ => {}
        }
    }

    let position = (!position_tokens.is_empty())
        .then(|| parse_background_position(&position_tokens.join(" ")));
    (size, position)
}

fn parse_background_size_keyword(value: &str) -> Option<BackgroundSize> {
    match value.trim() {
        "cover" => Some(BackgroundSize::Cover),
        "contain" => Some(BackgroundSize::Contain),
        "auto" => Some(BackgroundSize::Auto),
        _ => None,
    }
}

fn parse_background_clip(value: &str) -> Option<BackgroundClip> {
    let first_layer = split_top_level(value, &[','])
        .into_iter()
        .next()
        .unwrap_or(value)
        .trim();
    parse_background_box_keyword(first_layer)
}

fn parse_background_clip_from_shorthand(value: &str) -> Option<BackgroundClip> {
    let first_layer = split_top_level(value, &[','])
        .into_iter()
        .next()
        .unwrap_or(value);
    split_css_whitespace(first_layer)
        .into_iter()
        .filter_map(|token| parse_background_box_keyword(&token))
        .last()
}

fn parse_background_box_keyword(value: &str) -> Option<BackgroundClip> {
    match value.trim() {
        "border-box" => Some(BackgroundClip::BorderBox),
        "padding-box" => Some(BackgroundClip::PaddingBox),
        "content-box" => Some(BackgroundClip::ContentBox),
        // `background-clip: text` needs glyph clipping, so keep the current
        // border-box paint behavior instead of pretending full support.
        _ => None,
    }
}

fn normalize_background_slash_outside_functions(value: &str) -> String {
    let mut normalized = String::with_capacity(value.len() + 4);
    let mut depth = 0_i32;
    for ch in value.chars() {
        match ch {
            '(' => {
                depth += 1;
                normalized.push(ch);
            }
            ')' => {
                depth = (depth - 1).max(0);
                normalized.push(ch);
            }
            '/' if depth == 0 => normalized.push_str(" / "),
            _ => normalized.push(ch),
        }
    }
    normalized
}

#[derive(Debug, Clone)]
pub struct ComputedStyle {
    pub display: Display,
    pub visibility: Visibility,
    pub content: Option<String>,
    pub font_size: f32,
    pub line_height: f32,
    pub line_height_multiplier: Option<f32>,
    pub line_height_is_normal: bool,
    pub color: Color,
    pub background: Option<Color>,
    pub background_gradient: Option<LinearGradient>,
    pub background_radials: Vec<RadialGradient>,
    pub background_image: Option<String>,
    pub background_size: BackgroundSize,
    pub background_clip: BackgroundClip,
    pub background_position_x: f32,
    pub background_position_y: f32,
    pub box_shadows: Vec<BoxShadow>,
    pub text_shadow: Option<BoxShadow>,
    pub margin_top: f32,
    pub margin_right: f32,
    pub margin_bottom: f32,
    pub margin_left: f32,
    pub margin_right_auto: bool,
    pub margin_left_auto: bool,
    pub padding_top: f32,
    pub padding_right: f32,
    pub padding_bottom: f32,
    pub padding_left: f32,
    pub border_radius: f32,
    pub border_width: f32,
    pub border_color: Option<Color>,
    pub border_style: BorderLineStyle,
    pub border_top_width: f32,
    pub border_right_width: f32,
    pub border_bottom_width: f32,
    pub border_left_width: f32,
    pub border_top_color: Option<Color>,
    pub border_right_color: Option<Color>,
    pub border_bottom_color: Option<Color>,
    pub border_left_color: Option<Color>,
    pub outline_width: f32,
    pub outline_color: Option<Color>,
    pub outline_style: BorderLineStyle,
    pub outline_offset: f32,
    pub opacity: f32,
    pub gap: f32,
    pub row_gap: f32,
    pub column_gap: f32,
    pub width: Option<CssLength>,
    pub min_width: Option<CssLength>,
    pub max_width: Option<CssLength>,
    pub height: Option<CssLength>,
    pub min_height: Option<CssLength>,
    pub max_height: Option<CssLength>,
    pub aspect_ratio: Option<f32>,
    pub position: Position,
    pub float: FloatSide,
    pub clear: ClearSide,
    pub z_index: Option<i32>,
    pub inset_top: Option<CssLength>,
    pub inset_right: Option<CssLength>,
    pub inset_bottom: Option<CssLength>,
    pub inset_left: Option<CssLength>,
    pub flex_direction: FlexDirection,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Option<CssLength>,
    pub flex_wrap: FlexWrap,
    pub justify_content: JustifyContent,
    pub align_content: JustifyContent,
    pub align_items: AlignItems,
    pub align_self: Option<AlignItems>,
    pub justify_items: AlignItems,
    pub justify_self: Option<AlignItems>,
    pub order: i32,
    pub grid_columns: Option<usize>,
    pub grid_min_column_width: Option<CssLength>,
    pub grid_tracks: Vec<GridTrack>,
    pub grid_row_tracks: Vec<GridTrack>,
    pub grid_column_start: Option<usize>,
    pub grid_column_end: Option<usize>,
    pub grid_column_span: usize,
    pub grid_row_start: Option<usize>,
    pub grid_row_end: Option<usize>,
    pub grid_row_span: usize,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
    pub font_variant_numeric: FontVariantNumeric,
    pub font_face: FontFace,
    pub letter_spacing: f32,
    pub word_spacing: f32,
    pub tab_size: TabSize,
    pub text_decoration: TextDecoration,
    pub text_decoration_color: Option<Color>,
    pub text_decoration_thickness: Option<f32>,
    pub text_underline_offset: Option<f32>,
    pub text_align: TextAlign,
    pub text_align_last: Option<TextAlign>,
    pub direction: TextDirection,
    pub vertical_align: VerticalAlign,
    pub text_transform: TextTransform,
    pub list_style_type: ListStyleType,
    pub transform_rotate_deg: f32,
    pub transform_origin_x: CssLength,
    pub transform_origin_y: CssLength,
    pub transform_translate_x: Option<CssLength>,
    pub transform_translate_y: Option<CssLength>,
    pub transform_scale_x: f32,
    pub transform_scale_y: f32,
    pub white_space: WhiteSpace,
    pub text_wrap_style: TextWrapStyle,
    pub text_overflow: TextOverflow,
    pub overflow_wrap: OverflowWrap,
    pub object_fit: ObjectFit,
    pub object_position_x: f32,
    pub object_position_y: f32,
    pub overflow_hidden: bool,
    pub page_break_inside_avoid: bool,
    pub page_break_before: bool,
    pub page_break_after: bool,
    pub page_break_after_avoid: bool,
    pub border_collapse: BorderCollapse,
    pub border_spacing_horizontal: f32,
    pub border_spacing_vertical: f32,
    pub box_sizing: BoxSizing,
    pub custom_properties: BTreeMap<String, String>,
}

impl ComputedStyle {
    pub fn for_node(document: &Document, stylesheet: &Stylesheet, id: NodeId) -> Self {
        Self::for_node_with_parent(document, stylesheet, id, None)
    }

    pub(crate) fn for_node_with_parent(
        document: &Document,
        stylesheet: &Stylesheet,
        id: NodeId,
        parent_style: Option<&ComputedStyle>,
    ) -> Self {
        let Some(Node::Element(element)) = document.node(id) else {
            return Self::default();
        };

        let parent_id = document.parent_of(id);
        let mut style = parent_style
            .map(Self::inherited_from)
            .or_else(|| {
                parent_id.and_then(|parent_id| match document.node(parent_id) {
                    Some(Node::Element(_)) => Some(Self::inherited_from(&Self::for_node(
                        document, stylesheet, parent_id,
                    ))),
                    _ => None,
                })
            })
            .unwrap_or_default();
        if parent_id.is_none() {
            style.custom_properties.extend(
                stylesheet
                    .variables
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone())),
            );
        }
        style.apply_user_agent_defaults(element);

        let matching_rules = stylesheet
            .candidate_rule_indexes(element)
            .into_iter()
            .filter_map(|source_order| {
                let rule = stylesheet.rules.get(source_order)?;
                (rule.selector.could_match_element(element)
                    && rule.selector.matches(document, id, element))
                .then_some((source_order, rule))
            })
            .collect::<Vec<_>>();
        let mut declarations = ordered_rule_declarations(
            matching_rules
                .into_iter()
                .filter(|(_, rule)| rule.pseudo.is_none()),
            stylesheet.layer_count,
        );

        if let Some(inline) = element.attr("style") {
            append_inline_declarations(&mut declarations, parse_declarations(inline));
        }
        style.apply_custom_properties(&declarations, &stylesheet.referenced_variables);
        style.apply_declarations(&declarations, stylesheet.length_context());

        style
    }

    pub(crate) fn inherited_from(parent: &Self) -> Self {
        let mut style = Self::default();
        style.visibility = parent.visibility;
        style.content = None;
        style.font_size = parent.font_size;
        style.line_height = parent.line_height;
        style.line_height_multiplier = parent.line_height_multiplier;
        style.line_height_is_normal = parent.line_height_is_normal;
        style.color = parent.color;
        style.opacity = parent.opacity;
        style.font_weight = parent.font_weight;
        style.font_style = parent.font_style;
        style.font_variant_numeric = parent.font_variant_numeric;
        style.font_face = parent.font_face;
        style.letter_spacing = parent.letter_spacing;
        style.word_spacing = parent.word_spacing;
        style.tab_size = parent.tab_size;
        style.text_decoration = parent.text_decoration;
        style.text_decoration_color = parent.text_decoration_color;
        style.text_decoration_thickness = parent.text_decoration_thickness;
        style.text_underline_offset = parent.text_underline_offset;
        style.text_align = parent.text_align;
        style.text_align_last = parent.text_align_last;
        style.direction = parent.direction;
        style.text_transform = parent.text_transform;
        style.list_style_type = parent.list_style_type;
        style.text_shadow = parent.text_shadow;
        style.transform_rotate_deg = parent.transform_rotate_deg;
        style.white_space = parent.white_space;
        style.text_wrap_style = parent.text_wrap_style;
        style.text_overflow = parent.text_overflow;
        style.overflow_wrap = parent.overflow_wrap;
        style.custom_properties = parent.custom_properties.clone();
        style
    }

    fn apply_user_agent_defaults(&mut self, element: &ElementNode) {
        if element.attr("hidden").is_some() {
            self.display = Display::None;
            return;
        }
        let tag = element.tag.as_str();
        if matches!(
            tag,
            "a" | "abbr"
                | "b"
                | "cite"
                | "code"
                | "dfn"
                | "em"
                | "i"
                | "ins"
                | "del"
                | "s"
                | "strike"
                | "small"
                | "span"
                | "strong"
                | "sub"
                | "sup"
                | "u"
                | "var"
                | "mark"
                | "label"
        ) {
            self.display = Display::Inline;
        }
        match tag {
            "h1" => {
                self.font_size = 28.0;
                self.line_height = 34.0;
                self.line_height_is_normal = false;
                self.margin_bottom = 14.0;
                self.font_weight = FontWeight::Bold;
            }
            "h2" => {
                self.font_size = 22.0;
                self.line_height = 28.0;
                self.line_height_is_normal = false;
                self.margin_bottom = 12.0;
                self.font_weight = FontWeight::Bold;
            }
            "h3" => {
                self.font_size = 18.0;
                self.line_height = 24.0;
                self.line_height_is_normal = false;
                self.margin_bottom = 10.0;
                self.font_weight = FontWeight::Bold;
            }
            "p" => {
                self.margin_bottom = 10.0;
            }
            "small" => {
                self.font_size = 10.0;
                self.line_height = 14.0;
                self.line_height_is_normal = false;
            }
            "strong" | "b" => {
                self.font_weight = FontWeight::Bold;
            }
            "em" | "i" | "cite" | "dfn" | "var" => {
                self.font_style = FontStyle::Italic;
            }
            "u" | "ins" => {
                self.text_decoration.underline = true;
            }
            "s" | "strike" | "del" => {
                self.text_decoration.line_through = true;
            }
            "table" => {
                self.margin_bottom = 12.0;
            }
            "ul" => {
                self.margin_bottom = 10.0;
                self.padding_left = 30.0;
                self.list_style_type = ListStyleType::Disc;
            }
            "ol" => {
                self.margin_bottom = 10.0;
                self.padding_left = 30.0;
                self.list_style_type = ListStyleType::Decimal;
            }
            "th" => {
                self.font_size = 10.0;
                self.line_height = 14.0;
                self.line_height_is_normal = false;
                self.font_weight = FontWeight::Bold;
                self.padding_top = 6.0;
                self.padding_right = 6.0;
                self.padding_bottom = 6.0;
                self.padding_left = 6.0;
            }
            "td" => {
                self.font_size = 10.0;
                self.line_height = 14.0;
                self.line_height_is_normal = false;
                self.padding_top = 6.0;
                self.padding_right = 6.0;
                self.padding_bottom = 6.0;
                self.padding_left = 6.0;
            }
            "script" | "style" | "head" | "meta" | "title" | "noscript" | "template" => {
                self.display = Display::None;
            }
            _ => {}
        }
    }

    fn apply_custom_properties(
        &mut self,
        declarations: &[Declaration],
        referenced_variables: &BTreeSet<String>,
    ) {
        for declaration in declarations {
            if declaration.property.starts_with("--")
                && referenced_variables.contains(&declaration.property)
            {
                self.custom_properties
                    .insert(declaration.property.clone(), declaration.value.clone());
            }
        }
    }

    fn apply_declarations(&mut self, declarations: &[Declaration], length_context: LengthContext) {
        self.prime_font_relative_context(declarations, length_context);
        for declaration in declarations {
            if declaration.property.starts_with("--") {
                continue;
            }
            if !is_supported_property(&declaration.property) {
                continue;
            }
            let resolved_value = resolve_css_value(&declaration.value, &self.custom_properties);
            let value = resolved_value.to_ascii_lowercase();
            let length_context = length_context
                .with_em_base(self.font_size)
                .with_lh_base(self.line_height);
            match declaration.property.as_str() {
                "all" => {
                    if value == "initial" {
                        self.reset_all_except_custom_properties();
                    }
                }
                "content" => {
                    self.content = parse_css_content_value(&resolved_value);
                }
                "display" => match value.as_str() {
                    "none" => self.display = Display::None,
                    "contents" => self.display = Display::Contents,
                    "inline" => self.display = Display::Inline,
                    "flex" => self.display = Display::Flex,
                    "inline-flex" => self.display = Display::InlineFlex,
                    "grid" => self.display = Display::Grid,
                    "inline-grid" => self.display = Display::Grid,
                    "inline-block" => self.display = Display::InlineBlock,
                    "block" | "flow-root" | "list-item" | "table" => self.display = Display::Block,
                    value if value.contains("flow-root") => self.display = Display::Block,
                    _ => {}
                },
                "box-sizing" => match value.as_str() {
                    "border-box" => self.box_sizing = BoxSizing::BorderBox,
                    "content-box" => self.box_sizing = BoxSizing::ContentBox,
                    _ => {}
                },
                "visibility" | "content-visibility" => match value.as_str() {
                    "hidden" | "collapse" => self.visibility = Visibility::Hidden,
                    "visible" | "auto" => self.visibility = Visibility::Visible,
                    _ => {}
                },
                "clip" => {
                    if is_zero_rect_clip(&value) {
                        self.visibility = Visibility::Hidden;
                    }
                }
                "position" => match value.as_str() {
                    "relative" => self.position = Position::Relative,
                    "absolute" => self.position = Position::Absolute,
                    "fixed" => self.position = Position::Fixed,
                    "sticky" => self.position = Position::Sticky,
                    "static" => self.position = Position::Static,
                    _ => {}
                },
                "float" => {
                    self.float = match value.as_str() {
                        "left" => FloatSide::Left,
                        "right" => FloatSide::Right,
                        "inline-start" => match self.direction {
                            TextDirection::Ltr => FloatSide::Left,
                            TextDirection::Rtl => FloatSide::Right,
                        },
                        "inline-end" => match self.direction {
                            TextDirection::Ltr => FloatSide::Right,
                            TextDirection::Rtl => FloatSide::Left,
                        },
                        "none" => FloatSide::None,
                        _ => self.float,
                    };
                }
                "clear" => {
                    self.clear = match value.as_str() {
                        "left" => ClearSide::Left,
                        "right" => ClearSide::Right,
                        "both" => ClearSide::Both,
                        "inline-start" => match self.direction {
                            TextDirection::Ltr => ClearSide::Left,
                            TextDirection::Rtl => ClearSide::Right,
                        },
                        "inline-end" => match self.direction {
                            TextDirection::Ltr => ClearSide::Right,
                            TextDirection::Rtl => ClearSide::Left,
                        },
                        "none" => ClearSide::None,
                        _ => self.clear,
                    };
                }
                "z-index" => {
                    self.z_index = if value == "auto" {
                        None
                    } else {
                        value.parse::<i32>().ok()
                    };
                }
                "inset" => {
                    if let Some([top, right, bottom, left]) =
                        parse_box_length_shorthand_with_rem(&value, length_context)
                    {
                        self.inset_top = Some(top);
                        self.inset_right = Some(right);
                        self.inset_bottom = Some(bottom);
                        self.inset_left = Some(left);
                    }
                }
                "inset-block" => {
                    if let Some((start, end)) = parse_two_value_lengths(&value, length_context) {
                        self.inset_top = Some(start);
                        self.inset_bottom = Some(end);
                    }
                }
                "inset-inline" => {
                    if let Some((start, end)) = parse_two_value_lengths(&value, length_context) {
                        self.inset_left = Some(start);
                        self.inset_right = Some(end);
                    }
                }
                "top" | "inset-block-start" => {
                    self.inset_top = parse_css_length_with_rem(&value, length_context);
                }
                "right" | "inset-inline-end" => {
                    self.inset_right = parse_css_length_with_rem(&value, length_context);
                }
                "bottom" | "inset-block-end" => {
                    self.inset_bottom = parse_css_length_with_rem(&value, length_context);
                }
                "left" | "inset-inline-start" => {
                    self.inset_left = parse_css_length_with_rem(&value, length_context);
                }
                "opacity" => {
                    if let Some(opacity) = parse_alpha_channel(&value) {
                        self.opacity = (self.opacity * opacity).clamp(0.0, 1.0);
                    }
                }
                "font-size" => {
                    if let Some(value) = parse_pt_or_px_with_rem(&value, length_context) {
                        self.font_size = value;
                        self.line_height = self
                            .line_height_multiplier
                            .map(|multiplier| value * multiplier)
                            .unwrap_or(value * 1.2);
                        self.line_height_is_normal = self.line_height_multiplier.is_none();
                    }
                }
                "line-height" => {
                    if let Some(multiplier) = parse_unitless_line_height(&value) {
                        self.line_height_multiplier = Some(multiplier);
                        self.line_height = self.font_size * multiplier;
                        self.line_height_is_normal = false;
                    } else if let Some(value) = parse_pt_or_px_with_rem(&value, length_context) {
                        self.line_height_multiplier = None;
                        self.line_height = value;
                        self.line_height_is_normal = false;
                    }
                }
                "font" => {
                    self.apply_font_shorthand(&value, length_context);
                }
                "color" => {
                    if let Some(color) = parse_color_or_current(&value, self.color) {
                        self.color = color;
                    }
                }
                "background" | "background-image" => {
                    self.background_radials = parse_radial_gradients(&value, length_context);
                    if declaration.property == "background" {
                        let (size, position) = parse_background_shorthand_geometry(&value);
                        if let Some(size) = size {
                            self.background_size = size;
                        }
                        if let Some(background_clip) = parse_background_clip_from_shorthand(&value)
                        {
                            self.background_clip = background_clip;
                        }
                        if let Some((x, y)) = position {
                            self.background_position_x = x;
                            self.background_position_y = y;
                        }
                    }
                    if let Some(gradient) = parse_linear_gradient(&value) {
                        self.background_gradient = Some(gradient);
                        self.background = None;
                        self.background_image = None;
                    } else if let Some(url) = parse_css_url(&resolved_value) {
                        self.background_image = Some(url);
                        self.background_gradient = None;
                    } else if let Some(color) = parse_color_or_current(&value, self.color) {
                        self.background = Some(color);
                        self.background_gradient = None;
                        self.background_image = None;
                    }
                }
                "background-size" => match value.as_str() {
                    "cover" => self.background_size = BackgroundSize::Cover,
                    "contain" => self.background_size = BackgroundSize::Contain,
                    "auto" => self.background_size = BackgroundSize::Auto,
                    _ => {}
                },
                "background-clip" => {
                    if let Some(background_clip) = parse_background_clip(&value) {
                        self.background_clip = background_clip;
                    }
                }
                "background-position" => {
                    let (x, y) = parse_background_position(&value);
                    self.background_position_x = x;
                    self.background_position_y = y;
                }
                "background-color" => {
                    if let Some(color) = parse_color_or_current(&value, self.color) {
                        self.background = Some(color);
                    }
                }
                "box-shadow" => {
                    self.box_shadows = parse_box_shadows_with_rem(&value, length_context);
                }
                "text-shadow" => {
                    self.text_shadow = parse_text_shadow_with_rem(&value, length_context);
                }
                "filter" => {
                    if let Some(shadow) = parse_drop_shadow_filter_with_rem(&value, length_context)
                    {
                        self.box_shadows = vec![shadow];
                    }
                }
                "margin" => {
                    if let Some([top, right, bottom, left]) =
                        parse_margin_shorthand_with_rem(&value, length_context)
                    {
                        self.margin_top = top.value;
                        self.margin_right = right.value;
                        self.margin_bottom = bottom.value;
                        self.margin_left = left.value;
                        self.margin_right_auto = right.auto;
                        self.margin_left_auto = left.auto;
                    }
                }
                "margin-block" => {
                    if let Some((start, end)) = parse_two_value_axis(&value, length_context) {
                        self.margin_top = start;
                        self.margin_bottom = end;
                    }
                }
                "margin-inline" => {
                    if let Some((start, end)) = parse_two_value_margin_axis(&value, length_context)
                    {
                        self.margin_left = start.value;
                        self.margin_right = end.value;
                        self.margin_left_auto = start.auto;
                        self.margin_right_auto = end.auto;
                    }
                }
                "margin-top" => {
                    if let Some(value) = parse_pt_or_px_with_rem(&value, length_context) {
                        self.margin_top = value;
                    }
                }
                "margin-block-start" => {
                    if let Some(value) = parse_pt_or_px_with_rem(&value, length_context) {
                        self.margin_top = value;
                    }
                }
                "margin-bottom" => {
                    if let Some(value) = parse_pt_or_px_with_rem(&value, length_context) {
                        self.margin_bottom = value;
                    }
                }
                "margin-block-end" => {
                    if let Some(value) = parse_pt_or_px_with_rem(&value, length_context) {
                        self.margin_bottom = value;
                    }
                }
                "margin-left" => {
                    if let Some(value) = parse_margin_component(&value, length_context) {
                        self.margin_left = value.value;
                        self.margin_left_auto = value.auto;
                    }
                }
                "margin-inline-start" => {
                    if let Some(value) = parse_margin_component(&value, length_context) {
                        self.margin_left = value.value;
                        self.margin_left_auto = value.auto;
                    }
                }
                "margin-right" => {
                    if let Some(value) = parse_margin_component(&value, length_context) {
                        self.margin_right = value.value;
                        self.margin_right_auto = value.auto;
                    }
                }
                "margin-inline-end" => {
                    if let Some(value) = parse_margin_component(&value, length_context) {
                        self.margin_right = value.value;
                        self.margin_right_auto = value.auto;
                    }
                }
                "padding" => {
                    if let Some([top, right, bottom, left]) =
                        parse_box_shorthand_with_rem(&value, length_context)
                    {
                        self.padding_top = top;
                        self.padding_right = right;
                        self.padding_bottom = bottom;
                        self.padding_left = left;
                    }
                }
                "padding-block" => {
                    if let Some((start, end)) = parse_two_value_axis(&value, length_context) {
                        self.padding_top = start;
                        self.padding_bottom = end;
                    }
                }
                "padding-inline" => {
                    if let Some((start, end)) = parse_two_value_axis(&value, length_context) {
                        self.padding_left = start;
                        self.padding_right = end;
                    }
                }
                "padding-top" => {
                    if let Some(value) = parse_pt_or_px_with_rem(&value, length_context) {
                        self.padding_top = value;
                    }
                }
                "padding-block-start" => {
                    if let Some(value) = parse_pt_or_px_with_rem(&value, length_context) {
                        self.padding_top = value;
                    }
                }
                "padding-right" => {
                    if let Some(value) = parse_pt_or_px_with_rem(&value, length_context) {
                        self.padding_right = value;
                    }
                }
                "padding-inline-end" => {
                    if let Some(value) = parse_pt_or_px_with_rem(&value, length_context) {
                        self.padding_right = value;
                    }
                }
                "padding-bottom" => {
                    if let Some(value) = parse_pt_or_px_with_rem(&value, length_context) {
                        self.padding_bottom = value;
                    }
                }
                "padding-block-end" => {
                    if let Some(value) = parse_pt_or_px_with_rem(&value, length_context) {
                        self.padding_bottom = value;
                    }
                }
                "padding-left" => {
                    if let Some(value) = parse_pt_or_px_with_rem(&value, length_context) {
                        self.padding_left = value;
                    }
                }
                "padding-inline-start" => {
                    if let Some(value) = parse_pt_or_px_with_rem(&value, length_context) {
                        self.padding_left = value;
                    }
                }
                "border-radius" => {
                    if let Some(value) = parse_border_radius_with_rem(&value, length_context) {
                        self.border_radius = value;
                    }
                }
                "border-start-start-radius"
                | "border-start-end-radius"
                | "border-end-start-radius"
                | "border-end-end-radius"
                | "border-top-left-radius"
                | "border-top-right-radius"
                | "border-bottom-left-radius"
                | "border-bottom-right-radius"
                | "border-radius-top-left"
                | "border-radius-top-right"
                | "border-radius-bottom-left"
                | "border-radius-bottom-right" => {
                    if let Some(value) = parse_border_radius_with_rem(&value, length_context) {
                        self.border_radius = self.border_radius.max(value);
                    }
                }
                "border" => {
                    if let Some((width, color)) =
                        parse_border_shorthand_with_current(&value, length_context, self.color)
                    {
                        self.border_width = width;
                        self.border_color = color;
                        self.set_all_border_sides(width, color);
                    }
                    if let Some(border_style) = parse_border_line_style(&value) {
                        self.border_style = border_style;
                    }
                }
                "border-style" => {
                    if let Some(border_style) = parse_border_line_style(&value) {
                        self.border_style = border_style;
                    }
                    if self.border_style == BorderLineStyle::None {
                        self.set_all_border_sides(0.0, self.border_color);
                    }
                }
                "border-width" => {
                    if let Some([top, right, bottom, left]) =
                        parse_border_widths(&value, length_context)
                    {
                        self.border_width = top;
                        self.border_top_width = top;
                        self.border_right_width = right;
                        self.border_bottom_width = bottom;
                        self.border_left_width = left;
                    }
                }
                "border-top-width" | "border-block-start-width" => {
                    if let Some(value) = parse_border_width(&value, length_context) {
                        self.border_top_width = value;
                    }
                }
                "border-right-width" | "border-inline-end-width" => {
                    if let Some(value) = parse_border_width(&value, length_context) {
                        self.border_right_width = value;
                    }
                }
                "border-bottom-width" | "border-block-end-width" => {
                    if let Some(value) = parse_border_width(&value, length_context) {
                        self.border_bottom_width = value;
                    }
                }
                "border-left-width" | "border-inline-start-width" => {
                    if let Some(value) = parse_border_width(&value, length_context) {
                        self.border_left_width = value;
                    }
                }
                "border-block-width" => {
                    if let Some((start, end)) = parse_border_axis_widths(&value, length_context) {
                        self.border_top_width = start;
                        self.border_bottom_width = end;
                    }
                }
                "border-inline-width" => {
                    if let Some((start, end)) = parse_border_axis_widths(&value, length_context) {
                        self.border_left_width = start;
                        self.border_right_width = end;
                    }
                }
                "border-color" => {
                    if let Some([top, right, bottom, left]) =
                        parse_border_colors(&value, self.color)
                    {
                        self.border_color = Some(top);
                        self.border_top_color = Some(top);
                        self.border_right_color = Some(right);
                        self.border_bottom_color = Some(bottom);
                        self.border_left_color = Some(left);
                    }
                }
                "border-top-color" | "border-block-start-color" => {
                    if let Some(color) = parse_color_or_current(&value, self.color) {
                        self.border_top_color = Some(color);
                    }
                }
                "border-right-color" | "border-inline-end-color" => {
                    if let Some(color) = parse_color_or_current(&value, self.color) {
                        self.border_right_color = Some(color);
                    }
                }
                "border-bottom-color" | "border-block-end-color" => {
                    if let Some(color) = parse_color_or_current(&value, self.color) {
                        self.border_bottom_color = Some(color);
                    }
                }
                "border-left-color" | "border-inline-start-color" => {
                    if let Some(color) = parse_color_or_current(&value, self.color) {
                        self.border_left_color = Some(color);
                    }
                }
                "border-block-color" => {
                    if let Some((start, end)) = parse_two_value_colors(&value, self.color) {
                        self.border_top_color = Some(start);
                        self.border_bottom_color = Some(end);
                    }
                }
                "border-inline-color" => {
                    if let Some((start, end)) = parse_two_value_colors(&value, self.color) {
                        self.border_left_color = Some(start);
                        self.border_right_color = Some(end);
                    }
                }
                "border-top" => {
                    if let Some((width, color)) =
                        parse_border_shorthand_with_current(&value, length_context, self.color)
                    {
                        self.border_top_width = width;
                        self.border_top_color = color;
                    }
                }
                "border-block-start" => {
                    if let Some((width, color)) =
                        parse_border_shorthand_with_current(&value, length_context, self.color)
                    {
                        self.border_top_width = width;
                        self.border_top_color = color;
                    }
                }
                "border-right" => {
                    if let Some((width, color)) =
                        parse_border_shorthand_with_current(&value, length_context, self.color)
                    {
                        self.border_right_width = width;
                        self.border_right_color = color;
                    }
                }
                "border-inline-end" => {
                    if let Some((width, color)) =
                        parse_border_shorthand_with_current(&value, length_context, self.color)
                    {
                        self.border_right_width = width;
                        self.border_right_color = color;
                    }
                }
                "border-bottom" => {
                    if let Some((width, color)) =
                        parse_border_shorthand_with_current(&value, length_context, self.color)
                    {
                        self.border_bottom_width = width;
                        self.border_bottom_color = color;
                    }
                }
                "border-block-end" => {
                    if let Some((width, color)) =
                        parse_border_shorthand_with_current(&value, length_context, self.color)
                    {
                        self.border_bottom_width = width;
                        self.border_bottom_color = color;
                    }
                }
                "border-left" => {
                    if let Some((width, color)) =
                        parse_border_shorthand_with_current(&value, length_context, self.color)
                    {
                        self.border_left_width = width;
                        self.border_left_color = color;
                    }
                }
                "border-inline-start" => {
                    if let Some((width, color)) =
                        parse_border_shorthand_with_current(&value, length_context, self.color)
                    {
                        self.border_left_width = width;
                        self.border_left_color = color;
                    }
                }
                "border-block" => {
                    if let Some((width, color)) =
                        parse_border_shorthand_with_current(&value, length_context, self.color)
                    {
                        self.border_top_width = width;
                        self.border_top_color = color;
                        self.border_bottom_width = width;
                        self.border_bottom_color = color;
                    }
                }
                "border-inline" => {
                    if let Some((width, color)) =
                        parse_border_shorthand_with_current(&value, length_context, self.color)
                    {
                        self.border_left_width = width;
                        self.border_left_color = color;
                        self.border_right_width = width;
                        self.border_right_color = color;
                    }
                }
                "outline" => {
                    let mut width = None;
                    let mut line_style = None;
                    for part in split_css_whitespace(&value) {
                        if width.is_none() {
                            width = parse_outline_width(&part, length_context);
                        }
                        if line_style.is_none() {
                            line_style = parse_outline_line_style(&part);
                        }
                    }
                    self.outline_width = width.unwrap_or(DEFAULT_OUTLINE_WIDTH_PT);
                    self.outline_style = line_style.unwrap_or(BorderLineStyle::None);
                    self.outline_color = if split_css_whitespace(&value)
                        .iter()
                        .any(|part| part.eq_ignore_ascii_case("currentcolor"))
                    {
                        Some(self.color)
                    } else {
                        extract_css_color(&value)
                    };
                }
                "outline-width" => {
                    if let Some(value) = parse_outline_width(&value, length_context) {
                        self.outline_width = value;
                    }
                }
                "outline-color" => {
                    self.outline_color = parse_color_or_current(&value, self.color);
                }
                "outline-style" => {
                    if let Some(line_style) = parse_outline_line_style(&value) {
                        self.outline_style = line_style;
                    }
                }
                "outline-offset" => {
                    if let Some(value) = parse_pt_or_px_with_rem(&value, length_context) {
                        self.outline_offset = value;
                    }
                }
                "gap" => {
                    if let Some((row_gap, column_gap)) =
                        parse_gap_shorthand_with_rem(&value, length_context)
                    {
                        self.row_gap = row_gap;
                        self.column_gap = column_gap;
                        self.gap = column_gap;
                    }
                }
                "column-gap" => {
                    if let Some(value) = parse_pt_or_px_with_rem(&value, length_context) {
                        self.gap = value;
                        self.column_gap = value;
                    }
                }
                "row-gap" => {
                    if let Some(value) = parse_pt_or_px_with_rem(&value, length_context) {
                        self.row_gap = value;
                    }
                }
                "width" => {
                    self.width = parse_css_length_with_rem(&value, length_context);
                }
                "inline-size" => {
                    self.width = parse_css_length_with_rem(&value, length_context);
                }
                "min-width" => {
                    self.min_width = parse_css_length_with_rem(&value, length_context);
                }
                "min-inline-size" => {
                    self.min_width = parse_css_length_with_rem(&value, length_context);
                }
                "max-width" => {
                    self.max_width = parse_css_length_with_rem(&value, length_context);
                }
                "max-inline-size" => {
                    self.max_width = parse_css_length_with_rem(&value, length_context);
                }
                "height" => {
                    self.height = parse_css_length_with_rem(&value, length_context);
                }
                "block-size" => {
                    self.height = parse_css_length_with_rem(&value, length_context);
                }
                "min-height" => {
                    self.min_height = parse_css_length_with_rem(&value, length_context);
                }
                "min-block-size" => {
                    self.min_height = parse_css_length_with_rem(&value, length_context);
                }
                "max-height" => {
                    self.max_height = parse_css_length_with_rem(&value, length_context);
                }
                "max-block-size" => {
                    self.max_height = parse_css_length_with_rem(&value, length_context);
                }
                "aspect-ratio" => {
                    self.aspect_ratio = parse_aspect_ratio(&value);
                }
                "object-fit" => match value.as_str() {
                    "contain" => self.object_fit = ObjectFit::Contain,
                    "cover" => self.object_fit = ObjectFit::Cover,
                    "none" => self.object_fit = ObjectFit::None,
                    "scale-down" => self.object_fit = ObjectFit::ScaleDown,
                    "fill" => self.object_fit = ObjectFit::Fill,
                    _ => {}
                },
                "object-position" => {
                    let (x, y) = parse_background_position(&value);
                    self.object_position_x = x;
                    self.object_position_y = y;
                }
                "flex-direction" => match value.as_str() {
                    "column" | "column-reverse" => self.flex_direction = FlexDirection::Column,
                    "row" | "row-reverse" => self.flex_direction = FlexDirection::Row,
                    _ => {}
                },
                "flex-wrap" => {
                    if let Some(wrap) = parse_flex_wrap(&value) {
                        self.flex_wrap = wrap;
                    }
                }
                "flex-flow" => {
                    for part in split_css_whitespace(&value) {
                        match part.as_str() {
                            "column" | "column-reverse" => {
                                self.flex_direction = FlexDirection::Column
                            }
                            "row" | "row-reverse" => self.flex_direction = FlexDirection::Row,
                            _ => {
                                if let Some(wrap) = parse_flex_wrap(&part) {
                                    self.flex_wrap = wrap;
                                }
                            }
                        }
                    }
                }
                "flex-grow" => {
                    if let Some(grow) = parse_flex_factor(&value) {
                        self.flex_grow = grow;
                    }
                }
                "flex-shrink" => {
                    if let Some(shrink) = parse_flex_factor(&value) {
                        self.flex_shrink = shrink;
                    }
                }
                "flex-basis" => {
                    self.flex_basis = parse_flex_basis(&value, length_context);
                }
                "flex" => {
                    if let Some(flex) = parse_flex_shorthand(&value, length_context) {
                        self.flex_grow = flex.grow;
                        self.flex_shrink = flex.shrink;
                        self.flex_basis = flex.basis;
                    }
                }
                "order" => {
                    if let Ok(order) = value.parse::<i32>() {
                        self.order = order;
                    }
                }
                "justify-content" => {
                    self.justify_content =
                        parse_content_alignment_keyword(&value).unwrap_or(self.justify_content);
                }
                "align-content" => {
                    self.align_content =
                        parse_content_alignment_keyword(&value).unwrap_or(self.align_content);
                }
                "align-items" => {
                    self.align_items =
                        parse_box_alignment_keyword(&value).unwrap_or(self.align_items);
                }
                "align-self" => {
                    self.align_self = parse_optional_box_alignment_keyword(&value, self.align_self);
                }
                "justify-items" => {
                    self.justify_items =
                        parse_box_alignment_keyword(&value).unwrap_or(self.justify_items);
                }
                "justify-self" => {
                    self.justify_self =
                        parse_optional_box_alignment_keyword(&value, self.justify_self);
                }
                "place-items" => {
                    let values = split_css_whitespace(&value);
                    if let Some(align) = values.first() {
                        self.align_items =
                            parse_box_alignment_keyword(align).unwrap_or(self.align_items);
                    }
                    let justify = values.get(1).or_else(|| values.first());
                    if let Some(justify) = justify {
                        self.justify_items =
                            parse_box_alignment_keyword(justify).unwrap_or(self.justify_items);
                    }
                }
                "place-content" => {
                    let values = split_css_whitespace(&value);
                    if let Some(align) = values.first() {
                        self.align_content =
                            parse_content_alignment_keyword(align).unwrap_or(self.align_content);
                    }
                    let justify = values.get(1).or_else(|| values.first());
                    if let Some(justify) = justify {
                        self.justify_content = parse_content_alignment_keyword(justify)
                            .unwrap_or(self.justify_content);
                    }
                }
                "place-self" => {
                    let values = split_css_whitespace(&value);
                    if values.first().is_some_and(|value| value == "auto") {
                        self.align_self = None;
                    } else if let Some(align) = values.first() {
                        self.align_self = parse_box_alignment_keyword(align).or(self.align_self);
                    }
                    let justify = values.get(1).or_else(|| values.first());
                    if justify.is_some_and(|value| value == "auto") {
                        self.justify_self = None;
                    } else if let Some(justify) = justify {
                        self.justify_self =
                            parse_box_alignment_keyword(justify).or(self.justify_self);
                    }
                }
                "grid-template-columns" => {
                    let (columns, min_width, tracks) =
                        parse_grid_template_columns_with_context(&value, length_context);
                    self.grid_columns = columns;
                    self.grid_min_column_width = min_width;
                    self.grid_tracks = tracks;
                }
                "grid-template-rows" => {
                    let (_, _, tracks) =
                        parse_grid_template_columns_with_context(&value, length_context);
                    self.grid_row_tracks = tracks;
                }
                "grid-column" => {
                    if let Some(placement) = parse_grid_column_placement(&value) {
                        self.set_grid_column_placement(placement);
                    }
                }
                "grid-column-start" => self.set_grid_column_start_value(&value),
                "grid-column-end" => self.set_grid_column_end_value(&value),
                "grid-row" => {
                    if let Some(placement) = parse_grid_column_placement(&value) {
                        self.set_grid_row_placement(placement);
                    }
                }
                "grid-row-start" => self.set_grid_row_start_value(&value),
                "grid-row-end" => self.set_grid_row_end_value(&value),
                "font-weight" => {
                    self.font_weight = match value.as_str() {
                        "black" => FontWeight::Heavy,
                        "bold" | "bolder" => FontWeight::Bold,
                        "normal" | "lighter" => FontWeight::Normal,
                        _ if value.parse::<u16>().unwrap_or(400) >= 800 => FontWeight::Heavy,
                        _ if value.parse::<u16>().unwrap_or(400) >= 600 => FontWeight::Bold,
                        _ => FontWeight::Normal,
                    }
                }
                "font-style" => {
                    if let Some(font_style) = parse_font_style_token(&value) {
                        self.font_style = font_style;
                    }
                }
                "font-variant-numeric" => {
                    if let Some(numeric) = parse_font_variant_numeric(&value) {
                        self.font_variant_numeric = numeric;
                    }
                }
                "font-feature-settings" => {
                    if font_feature_settings_enable_tabular_numbers(&value) {
                        self.font_variant_numeric = FontVariantNumeric::TabularNums;
                    } else if value == "normal" {
                        self.font_variant_numeric = FontVariantNumeric::Normal;
                    }
                }
                "font-family" => {
                    self.font_face = parse_font_face(&value);
                }
                "letter-spacing" => {
                    if let Some(value) =
                        parse_letter_spacing_with_rem(&value, self.font_size, length_context)
                    {
                        self.letter_spacing = value;
                    }
                }
                "word-spacing" => {
                    if let Some(value) =
                        parse_word_spacing_with_rem(&value, self.font_size, length_context)
                    {
                        self.word_spacing = value;
                    }
                }
                "tab-size" => {
                    if let Ok(number) = value.parse::<f32>() {
                        if number.is_finite() && number >= 0.0 {
                            self.tab_size = TabSize::Spaces(number);
                        }
                    } else if !value.contains('%') {
                        if let Some(length) = parse_css_length_with_rem(&value, length_context) {
                            let points = length.resolve(0.0);
                            if points.is_finite() && points >= 0.0 {
                                self.tab_size = TabSize::Points(points);
                            }
                        }
                    }
                }
                "text-decoration" | "text-decoration-line" => {
                    if let Some(text_decoration) = parse_text_decoration_line(&value) {
                        self.text_decoration = text_decoration;
                    }
                    if declaration.property == "text-decoration" {
                        if let Some(color) = extract_css_color(&value) {
                            self.text_decoration_color = Some(color);
                        }
                        if let Some(thickness) =
                            parse_text_decoration_thickness(&value, self.font_size, length_context)
                        {
                            self.text_decoration_thickness = Some(thickness);
                        }
                    }
                }
                "text-decoration-color" => {
                    self.text_decoration_color = parse_color_or_current(&value, self.color);
                }
                "text-decoration-thickness" => {
                    self.text_decoration_thickness =
                        parse_text_decoration_thickness(&value, self.font_size, length_context);
                }
                "text-underline-offset" => {
                    self.text_underline_offset =
                        parse_text_underline_offset(&value, self.font_size, length_context);
                }
                "text-align" | "text-align-all" => {
                    if let Some(text_align) = parse_text_align_keyword(&value) {
                        self.text_align = text_align;
                    }
                }
                "text-align-last" => match value.as_str() {
                    "auto" => self.text_align_last = None,
                    _ => {
                        if let Some(text_align) = parse_text_align_keyword(&value) {
                            self.text_align_last = Some(text_align);
                        }
                    }
                },
                "direction" => match value.as_str() {
                    "rtl" => self.direction = TextDirection::Rtl,
                    "ltr" => self.direction = TextDirection::Ltr,
                    _ => {}
                },
                "vertical-align" => match value.as_str() {
                    "top" | "text-top" => self.vertical_align = VerticalAlign::Top,
                    "middle" => self.vertical_align = VerticalAlign::Middle,
                    "bottom" | "text-bottom" => self.vertical_align = VerticalAlign::Bottom,
                    "baseline" | "sub" | "super" => self.vertical_align = VerticalAlign::Baseline,
                    _ => {}
                },
                "text-transform" => match value.as_str() {
                    "uppercase" => self.text_transform = TextTransform::Uppercase,
                    "lowercase" => self.text_transform = TextTransform::Lowercase,
                    "capitalize" => self.text_transform = TextTransform::Capitalize,
                    "none" => self.text_transform = TextTransform::None,
                    _ => {}
                },
                "list-style" | "list-style-type" => {
                    if let Some(list_style_type) = parse_list_style_type(&value) {
                        self.list_style_type = list_style_type;
                    }
                }
                "transform" | "-webkit-transform" => {
                    let transform = parse_css_transform(&value, length_context);
                    self.transform_rotate_deg += transform.rotate_deg;
                    if let Some(translate_x) = transform.translate_x {
                        self.transform_translate_x = Some(add_transform_lengths(
                            self.transform_translate_x.take(),
                            translate_x,
                        ));
                    }
                    if let Some(translate_y) = transform.translate_y {
                        self.transform_translate_y = Some(add_transform_lengths(
                            self.transform_translate_y.take(),
                            translate_y,
                        ));
                    }
                    self.transform_scale_x *= transform.scale_x;
                    self.transform_scale_y *= transform.scale_y;
                }
                "translate" => {
                    if let Some((x, y)) = parse_translate_property(&value, length_context) {
                        self.transform_translate_x = Some(x);
                        self.transform_translate_y = Some(y);
                    }
                }
                "scale" => {
                    if let Some((x, y)) = parse_scale_property(&value) {
                        self.transform_scale_x = x;
                        self.transform_scale_y = y;
                    }
                }
                "rotate" => {
                    self.transform_rotate_deg = parse_rotate_value(&value).unwrap_or(0.0);
                }
                "transform-origin" => {
                    if let Some((x, y)) = parse_transform_origin(&value, length_context) {
                        self.transform_origin_x = x;
                        self.transform_origin_y = y;
                    }
                }
                "white-space" | "text-wrap-mode" => match value.as_str() {
                    "nowrap" => self.white_space = WhiteSpace::NoWrap,
                    "pre-line" => self.white_space = WhiteSpace::PreLine,
                    "normal" | "wrap" => self.white_space = WhiteSpace::Normal,
                    _ => {}
                },
                "text-wrap" => match value.as_str() {
                    "nowrap" => self.white_space = WhiteSpace::NoWrap,
                    "normal" | "wrap" | "stable" | "auto" => {
                        self.white_space = WhiteSpace::Normal;
                        self.text_wrap_style = TextWrapStyle::Auto;
                    }
                    "balance" => {
                        self.white_space = WhiteSpace::Normal;
                        self.text_wrap_style = TextWrapStyle::Balance;
                    }
                    "pretty" => {
                        self.white_space = WhiteSpace::Normal;
                        self.text_wrap_style = TextWrapStyle::Pretty;
                    }
                    _ => {}
                },
                "text-wrap-style" => match value.as_str() {
                    "balance" => self.text_wrap_style = TextWrapStyle::Balance,
                    "pretty" => self.text_wrap_style = TextWrapStyle::Pretty,
                    "auto" | "stable" => self.text_wrap_style = TextWrapStyle::Auto,
                    _ => {}
                },
                "text-overflow" => {
                    self.text_overflow = if value
                        .split_whitespace()
                        .any(|part| part.trim_matches('"') == "ellipsis")
                    {
                        TextOverflow::Ellipsis
                    } else if value == "clip" {
                        TextOverflow::Clip
                    } else {
                        self.text_overflow
                    };
                }
                "overflow" | "overflow-x" | "overflow-y" => match value.as_str() {
                    "hidden" | "clip" => self.overflow_hidden = true,
                    "visible" | "auto" | "scroll" => self.overflow_hidden = false,
                    _ => {}
                },
                "overflow-wrap" | "word-wrap" => match value.as_str() {
                    "anywhere" | "break-word" => self.overflow_wrap = OverflowWrap::BreakWord,
                    "normal" => self.overflow_wrap = OverflowWrap::Normal,
                    _ => {}
                },
                "word-break" => match value.as_str() {
                    "break-all" | "break-word" => self.overflow_wrap = OverflowWrap::BreakWord,
                    "normal" | "keep-all" => self.overflow_wrap = OverflowWrap::Normal,
                    _ => {}
                },
                "page-break-inside" | "break-inside" => {
                    if value.contains("avoid") {
                        self.page_break_inside_avoid = true;
                    }
                }
                "page-break-before" | "break-before" => {
                    if matches!(value.as_str(), "always" | "page") {
                        self.page_break_before = true;
                    }
                }
                "page-break-after" | "break-after" => {
                    if matches!(value.as_str(), "always" | "page") {
                        self.page_break_after = true;
                    } else if value.contains("avoid") {
                        self.page_break_after_avoid = true;
                    }
                }
                "border-collapse" => match value.as_str() {
                    "collapse" => self.border_collapse = BorderCollapse::Collapse,
                    "separate" => self.border_collapse = BorderCollapse::Separate,
                    _ => {}
                },
                "border-spacing" => {
                    if let Some((horizontal, vertical)) =
                        parse_border_spacing(&value, length_context)
                    {
                        self.border_spacing_horizontal = horizontal;
                        self.border_spacing_vertical = vertical;
                    }
                }
                _ => {}
            }
        }
    }

    fn prime_font_relative_context(
        &mut self,
        declarations: &[Declaration],
        length_context: LengthContext,
    ) {
        for declaration in declarations {
            if declaration.property.starts_with("--")
                || !is_supported_property(&declaration.property)
            {
                continue;
            }
            let resolved_value = resolve_css_value(&declaration.value, &self.custom_properties);
            let value = resolved_value.to_ascii_lowercase();
            let length_context = length_context
                .with_em_base(self.font_size)
                .with_lh_base(self.line_height);
            match declaration.property.as_str() {
                "font-size" => {
                    if let Some(value) = parse_pt_or_px_with_rem(&value, length_context) {
                        self.font_size = value;
                        self.line_height = self
                            .line_height_multiplier
                            .map(|multiplier| value * multiplier)
                            .unwrap_or(value * 1.2);
                        self.line_height_is_normal = self.line_height_multiplier.is_none();
                    }
                }
                "line-height" => {
                    if let Some(multiplier) = parse_unitless_line_height(&value) {
                        self.line_height_multiplier = Some(multiplier);
                        self.line_height = self.font_size * multiplier;
                        self.line_height_is_normal = false;
                    } else if let Some(value) = parse_pt_or_px_with_rem(&value, length_context) {
                        self.line_height_multiplier = None;
                        self.line_height = value;
                        self.line_height_is_normal = false;
                    }
                }
                "font" => {
                    self.apply_font_shorthand(&value, length_context);
                }
                _ => {}
            }
        }
    }

    fn reset_all_except_custom_properties(&mut self) {
        let custom_properties = std::mem::take(&mut self.custom_properties);
        *self = Self::default();
        self.custom_properties = custom_properties;
    }

    fn set_all_border_sides(&mut self, width: f32, color: Option<Color>) {
        self.border_top_width = width;
        self.border_right_width = width;
        self.border_bottom_width = width;
        self.border_left_width = width;
        self.border_top_color = color;
        self.border_right_color = color;
        self.border_bottom_color = color;
        self.border_left_color = color;
    }

    fn apply_font_shorthand(
        &mut self,
        value: &str,
        length_context: impl Into<LengthContext> + Copy,
    ) {
        let length_context = length_context.into();
        let value = value.trim();
        if value.is_empty() || matches!(value, "inherit" | "initial" | "unset" | "revert") {
            return;
        }

        let parts = split_css_whitespace(value);
        let mut size_index = None;
        let mut parsed_size = None;
        let mut parsed_line_height = None;
        let mut parsed_multiplier = None;
        let mut parsed_weight = None;
        let mut parsed_style = None;

        for (idx, part) in parts.iter().enumerate() {
            if parsed_weight.is_none() {
                parsed_weight = parse_font_weight_token(part);
            }
            if parsed_style.is_none() {
                parsed_style = parse_font_style_token(part);
            }

            let (size_part, line_height_part) = part
                .split_once('/')
                .map(|(size, line_height)| (size.trim(), Some(line_height.trim())))
                .unwrap_or((part.trim(), None));

            if let Some(size) = parse_font_size_token(size_part, length_context) {
                size_index = Some(idx);
                parsed_size = Some(size);
                if let Some(line_height) = line_height_part {
                    if let Some(multiplier) = parse_unitless_line_height(line_height) {
                        parsed_multiplier = Some(multiplier);
                        parsed_line_height = Some(size * multiplier);
                    } else if let Some(length) =
                        parse_pt_or_px_with_rem(line_height, length_context)
                    {
                        parsed_multiplier = None;
                        parsed_line_height = Some(length);
                    }
                }
                break;
            }
        }

        if let Some(weight) = parsed_weight {
            self.font_weight = weight;
        }
        if let Some(font_style) = parsed_style {
            self.font_style = font_style;
        }

        let Some(size_index) = size_index else {
            return;
        };
        let Some(font_size) = parsed_size else {
            return;
        };
        self.font_size = font_size;
        if let Some(line_height) = parsed_line_height {
            self.line_height = line_height;
            self.line_height_multiplier = parsed_multiplier;
            self.line_height_is_normal = false;
        } else {
            self.line_height = font_size * 1.2;
            self.line_height_multiplier = None;
            self.line_height_is_normal = true;
        }

        if parts.len() > size_index + 1 {
            let family = parts[size_index + 1..].join(" ");
            self.font_face = parse_font_face(&family);
        }
    }

    fn set_grid_column_placement(&mut self, placement: GridColumnPlacement) {
        self.grid_column_start = placement.start;
        self.grid_column_end = placement.end;
        self.grid_column_span = placement.span;
    }

    fn set_grid_column_start_value(&mut self, value: &str) {
        if let Some(start) = parse_grid_positive_line(value) {
            self.grid_column_start = Some(start);
            self.update_grid_column_span_from_lines();
        } else if let Some(span) = parse_grid_line_span(value) {
            self.grid_column_start = None;
            self.grid_column_span = span;
        }
    }

    fn set_grid_column_end_value(&mut self, value: &str) {
        if let Some(span) = parse_grid_line_span(value) {
            self.grid_column_end = None;
            self.grid_column_span = span;
        } else if let Some(end) = parse_grid_positive_line(value) {
            self.grid_column_end = Some(end);
            self.update_grid_column_span_from_lines();
        }
    }

    fn update_grid_column_span_from_lines(&mut self) {
        if let (Some(start), Some(end)) = (self.grid_column_start, self.grid_column_end) {
            if end > start {
                self.grid_column_span = end - start;
            }
        }
    }

    fn set_grid_row_placement(&mut self, placement: GridColumnPlacement) {
        self.grid_row_start = placement.start;
        self.grid_row_end = placement.end;
        self.grid_row_span = placement.span;
    }

    fn set_grid_row_start_value(&mut self, value: &str) {
        if let Some(start) = parse_grid_positive_line(value) {
            self.grid_row_start = Some(start);
            self.update_grid_row_span_from_lines();
        } else if let Some(span) = parse_grid_line_span(value) {
            self.grid_row_start = None;
            self.grid_row_span = span;
        }
    }

    fn set_grid_row_end_value(&mut self, value: &str) {
        if let Some(span) = parse_grid_line_span(value) {
            self.grid_row_end = None;
            self.grid_row_span = span;
        } else if let Some(end) = parse_grid_positive_line(value) {
            self.grid_row_end = Some(end);
            self.update_grid_row_span_from_lines();
        }
    }

    fn update_grid_row_span_from_lines(&mut self) {
        if let (Some(start), Some(end)) = (self.grid_row_start, self.grid_row_end) {
            if end > start {
                self.grid_row_span = end - start;
            }
        }
    }
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self {
            display: Display::Block,
            visibility: Visibility::Visible,
            content: None,
            font_size: 12.0,
            line_height: 14.4,
            line_height_multiplier: None,
            line_height_is_normal: true,
            color: Color::BLACK,
            background: None,
            background_gradient: None,
            background_radials: Vec::new(),
            background_image: None,
            background_size: BackgroundSize::Auto,
            background_clip: BackgroundClip::BorderBox,
            background_position_x: 0.5,
            background_position_y: 0.5,
            box_shadows: Vec::new(),
            text_shadow: None,
            margin_top: 0.0,
            margin_right: 0.0,
            margin_bottom: 0.0,
            margin_left: 0.0,
            margin_right_auto: false,
            margin_left_auto: false,
            padding_top: 0.0,
            padding_right: 0.0,
            padding_bottom: 0.0,
            padding_left: 0.0,
            border_radius: 0.0,
            border_width: 0.0,
            border_color: None,
            border_style: BorderLineStyle::Solid,
            border_top_width: 0.0,
            border_right_width: 0.0,
            border_bottom_width: 0.0,
            border_left_width: 0.0,
            border_top_color: None,
            border_right_color: None,
            border_bottom_color: None,
            border_left_color: None,
            outline_width: DEFAULT_OUTLINE_WIDTH_PT,
            outline_color: None,
            outline_style: BorderLineStyle::None,
            outline_offset: 0.0,
            opacity: 1.0,
            gap: 0.0,
            row_gap: 0.0,
            column_gap: 0.0,
            width: None,
            min_width: None,
            max_width: None,
            height: None,
            min_height: None,
            max_height: None,
            aspect_ratio: None,
            position: Position::Static,
            float: FloatSide::None,
            clear: ClearSide::None,
            z_index: None,
            inset_top: None,
            inset_right: None,
            inset_bottom: None,
            inset_left: None,
            flex_direction: FlexDirection::Row,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: None,
            flex_wrap: FlexWrap::NoWrap,
            justify_content: JustifyContent::Start,
            align_content: JustifyContent::Start,
            align_items: AlignItems::Stretch,
            align_self: None,
            justify_items: AlignItems::Stretch,
            justify_self: None,
            order: 0,
            grid_columns: None,
            grid_min_column_width: None,
            grid_tracks: Vec::new(),
            grid_row_tracks: Vec::new(),
            grid_column_start: None,
            grid_column_end: None,
            grid_column_span: 1,
            grid_row_start: None,
            grid_row_end: None,
            grid_row_span: 1,
            font_weight: FontWeight::Normal,
            font_style: FontStyle::Normal,
            font_variant_numeric: FontVariantNumeric::Normal,
            font_face: FontFace::Sans,
            letter_spacing: 0.0,
            word_spacing: 0.0,
            tab_size: TabSize::Spaces(8.0),
            text_decoration: TextDecoration::none(),
            text_decoration_color: None,
            text_decoration_thickness: None,
            text_underline_offset: None,
            text_align: TextAlign::Left,
            text_align_last: None,
            direction: TextDirection::Ltr,
            vertical_align: VerticalAlign::Baseline,
            text_transform: TextTransform::None,
            list_style_type: ListStyleType::Disc,
            transform_rotate_deg: 0.0,
            transform_origin_x: CssLength::percent(0.5),
            transform_origin_y: CssLength::percent(0.5),
            transform_translate_x: None,
            transform_translate_y: None,
            transform_scale_x: 1.0,
            transform_scale_y: 1.0,
            white_space: WhiteSpace::Normal,
            text_wrap_style: TextWrapStyle::Auto,
            text_overflow: TextOverflow::Clip,
            overflow_wrap: OverflowWrap::BreakWord,
            object_fit: ObjectFit::Fill,
            object_position_x: 0.5,
            object_position_y: 0.5,
            overflow_hidden: false,
            page_break_inside_avoid: false,
            page_break_before: false,
            page_break_after: false,
            page_break_after_avoid: false,
            border_collapse: BorderCollapse::Separate,
            border_spacing_horizontal: 0.0,
            border_spacing_vertical: 0.0,
            box_sizing: BoxSizing::ContentBox,
            custom_properties: BTreeMap::new(),
        }
    }
}

fn is_supported_property(property: &str) -> bool {
    matches!(
        property,
        "content"
            | "all"
            | "display"
            | "box-sizing"
            | "visibility"
            | "content-visibility"
            | "clip"
            | "position"
            | "float"
            | "clear"
            | "z-index"
            | "isolation"
            | "inset"
            | "inset-block"
            | "inset-inline"
            | "top"
            | "inset-block-start"
            | "right"
            | "inset-inline-end"
            | "bottom"
            | "inset-block-end"
            | "left"
            | "inset-inline-start"
            | "opacity"
            | "font"
            | "font-size"
            | "line-height"
            | "font-feature-settings"
            | "font-variation-settings"
            | "color"
            | "background"
            | "background-image"
            | "background-attachment"
            | "background-size"
            | "background-clip"
            | "background-position"
            | "background-color"
            | "box-shadow"
            | "text-shadow"
            | "filter"
            | "backdrop-filter"
            | "-webkit-backdrop-filter"
            | "cursor"
            | "touch-action"
            | "user-select"
            | "-webkit-user-select"
            | "pointer-events"
            | "transition"
            | "-webkit-transition"
            | "transition-property"
            | "transition-duration"
            | "transition-delay"
            | "transition-timing-function"
            | "animation"
            | "-webkit-animation"
            | "animation-name"
            | "animation-duration"
            | "animation-delay"
            | "animation-timing-function"
            | "animation-iteration-count"
            | "animation-direction"
            | "animation-fill-mode"
            | "animation-play-state"
            | "will-change"
            | "appearance"
            | "-webkit-appearance"
            | "-moz-appearance"
            | "-webkit-font-smoothing"
            | "-moz-osx-font-smoothing"
            | "-webkit-text-size-adjust"
            | "-webkit-tap-highlight-color"
            | "hyphens"
            | "-webkit-hyphens"
            | "-moz-hyphens"
            | "-moz-orient"
            | "text-rendering"
            | "font-kerning"
            | "font-optical-sizing"
            | "font-synthesis"
            | "color-scheme"
            | "print-color-adjust"
            | "-webkit-print-color-adjust"
            | "backface-visibility"
            | "contain"
            | "container-type"
            | "container-name"
            | "overscroll-behavior"
            | "scroll-behavior"
            | "scroll-margin"
            | "scroll-padding"
            | "accent-color"
            | "caret-color"
            | "resize"
            | "margin"
            | "margin-block"
            | "margin-inline"
            | "margin-top"
            | "margin-block-start"
            | "margin-bottom"
            | "margin-block-end"
            | "margin-left"
            | "margin-inline-start"
            | "margin-right"
            | "margin-inline-end"
            | "padding"
            | "padding-block"
            | "padding-inline"
            | "padding-top"
            | "padding-block-start"
            | "padding-right"
            | "padding-inline-end"
            | "padding-bottom"
            | "padding-block-end"
            | "padding-left"
            | "padding-inline-start"
            | "border-radius"
            | "border-start-start-radius"
            | "border-start-end-radius"
            | "border-end-start-radius"
            | "border-end-end-radius"
            | "border-top-left-radius"
            | "border-top-right-radius"
            | "border-bottom-left-radius"
            | "border-bottom-right-radius"
            | "border-radius-top-left"
            | "border-radius-top-right"
            | "border-radius-bottom-left"
            | "border-radius-bottom-right"
            | "border"
            | "border-style"
            | "border-width"
            | "border-top-width"
            | "border-block-start-width"
            | "border-right-width"
            | "border-inline-end-width"
            | "border-bottom-width"
            | "border-block-end-width"
            | "border-left-width"
            | "border-inline-start-width"
            | "border-block-width"
            | "border-inline-width"
            | "border-color"
            | "border-top-color"
            | "border-block-start-color"
            | "border-right-color"
            | "border-inline-end-color"
            | "border-bottom-color"
            | "border-block-end-color"
            | "border-left-color"
            | "border-inline-start-color"
            | "border-block-color"
            | "border-inline-color"
            | "border-top"
            | "border-block-start"
            | "border-right"
            | "border-inline-end"
            | "border-bottom"
            | "border-block-end"
            | "border-left"
            | "border-inline-start"
            | "border-block"
            | "border-inline"
            | "outline"
            | "outline-width"
            | "outline-color"
            | "outline-style"
            | "outline-offset"
            | "gap"
            | "column-gap"
            | "row-gap"
            | "width"
            | "inline-size"
            | "min-width"
            | "min-inline-size"
            | "max-width"
            | "max-inline-size"
            | "height"
            | "block-size"
            | "min-height"
            | "min-block-size"
            | "max-height"
            | "max-block-size"
            | "aspect-ratio"
            | "object-fit"
            | "object-position"
            | "flex-direction"
            | "flex-wrap"
            | "flex-flow"
            | "flex-grow"
            | "flex-shrink"
            | "flex-basis"
            | "flex"
            | "order"
            | "justify-content"
            | "align-content"
            | "justify-items"
            | "justify-self"
            | "align-items"
            | "align-self"
            | "place-content"
            | "place-items"
            | "place-self"
            | "grid-template-columns"
            | "grid-template-rows"
            | "grid-column"
            | "grid-column-start"
            | "grid-column-end"
            | "grid-row"
            | "grid-row-start"
            | "grid-row-end"
            | "font-weight"
            | "font-style"
            | "font-variant-numeric"
            | "font-family"
            | "tab-size"
            | "letter-spacing"
            | "word-spacing"
            | "text-decoration"
            | "text-decoration-line"
            | "text-decoration-color"
            | "text-decoration-thickness"
            | "text-underline-offset"
            | "text-align"
            | "text-align-all"
            | "text-align-last"
            | "direction"
            | "vertical-align"
            | "text-transform"
            | "list-style"
            | "list-style-type"
            | "transform"
            | "-webkit-transform"
            | "transform-origin"
            | "translate"
            | "scale"
            | "rotate"
            | "white-space"
            | "text-wrap"
            | "text-wrap-mode"
            | "text-wrap-style"
            | "white-space-collapse"
            | "text-overflow"
            | "overflow"
            | "overflow-x"
            | "overflow-y"
            | "overflow-wrap"
            | "word-wrap"
            | "word-break"
            | "page-break-inside"
            | "break-inside"
            | "page-break-before"
            | "break-before"
            | "page-break-after"
            | "break-after"
            | "border-collapse"
            | "border-spacing"
    )
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinearGradient {
    pub start: Color,
    pub end: Color,
    pub angle_deg: f32,
    pub stops: Vec<GradientStop>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradientStop {
    pub color: Color,
    pub position: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RadialGradient {
    pub color: Color,
    pub center_x: f32,
    pub center_y: f32,
    pub radius: CssLength,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxShadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct LengthContext {
    rem_base_pt: f32,
    em_base_pt: f32,
    lh_base_pt: f32,
    rlh_base_pt: f32,
    viewport_width_pt: f32,
    viewport_height_pt: f32,
}

impl LengthContext {
    fn for_page(rem_base_pt: f32, page: PageOptions) -> Self {
        let root_line_height_pt = rem_base_pt * 1.2;
        Self {
            rem_base_pt,
            em_base_pt: rem_base_pt,
            lh_base_pt: root_line_height_pt,
            rlh_base_pt: root_line_height_pt,
            viewport_width_pt: page.width_pt,
            viewport_height_pt: page.height_pt,
        }
    }

    fn with_em_base(mut self, em_base_pt: f32) -> Self {
        self.em_base_pt = em_base_pt;
        self
    }

    fn with_lh_base(mut self, lh_base_pt: f32) -> Self {
        self.lh_base_pt = lh_base_pt;
        self
    }
}

impl From<f32> for LengthContext {
    fn from(rem_base_pt: f32) -> Self {
        Self::for_page(rem_base_pt, PageOptions::letter())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssLength {
    Linear {
        percent: f32,
        points: f32,
    },
    Min(Vec<CssLength>),
    Max(Vec<CssLength>),
    Clamp {
        min: Box<CssLength>,
        preferred: Box<CssLength>,
        max: Box<CssLength>,
    },
}

impl CssLength {
    pub fn resolve(&self, base: f32) -> f32 {
        match self {
            Self::Linear { percent, points } => base * percent + points,
            Self::Min(values) => values
                .iter()
                .map(|value| value.resolve(base))
                .fold(f32::INFINITY, f32::min),
            Self::Max(values) => values
                .iter()
                .map(|value| value.resolve(base))
                .fold(f32::NEG_INFINITY, f32::max),
            Self::Clamp {
                min,
                preferred,
                max,
            } => preferred
                .resolve(base)
                .clamp(min.resolve(base), max.resolve(base)),
        }
    }

    fn points(points: f32) -> Self {
        Self::Linear {
            percent: 0.0,
            points,
        }
    }

    fn percent(percent: f32) -> Self {
        Self::Linear {
            percent,
            points: 0.0,
        }
    }

    fn combine_linear(self, other: Self, sign: f32) -> Option<Self> {
        match (self, other) {
            (
                Self::Linear { percent, points },
                Self::Linear {
                    percent: other_percent,
                    points: other_points,
                },
            ) => Some(Self::Linear {
                percent: percent + sign * other_percent,
                points: points + sign * other_points,
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    InlineBlock,
    None,
    Block,
    Inline,
    Contents,
    Flex,
    InlineFlex,
    Grid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Visible,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    Static,
    Relative,
    Absolute,
    Fixed,
    Sticky,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatSide {
    None,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClearSide {
    None,
    Left,
    Right,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    Normal,
    Bold,
    Heavy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontVariantNumeric {
    Normal,
    TabularNums,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFace {
    Sans,
    Serif,
    Mono,
    Lato,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextDecoration {
    pub underline: bool,
    pub line_through: bool,
}

impl TextDecoration {
    pub const fn none() -> Self {
        Self {
            underline: false,
            line_through: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
    Start,
    End,
    Justify,
}

fn parse_text_align_keyword(value: &str) -> Option<TextAlign> {
    match value {
        "left" => Some(TextAlign::Left),
        "center" => Some(TextAlign::Center),
        "right" => Some(TextAlign::Right),
        "start" | "match-parent" => Some(TextAlign::Start),
        "end" => Some(TextAlign::End),
        "justify" | "justify-all" => Some(TextAlign::Justify),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextDirection {
    Ltr,
    Rtl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAlign {
    Baseline,
    Top,
    Middle,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TabSize {
    Spaces(f32),
    Points(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextTransform {
    None,
    Uppercase,
    Lowercase,
    Capitalize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListStyleType {
    Disc,
    Decimal,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhiteSpace {
    Normal,
    NoWrap,
    PreLine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextWrapStyle {
    Auto,
    Balance,
    Pretty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextOverflow {
    Clip,
    Ellipsis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowWrap {
    Normal,
    BreakWord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundSize {
    Auto,
    Cover,
    Contain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundClip {
    BorderBox,
    PaddingBox,
    ContentBox,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectFit {
    Fill,
    Contain,
    Cover,
    None,
    ScaleDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexDirection {
    Row,
    Column,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexWrap {
    NoWrap,
    Wrap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JustifyContent {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignItems {
    Stretch,
    Start,
    Center,
    End,
    Baseline,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GridTrack {
    Fr { factor: f32, min: Option<CssLength> },
    Length(CssLength),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderCollapse {
    Separate,
    Collapse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderLineStyle {
    Solid,
    Dashed,
    Dotted,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxSizing {
    ContentBox,
    BorderBox,
}

#[derive(Debug, Clone)]
pub struct Stylesheet {
    rules: Vec<Rule>,
    rule_index: RuleIndex,
    layer_count: usize,
    variables: BTreeMap<String, String>,
    referenced_variables: BTreeSet<String>,
    page: PageStyle,
    rem_base_pt: f32,
    viewport_width_pt: f32,
    viewport_height_pt: f32,
}

impl Stylesheet {
    pub fn from_document(document: &Document) -> Self {
        Self::from_css_chunks(document.styles())
    }

    pub fn from_css_chunks(styles: Vec<String>) -> Self {
        let styles = styles
            .into_iter()
            .map(|style| strip_css_comments(&style))
            .collect::<Vec<_>>();
        let mut rules = Vec::new();
        let mut variables = BTreeMap::new();
        let mut page = PageStyle::default();
        let mut rem_base_pt = 12.0;
        for style in &styles {
            page.merge(parse_page_style(&style));
        }
        let length_page = page.apply(PageOptions::letter());
        let media = MediaContext::for_page(length_page);
        for style in styles {
            variables.extend(parse_registered_custom_property_initials(&style, media));
            variables.extend(parse_custom_properties(&style, media));
            let parsed_rules = parse_rules(&style, media);
            for rule in &parsed_rules {
                if rule.selector.is_html_root() {
                    for declaration in &rule.declarations {
                        if declaration.property == "font-size" {
                            if let Some(value) =
                                parse_pt_or_px(&declaration.value.to_ascii_lowercase())
                            {
                                rem_base_pt = value;
                            }
                        }
                    }
                }
            }
            rules.extend(parsed_rules);
        }
        let referenced_variables = collect_referenced_variables(&rules, &variables);
        variables.retain(|name, _| referenced_variables.contains(name));
        let rule_index = RuleIndex::build(&rules);
        let layer_count = rules
            .iter()
            .filter_map(|rule| rule.layer_index)
            .max()
            .map(|idx| idx + 1)
            .unwrap_or(0);
        Self {
            rules,
            rule_index,
            layer_count,
            variables,
            referenced_variables,
            page,
            rem_base_pt,
            viewport_width_pt: length_page.width_pt,
            viewport_height_pt: length_page.height_pt,
        }
    }

    pub fn page_options(&self, default: PageOptions) -> PageOptions {
        self.page.apply(default)
    }

    fn length_context(&self) -> LengthContext {
        LengthContext {
            rem_base_pt: self.rem_base_pt,
            em_base_pt: self.rem_base_pt,
            lh_base_pt: self.rem_base_pt * 1.2,
            rlh_base_pt: self.rem_base_pt * 1.2,
            viewport_width_pt: self.viewport_width_pt,
            viewport_height_pt: self.viewport_height_pt,
        }
    }

    fn candidate_rule_indexes(&self, element: &ElementNode) -> Vec<usize> {
        self.rule_index.candidates(element)
    }

    pub(crate) fn pseudo_style_for_node(
        &self,
        document: &Document,
        id: NodeId,
        kind: PseudoElement,
        parent_style: &ComputedStyle,
    ) -> Option<ComputedStyle> {
        let Some(Node::Element(element)) = document.node(id) else {
            return None;
        };

        let matching_rules = self
            .candidate_rule_indexes(element)
            .into_iter()
            .filter_map(|source_order| {
                let rule = self.rules.get(source_order)?;
                (rule.pseudo == Some(kind)
                    && rule.selector.could_match_element(element)
                    && rule.selector.matches(document, id, element))
                .then_some((source_order, rule))
            })
            .collect::<Vec<_>>();
        if matching_rules.is_empty() {
            return None;
        }
        let declarations = ordered_rule_declarations(matching_rules, self.layer_count);

        let mut style = ComputedStyle::inherited_from(parent_style);
        style.display = Display::Inline;
        style.apply_custom_properties(&declarations, &self.referenced_variables);
        style.apply_declarations(&declarations, self.length_context());
        style.content.as_ref()?;
        Some(style)
    }
}

#[derive(Debug, Clone, Default)]
struct RuleIndex {
    universal: Vec<usize>,
    by_key: BTreeMap<String, Vec<usize>>,
}

impl RuleIndex {
    fn build(rules: &[Rule]) -> Self {
        let mut index = Self::default();
        for (rule_index, rule) in rules.iter().enumerate() {
            if let Some(key) = rule.selector.index_key() {
                index.by_key.entry(key).or_default().push(rule_index);
            } else {
                index.universal.push(rule_index);
            }
        }
        index
    }

    fn candidates(&self, element: &ElementNode) -> Vec<usize> {
        let mut candidates = self.universal.clone();
        if let Some(indexes) = self.by_key.get(&format!("tag:{}", element.tag)) {
            candidates.extend(indexes.iter().copied());
        }
        if let Some(id) = element.attr("id") {
            if let Some(indexes) = self.by_key.get(&format!("id:{id}")) {
                candidates.extend(indexes.iter().copied());
            }
        }
        for class in element.classes() {
            if let Some(indexes) = self.by_key.get(&format!("class:{class}")) {
                candidates.extend(indexes.iter().copied());
            }
        }
        candidates.sort_unstable();
        candidates.dedup();
        candidates
    }
}

fn collect_referenced_variables(
    rules: &[Rule],
    root_variables: &BTreeMap<String, String>,
) -> BTreeSet<String> {
    let mut referenced = BTreeSet::new();
    let mut definitions = root_variables.clone();
    for rule in rules {
        for declaration in &rule.declarations {
            if declaration.property.starts_with("--") {
                definitions.insert(declaration.property.clone(), declaration.value.clone());
            } else if is_supported_property(&declaration.property) {
                collect_var_references(&declaration.value, &mut referenced);
            }
        }
    }

    let mut changed = true;
    while changed {
        changed = false;
        let current = referenced.iter().cloned().collect::<Vec<_>>();
        for name in current {
            if let Some(value) = definitions.get(&name) {
                let before = referenced.len();
                collect_var_references(value, &mut referenced);
                changed |= referenced.len() != before;
            }
        }
    }
    referenced
}

fn collect_var_references(value: &str, out: &mut BTreeSet<String>) {
    let mut rest = value;
    while let Some((_, end, body)) = find_var_function(rest) {
        let parts = split_top_level(body, &[',']);
        if let Some(name) = parts.first().map(|part| part.trim()) {
            if name.starts_with("--") {
                out.insert(name.to_string());
            }
        }
        for fallback_part in parts.iter().skip(1) {
            collect_var_references(fallback_part, out);
        }
        rest = &rest[end + 1..];
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct PageStyle {
    width_pt: Option<f32>,
    height_pt: Option<f32>,
    margin_top_pt: Option<f32>,
    margin_right_pt: Option<f32>,
    margin_bottom_pt: Option<f32>,
    margin_left_pt: Option<f32>,
}

impl PageStyle {
    fn merge(&mut self, other: Self) {
        self.width_pt = other.width_pt.or(self.width_pt);
        self.height_pt = other.height_pt.or(self.height_pt);
        self.margin_top_pt = other.margin_top_pt.or(self.margin_top_pt);
        self.margin_right_pt = other.margin_right_pt.or(self.margin_right_pt);
        self.margin_bottom_pt = other.margin_bottom_pt.or(self.margin_bottom_pt);
        self.margin_left_pt = other.margin_left_pt.or(self.margin_left_pt);
    }

    fn apply(&self, mut page: PageOptions) -> PageOptions {
        if let Some(width) = self.width_pt {
            page.width_pt = width;
        }
        if let Some(height) = self.height_pt {
            page.height_pt = height;
        }
        if let Some(margin) = self.margin_top_pt {
            page.margin_top_pt = margin;
        }
        if let Some(margin) = self.margin_right_pt {
            page.margin_right_pt = margin;
        }
        if let Some(margin) = self.margin_bottom_pt {
            page.margin_bottom_pt = margin;
        }
        if let Some(margin) = self.margin_left_pt {
            page.margin_left_pt = margin;
        }
        page
    }
}

#[derive(Debug, Clone)]
struct Rule {
    selector: Selector,
    pseudo: Option<PseudoElement>,
    specificity: (u16, u16, u16),
    layer_index: Option<usize>,
    declarations: Vec<Declaration>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PseudoElement {
    Before,
    After,
}

#[derive(Debug, Clone)]
struct Declaration {
    property: String,
    value: String,
    important: bool,
}

fn cascade_ordered_declarations(declarations: Vec<Declaration>) -> Vec<Declaration> {
    let mut normal = Vec::with_capacity(declarations.len());
    let mut important = Vec::new();
    for declaration in declarations {
        if declaration.important {
            important.push(declaration);
        } else {
            normal.push(declaration);
        }
    }
    normal.extend(important);
    normal
}

fn ordered_rule_declarations<'a>(
    matching_rules: impl IntoIterator<Item = (usize, &'a Rule)>,
    layer_count: usize,
) -> Vec<Declaration> {
    let mut rules = matching_rules.into_iter().collect::<Vec<_>>();

    rules.sort_by_key(|(source_order, rule)| normal_rule_cascade_key(*source_order, rule));
    let mut declarations = Vec::new();
    for (_, rule) in &rules {
        declarations.extend(
            rule.declarations
                .iter()
                .filter(|declaration| !declaration.important)
                .cloned(),
        );
    }

    rules.sort_by_key(|(source_order, rule)| {
        important_rule_cascade_key(*source_order, rule, layer_count)
    });
    for (_, rule) in &rules {
        declarations.extend(
            rule.declarations
                .iter()
                .filter(|declaration| declaration.important)
                .cloned(),
        );
    }

    declarations
}

fn append_inline_declarations(out: &mut Vec<Declaration>, inline: Vec<Declaration>) {
    let mut normal = Vec::new();
    let mut important = Vec::new();
    for declaration in inline {
        if declaration.important {
            important.push(declaration);
        } else {
            normal.push(declaration);
        }
    }
    let first_important = out
        .iter()
        .position(|declaration| declaration.important)
        .unwrap_or(out.len());
    out.splice(first_important..first_important, normal);
    out.extend(important);
}

fn normal_rule_cascade_key(source_order: usize, rule: &Rule) -> (usize, u16, u16, u16, usize) {
    let layer_rank = rule.layer_index.unwrap_or(usize::MAX / 2);
    let (ids, classes, tags) = rule.specificity;
    (layer_rank, ids, classes, tags, source_order)
}

fn important_rule_cascade_key(
    source_order: usize,
    rule: &Rule,
    layer_count: usize,
) -> (usize, u16, u16, u16, usize) {
    let layer_rank = rule
        .layer_index
        .map(|idx| layer_count.saturating_sub(idx))
        .unwrap_or(0);
    let (ids, classes, tags) = rule.specificity;
    (layer_rank, ids, classes, tags, source_order)
}

#[derive(Debug, Clone, Copy)]
struct MediaContext {
    print: bool,
    width_px: f32,
    height_px: f32,
}

impl MediaContext {
    fn for_page(page: PageOptions) -> Self {
        let content_width_pt =
            (page.width_pt - page.margin_left_pt - page.margin_right_pt).max(1.0);
        let content_height_pt =
            (page.height_pt - page.margin_top_pt - page.margin_bottom_pt).max(1.0);
        Self {
            print: true,
            width_px: content_width_pt * 96.0 / 72.0,
            height_px: content_height_pt * 96.0 / 72.0,
        }
    }
}

#[derive(Debug, Clone)]
struct Selector {
    parts: Vec<SelectorPart>,
    combinators: Vec<Combinator>,
}

impl Selector {
    fn parse(raw: &str) -> Option<Self> {
        let (raw_parts, combinators) = split_selector_parts_for_parse(raw);
        let parts = raw_parts
            .into_iter()
            .map(SelectorPart::parse)
            .collect::<Option<Vec<_>>>()?;
        if parts.is_empty() {
            return None;
        }
        if combinators.len() + 1 != parts.len() {
            return None;
        }
        Some(Self { parts, combinators })
    }

    fn matches(&self, document: &Document, id: NodeId, element: &ElementNode) -> bool {
        let Some(last) = self.parts.last() else {
            return false;
        };
        if !last.matches(document, id, element) {
            return false;
        }

        let mut current_id = id;
        for part_index in (0..self.parts.len().saturating_sub(1)).rev() {
            let part = &self.parts[part_index];
            let Some(candidate_id) = (match self
                .combinators
                .get(part_index)
                .copied()
                .unwrap_or_default()
            {
                Combinator::Descendant => {
                    let mut ancestor = document.parent_of(current_id);
                    let mut found = None;
                    while let Some(candidate_id) = ancestor {
                        if let Some(Node::Element(candidate)) = document.node(candidate_id) {
                            if part.matches(document, candidate_id, candidate) {
                                found = Some(candidate_id);
                                break;
                            }
                        }
                        ancestor = document.parent_of(candidate_id);
                    }
                    found
                }
                Combinator::Child => document.parent_of(current_id).and_then(|candidate_id| {
                    let Some(Node::Element(candidate)) = document.node(candidate_id) else {
                        return None;
                    };
                    part.matches(document, candidate_id, candidate)
                        .then_some(candidate_id)
                }),
                Combinator::AdjacentSibling => previous_element_sibling(document, current_id)
                    .and_then(|candidate_id| {
                        let Some(Node::Element(candidate)) = document.node(candidate_id) else {
                            return None;
                        };
                        part.matches(document, candidate_id, candidate)
                            .then_some(candidate_id)
                    }),
                Combinator::GeneralSibling => previous_element_siblings(document, current_id)
                    .into_iter()
                    .find(|candidate_id| {
                        let Some(Node::Element(candidate)) = document.node(*candidate_id) else {
                            return false;
                        };
                        part.matches(document, *candidate_id, candidate)
                    }),
            }) else {
                return false;
            };
            current_id = candidate_id;
        }
        true
    }

    fn could_match_element(&self, element: &ElementNode) -> bool {
        self.parts
            .last()
            .is_some_and(|part| part.could_match_element(element))
    }

    fn index_key(&self) -> Option<String> {
        self.parts.last().and_then(SelectorPart::index_key)
    }

    fn is_html_root(&self) -> bool {
        self.parts.len() == 1
            && matches!(
                self.parts.first(),
                Some(SelectorPart {
                    tag: Some(tag),
                    id: None,
                    classes,
                    attributes,
                    pseudos,
                    not_selectors,
                    has_selectors,
                }) if tag == "html"
                    && classes.is_empty()
                    && attributes.is_empty()
                    && pseudos.is_empty()
                    && not_selectors.is_empty()
                    && has_selectors.is_empty()
            )
    }

    fn matches_relative_to(
        &self,
        document: &Document,
        root: NodeId,
        combinator: Combinator,
    ) -> bool {
        match combinator {
            Combinator::Descendant => self.matches_in_descendants(document, root),
            Combinator::Child => document.children(root).iter().copied().any(|candidate_id| {
                let Some(Node::Element(candidate)) = document.node(candidate_id) else {
                    return false;
                };
                self.matches(document, candidate_id, candidate)
            }),
            Combinator::AdjacentSibling => {
                next_element_sibling(document, root).is_some_and(|candidate_id| {
                    let Some(Node::Element(candidate)) = document.node(candidate_id) else {
                        return false;
                    };
                    self.matches(document, candidate_id, candidate)
                })
            }
            Combinator::GeneralSibling => {
                next_element_siblings(document, root)
                    .into_iter()
                    .any(|candidate_id| {
                        let Some(Node::Element(candidate)) = document.node(candidate_id) else {
                            return false;
                        };
                        self.matches(document, candidate_id, candidate)
                    })
            }
        }
    }

    fn matches_in_descendants(&self, document: &Document, root: NodeId) -> bool {
        let mut stack = document.children(root).to_vec();
        while let Some(candidate_id) = stack.pop() {
            if let Some(Node::Element(candidate)) = document.node(candidate_id) {
                if self.matches(document, candidate_id, candidate) {
                    return true;
                }
                stack.extend(document.children(candidate_id));
            }
        }
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Combinator {
    #[default]
    Descendant,
    Child,
    AdjacentSibling,
    GeneralSibling,
}

fn split_selector_parts_for_parse(raw: &str) -> (Vec<&str>, Vec<Combinator>) {
    let mut parts = Vec::new();
    let mut combinators = Vec::new();
    let mut bracket_depth = 0_i32;
    let mut paren_depth = 0_i32;
    let mut quote = None;
    let mut start = None;
    let mut pending_combinator = None;

    for (idx, ch) in raw.char_indices() {
        if let Some(active_quote) = quote {
            if ch == active_quote {
                quote = None;
            }
            continue;
        }

        match ch {
            '"' | '\'' if bracket_depth > 0 => quote = Some(ch),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = (bracket_depth - 1).max(0),
            '(' if bracket_depth == 0 => paren_depth += 1,
            ')' if bracket_depth == 0 => paren_depth = (paren_depth - 1).max(0),
            '>' | '+' | '~' if bracket_depth == 0 && paren_depth == 0 => {
                if let Some(segment_start) = start.take() {
                    push_selector_part(
                        &mut parts,
                        &mut combinators,
                        &mut pending_combinator,
                        &raw[segment_start..idx],
                    );
                }
                pending_combinator = Some(match ch {
                    '>' => Combinator::Child,
                    '+' => Combinator::AdjacentSibling,
                    '~' => Combinator::GeneralSibling,
                    _ => Combinator::Descendant,
                });
                continue;
            }
            ch if ch.is_whitespace() && bracket_depth == 0 && paren_depth == 0 => {
                if let Some(segment_start) = start.take() {
                    push_selector_part(
                        &mut parts,
                        &mut combinators,
                        &mut pending_combinator,
                        &raw[segment_start..idx],
                    );
                    pending_combinator.get_or_insert(Combinator::Descendant);
                }
                continue;
            }
            _ => {}
        }

        if start.is_none() && !ch.is_whitespace() {
            start = Some(idx);
        }
    }

    if let Some(segment_start) = start {
        push_selector_part(
            &mut parts,
            &mut combinators,
            &mut pending_combinator,
            &raw[segment_start..],
        );
    }

    (parts, combinators)
}

fn push_selector_part<'a>(
    parts: &mut Vec<&'a str>,
    combinators: &mut Vec<Combinator>,
    pending_combinator: &mut Option<Combinator>,
    segment: &'a str,
) {
    let segment = segment.trim();
    if segment.is_empty() {
        return;
    }
    if !parts.is_empty() {
        combinators.push(pending_combinator.take().unwrap_or_default());
    }
    parts.push(segment);
}

fn previous_element_sibling(document: &Document, id: NodeId) -> Option<NodeId> {
    previous_element_siblings(document, id).into_iter().next()
}

fn previous_element_siblings(document: &Document, id: NodeId) -> Vec<NodeId> {
    let Some(parent) = document.parent_of(id) else {
        return Vec::new();
    };
    let Some(index) = document
        .children(parent)
        .iter()
        .position(|candidate_id| *candidate_id == id)
    else {
        return Vec::new();
    };
    document.children(parent)[..index]
        .iter()
        .rev()
        .copied()
        .filter(|candidate_id| matches!(document.node(*candidate_id), Some(Node::Element(_))))
        .collect()
}

fn next_element_sibling(document: &Document, id: NodeId) -> Option<NodeId> {
    next_element_siblings(document, id).into_iter().next()
}

fn next_element_siblings(document: &Document, id: NodeId) -> Vec<NodeId> {
    let Some(parent) = document.parent_of(id) else {
        return Vec::new();
    };
    let Some(index) = document
        .children(parent)
        .iter()
        .position(|candidate_id| *candidate_id == id)
    else {
        return Vec::new();
    };
    document.children(parent)[index.saturating_add(1)..]
        .iter()
        .copied()
        .filter(|candidate_id| matches!(document.node(*candidate_id), Some(Node::Element(_))))
        .collect()
}

#[derive(Debug, Clone)]
struct SelectorPart {
    tag: Option<String>,
    id: Option<String>,
    classes: Vec<String>,
    attributes: Vec<AttributeSelector>,
    pseudos: Vec<PseudoClass>,
    not_selectors: Vec<Selector>,
    has_selectors: Vec<RelativeSelector>,
}

impl SelectorPart {
    fn parse(raw: &str) -> Option<Self> {
        let attributes = parse_attribute_selectors(raw);
        let not_selectors = parse_negation_selectors(raw);
        let has_selectors = parse_has_selectors(raw);
        let positive_raw = strip_selector_functions(raw, &[":not", ":has"]);
        let pseudos = parse_pseudo_classes(&positive_raw);
        let normalized = normalize_selector_part(raw);
        let raw = normalized.trim();
        if raw == ":root" || raw == ":host" {
            return Some(Self {
                tag: Some("html".to_string()),
                id: None,
                classes: Vec::new(),
                attributes,
                pseudos: Vec::new(),
                not_selectors,
                has_selectors,
            });
        }
        if raw.is_empty() {
            if attributes.is_empty()
                && pseudos.is_empty()
                && not_selectors.is_empty()
                && has_selectors.is_empty()
            {
                return None;
            }
            return Some(Self {
                tag: None,
                id: None,
                classes: Vec::new(),
                attributes,
                pseudos,
                not_selectors,
                has_selectors,
            });
        }

        let bytes = raw.as_bytes();
        let mut i = 0usize;
        let mut tag = None;
        let mut id = None;
        let mut classes = Vec::new();

        while i < bytes.len() {
            match bytes[i] as char {
                '*' => {
                    i += 1;
                }
                '.' => {
                    i += 1;
                    let start = i;
                    while i < bytes.len() && !matches!(bytes[i] as char, '.' | '#') {
                        i += 1;
                    }
                    if start == i {
                        return None;
                    }
                    classes.push(raw[start..i].to_ascii_lowercase());
                }
                '#' => {
                    i += 1;
                    let start = i;
                    while i < bytes.len() && !matches!(bytes[i] as char, '.' | '#') {
                        i += 1;
                    }
                    if start == i {
                        return None;
                    }
                    id = Some(raw[start..i].to_ascii_lowercase());
                }
                _ => {
                    let start = i;
                    while i < bytes.len() && !matches!(bytes[i] as char, '.' | '#') {
                        i += 1;
                    }
                    if tag.is_some() || start == i {
                        return None;
                    }
                    tag = Some(raw[start..i].to_ascii_lowercase());
                }
            }
        }

        Some(Self {
            tag,
            id,
            classes,
            attributes,
            pseudos,
            not_selectors,
            has_selectors,
        })
    }

    fn matches(&self, document: &Document, id: NodeId, element: &ElementNode) -> bool {
        if let Some(tag) = &self.tag {
            if element.tag != *tag {
                return false;
            }
        }
        if let Some(id) = &self.id {
            if element.attr("id") != Some(id.as_str()) {
                return false;
            }
        }
        self.classes
            .iter()
            .all(|class| element.classes().any(|candidate| candidate == class))
            && self
                .attributes
                .iter()
                .all(|attribute| attribute.matches(element))
            && self
                .pseudos
                .iter()
                .all(|pseudo| pseudo.matches(document, id, element))
            && self
                .not_selectors
                .iter()
                .all(|selector| !selector.matches(document, id, element))
            && self
                .has_selectors
                .iter()
                .all(|selector| selector.matches(document, id))
    }

    fn could_match_element(&self, element: &ElementNode) -> bool {
        if let Some(tag) = &self.tag {
            if element.tag != *tag {
                return false;
            }
        }
        if let Some(id) = &self.id {
            if element.attr("id") != Some(id.as_str()) {
                return false;
            }
        }
        self.classes
            .iter()
            .all(|class| element.classes().any(|candidate| candidate == class))
    }

    fn index_key(&self) -> Option<String> {
        if let Some(id) = &self.id {
            return Some(format!("id:{id}"));
        }
        if let Some(class) = self.classes.first() {
            return Some(format!("class:{class}"));
        }
        self.tag.as_ref().map(|tag| format!("tag:{tag}"))
    }
}

#[derive(Debug, Clone)]
struct RelativeSelector {
    combinator: Combinator,
    selector: Selector,
}

impl RelativeSelector {
    fn matches(&self, document: &Document, root: NodeId) -> bool {
        self.selector
            .matches_relative_to(document, root, self.combinator)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AttributeSelector {
    name: String,
    operator: AttributeOperator,
    value: Option<String>,
    case_sensitivity: AttributeCaseSensitivity,
}

impl AttributeSelector {
    fn matches(&self, element: &ElementNode) -> bool {
        let Some(actual) = element.attr(&self.name) else {
            return false;
        };
        let Some(expected) = self.value.as_deref() else {
            return true;
        };
        match self.operator {
            AttributeOperator::Presence => true,
            AttributeOperator::Equals => self.eq(actual, expected),
            AttributeOperator::Includes => actual
                .split_whitespace()
                .any(|part| self.eq(part, expected)),
            AttributeOperator::DashMatch => {
                self.eq(actual, expected)
                    || self
                        .strip_prefix(actual, expected)
                        .is_some_and(|suffix| suffix.starts_with('-'))
            }
            AttributeOperator::Prefix => self.strip_prefix(actual, expected).is_some(),
            AttributeOperator::Suffix => self.ends_with(actual, expected),
            AttributeOperator::Substring => self.contains(actual, expected),
        }
    }

    fn eq(&self, actual: &str, expected: &str) -> bool {
        match self.case_sensitivity {
            AttributeCaseSensitivity::Sensitive => actual == expected,
            AttributeCaseSensitivity::AsciiInsensitive => actual.eq_ignore_ascii_case(expected),
        }
    }

    fn strip_prefix<'a>(&self, actual: &'a str, expected: &str) -> Option<&'a str> {
        match self.case_sensitivity {
            AttributeCaseSensitivity::Sensitive => actual.strip_prefix(expected),
            AttributeCaseSensitivity::AsciiInsensitive => {
                let prefix = actual.get(..expected.len())?;
                if prefix.eq_ignore_ascii_case(expected) {
                    actual.get(expected.len()..)
                } else {
                    None
                }
            }
        }
    }

    fn ends_with(&self, actual: &str, expected: &str) -> bool {
        match self.case_sensitivity {
            AttributeCaseSensitivity::Sensitive => actual.ends_with(expected),
            AttributeCaseSensitivity::AsciiInsensitive => actual
                .get(actual.len().saturating_sub(expected.len())..)
                .is_some_and(|suffix| suffix.eq_ignore_ascii_case(expected)),
        }
    }

    fn contains(&self, actual: &str, expected: &str) -> bool {
        match self.case_sensitivity {
            AttributeCaseSensitivity::Sensitive => actual.contains(expected),
            AttributeCaseSensitivity::AsciiInsensitive => actual
                .to_ascii_lowercase()
                .contains(&expected.to_ascii_lowercase()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AttributeOperator {
    Presence,
    Equals,
    Includes,
    DashMatch,
    Prefix,
    Suffix,
    Substring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AttributeCaseSensitivity {
    Sensitive,
    AsciiInsensitive,
}

#[derive(Debug, Clone)]
enum PseudoClass {
    Root,
    Empty,
    FirstChild,
    LastChild,
    OnlyChild,
    NthChild(NthChildSelector),
    NthLastChild(NthChildSelector),
    FirstOfType,
    LastOfType,
    OnlyOfType,
    NthOfType(NthChild),
    NthLastOfType(NthChild),
    Disabled,
    Enabled,
    Checked,
    Default,
    Required,
    Optional,
    ReadOnly,
    ReadWrite,
    PlaceholderShown,
    AnyLink,
    Link,
    Lang(Vec<String>),
    Dir(TextDirectionPseudo),
    Never,
}

impl PseudoClass {
    fn matches(&self, document: &Document, id: NodeId, element: &ElementNode) -> bool {
        match self {
            Self::Root => element.tag == "html",
            Self::Empty => element
                .children
                .iter()
                .all(|child_id| match document.node(*child_id) {
                    Some(Node::Element(_)) => false,
                    Some(Node::Text(text)) => text.is_empty(),
                    None => true,
                }),
            Self::FirstChild
            | Self::LastChild
            | Self::OnlyChild
            | Self::NthChild(_)
            | Self::NthLastChild(_) => {
                let (index, total) = match self {
                    Self::NthChild(selector) | Self::NthLastChild(selector) => {
                        let Some(position) =
                            element_sibling_position_matching(document, id, &selector.of_selectors)
                        else {
                            return false;
                        };
                        position
                    }
                    _ => {
                        let Some(position) = element_sibling_position(document, id) else {
                            return false;
                        };
                        position
                    }
                };
                match self {
                    Self::FirstChild => index == 1,
                    Self::LastChild => index == total,
                    Self::OnlyChild => total == 1,
                    Self::NthChild(selector) => selector.nth.matches(index),
                    Self::NthLastChild(selector) => {
                        selector.nth.matches(total.saturating_sub(index) + 1)
                    }
                    _ => false,
                }
            }
            Self::FirstOfType
            | Self::LastOfType
            | Self::OnlyOfType
            | Self::NthOfType(_)
            | Self::NthLastOfType(_) => {
                let Some((index, total)) = element_sibling_position_of_type(document, id) else {
                    return false;
                };
                match self {
                    Self::FirstOfType => index == 1,
                    Self::LastOfType => index == total,
                    Self::OnlyOfType => total == 1,
                    Self::NthOfType(nth) => (*nth).matches(index),
                    Self::NthLastOfType(nth) => (*nth).matches(total.saturating_sub(index) + 1),
                    _ => false,
                }
            }
            Self::Disabled => is_disabled_form_control(element),
            Self::Enabled => {
                is_enableable_form_control(element) && !is_disabled_form_control(element)
            }
            Self::Checked => is_checked_control(element),
            Self::Default => is_default_form_control(element),
            Self::Required => is_required_form_control(element),
            Self::Optional => is_optionally_fillable_form_control(element),
            Self::ReadOnly => is_read_only_control(element),
            Self::ReadWrite => is_read_write_control(element),
            Self::PlaceholderShown => is_placeholder_shown(element),
            Self::AnyLink | Self::Link => is_link_element(element),
            Self::Lang(ranges) => inherited_language(document, id, element)
                .as_deref()
                .is_some_and(|actual| ranges.iter().any(|range| language_matches(actual, range))),
            Self::Dir(direction) => inherited_direction(document, id, element) == *direction,
            Self::Never => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextDirectionPseudo {
    Ltr,
    Rtl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NthChild {
    AnPlusB { a: i32, b: i32 },
}

#[derive(Debug, Clone)]
struct NthChildSelector {
    nth: NthChild,
    of_selectors: Vec<Selector>,
}

impl NthChild {
    fn matches(self, index: usize) -> bool {
        match self {
            Self::AnPlusB { a, b } => {
                let index = index as i32;
                if a == 0 {
                    return index == b;
                }
                let diff = index - b;
                diff % a == 0 && diff / a >= 0
            }
        }
    }
}

fn parse_attribute_selectors(raw: &str) -> Vec<AttributeSelector> {
    let mut attributes = Vec::new();
    let mut paren_depth = 0_i32;
    let mut cursor = 0usize;
    while cursor < raw.len() {
        let Some((relative_idx, ch)) = raw[cursor..].char_indices().next() else {
            break;
        };
        let idx = cursor + relative_idx;
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = (paren_depth - 1).max(0),
            '[' if paren_depth == 0 => {
                if let Some(close) = raw[idx + 1..].find(']') {
                    let body = &raw[idx + 1..idx + 1 + close];
                    if let Some(attribute) = parse_attribute_selector(body) {
                        attributes.push(attribute);
                    }
                    cursor = idx + close + 2;
                    continue;
                }
            }
            _ => {}
        }
        cursor = idx + ch.len_utf8();
    }
    attributes
}

fn parse_attribute_selector(body: &str) -> Option<AttributeSelector> {
    let body = body.trim();
    if body.is_empty() {
        return None;
    }
    let operators = [
        ("~=", AttributeOperator::Includes),
        ("|=", AttributeOperator::DashMatch),
        ("^=", AttributeOperator::Prefix),
        ("$=", AttributeOperator::Suffix),
        ("*=", AttributeOperator::Substring),
        ("=", AttributeOperator::Equals),
    ];
    for (token, operator) in operators {
        if let Some(index) = body.find(token) {
            let name = body[..index].trim().to_ascii_lowercase();
            if name.is_empty() {
                return None;
            }
            let (value, case_sensitivity) = normalize_attribute_value(&body[index + token.len()..]);
            return Some(AttributeSelector {
                name,
                operator,
                value: Some(value),
                case_sensitivity,
            });
        }
    }
    let name = body.split_whitespace().next()?.to_ascii_lowercase();
    if name.is_empty() {
        return None;
    }
    Some(AttributeSelector {
        name,
        operator: AttributeOperator::Presence,
        value: None,
        case_sensitivity: AttributeCaseSensitivity::Sensitive,
    })
}

fn normalize_attribute_value(raw: &str) -> (String, AttributeCaseSensitivity) {
    let trimmed = raw.trim();
    let (without_case_flag, case_sensitivity) = if let Some(value) = trimmed
        .strip_suffix(" i")
        .or_else(|| trimmed.strip_suffix(" I"))
    {
        (value.trim(), AttributeCaseSensitivity::AsciiInsensitive)
    } else if let Some(value) = trimmed
        .strip_suffix(" s")
        .or_else(|| trimmed.strip_suffix(" S"))
    {
        (value.trim(), AttributeCaseSensitivity::Sensitive)
    } else {
        (trimmed, AttributeCaseSensitivity::Sensitive)
    };
    let value = without_case_flag
        .trim_matches('"')
        .trim_matches('\'')
        .to_string();
    (value, case_sensitivity)
}

fn parse_negation_selectors(raw: &str) -> Vec<Selector> {
    let mut selectors = Vec::new();
    let raw = normalize_selector_case_for_parse(raw);
    let mut rest = raw.as_str();
    while let Some(start) = rest.find(":not(") {
        let function = &rest[start..];
        if let Some((_, body, end)) = extract_selector_function(function, &[":not"]) {
            for selector in split_top_level(body, &[',']) {
                let selector = selector.trim();
                if selector.is_empty() {
                    continue;
                }
                for selector in expand_selector_list_functions(selector) {
                    if let Some(selector) = Selector::parse(selector.trim()) {
                        selectors.push(selector);
                    }
                }
            }
            rest = &function[end..];
        } else {
            break;
        }
    }
    selectors
}

fn parse_has_selectors(raw: &str) -> Vec<RelativeSelector> {
    let mut selectors = Vec::new();
    let raw = normalize_selector_case_for_parse(raw);
    let mut rest = raw.as_str();
    while let Some(start) = rest.find(":has(") {
        let function = &rest[start..];
        if let Some((_, body, end)) = extract_selector_function(function, &[":has"]) {
            for selector in split_top_level(body, &[',']) {
                let (combinator, selector) = normalize_relative_has_selector(selector.trim());
                if selector.is_empty() {
                    continue;
                }
                if let Some(selector) = Selector::parse(selector) {
                    selectors.push(RelativeSelector {
                        combinator,
                        selector,
                    });
                }
            }
            rest = &function[end..];
        } else {
            break;
        }
    }
    selectors
}

fn normalize_relative_has_selector(selector: &str) -> (Combinator, &str) {
    let selector = selector.trim();
    let Some(first) = selector.chars().next() else {
        return (Combinator::Descendant, selector);
    };
    match first {
        '>' => (Combinator::Child, selector[1..].trim()),
        '+' => (Combinator::AdjacentSibling, selector[1..].trim()),
        '~' => (Combinator::GeneralSibling, selector[1..].trim()),
        _ => (Combinator::Descendant, selector),
    }
}

fn parse_pseudo_classes(raw: &str) -> Vec<PseudoClass> {
    let raw = raw.to_ascii_lowercase();
    let mut pseudos = Vec::new();
    let mut bracket_depth = 0_i32;
    let mut quote = None;
    let mut cursor = 0usize;

    while cursor < raw.len() {
        let Some((relative_idx, ch)) = raw[cursor..].char_indices().next() else {
            break;
        };
        let idx = cursor + relative_idx;

        if let Some(active_quote) = quote {
            if ch == active_quote {
                quote = None;
            }
            cursor = idx + ch.len_utf8();
            continue;
        }

        match ch {
            '"' | '\'' if bracket_depth > 0 => {
                quote = Some(ch);
                cursor = idx + ch.len_utf8();
            }
            '[' => {
                bracket_depth += 1;
                cursor = idx + ch.len_utf8();
            }
            ']' => {
                bracket_depth = (bracket_depth - 1).max(0);
                cursor = idx + ch.len_utf8();
            }
            ':' if bracket_depth == 0 => {
                if raw.as_bytes().get(idx + 1) == Some(&b':') {
                    cursor = consume_css_identifier(&raw, idx + 2);
                    continue;
                }
                let name_start = idx + 1;
                let name_end = consume_css_identifier(&raw, name_start);
                if name_end == name_start {
                    cursor = idx + ch.len_utf8();
                    continue;
                }
                let name = &raw[name_start..name_end];
                let mut body = None;
                cursor = name_end;
                if raw.as_bytes().get(name_end) == Some(&b'(') {
                    if let Some(end) = matching_function_end(&raw[name_end..]) {
                        let close = name_end + end;
                        body = raw.get(name_end + 1..close);
                        cursor = close + 1;
                    } else {
                        cursor = raw.len();
                    }
                }
                pseudos.push(parse_pseudo_class(name, body));
            }
            _ => cursor = idx + ch.len_utf8(),
        }
    }
    pseudos
}

fn parse_pseudo_class(name: &str, body: Option<&str>) -> PseudoClass {
    match name {
        "root" => PseudoClass::Root,
        "empty" => PseudoClass::Empty,
        "first-child" => PseudoClass::FirstChild,
        "last-child" => PseudoClass::LastChild,
        "only-child" => PseudoClass::OnlyChild,
        "first-of-type" => PseudoClass::FirstOfType,
        "last-of-type" => PseudoClass::LastOfType,
        "only-of-type" => PseudoClass::OnlyOfType,
        "nth-child" => body
            .and_then(parse_nth_child_selector)
            .map(PseudoClass::NthChild)
            .unwrap_or(PseudoClass::Never),
        "nth-last-child" => body
            .and_then(parse_nth_child_selector)
            .map(PseudoClass::NthLastChild)
            .unwrap_or(PseudoClass::Never),
        "nth-of-type" => body
            .and_then(parse_nth_child_expression)
            .map(PseudoClass::NthOfType)
            .unwrap_or(PseudoClass::Never),
        "nth-last-of-type" => body
            .and_then(parse_nth_child_expression)
            .map(PseudoClass::NthLastOfType)
            .unwrap_or(PseudoClass::Never),
        "disabled" => PseudoClass::Disabled,
        "enabled" => PseudoClass::Enabled,
        "checked" => PseudoClass::Checked,
        "default" => PseudoClass::Default,
        "required" => PseudoClass::Required,
        "optional" => PseudoClass::Optional,
        "read-only" => PseudoClass::ReadOnly,
        "read-write" => PseudoClass::ReadWrite,
        "placeholder-shown" => PseudoClass::PlaceholderShown,
        "any-link" => PseudoClass::AnyLink,
        "link" => PseudoClass::Link,
        "lang" => body
            .map(parse_language_ranges)
            .filter(|ranges| !ranges.is_empty())
            .map(PseudoClass::Lang)
            .unwrap_or(PseudoClass::Never),
        "dir" => body
            .and_then(parse_direction_pseudo)
            .map(PseudoClass::Dir)
            .unwrap_or(PseudoClass::Never),
        "visited" | "hover" | "active" | "focus" | "focus-visible" | "focus-within" | "target"
        | "target-within" | "modal" | "popover-open" | "fullscreen" | "autofill"
        | "-webkit-autofill" | "-moz-focusring" | "valid" | "invalid" | "user-valid"
        | "user-invalid" | "in-range" | "out-of-range" => PseudoClass::Never,
        _ => PseudoClass::Never,
    }
}

fn is_enableable_form_control(element: &ElementNode) -> bool {
    matches!(
        element.tag.as_str(),
        "button" | "fieldset" | "input" | "optgroup" | "option" | "select" | "textarea"
    )
}

fn is_disabled_form_control(element: &ElementNode) -> bool {
    is_enableable_form_control(element) && element.attr("disabled").is_some()
}

fn is_checked_control(element: &ElementNode) -> bool {
    let input_type = element.attr("type").unwrap_or("text");
    (element.tag == "input"
        && (input_type.eq_ignore_ascii_case("checkbox")
            || input_type.eq_ignore_ascii_case("radio"))
        && element.attr("checked").is_some())
        || (element.tag == "option" && element.attr("selected").is_some())
}

fn is_default_form_control(element: &ElementNode) -> bool {
    let input_type = element.attr("type").unwrap_or("text");
    (element.tag == "input"
        && (input_type.eq_ignore_ascii_case("checkbox")
            || input_type.eq_ignore_ascii_case("radio"))
        && element.attr("checked").is_some())
        || (element.tag == "option" && element.attr("selected").is_some())
}

fn is_fillable_form_control(element: &ElementNode) -> bool {
    matches!(element.tag.as_str(), "input" | "select" | "textarea")
}

fn is_required_form_control(element: &ElementNode) -> bool {
    is_fillable_form_control(element) && element.attr("required").is_some()
}

fn is_optionally_fillable_form_control(element: &ElementNode) -> bool {
    is_fillable_form_control(element) && element.attr("required").is_none()
}

fn is_read_only_control(element: &ElementNode) -> bool {
    matches!(element.tag.as_str(), "input" | "textarea")
        && (element.attr("readonly").is_some() || is_disabled_form_control(element))
}

fn is_read_write_control(element: &ElementNode) -> bool {
    matches!(element.tag.as_str(), "input" | "textarea")
        && element.attr("readonly").is_none()
        && !is_disabled_form_control(element)
}

fn is_placeholder_shown(element: &ElementNode) -> bool {
    matches!(element.tag.as_str(), "input" | "textarea")
        && element.attr("placeholder").is_some()
        && element.attr("value").unwrap_or_default().is_empty()
}

fn is_link_element(element: &ElementNode) -> bool {
    matches!(element.tag.as_str(), "a" | "area") && element.attr("href").is_some()
}

fn parse_language_ranges(raw: &str) -> Vec<String> {
    split_top_level(raw, &[','])
        .into_iter()
        .map(|range| range.trim().trim_matches('"').trim_matches('\''))
        .filter(|range| !range.is_empty())
        .map(|range| range.to_ascii_lowercase())
        .collect()
}

fn parse_direction_pseudo(raw: &str) -> Option<TextDirectionPseudo> {
    match raw.trim().trim_matches('"').trim_matches('\'') {
        "ltr" => Some(TextDirectionPseudo::Ltr),
        "rtl" => Some(TextDirectionPseudo::Rtl),
        _ => None,
    }
}

fn inherited_language(document: &Document, id: NodeId, element: &ElementNode) -> Option<String> {
    if let Some(language) = element
        .attr("lang")
        .or_else(|| element.attr("xml:lang"))
        .map(str::trim)
        .filter(|language| !language.is_empty())
    {
        return Some(language.to_ascii_lowercase());
    }

    let mut ancestor = document.parent_of(id);
    while let Some(ancestor_id) = ancestor {
        if let Some(Node::Element(ancestor_element)) = document.node(ancestor_id) {
            if let Some(language) = ancestor_element
                .attr("lang")
                .or_else(|| ancestor_element.attr("xml:lang"))
                .map(str::trim)
                .filter(|language| !language.is_empty())
            {
                return Some(language.to_ascii_lowercase());
            }
        }
        ancestor = document.parent_of(ancestor_id);
    }

    None
}

fn language_matches(actual: &str, range: &str) -> bool {
    let actual = actual.trim().to_ascii_lowercase();
    let range = range.trim().to_ascii_lowercase();
    if actual.is_empty() || range.is_empty() {
        return false;
    }
    if range == "*" {
        return true;
    }
    actual == range || actual.starts_with(&format!("{range}-"))
}

fn inherited_direction(
    document: &Document,
    id: NodeId,
    element: &ElementNode,
) -> TextDirectionPseudo {
    if let Some(direction) = element_direction(element) {
        return direction;
    }

    let mut ancestor = document.parent_of(id);
    while let Some(ancestor_id) = ancestor {
        if let Some(Node::Element(ancestor_element)) = document.node(ancestor_id) {
            if let Some(direction) = element_direction(ancestor_element) {
                return direction;
            }
        }
        ancestor = document.parent_of(ancestor_id);
    }

    TextDirectionPseudo::Ltr
}

fn element_direction(element: &ElementNode) -> Option<TextDirectionPseudo> {
    match element.attr("dir").map(str::trim) {
        Some(value) if value.eq_ignore_ascii_case("rtl") => Some(TextDirectionPseudo::Rtl),
        Some(value) if value.eq_ignore_ascii_case("ltr") => Some(TextDirectionPseudo::Ltr),
        // `auto` is content-dependent in browsers. For deterministic static PDF output,
        // fall through to the inherited/default direction rather than guessing from text.
        _ => None,
    }
}

fn parse_nth_child_selector(raw: &str) -> Option<NthChildSelector> {
    let (expression, selector_list) = split_nth_child_of_selector(raw);
    let nth = parse_nth_child_expression(expression)?;
    let of_selectors = selector_list
        .map(|selector_list| {
            split_top_level(selector_list, &[','])
                .into_iter()
                .flat_map(|selector| expand_selector_list_functions(selector.trim()))
                .filter_map(|selector| Selector::parse(selector.trim()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if selector_list.is_some() && of_selectors.is_empty() {
        return None;
    }

    Some(NthChildSelector { nth, of_selectors })
}

fn split_nth_child_of_selector(raw: &str) -> (&str, Option<&str>) {
    let mut bracket_depth = 0_i32;
    let mut paren_depth = 0_i32;
    let mut quote = None;
    let mut previous_was_whitespace = false;
    let mut cursor = 0usize;

    while cursor < raw.len() {
        let Some((relative_idx, ch)) = raw[cursor..].char_indices().next() else {
            break;
        };
        let idx = cursor + relative_idx;

        if let Some(active_quote) = quote {
            if ch == active_quote {
                quote = None;
            }
            cursor = idx + ch.len_utf8();
            continue;
        }

        match ch {
            '"' | '\'' => {
                quote = Some(ch);
                previous_was_whitespace = false;
                cursor = idx + ch.len_utf8();
            }
            '[' => {
                bracket_depth += 1;
                previous_was_whitespace = false;
                cursor = idx + ch.len_utf8();
            }
            ']' => {
                bracket_depth = (bracket_depth - 1).max(0);
                previous_was_whitespace = false;
                cursor = idx + ch.len_utf8();
            }
            '(' if bracket_depth == 0 => {
                paren_depth += 1;
                previous_was_whitespace = false;
                cursor = idx + ch.len_utf8();
            }
            ')' if bracket_depth == 0 => {
                paren_depth = (paren_depth - 1).max(0);
                previous_was_whitespace = false;
                cursor = idx + ch.len_utf8();
            }
            ch if ch.is_whitespace() && bracket_depth == 0 && paren_depth == 0 => {
                previous_was_whitespace = true;
                cursor = idx + ch.len_utf8();
            }
            _ if bracket_depth == 0
                && paren_depth == 0
                && previous_was_whitespace
                && raw
                    .get(idx..idx + 2)
                    .is_some_and(|token| token.eq_ignore_ascii_case("of"))
                && raw[idx + 2..]
                    .chars()
                    .next()
                    .is_some_and(|next| next.is_whitespace()) =>
            {
                let selector_start = idx + 2;
                return (&raw[..idx], Some(raw[selector_start..].trim()));
            }
            _ => {
                previous_was_whitespace = false;
                cursor = idx + ch.len_utf8();
            }
        }
    }

    (raw, None)
}

fn parse_nth_child_expression(raw: &str) -> Option<NthChild> {
    let value = raw
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    if value.is_empty() {
        return None;
    }
    match value.as_str() {
        "odd" => return Some(NthChild::AnPlusB { a: 2, b: 1 }),
        "even" => return Some(NthChild::AnPlusB { a: 2, b: 0 }),
        _ => {}
    }

    if let Some(n_pos) = value.find('n') {
        let a_raw = &value[..n_pos];
        let b_raw = &value[n_pos + 1..];
        let a = match a_raw {
            "" | "+" => 1,
            "-" => -1,
            value => value.parse::<i32>().ok()?,
        };
        let b = if b_raw.is_empty() {
            0
        } else {
            b_raw.parse::<i32>().ok()?
        };
        return Some(NthChild::AnPlusB { a, b });
    }

    value
        .parse::<i32>()
        .ok()
        .map(|b| NthChild::AnPlusB { a: 0, b })
}

fn element_sibling_position(document: &Document, id: NodeId) -> Option<(usize, usize)> {
    let parent = document.parent_of(id)?;
    let siblings = document
        .children(parent)
        .iter()
        .copied()
        .filter(|sibling_id| matches!(document.node(*sibling_id), Some(Node::Element(_))))
        .collect::<Vec<_>>();
    let index = siblings.iter().position(|sibling_id| *sibling_id == id)? + 1;
    Some((index, siblings.len()))
}

fn element_sibling_position_matching(
    document: &Document,
    id: NodeId,
    selectors: &[Selector],
) -> Option<(usize, usize)> {
    if selectors.is_empty() {
        return element_sibling_position(document, id);
    }

    let parent = document.parent_of(id)?;
    let siblings = document
        .children(parent)
        .iter()
        .copied()
        .filter(|sibling_id| {
            let Some(Node::Element(element)) = document.node(*sibling_id) else {
                return false;
            };
            selectors
                .iter()
                .any(|selector| selector.matches(document, *sibling_id, element))
        })
        .collect::<Vec<_>>();
    let index = siblings.iter().position(|sibling_id| *sibling_id == id)? + 1;
    Some((index, siblings.len()))
}

fn element_sibling_position_of_type(document: &Document, id: NodeId) -> Option<(usize, usize)> {
    let tag = match document.node(id)? {
        Node::Element(element) => element.tag.as_str(),
        _ => return None,
    };
    let parent = document.parent_of(id)?;
    let siblings = document
        .children(parent)
        .iter()
        .copied()
        .filter(|sibling_id| {
            matches!(
                document.node(*sibling_id),
                Some(Node::Element(element)) if element.tag == tag
            )
        })
        .collect::<Vec<_>>();
    let index = siblings.iter().position(|sibling_id| *sibling_id == id)? + 1;
    Some((index, siblings.len()))
}

fn normalize_selector_part(raw: &str) -> String {
    let expanded = expand_selector_functions(raw.trim());
    let raw = expanded.trim();
    if raw == ":root" || raw == ":host" {
        return raw.to_string();
    }

    let mut out = String::new();
    let mut bracket_depth = 0_i32;
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '[' => bracket_depth += 1,
            ']' => bracket_depth -= 1,
            ':' if bracket_depth == 0 => {
                if matches!(chars.peek(), Some(':')) {
                    let _ = chars.next();
                }
                while matches!(chars.peek(), Some(ch) if ch.is_ascii_alphanumeric() || *ch == '-') {
                    let _ = chars.next();
                }
                if matches!(chars.peek(), Some('(')) {
                    let _ = chars.next();
                    let mut paren_depth = 1_i32;
                    for next in chars.by_ref() {
                        match next {
                            '(' => paren_depth += 1,
                            ')' => {
                                paren_depth -= 1;
                                if paren_depth == 0 {
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            ch if bracket_depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out
}

fn expand_selector_functions(raw: &str) -> String {
    let mut out = String::new();
    let mut chars = raw.char_indices().peekable();
    while let Some((idx, ch)) = chars.next() {
        if ch == ':' {
            let rest = &raw[idx..];
            if let Some((name_len, body, end_idx)) =
                extract_selector_function(rest, &[":where", ":is"])
            {
                let _ = name_len;
                if let Some(first) = split_top_level(body, &[',']).first() {
                    out.push_str(first.trim());
                }
                while matches!(chars.peek(), Some((next_idx, _)) if *next_idx < idx + end_idx) {
                    let _ = chars.next();
                }
                continue;
            }
        }
        out.push(ch);
    }
    out
}

fn expand_selector_list_functions(raw: &str) -> Vec<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Vec::new();
    }

    let mut selectors = vec![raw.to_string()];
    for _ in 0..16 {
        let mut changed = false;
        let mut expanded = Vec::new();

        for selector in selectors {
            if let Some((start, end, body)) =
                find_selector_function_call(&selector, &[":is", ":where"])
            {
                let args = split_top_level(body, &[','])
                    .into_iter()
                    .map(str::trim)
                    .filter(|arg| !arg.is_empty())
                    .collect::<Vec<_>>();

                if args.is_empty() {
                    expanded.push(selector);
                    continue;
                }

                for arg in args {
                    let mut replacement = String::new();
                    replacement.push_str(&selector[..start]);
                    replacement.push_str(arg);
                    replacement.push_str(&selector[end..]);
                    expanded.push(replacement);
                }
                changed = true;
            } else {
                expanded.push(selector);
            }
        }

        selectors = dedupe_selectors(expanded);
        if !changed {
            break;
        }
    }

    selectors
}

fn find_selector_function_call<'a>(
    value: &'a str,
    names: &[&str],
) -> Option<(usize, usize, &'a str)> {
    let mut bracket_depth = 0_i32;
    for (idx, ch) in value.char_indices() {
        match ch {
            '[' => bracket_depth += 1,
            ']' => bracket_depth = (bracket_depth - 1).max(0),
            ':' if bracket_depth == 0 => {
                let rest = &value[idx..];
                if let Some((_, body, end_idx)) = extract_selector_function(rest, names) {
                    return Some((idx, idx + end_idx, body));
                }
            }
            _ => {}
        }
    }
    None
}

fn dedupe_selectors(selectors: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for selector in selectors {
        if !out.iter().any(|existing| existing == &selector) {
            out.push(selector);
        }
    }
    out
}

fn strip_selector_functions(raw: &str, names: &[&str]) -> String {
    let mut out = String::new();
    let mut chars = raw.char_indices().peekable();
    while let Some((idx, ch)) = chars.next() {
        if ch == ':' {
            let rest = &raw[idx..];
            if let Some((_, _, end_idx)) = extract_selector_function(rest, names) {
                while matches!(chars.peek(), Some((next_idx, _)) if *next_idx < idx + end_idx) {
                    let _ = chars.next();
                }
                continue;
            }
        }
        out.push(ch);
    }
    out
}

fn extract_selector_function<'a>(
    value: &'a str,
    names: &[&str],
) -> Option<(usize, &'a str, usize)> {
    let name = names.iter().find(|name| value.starts_with(**name))?;
    let open = name.len();
    if value.as_bytes().get(open).copied()? != b'(' {
        return None;
    }
    let mut depth = 0_i32;
    for (idx, ch) in value[open..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    let close = open + idx;
                    return Some((name.len(), &value[open + 1..close], close + 1));
                }
            }
            _ => {}
        }
    }
    None
}

fn selector_specificity_for_cascade(raw: &str) -> (u16, u16, u16) {
    let selector = raw.trim().to_ascii_lowercase();
    selector_specificity_sequence(&selector)
}

fn selector_specificity_sequence(selector: &str) -> (u16, u16, u16) {
    let mut specificity = (0, 0, 0);
    let mut bracket_depth = 0_i32;
    let mut paren_depth = 0_i32;
    let mut start = 0usize;
    for (idx, ch) in selector.char_indices() {
        match ch {
            '[' if paren_depth == 0 => bracket_depth += 1,
            ']' if paren_depth == 0 => bracket_depth = (bracket_depth - 1).max(0),
            '(' if bracket_depth == 0 => paren_depth += 1,
            ')' if bracket_depth == 0 => paren_depth = (paren_depth - 1).max(0),
            ',' | '>' | '+' | '~' if bracket_depth == 0 && paren_depth == 0 => {
                add_specificity(
                    &mut specificity,
                    compound_specificity(&selector[start..idx]),
                );
                start = idx + ch.len_utf8();
            }
            ch if ch.is_whitespace() && bracket_depth == 0 && paren_depth == 0 => {
                add_specificity(
                    &mut specificity,
                    compound_specificity(&selector[start..idx]),
                );
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    add_specificity(&mut specificity, compound_specificity(&selector[start..]));
    specificity
}

fn compound_specificity(compound: &str) -> (u16, u16, u16) {
    let mut specificity = (0, 0, 0);
    let mut cursor = 0usize;
    let mut expect_type_selector = true;
    while cursor < compound.len() {
        let Some((relative_idx, ch)) = compound[cursor..].char_indices().next() else {
            break;
        };
        let idx = cursor + relative_idx;
        match ch {
            '*' => {
                expect_type_selector = false;
                cursor = idx + ch.len_utf8();
            }
            '#' => {
                specificity.0 += 1;
                cursor = consume_css_identifier(compound, idx + ch.len_utf8());
                expect_type_selector = false;
            }
            '.' => {
                specificity.1 += 1;
                cursor = consume_css_identifier(compound, idx + ch.len_utf8());
                expect_type_selector = false;
            }
            '[' => {
                specificity.1 += 1;
                cursor = find_matching_square_bracket(compound, idx)
                    .map(|end| end + 1)
                    .unwrap_or(compound.len());
                expect_type_selector = false;
            }
            ':' => {
                let rest = &compound[idx..];
                if rest.starts_with("::") {
                    specificity.2 += 1;
                    cursor = consume_css_identifier(compound, idx + 2);
                } else if let Some((_, body, end)) = extract_selector_function(rest, &[":where"]) {
                    let _ = body;
                    cursor = idx + end;
                } else if let Some((_, body, end)) =
                    extract_selector_function(rest, &[":is", ":not", ":has"])
                {
                    add_specificity(&mut specificity, max_selector_list_specificity(body));
                    cursor = idx + end;
                } else {
                    specificity.1 += 1;
                    let after_name = consume_css_identifier(compound, idx + 1);
                    cursor = if compound.as_bytes().get(after_name) == Some(&b'(') {
                        matching_function_end(&compound[after_name..])
                            .map(|end| after_name + end + 1)
                            .unwrap_or(after_name)
                    } else {
                        after_name
                    };
                }
                expect_type_selector = false;
            }
            ch if is_css_identifier_start(ch) && expect_type_selector => {
                specificity.2 += 1;
                cursor = consume_css_identifier(compound, idx + ch.len_utf8());
                expect_type_selector = false;
            }
            _ => {
                cursor = idx + ch.len_utf8();
            }
        }
    }
    specificity
}

fn max_selector_list_specificity(selector_list: &str) -> (u16, u16, u16) {
    split_top_level(selector_list, &[','])
        .into_iter()
        .map(selector_specificity_sequence)
        .max()
        .unwrap_or((0, 0, 0))
}

fn add_specificity(total: &mut (u16, u16, u16), specificity: (u16, u16, u16)) {
    total.0 += specificity.0;
    total.1 += specificity.1;
    total.2 += specificity.2;
}

fn find_matching_square_bracket(value: &str, open: usize) -> Option<usize> {
    let mut quote = None;
    for (offset, ch) in value[open + 1..].char_indices() {
        if let Some(active_quote) = quote {
            if ch == active_quote {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => quote = Some(ch),
            ']' => return Some(open + 1 + offset),
            _ => {}
        }
    }
    None
}

fn consume_css_identifier(value: &str, start: usize) -> usize {
    let mut cursor = start;
    while let Some((relative_idx, ch)) = value[cursor..].char_indices().next() {
        if !is_css_identifier_char(ch) {
            break;
        }
        cursor += relative_idx + ch.len_utf8();
    }
    cursor
}

fn is_css_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_' || ch == '-'
}

fn is_css_identifier_char(ch: char) -> bool {
    is_css_identifier_start(ch) || ch.is_ascii_digit()
}

fn parse_rules(css: &str, media: MediaContext) -> Vec<Rule> {
    let mut rules = Vec::new();
    for block in collect_supported_rule_blocks(css, media) {
        let css = expand_nested_css(
            &format!("{}{{{}}}", block.selector, block.declarations),
            media,
        );
        for part in css.split('}') {
            let Some((selector, declarations)) = part.split_once('{') else {
                continue;
            };
            for selector in split_top_level(selector, &[',']) {
                let selector = normalize_selector_case_for_parse(selector.trim());
                let specificity = selector_specificity_for_cascade(&selector);
                for selector in expand_selector_list_functions(&selector) {
                    let Some((selector, pseudo)) = split_pseudo_element_selector(&selector) else {
                        continue;
                    };
                    let Some(selector) = Selector::parse(&selector) else {
                        continue;
                    };
                    rules.push(Rule {
                        selector,
                        pseudo,
                        specificity,
                        layer_index: block.layer_index,
                        declarations: parse_declarations(declarations),
                    });
                }
            }
        }
    }
    rules
}

#[derive(Debug, Clone)]
struct CssRuleBlock {
    selector: String,
    declarations: String,
    layer_index: Option<usize>,
}

fn collect_supported_rule_blocks(css: &str, media: MediaContext) -> Vec<CssRuleBlock> {
    let mut layers = CascadeLayerAccumulator::default();
    let mut blocks = Vec::new();
    collect_supported_rule_blocks_inner(css, media, None, &mut layers, &mut blocks);
    blocks
}

fn collect_supported_rule_blocks_inner(
    css: &str,
    media: MediaContext,
    current_layer: Option<usize>,
    layers: &mut CascadeLayerAccumulator,
    blocks: &mut Vec<CssRuleBlock>,
) {
    register_cascade_layer_statements(css, layers);
    let mut cursor = 0usize;
    while let Some(open_rel) = css[cursor..].find('{') {
        let open = cursor + open_rel;
        let selector = css[cursor..open].trim();
        let selector = selector.rsplit(';').next().unwrap_or(selector).trim();
        let selector_lower = selector.to_ascii_lowercase();
        let Some(close) = find_matching_brace(css, open) else {
            break;
        };
        let body = &css[open + 1..close];
        if selector_lower.starts_with("@media") {
            if media_query_matches(selector, media) {
                collect_supported_rule_blocks_inner(body, media, current_layer, layers, blocks);
            }
        } else if selector_lower.starts_with("@supports") {
            if supports_query_matches(selector) {
                collect_supported_rule_blocks_inner(body, media, current_layer, layers, blocks);
            }
        } else if selector_lower.starts_with("@layer") {
            let layer_name = parse_layer_block_name(selector);
            let layer_index = layers.ensure_layer_index(layer_name.as_deref().unwrap_or(""));
            collect_supported_rule_blocks_inner(body, media, Some(layer_index), layers, blocks);
        } else if selector_lower.starts_with("@container") {
            if container_query_matches(selector, media) {
                collect_supported_rule_blocks_inner(body, media, current_layer, layers, blocks);
            }
        } else if selector_lower.starts_with("@scope") {
            let scoped = flatten_scoped_css(selector, body, media);
            collect_supported_rule_blocks_inner(&scoped, media, current_layer, layers, blocks);
        } else if !selector_lower.starts_with('@') {
            blocks.push(CssRuleBlock {
                selector: selector.to_string(),
                declarations: body.to_string(),
                layer_index: current_layer,
            });
        }
        cursor = close + 1;
    }
}

fn normalize_selector_case_for_parse(selector: &str) -> String {
    let mut out = String::with_capacity(selector.len());
    let mut bracket_depth = 0_i32;
    let mut quote = None;

    for ch in selector.chars() {
        if let Some(active_quote) = quote {
            out.push(ch);
            if ch == active_quote {
                quote = None;
            }
            continue;
        }

        match ch {
            '"' | '\'' if bracket_depth > 0 => {
                quote = Some(ch);
                out.push(ch);
            }
            '[' => {
                bracket_depth += 1;
                out.push(ch);
            }
            ']' => {
                bracket_depth = (bracket_depth - 1).max(0);
                out.push(ch);
            }
            ch if bracket_depth > 0 => out.push(ch),
            ch => out.extend(ch.to_lowercase()),
        }
    }

    out
}

fn split_pseudo_element_selector(selector: &str) -> Option<(String, Option<PseudoElement>)> {
    let selector = selector.trim();
    for (token, pseudo) in [
        ("::before", PseudoElement::Before),
        (":before", PseudoElement::Before),
        ("::after", PseudoElement::After),
        (":after", PseudoElement::After),
    ] {
        if let Some(prefix) = selector.strip_suffix(token) {
            let base = prefix.trim();
            return Some((
                if base.is_empty() {
                    "*".to_string()
                } else {
                    base.to_string()
                },
                Some(pseudo),
            ));
        }
    }
    if contains_unsupported_pseudo_element(selector) {
        return None;
    }
    Some((selector.to_string(), None))
}

fn contains_unsupported_pseudo_element(selector: &str) -> bool {
    let mut bracket_depth = 0_i32;
    let mut paren_depth = 0_i32;
    let mut quote = None;
    let mut cursor = 0usize;

    while cursor < selector.len() {
        let Some((relative_idx, ch)) = selector[cursor..].char_indices().next() else {
            break;
        };
        let idx = cursor + relative_idx;

        if let Some(active_quote) = quote {
            if ch == active_quote {
                quote = None;
            }
            cursor = idx + ch.len_utf8();
            continue;
        }

        match ch {
            '"' | '\'' if bracket_depth > 0 => {
                quote = Some(ch);
                cursor = idx + ch.len_utf8();
            }
            '[' if paren_depth == 0 => {
                bracket_depth += 1;
                cursor = idx + ch.len_utf8();
            }
            ']' if paren_depth == 0 => {
                bracket_depth = (bracket_depth - 1).max(0);
                cursor = idx + ch.len_utf8();
            }
            '(' if bracket_depth == 0 => {
                paren_depth += 1;
                cursor = idx + ch.len_utf8();
            }
            ')' if bracket_depth == 0 => {
                paren_depth = (paren_depth - 1).max(0);
                cursor = idx + ch.len_utf8();
            }
            ':' if bracket_depth == 0 && paren_depth == 0 => {
                if selector.as_bytes().get(idx + 1) == Some(&b':') {
                    return true;
                }
                let name_start = idx + 1;
                let name_end = consume_css_identifier(selector, name_start);
                let name = &selector[name_start..name_end];
                if matches!(
                    name,
                    "first-line" | "first-letter" | "selection" | "marker" | "placeholder"
                ) {
                    return true;
                }
                cursor = name_end.max(idx + ch.len_utf8());
            }
            _ => cursor = idx + ch.len_utf8(),
        }
    }

    false
}

fn parse_custom_properties(css: &str, media: MediaContext) -> BTreeMap<String, String> {
    let css = flatten_supported_css(css, media);
    let css = expand_nested_css(&css, media);
    let mut variables = BTreeMap::new();
    for part in css.split('}') {
        let Some((selectors, declarations)) = part.split_once('{') else {
            continue;
        };
        let is_root_rule = split_top_level(selectors, &[','])
            .iter()
            .flat_map(|selector| expand_selector_list_functions(selector.trim()))
            .filter_map(|selector| Selector::parse(&selector.to_ascii_lowercase()))
            .any(|selector| selector.is_html_root());
        if !is_root_rule {
            continue;
        }
        for declaration in cascade_ordered_declarations(parse_declarations(declarations)) {
            if declaration.property.starts_with("--") {
                variables.insert(declaration.property, declaration.value);
            }
        }
    }
    variables
}

fn parse_registered_custom_property_initials(
    css: &str,
    media: MediaContext,
) -> BTreeMap<String, String> {
    let mut variables = BTreeMap::new();
    collect_registered_custom_property_initials(css, media, &mut variables);
    variables
}

fn collect_registered_custom_property_initials(
    css: &str,
    media: MediaContext,
    variables: &mut BTreeMap<String, String>,
) {
    let mut cursor = 0usize;
    while let Some(open_rel) = css[cursor..].find('{') {
        let open = cursor + open_rel;
        let selector = css[cursor..open].trim();
        let selector = selector.rsplit(';').next().unwrap_or(selector).trim();
        let selector_lower = selector.to_ascii_lowercase();
        let Some(close) = find_matching_brace(css, open) else {
            break;
        };
        let body = &css[open + 1..close];

        if selector_lower.starts_with("@media") {
            if media_query_matches(selector, media) {
                collect_registered_custom_property_initials(body, media, variables);
            }
        } else if selector_lower.starts_with("@supports") {
            if supports_query_matches(selector) {
                collect_registered_custom_property_initials(body, media, variables);
            }
        } else if selector_lower.starts_with("@layer") || selector_lower.starts_with("@scope") {
            collect_registered_custom_property_initials(body, media, variables);
        } else if selector_lower.starts_with("@container") {
            if container_query_matches(selector, media) {
                collect_registered_custom_property_initials(body, media, variables);
            }
        } else if selector_lower.starts_with("@property") {
            let name = selector["@property".len()..].trim();
            if name.starts_with("--") {
                for declaration in parse_declarations(body) {
                    if declaration.property == "initial-value" && !declaration.value.is_empty() {
                        variables.insert(name.to_string(), declaration.value);
                    }
                }
            }
        }

        cursor = close + 1;
    }
}

fn parse_page_style(css: &str) -> PageStyle {
    let mut page = PageStyle::default();
    let mut cursor = 0usize;
    while let Some(open_rel) = css[cursor..].find('{') {
        let open = cursor + open_rel;
        let selector = css[cursor..open].trim().to_ascii_lowercase();
        let Some(close) = find_matching_brace(css, open) else {
            break;
        };
        let body = &css[open + 1..close];
        if selector.starts_with("@media") {
            if selector.contains("print") {
                page.merge(parse_page_style(body));
            }
        } else if selector.starts_with("@supports") || selector.starts_with("@layer") {
            page.merge(parse_page_style(body));
        } else if selector.starts_with("@page") {
            page.merge(parse_page_declarations(body));
        }
        cursor = close + 1;
    }
    page
}

fn parse_page_declarations(input: &str) -> PageStyle {
    let mut page = PageStyle::default();
    for declaration in parse_declarations(input) {
        let value = declaration.value.to_ascii_lowercase();
        match declaration.property.as_str() {
            "size" => {
                if value.contains("a4") {
                    page.width_pt = Some(PageOptions::A4_WIDTH_PT);
                    page.height_pt = Some(PageOptions::A4_HEIGHT_PT);
                    if value.contains("landscape") {
                        std::mem::swap(&mut page.width_pt, &mut page.height_pt);
                    }
                } else if value.contains("letter") {
                    page.width_pt = Some(612.0);
                    page.height_pt = Some(792.0);
                    if value.contains("landscape") {
                        std::mem::swap(&mut page.width_pt, &mut page.height_pt);
                    }
                } else {
                    let dimensions = value.split_whitespace().collect::<Vec<_>>();
                    if dimensions.len() >= 2 {
                        if let (Some(width), Some(height)) =
                            (parse_pt_or_px(dimensions[0]), parse_pt_or_px(dimensions[1]))
                        {
                            page.width_pt = Some(width.max(1.0));
                            page.height_pt = Some(height.max(1.0));
                        }
                    }
                }
            }
            "margin" => {
                if let Some([top, right, bottom, left]) = parse_box_shorthand(&value) {
                    page.margin_top_pt = Some(top);
                    page.margin_right_pt = Some(right);
                    page.margin_bottom_pt = Some(bottom);
                    page.margin_left_pt = Some(left);
                }
            }
            "margin-top" => {
                page.margin_top_pt = parse_pt_or_px(&value);
            }
            "margin-right" => {
                page.margin_right_pt = parse_pt_or_px(&value);
            }
            "margin-bottom" => {
                page.margin_bottom_pt = parse_pt_or_px(&value);
            }
            "margin-left" => {
                page.margin_left_pt = parse_pt_or_px(&value);
            }
            _ => {}
        }
    }
    page
}

fn flatten_supported_css(css: &str, media: MediaContext) -> String {
    let mut layers = CascadeLayerAccumulator::default();
    let out = flatten_supported_css_inner(css, media, &mut layers);
    let mut layered_out = layers.render();
    if !layered_out.is_empty() {
        layered_out.push_str(&out);
        return layered_out;
    }
    out
}

#[derive(Default)]
struct CascadeLayerAccumulator {
    order: Vec<String>,
    chunks: BTreeMap<String, String>,
    anonymous_count: usize,
}

impl CascadeLayerAccumulator {
    fn ensure_layer(&mut self, name: &str) -> String {
        let name = name.trim();
        let key = if name.is_empty() {
            self.anonymous_count += 1;
            format!("__anonymous_layer_{}", self.anonymous_count)
        } else {
            name.to_ascii_lowercase()
        };
        if !self.order.iter().any(|existing| existing == &key) {
            self.order.push(key.clone());
        }
        self.chunks.entry(key.clone()).or_default();
        key
    }

    fn ensure_layer_index(&mut self, name: &str) -> usize {
        let key = self.ensure_layer(name);
        self.order
            .iter()
            .position(|existing| existing == &key)
            .unwrap_or_else(|| self.order.len().saturating_sub(1))
    }

    fn register_layer_list(&mut self, names: &str) {
        for name in split_top_level(names, &[',']) {
            let name = name.trim();
            if !name.is_empty() {
                self.ensure_layer(name);
            }
        }
    }

    fn push_layer_chunk(&mut self, name: &str, css: &str) {
        let key = self.ensure_layer(name);
        self.chunks.entry(key).or_default().push_str(css);
    }

    fn render(&self) -> String {
        let mut out = String::new();
        for name in &self.order {
            if let Some(chunk) = self.chunks.get(name) {
                out.push_str(chunk);
            }
        }
        out
    }
}

fn flatten_supported_css_inner(
    css: &str,
    media: MediaContext,
    layers: &mut CascadeLayerAccumulator,
) -> String {
    register_cascade_layer_statements(css, layers);
    let mut out = String::new();
    let mut cursor = 0usize;
    while let Some(open_rel) = css[cursor..].find('{') {
        let open = cursor + open_rel;
        let selector = css[cursor..open].trim();
        let selector = selector.rsplit(';').next().unwrap_or(selector).trim();
        let selector_lower = selector.to_ascii_lowercase();
        let Some(close) = find_matching_brace(css, open) else {
            break;
        };
        let body = &css[open + 1..close];
        if selector_lower.starts_with("@media") {
            if media_query_matches(selector, media) {
                out.push_str(&flatten_supported_css_inner(body, media, layers));
            }
        } else if selector_lower.starts_with("@supports") {
            if supports_query_matches(selector) {
                out.push_str(&flatten_supported_css_inner(body, media, layers));
            }
        } else if selector_lower.starts_with("@layer") {
            let layer_name = parse_layer_block_name(selector);
            let flattened = flatten_supported_css_inner(body, media, layers);
            layers.push_layer_chunk(layer_name.as_deref().unwrap_or(""), &flattened);
        } else if selector_lower.starts_with("@container") {
            if container_query_matches(selector, media) {
                out.push_str(&flatten_supported_css_inner(body, media, layers));
            }
        } else if selector_lower.starts_with("@scope") {
            out.push_str(&flatten_scoped_css(selector, body, media));
        } else if !selector_lower.starts_with('@') {
            out.push_str(selector);
            out.push('{');
            out.push_str(body);
            out.push_str("}\n");
        }
        cursor = close + 1;
    }
    out
}

fn register_cascade_layer_statements(css: &str, layers: &mut CascadeLayerAccumulator) {
    let mut cursor = 0usize;
    while let Some(start_rel) = find_ascii_case_insensitive(&css[cursor..], "@layer") {
        let start = cursor + start_rel;
        let after = start + "@layer".len();
        let next_semicolon = css[after..].find(';').map(|idx| after + idx);
        let next_open = css[after..].find('{').map(|idx| after + idx);
        if let Some(semicolon) = next_semicolon {
            if next_open.is_none_or(|open| semicolon < open) {
                layers.register_layer_list(&css[after..semicolon]);
                cursor = semicolon + 1;
                continue;
            }
        }
        cursor = after;
    }
}

fn parse_layer_block_name(selector: &str) -> Option<String> {
    let selector = selector.trim();
    if !selector.to_ascii_lowercase().starts_with("@layer") {
        return None;
    }
    let name = selector["@layer".len()..].trim();
    if name.is_empty() {
        None
    } else {
        split_top_level(name, &[','])
            .first()
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty())
    }
}

fn flatten_scoped_css(scope_rule: &str, body: &str, media: MediaContext) -> String {
    let flattened = flatten_supported_css(body, media);
    let Some(scope_root) = parse_scope_root_selector(scope_rule) else {
        return flattened;
    };
    prefix_scoped_rules(&flattened, &scope_root)
}

fn parse_scope_root_selector(scope_rule: &str) -> Option<String> {
    let open = scope_rule.find('(')?;
    let close = matching_function_end(&scope_rule[open..]).map(|idx| open + idx)?;
    let root = scope_rule[open + 1..close].trim();
    (!root.is_empty()).then(|| root.to_string())
}

fn prefix_scoped_rules(css: &str, scope_root: &str) -> String {
    let mut out = String::new();
    let mut cursor = 0usize;
    while let Some(open_rel) = css[cursor..].find('{') {
        let open = cursor + open_rel;
        let selector = css[cursor..open].trim();
        let Some(close) = find_matching_brace(css, open) else {
            break;
        };
        let body = &css[open + 1..close];
        if !selector.starts_with('@') {
            let scoped_selectors = split_top_level(selector, &[','])
                .into_iter()
                .map(|selector| scoped_selector(scope_root, selector.trim()))
                .filter(|selector| !selector.is_empty())
                .collect::<Vec<_>>();
            if !scoped_selectors.is_empty() {
                out.push_str(&scoped_selectors.join(", "));
                out.push('{');
                out.push_str(body);
                out.push_str("}\n");
            }
        }
        cursor = close + 1;
    }
    out
}

fn scoped_selector(scope_root: &str, selector: &str) -> String {
    if selector.is_empty() {
        return String::new();
    }
    if selector.contains(":scope") {
        return selector.replace(":scope", scope_root);
    }
    if selector.contains('&') {
        return selector.replace('&', scope_root);
    }
    format!("{scope_root} {selector}")
}

fn expand_nested_css(css: &str, media: MediaContext) -> String {
    let mut out = String::new();
    let mut cursor = 0usize;
    while let Some(open_rel) = css[cursor..].find('{') {
        let open = cursor + open_rel;
        let selector = css[cursor..open].trim();
        let Some(close) = find_matching_brace(css, open) else {
            break;
        };
        let body = &css[open + 1..close];
        if !selector.starts_with('@') {
            expand_nested_rule(selector, body, media, &mut out);
        }
        cursor = close + 1;
    }
    out
}

fn expand_nested_rule(selector: &str, body: &str, media: MediaContext, out: &mut String) {
    let mut declarations = String::new();
    let mut nested_blocks = Vec::new();
    let mut cursor = 0usize;

    while let Some(open_rel) = body[cursor..].find('{') {
        let open = cursor + open_rel;
        let prefix = &body[cursor..open];
        let (declaration_prefix, nested_selector) = split_nested_selector_prefix(prefix);
        declarations.push_str(declaration_prefix);

        let Some(close) = find_matching_brace(body, open) else {
            break;
        };
        nested_blocks.push((
            nested_selector.trim().to_string(),
            body[open + 1..close].to_string(),
        ));
        cursor = close + 1;
    }
    declarations.push_str(&body[cursor..]);

    if !declarations.trim().is_empty() {
        out.push_str(selector);
        out.push('{');
        out.push_str(declarations.trim());
        out.push_str("}\n");
    }

    for (nested_selector, nested_body) in nested_blocks {
        let nested_selector = nested_selector.trim();
        let nested_selector_lower = nested_selector.to_ascii_lowercase();
        if nested_selector_lower.starts_with("@media") {
            if media_query_matches(nested_selector, media) {
                expand_nested_rule(selector, &nested_body, media, out);
            }
            continue;
        }
        if nested_selector_lower.starts_with("@supports") {
            if supports_query_matches(nested_selector) {
                expand_nested_rule(selector, &nested_body, media, out);
            }
            continue;
        }
        if nested_selector_lower.starts_with("@layer") {
            expand_nested_rule(selector, &nested_body, media, out);
            continue;
        }
        if nested_selector_lower.starts_with("@container") {
            if container_query_matches(nested_selector, media) {
                expand_nested_rule(selector, &nested_body, media, out);
            }
            continue;
        }

        for combined in combine_nested_selectors(selector, nested_selector) {
            expand_nested_rule(&combined, &nested_body, media, out);
        }
    }
}

fn split_nested_selector_prefix(prefix: &str) -> (&str, &str) {
    if let Some(last_semicolon) = prefix.rfind(';') {
        prefix.split_at(last_semicolon + 1)
    } else {
        ("", prefix)
    }
}

fn combine_nested_selectors(parent_selector: &str, nested_selector: &str) -> Vec<String> {
    let parents = split_top_level(parent_selector, &[',']);
    let children = split_top_level(nested_selector, &[',']);
    let mut selectors = Vec::new();
    for parent in parents {
        let parent = parent.trim();
        if parent.is_empty() {
            continue;
        }
        for child in &children {
            let child = child.trim();
            if child.is_empty() {
                continue;
            }
            if child.contains('&') {
                selectors.push(child.replace('&', parent));
            } else {
                selectors.push(format!("{parent} {child}"));
            }
        }
    }
    selectors
}

fn supports_query_matches(selector: &str) -> bool {
    let selector = selector.trim();
    let condition = if selector.to_ascii_lowercase().starts_with("@supports") {
        selector["@supports".len()..].trim()
    } else {
        selector
    };
    eval_supports_condition(condition).unwrap_or(true)
}

fn eval_supports_condition(condition: &str) -> Option<bool> {
    let condition = strip_supports_outer_parentheses(condition.trim());
    if condition.is_empty() {
        return Some(true);
    }

    if let Some(parts) = split_supports_top_level_keyword(condition, "or") {
        return Some(
            parts
                .into_iter()
                .any(|part| eval_supports_condition(part).unwrap_or(true)),
        );
    }
    if let Some(parts) = split_supports_top_level_keyword(condition, "and") {
        return Some(
            parts
                .into_iter()
                .all(|part| eval_supports_condition(part).unwrap_or(true)),
        );
    }

    let lower = condition.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("not ") {
        let original_rest = &condition[condition.len() - rest.len()..];
        return Some(!eval_supports_condition(original_rest).unwrap_or(false));
    }
    if lower.starts_with("not(") {
        let rest = &condition["not".len()..];
        return Some(!eval_supports_condition(rest).unwrap_or(false));
    }
    if lower.starts_with("selector(") {
        return Some(true);
    }

    if let Some((property, value)) = condition.split_once(':') {
        let property = property.trim().to_ascii_lowercase();
        let value = value.trim().to_ascii_lowercase();
        return Some(
            is_supported_property(&property) && supports_property_value(&property, &value),
        );
    }

    Some(true)
}

fn supports_property_value(_property: &str, _value: &str) -> bool {
    true
}

fn strip_supports_outer_parentheses(mut value: &str) -> &str {
    loop {
        let trimmed = value.trim();
        if !trimmed.starts_with('(') {
            return trimmed;
        }
        let Some(end) = matching_function_end(trimmed) else {
            return trimmed;
        };
        if end + 1 != trimmed.len() {
            return trimmed;
        }
        value = &trimmed[1..end];
    }
}

fn split_supports_top_level_keyword<'a>(value: &'a str, keyword: &str) -> Option<Vec<&'a str>> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth = 0i32;
    let mut quote = None;
    let lower = value.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let keyword_bytes = keyword.as_bytes();
    let mut cursor = 0usize;

    while cursor < value.len() {
        let ch = value[cursor..].chars().next()?;
        if let Some(active_quote) = quote {
            if ch == active_quote {
                quote = None;
            }
            cursor += ch.len_utf8();
            continue;
        }

        match ch {
            '"' | '\'' => quote = Some(ch),
            '(' => depth += 1,
            ')' => depth = (depth - 1).max(0),
            _ if depth == 0
                && bytes[cursor..].starts_with(keyword_bytes)
                && is_supports_keyword_boundary(bytes, cursor, keyword_bytes.len()) =>
            {
                parts.push(value[start..cursor].trim());
                cursor += keyword_bytes.len();
                start = cursor;
                continue;
            }
            _ => {}
        }
        cursor += ch.len_utf8();
    }

    if parts.is_empty() {
        return None;
    }
    parts.push(value[start..].trim());
    Some(parts)
}

fn is_supports_keyword_boundary(bytes: &[u8], start: usize, len: usize) -> bool {
    let before = start.checked_sub(1).and_then(|idx| bytes.get(idx)).copied();
    let after = bytes.get(start + len).copied();
    before.is_none_or(|byte| byte.is_ascii_whitespace() || byte == b'(')
        && after.is_none_or(|byte| byte.is_ascii_whitespace() || byte == b')')
}

fn container_query_matches(selector: &str, media: MediaContext) -> bool {
    let selector = selector.trim().to_ascii_lowercase();
    if selector.contains(" not ")
        || selector.starts_with("@container not")
        || selector.starts_with("@container(not")
    {
        return false;
    }

    if let Some(max_width) = parse_container_size_constraint(&selector, true) {
        if media.width_px > max_width {
            return false;
        }
    }
    if let Some(min_width) = parse_container_size_constraint(&selector, false) {
        if media.width_px < min_width {
            return false;
        }
    }
    if let Some(max_width) = parse_container_size_range_constraint(&selector, true) {
        if media.width_px > max_width {
            return false;
        }
    }
    if let Some(min_width) = parse_container_size_range_constraint(&selector, false) {
        if media.width_px < min_width {
            return false;
        }
    }

    true
}

fn parse_container_size_constraint(selector: &str, max_constraint: bool) -> Option<f32> {
    let properties = if max_constraint {
        ["max-width", "max-inline-size"]
    } else {
        ["min-width", "min-inline-size"]
    };
    properties
        .into_iter()
        .find_map(|property| parse_media_width_constraint(selector, property))
}

fn parse_container_size_range_constraint(selector: &str, max_constraint: bool) -> Option<f32> {
    parse_container_range_axis_constraint(selector, "inline-size", max_constraint)
        .or_else(|| parse_container_range_axis_constraint(selector, "width", max_constraint))
}

fn parse_container_range_axis_constraint(
    selector: &str,
    axis: &str,
    max_constraint: bool,
) -> Option<f32> {
    let forward_patterns = if max_constraint {
        [format!("{axis} <="), format!("{axis} <")]
    } else {
        [format!("{axis} >="), format!("{axis} >")]
    };
    for pattern in forward_patterns {
        if let Some(start) = selector.find(&pattern) {
            let after = selector[start + pattern.len()..].trim_start();
            return parse_media_length_token(after);
        }
    }

    let axis_start = selector.find(axis)?;
    let before_axis = selector[..axis_start].trim_end();
    let reversed_tokens = if max_constraint {
        [">=", ">"]
    } else {
        ["<=", "<"]
    };
    for token in reversed_tokens {
        if before_axis.ends_with(token) {
            let value = before_axis[..before_axis.len() - token.len()].trim_end();
            return value
                .split_whitespace()
                .last()
                .and_then(parse_media_length_token);
        }
    }
    None
}

fn media_query_matches(selector: &str, media: MediaContext) -> bool {
    let selector = selector.trim();
    let condition = if selector.to_ascii_lowercase().starts_with("@media") {
        selector["@media".len()..].trim()
    } else {
        selector
    };

    split_top_level(condition, &[','])
        .into_iter()
        .any(|query| single_media_query_matches(query.trim(), media))
}

fn single_media_query_matches(query: &str, media: MediaContext) -> bool {
    let mut selector = query.trim().to_ascii_lowercase();
    let negated = if let Some(rest) = selector.strip_prefix("not ") {
        let rest = rest.trim_start().to_string();
        selector = rest;
        true
    } else {
        false
    };
    if let Some(rest) = selector.strip_prefix("only ") {
        selector = rest.trim_start().to_string();
    }

    let matches = media_condition_matches(&selector, media);
    if negated {
        !matches
    } else {
        matches
    }
}

fn media_condition_matches(condition: &str, media: MediaContext) -> bool {
    let condition = strip_media_outer_parentheses(condition.trim());
    if condition.is_empty() {
        return true;
    }

    if let Some(parts) = split_supports_top_level_keyword(condition, "or") {
        return parts
            .into_iter()
            .any(|part| media_condition_matches(part, media));
    }
    if let Some(parts) = split_supports_top_level_keyword(condition, "and") {
        return parts
            .into_iter()
            .all(|part| media_condition_matches(part, media));
    }

    media_clause_matches(condition, media)
}

fn strip_media_outer_parentheses(mut value: &str) -> &str {
    loop {
        let trimmed = value.trim();
        if !trimmed.starts_with('(') {
            return trimmed;
        }
        let Some(end) = matching_function_end(trimmed) else {
            return trimmed;
        };
        if end + 1 != trimmed.len() {
            return trimmed;
        }
        value = &trimmed[1..end];
    }
}

fn media_clause_matches(selector: &str, media: MediaContext) -> bool {
    let mut matches = true;
    if selector.contains("screen") && media.print {
        matches = false;
    }
    if selector.contains("print") && !media.print {
        matches = false;
    }
    if let Some(max_width) = parse_media_size_constraint(selector, "width", true) {
        if media.width_px > max_width {
            matches = false;
        }
    }
    if let Some(max_width) = parse_media_width_range_constraint(selector, true) {
        if media.width_px > max_width {
            matches = false;
        }
    }
    if let Some(min_width) = parse_media_size_constraint(selector, "width", false) {
        if media.width_px < min_width {
            matches = false;
        }
    }
    if let Some(min_width) = parse_media_width_range_constraint(selector, false) {
        if media.width_px < min_width {
            matches = false;
        }
    }
    if let Some(max_height) = parse_media_size_constraint(selector, "height", true) {
        if media.height_px > max_height {
            matches = false;
        }
    }
    if let Some(max_height) = parse_media_size_range_constraint(selector, "height", true) {
        if media.height_px > max_height {
            matches = false;
        }
    }
    if let Some(min_height) = parse_media_size_constraint(selector, "height", false) {
        if media.height_px < min_height {
            matches = false;
        }
    }
    if let Some(min_height) = parse_media_size_range_constraint(selector, "height", false) {
        if media.height_px < min_height {
            matches = false;
        }
    }
    if !media_feature_value_matches(selector, "orientation", media_orientation(media)) {
        matches = false;
    }
    if !media_feature_value_matches(selector, "prefers-color-scheme", "light") {
        matches = false;
    }
    if !media_feature_value_matches(selector, "prefers-reduced-motion", "no-preference") {
        matches = false;
    }
    if !media_feature_value_matches(selector, "prefers-contrast", "no-preference") {
        matches = false;
    }
    if !media_feature_value_matches(selector, "hover", "none") {
        matches = false;
    }
    if !media_feature_value_matches(selector, "any-hover", "none") {
        matches = false;
    }
    if !media_feature_value_matches(selector, "pointer", "none") {
        matches = false;
    }
    if !media_feature_value_matches(selector, "any-pointer", "none") {
        matches = false;
    }
    if !media_feature_value_matches(selector, "update", "none") {
        matches = false;
    }
    if !media_feature_value_matches(selector, "color-gamut", "srgb") {
        matches = false;
    }

    matches
}

fn media_orientation(media: MediaContext) -> &'static str {
    if media.width_px > media.height_px {
        "landscape"
    } else {
        "portrait"
    }
}

fn media_feature_value_matches(selector: &str, feature: &str, expected: &str) -> bool {
    let Some(actual) = parse_media_feature_value(selector, feature) else {
        return true;
    };
    actual == expected
}

fn parse_media_feature_value(selector: &str, feature: &str) -> Option<String> {
    let start = selector.find(feature)?;
    let after_property = &selector[start + feature.len()..];
    let colon = after_property.find(':')?;
    let after_colon = after_property[colon + 1..].trim_start();
    let end = after_colon
        .char_indices()
        .find_map(|(idx, ch)| matches!(ch, ')' | ' ' | '\t' | '\n').then_some(idx))
        .unwrap_or(after_colon.len());
    Some(
        after_colon[..end]
            .trim()
            .trim_matches(')')
            .to_ascii_lowercase(),
    )
}

fn parse_media_size_range_constraint(
    selector: &str,
    axis: &str,
    max_constraint: bool,
) -> Option<f32> {
    let patterns = if max_constraint {
        [format!("{axis} <="), format!("{axis} <")]
    } else {
        [format!("{axis} >="), format!("{axis} >")]
    };
    for pattern in patterns {
        if let Some(start) = selector.find(&pattern) {
            let after = selector[start + pattern.len()..].trim_start();
            return parse_media_length_token(after);
        }
    }

    let axis_start = selector.find(axis)?;
    let before_axis = selector[..axis_start].trim_end();
    let reversed_patterns = if max_constraint {
        [">=", ">"]
    } else {
        ["<=", "<"]
    };
    for pattern in reversed_patterns {
        if before_axis.ends_with(pattern) {
            let value = before_axis[..before_axis.len() - pattern.len()].trim_end();
            return value
                .split_whitespace()
                .last()
                .and_then(parse_media_length_token);
        }
    }
    None
}

fn parse_media_size_constraint(selector: &str, axis: &str, max_constraint: bool) -> Option<f32> {
    let property = if max_constraint {
        format!("max-{axis}")
    } else {
        format!("min-{axis}")
    };
    parse_media_axis_constraint(selector, &property)
}

fn parse_media_axis_constraint(selector: &str, property: &str) -> Option<f32> {
    let start = selector.find(property)?;
    let after_property = &selector[start + property.len()..];
    let colon = after_property.find(':')?;
    let after_colon = after_property[colon + 1..].trim_start();
    let end = after_colon
        .char_indices()
        .find_map(|(idx, ch)| matches!(ch, ')' | ' ' | '\t' | '\n').then_some(idx))
        .unwrap_or(after_colon.len());
    parse_media_length_token(after_colon[..end].trim())
}

fn parse_media_width_range_constraint(selector: &str, max_constraint: bool) -> Option<f32> {
    parse_media_size_range_constraint(selector, "width", max_constraint)
}

fn parse_media_width_constraint(selector: &str, property: &str) -> Option<f32> {
    parse_media_axis_constraint(selector, property)
}

fn parse_media_length_token(value: &str) -> Option<f32> {
    let value = value
        .trim()
        .trim_start_matches('(')
        .trim_end_matches(')')
        .trim();
    if let Some(px) = value.strip_suffix("px") {
        return px.trim().parse::<f32>().ok();
    }
    if let Some(rem) = value.strip_suffix("rem") {
        return rem.trim().parse::<f32>().ok().map(|value| value * 16.0);
    }
    if let Some(em) = value.strip_suffix("em") {
        return em.trim().parse::<f32>().ok().map(|value| value * 16.0);
    }
    value.parse::<f32>().ok()
}

fn find_matching_brace(input: &str, open: usize) -> Option<usize> {
    let mut depth = 0_i32;
    for (offset, ch) in input[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_declarations(input: &str) -> Vec<Declaration> {
    split_top_level(input, &[';'])
        .into_iter()
        .filter_map(|decl| {
            let (property, value) = decl.split_once(':')?;
            let raw_property = property.trim();
            let property = if raw_property.starts_with("--") {
                raw_property.to_string()
            } else {
                raw_property.to_ascii_lowercase()
            };
            let (raw_value, important) = strip_important_flag(value.trim());
            let value = raw_value.to_string();
            Some(Declaration {
                property,
                value,
                important,
            })
        })
        .collect()
}

fn strip_css_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut quote: Option<char> = None;
    while let Some(ch) = chars.next() {
        if let Some(active_quote) = quote {
            out.push(ch);
            if ch == '\\' {
                if let Some(next) = chars.next() {
                    out.push(next);
                }
            } else if ch == active_quote {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => {
                quote = Some(ch);
                out.push(ch);
            }
            '/' if matches!(chars.peek(), Some('*')) => {
                chars.next();
                while let Some(comment_ch) = chars.next() {
                    if comment_ch == '*' && matches!(chars.peek(), Some('/')) {
                        chars.next();
                        break;
                    }
                }
                out.push(' ');
            }
            _ => out.push(ch),
        }
    }
    out
}

fn strip_important_flag(value: &str) -> (&str, bool) {
    let trimmed = value.trim();
    if trimmed.len() >= "!important".len()
        && trimmed[trimmed.len() - "!important".len()..].eq_ignore_ascii_case("!important")
    {
        (
            trimmed[..trimmed.len() - "!important".len()].trim_end(),
            true,
        )
    } else {
        (trimmed, false)
    }
}

fn parse_css_content_value(value: &str) -> Option<String> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("none")
        || value.eq_ignore_ascii_case("normal")
        || value.eq_ignore_ascii_case("unset")
        || value.eq_ignore_ascii_case("initial")
        || value.eq_ignore_ascii_case("inherit")
    {
        return None;
    }
    let mut out = String::new();
    let mut saw_string = false;
    for token in split_css_whitespace(value) {
        let token = token.trim();
        if let Some(text) = parse_css_quoted_string(token) {
            out.push_str(&text);
            saw_string = true;
        }
    }
    if saw_string {
        return Some(out);
    }
    parse_css_quoted_string(value)
        .or_else(|| Some(String::new()).filter(|_| value == "\"\"" || value == "''"))
}

fn parse_css_quoted_string(value: &str) -> Option<String> {
    let value = value.trim();
    let quote = value.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    if !value.ends_with(quote) || value.len() < 2 {
        return None;
    }
    Some(unescape_css_string(
        &value[quote.len_utf8()..value.len() - quote.len_utf8()],
    ))
}

fn unescape_css_string(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        let mut hex = String::new();
        while matches!(chars.peek(), Some(next) if next.is_ascii_hexdigit()) && hex.len() < 6 {
            hex.push(chars.next().unwrap());
        }
        if !hex.is_empty() {
            if matches!(chars.peek(), Some(next) if next.is_whitespace()) {
                let _ = chars.next();
            }
            if let Ok(codepoint) = u32::from_str_radix(&hex, 16) {
                if let Some(decoded) = char::from_u32(codepoint) {
                    out.push(decoded);
                }
            }
        } else if let Some(escaped) = chars.next() {
            out.push(escaped);
        }
    }
    out
}

fn resolve_css_value(value: &str, variables: &BTreeMap<String, String>) -> String {
    resolve_css_value_depth(value, variables, 0)
}

fn resolve_css_value_depth(
    value: &str,
    variables: &BTreeMap<String, String>,
    depth: usize,
) -> String {
    if depth > 8 {
        return String::new();
    }

    let mut out = value.to_string();
    for _ in 0..64 {
        let Some((start, end, body)) = find_var_function(&out) else {
            break;
        };
        let parts = split_top_level(body, &[',']);
        let name = parts.first().map(|part| part.trim()).unwrap_or_default();
        let fallback = parts.get(1).map(|part| part.trim());
        let replacement = match variables.get(name) {
            Some(value) if css_value_references_var(value, name) => fallback
                .map(|value| resolve_css_value_depth(value, variables, depth + 1))
                .unwrap_or_default(),
            Some(value) => resolve_css_value_depth(value, variables, depth + 1),
            None => fallback
                .map(|value| resolve_css_value_depth(value, variables, depth + 1))
                .unwrap_or_default(),
        };
        if replacement == out[start..=end] {
            out.replace_range(start..=end, "");
        } else {
            out.replace_range(start..=end, &replacement);
        }
    }
    out
}

fn css_value_references_var(value: &str, name: &str) -> bool {
    let mut rest = value;
    while let Some((_, end, body)) = find_var_function(rest) {
        if split_top_level(body, &[','])
            .first()
            .is_some_and(|candidate| candidate.trim() == name)
        {
            return true;
        }
        rest = &rest[end + 1..];
    }
    false
}

fn find_var_function(value: &str) -> Option<(usize, usize, &str)> {
    let start = find_ascii_case_insensitive(value, "var(")?;
    let open = start + 3;
    let mut depth = 0_i32;
    for (offset, ch) in value[open..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    let close = open + offset;
                    return Some((start, close, &value[open + 1..close]));
                }
            }
            _ => {}
        }
    }
    None
}

fn find_ascii_case_insensitive(value: &str, needle: &str) -> Option<usize> {
    let needle = needle.as_bytes();
    value
        .as_bytes()
        .windows(needle.len())
        .position(|candidate| candidate.eq_ignore_ascii_case(needle))
}

fn parse_pt_or_px(value: &str) -> Option<f32> {
    parse_pt_or_px_with_rem(value, 12.0)
}

fn parse_pt_or_px_with_rem(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<f32> {
    let length_context = length_context.into();
    let trimmed = value.trim();
    if trimmed.contains('%') {
        return None;
    }
    if let Some(length) = parse_css_length_with_rem(trimmed, length_context) {
        return Some(length.resolve(0.0));
    }
    if let Some(px) = trimmed.strip_suffix("px") {
        return px.trim().parse::<f32>().ok().map(|v| v * 0.75);
    }
    if let Some(pt) = trimmed.strip_suffix("pt") {
        return pt.trim().parse::<f32>().ok();
    }
    if let Some(mm) = trimmed.strip_suffix("mm") {
        return mm.trim().parse::<f32>().ok().map(mm_to_pt);
    }
    if let Some(cm) = trimmed.strip_suffix("cm") {
        return cm.trim().parse::<f32>().ok().map(|v| mm_to_pt(v * 10.0));
    }
    if let Some(inches) = trimmed.strip_suffix("in") {
        return inches.trim().parse::<f32>().ok().map(|v| v * 72.0);
    }
    if let Some(rem) = trimmed.strip_suffix("rem") {
        return rem
            .trim()
            .parse::<f32>()
            .ok()
            .map(|v| v * length_context.rem_base_pt);
    }
    if let Some(em) = trimmed.strip_suffix("em") {
        return em
            .trim()
            .parse::<f32>()
            .ok()
            .map(|v| v * length_context.em_base_pt);
    }
    if let Some(rlh) = trimmed.strip_suffix("rlh") {
        return rlh
            .trim()
            .parse::<f32>()
            .ok()
            .map(|v| v * length_context.rlh_base_pt);
    }
    if let Some(lh) = trimmed.strip_suffix("lh") {
        return lh
            .trim()
            .parse::<f32>()
            .ok()
            .map(|v| v * length_context.lh_base_pt);
    }
    if let Some(ch) = trimmed.strip_suffix("ch") {
        return ch
            .trim()
            .parse::<f32>()
            .ok()
            .map(|v| v * css_ch_unit_pt(length_context));
    }
    trimmed.parse::<f32>().ok()
}

fn parse_unitless_line_height(value: &str) -> Option<f32> {
    let multiplier = parse_unitless_number(value)?;
    (0.5..=4.0).contains(&multiplier).then_some(multiplier)
}

fn parse_unitless_number(value: &str) -> Option<f32> {
    let value = value.trim();
    if let Some(body) = strip_css_function(value, "calc") {
        return parse_unitless_calc(body);
    }
    if value
        .chars()
        .any(|ch| ch.is_ascii_alphabetic() || ch == '%' || ch == '(')
    {
        return None;
    }
    value.parse::<f32>().ok()
}

#[derive(Debug, Clone, PartialEq)]
struct ParsedFlex {
    grow: f32,
    shrink: f32,
    basis: Option<CssLength>,
}

fn parse_flex_factor(value: &str) -> Option<f32> {
    parse_unitless_number(value).map(|value| value.max(0.0))
}

fn parse_flex_wrap(value: &str) -> Option<FlexWrap> {
    match value.trim() {
        "nowrap" => Some(FlexWrap::NoWrap),
        "wrap" | "wrap-reverse" => Some(FlexWrap::Wrap),
        _ => None,
    }
}

fn parse_content_alignment_keyword(value: &str) -> Option<JustifyContent> {
    match value.trim() {
        "center" => Some(JustifyContent::Center),
        "end" | "flex-end" | "right" => Some(JustifyContent::End),
        "space-between" => Some(JustifyContent::SpaceBetween),
        "space-around" => Some(JustifyContent::SpaceAround),
        "space-evenly" => Some(JustifyContent::SpaceEvenly),
        "start" | "flex-start" | "left" | "normal" | "stretch" => Some(JustifyContent::Start),
        _ => None,
    }
}

fn parse_box_alignment_keyword(value: &str) -> Option<AlignItems> {
    match value.trim() {
        "center" => Some(AlignItems::Center),
        "end" | "flex-end" | "self-end" | "right" => Some(AlignItems::End),
        "baseline" | "first baseline" | "last baseline" => Some(AlignItems::Baseline),
        "start" | "flex-start" | "self-start" | "left" => Some(AlignItems::Start),
        "stretch" | "normal" => Some(AlignItems::Stretch),
        _ => None,
    }
}

fn parse_optional_box_alignment_keyword(
    value: &str,
    current: Option<AlignItems>,
) -> Option<AlignItems> {
    if value.trim() == "auto" {
        None
    } else {
        parse_box_alignment_keyword(value).or(current)
    }
}

fn parse_flex_basis(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<CssLength> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("auto")
        || matches!(value, "content" | "min-content" | "max-content")
    {
        return None;
    }
    if let Some(body) = strip_css_function(value, "fit-content") {
        let args = split_css_function_args(body);
        if let Some(first) = args.first() {
            return parse_css_length_with_rem(first, length_context);
        }
        return None;
    }
    parse_css_length_with_rem(value, length_context)
}

fn parse_flex_shorthand(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<ParsedFlex> {
    let length_context = length_context.into();
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    match value {
        "none" => {
            return Some(ParsedFlex {
                grow: 0.0,
                shrink: 0.0,
                basis: None,
            });
        }
        "auto" => {
            return Some(ParsedFlex {
                grow: 1.0,
                shrink: 1.0,
                basis: None,
            });
        }
        "initial" => {
            return Some(ParsedFlex {
                grow: 0.0,
                shrink: 1.0,
                basis: None,
            });
        }
        _ => {}
    }

    let parts = split_css_whitespace(value);
    if parts.is_empty() || parts.len() > 3 {
        return None;
    }

    if parts.len() == 1 {
        if let Some(grow) = parse_flex_factor(&parts[0]) {
            return Some(ParsedFlex {
                grow,
                shrink: 1.0,
                basis: Some(CssLength::percent(0.0)),
            });
        }
        return Some(ParsedFlex {
            grow: 1.0,
            shrink: 1.0,
            basis: parse_flex_basis(&parts[0], length_context),
        });
    }

    let grow = parse_flex_factor(&parts[0])?;
    let mut shrink = 1.0;
    let basis = if let Some(second_factor) = parse_flex_factor(&parts[1]) {
        shrink = second_factor;
        if let Some(third) = parts.get(2) {
            parse_flex_basis(third, length_context)
        } else {
            Some(CssLength::percent(0.0))
        }
    } else {
        parse_flex_basis(&parts[1], length_context)
    };

    Some(ParsedFlex {
        grow,
        shrink,
        basis,
    })
}

fn parse_unitless_calc(value: &str) -> Option<f32> {
    let mut total = 0.0_f32;
    for (sign, term) in split_calc_terms(value) {
        total += sign * parse_unitless_product(&term)?;
    }
    Some(total)
}

fn parse_unitless_product(value: &str) -> Option<f32> {
    let parts = split_top_level(value, &['*', '/']);
    if parts.is_empty() {
        return None;
    }
    let mut result = parse_unitless_number(parts[0].trim())?;
    let mut cursor = 0usize;
    for part in parts.iter().skip(1) {
        let next_operator = value[cursor..]
            .find(|ch| ch == '*' || ch == '/')
            .map(|idx| cursor + idx)?;
        let divisor_or_multiplier = parse_unitless_number(part.trim())?;
        match value.as_bytes().get(next_operator).copied() {
            Some(b'*') => result *= divisor_or_multiplier,
            Some(b'/') if divisor_or_multiplier != 0.0 => result /= divisor_or_multiplier,
            _ => return None,
        }
        cursor = next_operator + 1;
    }
    Some(result)
}

fn parse_letter_spacing_with_rem(
    value: &str,
    font_size: f32,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<f32> {
    let length_context = length_context.into();
    let value = value.trim();
    if value == "normal" {
        return Some(0.0);
    }
    if let Some(em) = value.strip_suffix("em") {
        return em.trim().parse::<f32>().ok().map(|v| v * font_size);
    }
    parse_pt_or_px_with_rem(value, length_context)
}

fn parse_word_spacing_with_rem(
    value: &str,
    font_size: f32,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<f32> {
    parse_letter_spacing_with_rem(value, font_size, length_context)
}

fn parse_font_weight_token(value: &str) -> Option<FontWeight> {
    match value.trim() {
        "black" => Some(FontWeight::Heavy),
        "bold" | "bolder" => Some(FontWeight::Bold),
        "normal" | "lighter" => Some(FontWeight::Normal),
        value => value.parse::<u16>().ok().map(|weight| {
            if weight >= 800 {
                FontWeight::Heavy
            } else if weight >= 600 {
                FontWeight::Bold
            } else {
                FontWeight::Normal
            }
        }),
    }
}

fn parse_font_style_token(value: &str) -> Option<FontStyle> {
    let value = value.trim();
    match value {
        "normal" => Some(FontStyle::Normal),
        "italic" => Some(FontStyle::Italic),
        "oblique" => Some(FontStyle::Oblique),
        _ if value.starts_with("oblique ") => Some(FontStyle::Oblique),
        _ => None,
    }
}

fn parse_font_variant_numeric(value: &str) -> Option<FontVariantNumeric> {
    let parts = split_css_whitespace(value);
    if parts.iter().any(|part| part == "tabular-nums") {
        return Some(FontVariantNumeric::TabularNums);
    }
    if parts
        .iter()
        .any(|part| matches!(part.as_str(), "normal" | "proportional-nums"))
    {
        return Some(FontVariantNumeric::Normal);
    }
    None
}

fn font_feature_settings_enable_tabular_numbers(value: &str) -> bool {
    if value == "normal" {
        return false;
    }
    split_top_level(value, &[',']).into_iter().any(|feature| {
        let feature = feature.trim();
        (feature.starts_with("\"tnum\"") || feature.starts_with("'tnum'"))
            && !feature.ends_with(" 0")
            && !feature.ends_with(" off")
    })
}

fn parse_text_decoration_line(value: &str) -> Option<TextDecoration> {
    let mut decoration = TextDecoration::none();
    let mut saw_known = false;
    for part in split_css_whitespace(value) {
        match part.as_str() {
            "none" => {
                decoration = TextDecoration::none();
                saw_known = true;
            }
            "underline" => {
                decoration.underline = true;
                saw_known = true;
            }
            "line-through" => {
                decoration.line_through = true;
                saw_known = true;
            }
            "overline" | "blink" => {
                saw_known = true;
            }
            _ => {}
        }
    }
    saw_known.then_some(decoration)
}

fn parse_text_decoration_thickness_token(
    value: &str,
    font_size: f32,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<f32> {
    let value = value.trim();
    if let Some(percent) = value.strip_suffix('%') {
        return percent
            .trim()
            .parse::<f32>()
            .ok()
            .map(|value| (font_size * value / 100.0).max(0.0));
    }
    parse_pt_or_px_with_rem(value, length_context)
        .filter(|value| value.is_finite() && *value >= 0.0)
}

fn parse_text_decoration_thickness(
    value: &str,
    font_size: f32,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<f32> {
    let value = value.trim();
    if matches!(value, "auto" | "from-font") {
        return None;
    }
    split_css_whitespace(value)
        .into_iter()
        .find_map(|part| parse_text_decoration_thickness_token(&part, font_size, length_context))
}

fn parse_text_underline_offset(
    value: &str,
    font_size: f32,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<f32> {
    let value = value.trim();
    if value == "auto" {
        return None;
    }
    if let Some(percent) = value.strip_suffix('%') {
        return percent
            .trim()
            .parse::<f32>()
            .ok()
            .map(|value| font_size * value / 100.0);
    }
    parse_pt_or_px_with_rem(value, length_context)
}

fn parse_list_style_type(value: &str) -> Option<ListStyleType> {
    split_css_whitespace(value)
        .into_iter()
        .find_map(|part| match part.as_str() {
            "none" => Some(ListStyleType::None),
            "disc" | "circle" | "square" => Some(ListStyleType::Disc),
            "decimal" | "decimal-leading-zero" => Some(ListStyleType::Decimal),
            _ => None,
        })
}

fn parse_font_size_token(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<f32> {
    let length_context = length_context.into();
    match value.trim() {
        "xx-small" => Some(6.75),
        "x-small" => Some(7.5),
        "small" => Some(9.75),
        "medium" => Some(12.0),
        "large" => Some(13.5),
        "x-large" => Some(18.0),
        "xx-large" => Some(24.0),
        "xxx-large" => Some(36.0),
        value
            if value
                .chars()
                .any(|ch| ch.is_ascii_alphabetic() || ch == '%' || ch == '(') =>
        {
            parse_pt_or_px_with_rem(value, length_context)
        }
        _ => None,
    }
}

fn parse_aspect_ratio(value: &str) -> Option<f32> {
    let cleaned = split_css_whitespace(value)
        .into_iter()
        .filter(|part| part != "auto")
        .collect::<Vec<_>>()
        .join(" ");
    let cleaned = cleaned.trim();
    if cleaned.is_empty() || cleaned == "auto" {
        return None;
    }
    if cleaned.contains('/') {
        let parts = split_top_level(cleaned, &['/']);
        if parts.len() != 2 {
            return None;
        }
        let width = parse_positive_ratio_number(&parts[0])?;
        let height = parse_positive_ratio_number(&parts[1])?;
        return (height > 0.0).then_some(width / height);
    }
    parse_positive_ratio_number(cleaned)
}

fn parse_positive_ratio_number(value: &str) -> Option<f32> {
    let value = parse_unitless_number(value.trim())?;
    value
        .is_finite()
        .then_some(value)
        .filter(|value| *value > 0.0)
}

#[derive(Debug, Clone, PartialEq)]
struct CssTransform {
    rotate_deg: f32,
    translate_x: Option<CssLength>,
    translate_y: Option<CssLength>,
    scale_x: f32,
    scale_y: f32,
}

impl Default for CssTransform {
    fn default() -> Self {
        Self {
            rotate_deg: 0.0,
            translate_x: None,
            translate_y: None,
            scale_x: 1.0,
            scale_y: 1.0,
        }
    }
}

fn parse_css_transform(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> CssTransform {
    let length_context = length_context.into();
    let value = value.trim().to_ascii_lowercase();
    if value == "none" || value.is_empty() {
        return CssTransform::default();
    }

    let mut transform = CssTransform::default();
    for function in parse_transform_functions(&value) {
        if let Some(body) = strip_css_function(&function, "rotate") {
            if let Some(rotate) = parse_rotate_value(body) {
                transform.rotate_deg += rotate;
            }
        } else if let Some(body) = strip_css_function(&function, "translate") {
            if let Some((x, y)) = parse_translate_property(body, length_context) {
                transform.translate_x = Some(add_transform_lengths(transform.translate_x, x));
                transform.translate_y = Some(add_transform_lengths(transform.translate_y, y));
            }
        } else if let Some(body) = strip_css_function(&function, "translatex") {
            if let Some(x) = parse_css_length_with_rem(body.trim(), length_context) {
                transform.translate_x = Some(add_transform_lengths(transform.translate_x, x));
            }
        } else if let Some(body) = strip_css_function(&function, "translatey") {
            if let Some(y) = parse_css_length_with_rem(body.trim(), length_context) {
                transform.translate_y = Some(add_transform_lengths(transform.translate_y, y));
            }
        } else if let Some(body) = strip_css_function(&function, "translate3d") {
            let values = split_css_function_args(body);
            if values.len() >= 2 {
                if let Some(x) = parse_css_length_with_rem(&values[0], length_context) {
                    transform.translate_x = Some(add_transform_lengths(transform.translate_x, x));
                }
                if let Some(y) = parse_css_length_with_rem(&values[1], length_context) {
                    transform.translate_y = Some(add_transform_lengths(transform.translate_y, y));
                }
            }
        } else if let Some(body) = strip_css_function(&function, "scale") {
            if let Some((x, y)) = parse_scale_property(body) {
                transform.scale_x *= x;
                transform.scale_y *= y;
            }
        } else if let Some(body) = strip_css_function(&function, "scalex") {
            if let Some(scale) = parse_css_scale(body.trim()) {
                transform.scale_x *= scale;
            }
        } else if let Some(body) = strip_css_function(&function, "scaley") {
            if let Some(scale) = parse_css_scale(body.trim()) {
                transform.scale_y *= scale;
            }
        } else if let Some(body) = strip_css_function(&function, "scale3d") {
            let values = split_css_function_args(body);
            if values.len() >= 2 {
                if let Some(scale) = parse_css_scale(&values[0]) {
                    transform.scale_x *= scale;
                }
                if let Some(scale) = parse_css_scale(&values[1]) {
                    transform.scale_y *= scale;
                }
            }
        }
    }
    transform
}

fn parse_transform_functions(value: &str) -> Vec<String> {
    let mut functions = Vec::new();
    let mut rest = value.trim();
    while let Some(open) = rest.find('(') {
        let name_start = rest[..open]
            .rfind(|ch: char| ch.is_whitespace())
            .map(|idx| idx + 1)
            .unwrap_or(0);
        let candidate = &rest[name_start..];
        let Some(end) = matching_function_end(candidate) else {
            break;
        };
        functions.push(candidate[..=end].trim().to_string());
        rest = &candidate[end + 1..];
    }
    functions
}

fn parse_translate_property(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<(CssLength, CssLength)> {
    let length_context = length_context.into();
    let values = split_css_function_args(value);
    let parts = if values.len() <= 1 {
        split_css_whitespace(value)
    } else {
        values
    };
    match parts.as_slice() {
        [x] => Some((
            parse_css_length_with_rem(x, length_context)?,
            CssLength::points(0.0),
        )),
        [x, y, ..] => Some((
            parse_css_length_with_rem(x, length_context)?,
            parse_css_length_with_rem(y, length_context)?,
        )),
        _ => None,
    }
}

fn parse_scale_property(value: &str) -> Option<(f32, f32)> {
    let values = split_css_function_args(value);
    let parts = if values.len() <= 1 {
        split_css_whitespace(value)
    } else {
        values
    };
    match parts.as_slice() {
        [x] => {
            let x = parse_css_scale(x)?;
            Some((x, x))
        }
        [x, y, ..] => Some((parse_css_scale(x)?, parse_css_scale(y)?)),
        _ => None,
    }
}

fn parse_transform_origin(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<(CssLength, CssLength)> {
    let length_context = length_context.into();
    let parts = split_css_whitespace(value);
    if parts.is_empty() {
        return None;
    }

    let mut x = None;
    let mut y = None;
    for part in parts.into_iter().take(2) {
        match part.as_str() {
            "left" => x = Some(CssLength::percent(0.0)),
            "center" => {
                if x.is_none() {
                    x = Some(CssLength::percent(0.5));
                } else if y.is_none() {
                    y = Some(CssLength::percent(0.5));
                }
            }
            "right" => x = Some(CssLength::percent(1.0)),
            "top" => y = Some(CssLength::percent(0.0)),
            "bottom" => y = Some(CssLength::percent(1.0)),
            _ => {
                let length = parse_css_length_with_rem(&part, length_context)?;
                if x.is_none() {
                    x = Some(length);
                } else if y.is_none() {
                    y = Some(length);
                }
            }
        }
    }

    let first = split_css_whitespace(value).into_iter().next()?;
    let default_x = if matches!(first.as_str(), "top" | "bottom") {
        CssLength::percent(0.5)
    } else {
        CssLength::percent(0.5)
    };
    let default_y = if matches!(first.as_str(), "left" | "right") {
        CssLength::percent(0.5)
    } else {
        CssLength::percent(0.5)
    };
    Some((x.unwrap_or(default_x), y.unwrap_or(default_y)))
}

fn parse_css_scale(value: &str) -> Option<f32> {
    let value = value.trim();
    if let Some(percent) = value.strip_suffix('%') {
        return percent
            .trim()
            .parse::<f32>()
            .ok()
            .map(|value| value / 100.0);
    }
    parse_unitless_number(value)
}

fn parse_rotate_value(value: &str) -> Option<f32> {
    let body = value.trim();
    if body == "none" {
        return Some(0.0);
    }
    if let Some(deg) = body.strip_suffix("deg") {
        return deg.trim().parse::<f32>().ok();
    }
    if let Some(rad) = body.strip_suffix("rad") {
        return rad.trim().parse::<f32>().ok().map(f32::to_degrees);
    }
    if let Some(turn) = body.strip_suffix("turn") {
        return turn.trim().parse::<f32>().ok().map(|turn| turn * 360.0);
    }
    body.parse::<f32>().ok()
}

fn add_transform_lengths(current: Option<CssLength>, next: CssLength) -> CssLength {
    current
        .and_then(|current| current.combine_linear(next.clone(), 1.0))
        .unwrap_or(next)
}

fn parse_font_face(value: &str) -> FontFace {
    for family in split_top_level(value, &[',']) {
        let family = family
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .to_ascii_lowercase();
        if family.is_empty() || family.starts_with("var(") {
            continue;
        }
        if family.contains("lato") {
            return FontFace::Lato;
        }
        if family.contains("monospace")
            || family.contains("courier")
            || family.contains("mono")
            || family.contains("code")
        {
            return FontFace::Mono;
        }
        if (family == "serif" || family.contains("times") || family.contains("georgia"))
            && !family.contains("sans")
        {
            return FontFace::Serif;
        }
        return FontFace::Sans;
    }
    FontFace::Sans
}

fn parse_box_shorthand(value: &str) -> Option<[f32; 4]> {
    parse_box_shorthand_with_rem(value, 12.0)
}

#[derive(Debug, Clone, Copy)]
struct ParsedMargin {
    value: f32,
    auto: bool,
}

fn parse_margin_component(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<ParsedMargin> {
    if value.trim().eq_ignore_ascii_case("auto") {
        return Some(ParsedMargin {
            value: 0.0,
            auto: true,
        });
    }
    parse_pt_or_px_with_rem(value, length_context).map(|value| ParsedMargin { value, auto: false })
}

fn parse_margin_shorthand_with_rem(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<[ParsedMargin; 4]> {
    let length_context = length_context.into();
    let values = split_css_whitespace(value)
        .into_iter()
        .map(|part| parse_margin_component(&part, length_context))
        .collect::<Option<Vec<_>>>()?;
    match values.as_slice() {
        [all] => Some([*all, *all, *all, *all]),
        [vertical, horizontal] => Some([*vertical, *horizontal, *vertical, *horizontal]),
        [top, horizontal, bottom] => Some([*top, *horizontal, *bottom, *horizontal]),
        [top, right, bottom, left] => Some([*top, *right, *bottom, *left]),
        _ => None,
    }
}

fn parse_two_value_margin_axis(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<(ParsedMargin, ParsedMargin)> {
    let length_context = length_context.into();
    let parts = split_css_whitespace(value);
    match parts.as_slice() {
        [single] => {
            let value = parse_margin_component(single, length_context)?;
            Some((value, value))
        }
        [start, end] => Some((
            parse_margin_component(start, length_context)?,
            parse_margin_component(end, length_context)?,
        )),
        _ => None,
    }
}

fn parse_box_shorthand_with_rem(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<[f32; 4]> {
    let length_context = length_context.into();
    let values = split_css_whitespace(value)
        .into_iter()
        .map(|part| parse_pt_or_px_with_rem(&part, length_context))
        .collect::<Option<Vec<_>>>()?;
    match values.as_slice() {
        [all] => Some([*all, *all, *all, *all]),
        [vertical, horizontal] => Some([*vertical, *horizontal, *vertical, *horizontal]),
        [top, horizontal, bottom] => Some([*top, *horizontal, *bottom, *horizontal]),
        [top, right, bottom, left] => Some([*top, *right, *bottom, *left]),
        _ => None,
    }
}

fn parse_box_length_shorthand_with_rem(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<[CssLength; 4]> {
    let length_context = length_context.into();
    let values = split_css_whitespace(value)
        .into_iter()
        .map(|part| parse_css_length_with_rem(&part, length_context))
        .collect::<Option<Vec<_>>>()?;
    match values.as_slice() {
        [all] => Some([all.clone(), all.clone(), all.clone(), all.clone()]),
        [vertical, horizontal] => Some([
            vertical.clone(),
            horizontal.clone(),
            vertical.clone(),
            horizontal.clone(),
        ]),
        [top, horizontal, bottom] => Some([
            top.clone(),
            horizontal.clone(),
            bottom.clone(),
            horizontal.clone(),
        ]),
        [top, right, bottom, left] => {
            Some([top.clone(), right.clone(), bottom.clone(), left.clone()])
        }
        _ => None,
    }
}

fn parse_gap_shorthand_with_rem(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<(f32, f32)> {
    let length_context = length_context.into();
    let values = split_css_whitespace(value)
        .into_iter()
        .map(|part| parse_pt_or_px_with_rem(&part, length_context))
        .collect::<Option<Vec<_>>>()?;
    match values.as_slice() {
        [all] => Some((*all, *all)),
        [row, column] => Some((*row, *column)),
        _ => None,
    }
}

fn parse_border_width(value: &str, context: impl Into<LengthContext> + Copy) -> Option<f32> {
    let width = match value.trim().to_ascii_lowercase().as_str() {
        "thin" => 0.75,
        "medium" => 2.25,
        "thick" => 3.75,
        _ => parse_pt_or_px_with_rem(value, context)?,
    };
    (width.is_finite() && width >= 0.0).then_some(width)
}

fn parse_border_axis_widths(
    value: &str,
    context: impl Into<LengthContext> + Copy,
) -> Option<(f32, f32)> {
    let parts = split_css_whitespace(value);
    match parts.as_slice() {
        [all] => {
            let width = parse_border_width(all, context)?;
            Some((width, width))
        }
        [start, end] => Some((
            parse_border_width(start, context)?,
            parse_border_width(end, context)?,
        )),
        _ => None,
    }
}

fn parse_border_widths(value: &str, context: impl Into<LengthContext> + Copy) -> Option<[f32; 4]> {
    let values = split_css_whitespace(value)
        .into_iter()
        .map(|part| parse_border_width(&part, context))
        .collect::<Option<Vec<_>>>()?;
    match values.as_slice() {
        [all] => Some([*all; 4]),
        [vertical, horizontal] => Some([*vertical, *horizontal, *vertical, *horizontal]),
        [top, horizontal, bottom] => Some([*top, *horizontal, *bottom, *horizontal]),
        [top, right, bottom, left] => Some([*top, *right, *bottom, *left]),
        _ => None,
    }
}

fn parse_two_value_axis(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<(f32, f32)> {
    let length_context = length_context.into();
    let values = split_css_whitespace(value)
        .into_iter()
        .map(|part| parse_pt_or_px_with_rem(&part, length_context))
        .collect::<Option<Vec<_>>>()?;
    match values.as_slice() {
        [all] => Some((*all, *all)),
        [start, end] => Some((*start, *end)),
        _ => None,
    }
}

fn parse_border_spacing(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<(f32, f32)> {
    let length_context = length_context.into();
    let values = split_css_whitespace(value)
        .into_iter()
        .map(|part| parse_pt_or_px_with_rem(&part, length_context))
        .collect::<Option<Vec<_>>>()?;
    match values.as_slice() {
        [horizontal] => Some((*horizontal, *horizontal)),
        [horizontal, vertical] => Some((*horizontal, *vertical)),
        _ => None,
    }
}

fn parse_two_value_lengths(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<(CssLength, CssLength)> {
    let length_context = length_context.into();
    let values = split_css_whitespace(value)
        .into_iter()
        .map(|part| parse_css_length_with_rem(&part, length_context))
        .collect::<Option<Vec<_>>>()?;
    match values.as_slice() {
        [all] => Some((all.clone(), all.clone())),
        [start, end] => Some((start.clone(), end.clone())),
        _ => None,
    }
}

fn parse_border_colors(value: &str, current_color: Color) -> Option<[Color; 4]> {
    let values = split_css_whitespace(value)
        .into_iter()
        .map(|part| parse_color_or_current(&part, current_color))
        .collect::<Option<Vec<_>>>()?;
    match values.as_slice() {
        [all] => Some([*all; 4]),
        [vertical, horizontal] => Some([*vertical, *horizontal, *vertical, *horizontal]),
        [top, horizontal, bottom] => Some([*top, *horizontal, *bottom, *horizontal]),
        [top, right, bottom, left] => Some([*top, *right, *bottom, *left]),
        _ => None,
    }
}

fn parse_two_value_colors(value: &str, current_color: Color) -> Option<(Color, Color)> {
    let values = split_css_whitespace(value)
        .into_iter()
        .map(|part| parse_color_or_current(&part, current_color))
        .collect::<Option<Vec<_>>>()?;
    match values.as_slice() {
        [all] => Some((*all, *all)),
        [start, end] => Some((*start, *end)),
        _ => None,
    }
}

fn parse_border_shorthand_with_rem(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<(f32, Option<Color>)> {
    let length_context = length_context.into();
    let mut width = None;

    for part in split_css_whitespace(value) {
        if width.is_none() {
            width = parse_pt_or_px_with_rem(&part, length_context);
        }
    }

    let color = extract_css_color(value);
    width.map(|width| (width, color))
}

fn parse_border_shorthand_with_current(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
    current_color: Color,
) -> Option<(f32, Option<Color>)> {
    let length_context = length_context.into();
    let (width, color) = parse_border_shorthand_with_rem(value, length_context)?;
    let color = if color.is_none()
        && split_css_whitespace(value)
            .iter()
            .any(|part| part.eq_ignore_ascii_case("currentcolor"))
    {
        Some(current_color)
    } else {
        color
    };
    Some((width, color))
}

fn parse_border_radius_with_rem(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<f32> {
    let length_context = length_context.into();
    let first_radius_group = value.split('/').next()?.trim();
    split_css_whitespace(first_radius_group)
        .into_iter()
        .filter_map(|part| parse_pt_or_px_with_rem(&part, length_context))
        .next()
}

fn parse_border_line_style(value: &str) -> Option<BorderLineStyle> {
    split_css_whitespace(value)
        .into_iter()
        .find_map(|part| match part.as_str() {
            "dashed" => Some(BorderLineStyle::Dashed),
            "dotted" => Some(BorderLineStyle::Dotted),
            "none" | "hidden" => Some(BorderLineStyle::None),
            "solid" | "double" | "groove" | "ridge" | "inset" | "outset" => {
                Some(BorderLineStyle::Solid)
            }
            _ => None,
        })
}

const DEFAULT_OUTLINE_WIDTH_PT: f32 = 2.25;

fn parse_outline_width(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<f32> {
    match value.trim().to_ascii_lowercase().as_str() {
        "thin" => Some(0.75),
        "medium" => Some(DEFAULT_OUTLINE_WIDTH_PT),
        "thick" => Some(3.75),
        _ => parse_pt_or_px_with_rem(value, length_context),
    }
}

fn parse_outline_line_style(value: &str) -> Option<BorderLineStyle> {
    split_css_whitespace(value)
        .into_iter()
        .find_map(|part| match part.as_str() {
            "auto" | "solid" | "double" | "groove" | "ridge" | "inset" | "outset" => {
                Some(BorderLineStyle::Solid)
            }
            "dashed" => Some(BorderLineStyle::Dashed),
            "dotted" => Some(BorderLineStyle::Dotted),
            "none" | "hidden" => Some(BorderLineStyle::None),
            _ => None,
        })
}

fn parse_box_shadows_with_rem(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Vec<BoxShadow> {
    let length_context = length_context.into();
    let value = value.trim();
    if value.eq_ignore_ascii_case("none") {
        return Vec::new();
    }
    split_top_level(value, &[','])
        .into_iter()
        .filter_map(|layer| parse_box_shadow_layer_with_rem(layer, length_context))
        .collect()
}

fn parse_box_shadow_with_rem(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<BoxShadow> {
    parse_box_shadow_layer_with_rem(value, length_context)
}

fn parse_box_shadow_layer_with_rem(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<BoxShadow> {
    parse_shadow_with_rem(value, length_context, true)
}

fn parse_shadow_with_rem(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
    soften_dark_shadow: bool,
) -> Option<BoxShadow> {
    let length_context = length_context.into();
    let value = value.trim();
    if value.eq_ignore_ascii_case("none")
        || split_css_whitespace(value)
            .iter()
            .any(|part| part.eq_ignore_ascii_case("inset"))
    {
        return None;
    }
    let mut lengths = Vec::new();
    for part in split_css_whitespace(value) {
        if let Some(length) = parse_pt_or_px_with_rem(&part, length_context) {
            lengths.push(length);
        }
    }
    if lengths.len() < 2 {
        return None;
    }
    let mut color = extract_css_color(value).unwrap_or(Color {
        r: 0.86,
        g: 0.89,
        b: 0.94,
        a: 1.0,
    });
    let luminance = color.r * 0.2126 + color.g * 0.7152 + color.b * 0.0722;
    if soften_dark_shadow && luminance < 0.35 && color.a >= 0.95 {
        color = Color {
            r: 0.82,
            g: 0.86,
            b: 0.91,
            a: 1.0,
        };
    }
    Some(BoxShadow {
        offset_x: lengths[0],
        offset_y: lengths[1],
        blur: lengths.get(2).copied().unwrap_or(0.0),
        spread: lengths.get(3).copied().unwrap_or(0.0),
        color,
    })
}

fn parse_text_shadow_with_rem(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<BoxShadow> {
    if value.trim().eq_ignore_ascii_case("none") {
        return None;
    }
    parse_shadow_with_rem(value, length_context, false)
}

fn parse_drop_shadow_filter_with_rem(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<BoxShadow> {
    let length_context = length_context.into();
    let value = value.trim();
    let start = value.find("drop-shadow(")?;
    let body_start = start + "drop-shadow(".len();
    let mut depth = 1i32;
    let mut end = body_start;
    for (offset, ch) in value[body_start..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = body_start + offset;
                    break;
                }
            }
            _ => {}
        }
    }
    if end <= body_start {
        return None;
    }
    parse_box_shadow_with_rem(&value[body_start..end], length_context)
}

fn extract_css_color(value: &str) -> Option<Color> {
    if let Some(start) = value.find("color-mix(") {
        let end = matching_function_end(&value[start..])? + start;
        return Color::from_css(&value[start..=end]);
    }
    if let Some(start) = value.find("rgba(").or_else(|| value.find("rgb(")) {
        let end = matching_function_end(&value[start..])? + start;
        return Color::from_css(&value[start..=end]);
    }
    if let Some(start) = value.find("hsla(").or_else(|| value.find("hsl(")) {
        let end = matching_function_end(&value[start..])? + start;
        return Color::from_css(&value[start..=end]);
    }
    if let Some(start) = value.find("hwb(") {
        let end = matching_function_end(&value[start..])? + start;
        return Color::from_css(&value[start..=end]);
    }
    if let Some(start) = value.find("color(") {
        let end = matching_function_end(&value[start..])? + start;
        return Color::from_css(&value[start..=end]);
    }
    if let Some(start) = value.find("lab(") {
        let end = matching_function_end(&value[start..])? + start;
        return Color::from_css(&value[start..=end]);
    }
    if let Some(start) = value.find("lch(") {
        let end = matching_function_end(&value[start..])? + start;
        return Color::from_css(&value[start..=end]);
    }
    if let Some(start) = value.find("oklch(") {
        let end = matching_function_end(&value[start..])? + start;
        return Color::from_css(&value[start..=end]);
    }
    if let Some(start) = value.find("oklab(") {
        let end = matching_function_end(&value[start..])? + start;
        return Color::from_css(&value[start..=end]);
    }
    if let Some(start) = value.find("light-dark(") {
        let end = matching_function_end(&value[start..])? + start;
        return Color::from_css(&value[start..=end]);
    }
    split_css_whitespace(value)
        .into_iter()
        .find_map(|part| Color::from_css(&part))
}

#[cfg(test)]
#[allow(dead_code)]
fn parse_css_length(value: &str) -> Option<CssLength> {
    parse_css_length_with_rem(value, 12.0)
}

fn parse_css_length_with_rem(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<CssLength> {
    let length_context = length_context.into();
    let value = value.trim();
    if value == "auto" || value.is_empty() {
        return None;
    }
    if let Some(body) = strip_css_function(value, "calc") {
        return parse_calc_length_with_rem(body, length_context);
    }
    if let Some(length) = parse_env_length_fallback(value, length_context) {
        return Some(length);
    }
    if let Some(body) = strip_css_function(value, "min") {
        let values = split_css_function_args(body)
            .into_iter()
            .map(|arg| parse_css_length_with_rem(&arg, length_context))
            .collect::<Option<Vec<_>>>()?;
        if values.is_empty() {
            return None;
        }
        return Some(CssLength::Min(values));
    }
    if let Some(body) = strip_css_function(value, "max") {
        let values = split_css_function_args(body)
            .into_iter()
            .map(|arg| parse_css_length_with_rem(&arg, length_context))
            .collect::<Option<Vec<_>>>()?;
        if values.is_empty() {
            return None;
        }
        return Some(CssLength::Max(values));
    }
    if let Some(body) = strip_css_function(value, "clamp") {
        let values = split_css_function_args(body);
        if values.len() != 3 {
            return None;
        }
        return Some(CssLength::Clamp {
            min: Box::new(parse_css_length_with_rem(&values[0], length_context)?),
            preferred: Box::new(parse_css_length_with_rem(&values[1], length_context)?),
            max: Box::new(parse_css_length_with_rem(&values[2], length_context)?),
        });
    }
    if let Some(px) = value.strip_suffix("px") {
        return px
            .trim()
            .parse::<f32>()
            .ok()
            .map(|v| CssLength::points(v * 0.75));
    }
    if let Some(pt) = value.strip_suffix("pt") {
        return pt.trim().parse::<f32>().ok().map(CssLength::points);
    }
    if let Some(mm) = value.strip_suffix("mm") {
        return mm
            .trim()
            .parse::<f32>()
            .ok()
            .map(|v| CssLength::points(mm_to_pt(v)));
    }
    if let Some(cm) = value.strip_suffix("cm") {
        return cm
            .trim()
            .parse::<f32>()
            .ok()
            .map(|v| CssLength::points(mm_to_pt(v * 10.0)));
    }
    if let Some(viewport_width) = parse_viewport_unit(
        value,
        &["svi", "lvi", "dvi", "vi", "svw", "lvw", "dvw", "vw"],
    ) {
        return Some(CssLength::points(
            length_context.viewport_width_pt * viewport_width / 100.0,
        ));
    }
    if let Some(viewport_height) = parse_viewport_unit(
        value,
        &["svb", "lvb", "dvb", "vb", "svh", "lvh", "dvh", "vh"],
    ) {
        return Some(CssLength::points(
            length_context.viewport_height_pt * viewport_height / 100.0,
        ));
    }
    if let Some(vmin) = parse_viewport_unit(value, &["vmin"]) {
        return Some(CssLength::points(
            length_context
                .viewport_width_pt
                .min(length_context.viewport_height_pt)
                * vmin
                / 100.0,
        ));
    }
    if let Some(vmax) = parse_viewport_unit(value, &["vmax"]) {
        return Some(CssLength::points(
            length_context
                .viewport_width_pt
                .max(length_context.viewport_height_pt)
                * vmax
                / 100.0,
        ));
    }
    if let Some(container_width) = parse_viewport_unit(value, &["cqw", "cqi"]) {
        return Some(CssLength::points(
            length_context.viewport_width_pt * container_width / 100.0,
        ));
    }
    if let Some(container_height) = parse_viewport_unit(value, &["cqh", "cqb"]) {
        return Some(CssLength::points(
            length_context.viewport_height_pt * container_height / 100.0,
        ));
    }
    if let Some(container_min) = parse_viewport_unit(value, &["cqmin"]) {
        return Some(CssLength::points(
            length_context
                .viewport_width_pt
                .min(length_context.viewport_height_pt)
                * container_min
                / 100.0,
        ));
    }
    if let Some(container_max) = parse_viewport_unit(value, &["cqmax"]) {
        return Some(CssLength::points(
            length_context
                .viewport_width_pt
                .max(length_context.viewport_height_pt)
                * container_max
                / 100.0,
        ));
    }
    if let Some(inches) = value.strip_suffix("in") {
        return inches
            .trim()
            .parse::<f32>()
            .ok()
            .map(|v| CssLength::points(v * 72.0));
    }
    if let Some(rem) = value.strip_suffix("rem") {
        return rem
            .trim()
            .parse::<f32>()
            .ok()
            .map(|v| CssLength::points(v * length_context.rem_base_pt));
    }
    if let Some(em) = value.strip_suffix("em") {
        return em
            .trim()
            .parse::<f32>()
            .ok()
            .map(|v| CssLength::points(v * length_context.em_base_pt));
    }
    if let Some(ex) = value.strip_suffix("ex") {
        return ex
            .trim()
            .parse::<f32>()
            .ok()
            .map(|v| CssLength::points(v * css_ex_unit_pt(length_context)));
    }
    if let Some(cap) = value.strip_suffix("cap") {
        return cap
            .trim()
            .parse::<f32>()
            .ok()
            .map(|v| CssLength::points(v * css_cap_unit_pt(length_context)));
    }
    if let Some(ic) = value.strip_suffix("ic") {
        return ic
            .trim()
            .parse::<f32>()
            .ok()
            .map(|v| CssLength::points(v * css_ic_unit_pt(length_context)));
    }
    if let Some(rlh) = value.strip_suffix("rlh") {
        return rlh
            .trim()
            .parse::<f32>()
            .ok()
            .map(|v| CssLength::points(v * length_context.rlh_base_pt));
    }
    if let Some(lh) = value.strip_suffix("lh") {
        return lh
            .trim()
            .parse::<f32>()
            .ok()
            .map(|v| CssLength::points(v * length_context.lh_base_pt));
    }
    if let Some(ch) = value.strip_suffix("ch") {
        return ch
            .trim()
            .parse::<f32>()
            .ok()
            .map(|v| CssLength::points(v * css_ch_unit_pt(length_context)));
    }
    if let Some(percent) = value.strip_suffix('%') {
        return percent
            .trim()
            .parse::<f32>()
            .ok()
            .map(|v| CssLength::percent(v / 100.0));
    }
    value.trim().parse::<f32>().ok().map(CssLength::points)
}

fn parse_env_length_fallback(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<CssLength> {
    let length_context = length_context.into();
    let body = strip_css_function(value.trim(), "env")?;
    let args = split_css_function_args(body);
    match args.as_slice() {
        [_name, fallback, ..] => parse_css_length_with_rem(fallback, length_context),
        [_name] => Some(CssLength::points(0.0)),
        _ => None,
    }
}

fn css_ch_unit_pt(length_context: LengthContext) -> f32 {
    // CSS defines `ch` from the advance of the "0" glyph. At parse time this
    // lightweight engine does not carry the resolved font face, so approximate
    // common sans/serif zero advances rather than treating `ch` as a raw em.
    length_context.em_base_pt * 0.56
}

fn css_ex_unit_pt(length_context: LengthContext) -> f32 {
    // CSS defines `ex` from the font's x-height. The layout parser resolves
    // lengths before concrete glyph metrics are available, so use a stable
    // sans/serif approximation instead of dropping the declaration.
    length_context.em_base_pt * 0.5
}

fn css_cap_unit_pt(length_context: LengthContext) -> f32 {
    // CSS `cap` is the cap-height of the resolved font. Most Latin UI fonts are
    // close to 0.7em; using that approximation keeps modern CSS printable even
    // without a full shaping engine at parse time.
    length_context.em_base_pt * 0.7
}

fn css_ic_unit_pt(length_context: LengthContext) -> f32 {
    // CSS `ic` is based on the advance of the CJK water ideograph. Without font
    // metrics in this parser, 1em is the least surprising print-safe fallback.
    length_context.em_base_pt
}

fn parse_viewport_unit(value: &str, units: &[&str]) -> Option<f32> {
    units
        .iter()
        .find_map(|unit| value.strip_suffix(unit))
        .and_then(|number| number.trim().parse::<f32>().ok())
}

fn mm_to_pt(value: f32) -> f32 {
    value * 72.0 / 25.4
}

fn parse_calc_length_with_rem(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<CssLength> {
    let length_context = length_context.into();
    let mut total = CssLength::points(0.0);
    for (sign, term) in split_calc_terms(value) {
        let term = parse_calc_product_with_rem(&term, length_context)?;
        total = total.combine_linear(term, sign)?;
    }
    Some(total)
}

fn parse_calc_product_with_rem(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<CssLength> {
    let length_context = length_context.into();
    let parts = split_top_level(value, &['*', '/']);
    if parts.len() == 1 {
        return parse_css_length_with_rem(parts[0].trim(), length_context);
    }
    if parts.len() != 2 {
        return None;
    }

    let left = parts[0].trim();
    let right = parts[1].trim();
    if value.contains('*') {
        if let Ok(multiplier) = left.parse::<f32>() {
            return scale_length(
                parse_css_length_with_rem(right, length_context)?,
                multiplier,
            );
        }
        if let Ok(multiplier) = right.parse::<f32>() {
            return scale_length(parse_css_length_with_rem(left, length_context)?, multiplier);
        }
    }
    if value.contains('/') {
        let divisor = right.parse::<f32>().ok()?;
        if divisor == 0.0 {
            return None;
        }
        return scale_length(
            parse_css_length_with_rem(left, length_context)?,
            1.0 / divisor,
        );
    }
    None
}

fn scale_length(length: CssLength, multiplier: f32) -> Option<CssLength> {
    match length {
        CssLength::Linear { percent, points } => Some(CssLength::Linear {
            percent: percent * multiplier,
            points: points * multiplier,
        }),
        _ => None,
    }
}

fn split_calc_terms(value: &str) -> Vec<(f32, String)> {
    let mut parts = Vec::new();
    let mut depth = 0_i32;
    let mut start = 0usize;
    let mut sign = 1.0;
    for (idx, ch) in value.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            '+' | '-' if depth == 0 && idx > start => {
                let term = value[start..idx].trim();
                if !term.is_empty() {
                    parts.push((sign, term.to_string()));
                }
                sign = if ch == '-' { -1.0 } else { 1.0 };
                start = idx + ch.len_utf8();
            }
            '-' if depth == 0 && idx == start => {
                sign = -1.0;
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    let term = value[start..].trim();
    if !term.is_empty() {
        parts.push((sign, term.to_string()));
    }
    parts
}

fn parse_linear_gradient(value: &str) -> Option<LinearGradient> {
    let layer = split_top_level(value, &[','])
        .into_iter()
        .rev()
        .map(str::trim)
        .find(|layer| layer.starts_with("linear-gradient("))
        .unwrap_or_else(|| value.trim());
    let body = strip_css_function(layer, "linear-gradient")?;
    let args = split_css_function_args(body);
    let angle_deg = args
        .first()
        .and_then(|arg| parse_linear_gradient_angle(arg))
        .unwrap_or(180.0);
    let mut stops = Vec::new();
    for arg in args {
        if let Some(color) = extract_css_color(&arg) {
            stops.push((color, parse_gradient_stop_position(&arg)));
        }
    }
    let stops = normalize_gradient_stops(stops);
    Some(LinearGradient {
        start: stops.first()?.color,
        end: stops.last()?.color,
        angle_deg,
        stops,
    })
}

fn parse_gradient_stop_position(value: &str) -> Option<f32> {
    split_css_whitespace(value)
        .into_iter()
        .rev()
        .find_map(|part| {
            let part = part.trim();
            if let Some(raw) = part.strip_suffix('%') {
                return raw
                    .trim()
                    .parse::<f32>()
                    .ok()
                    .map(|value| (value / 100.0).clamp(0.0, 1.0));
            }
            if part == "0" || part == "0.0" {
                return Some(0.0);
            }
            None
        })
}

fn normalize_gradient_stops(raw_stops: Vec<(Color, Option<f32>)>) -> Vec<GradientStop> {
    if raw_stops.is_empty() {
        return Vec::new();
    }
    let last_index = raw_stops.len().saturating_sub(1);
    let mut positions = raw_stops
        .iter()
        .map(|(_, position)| position.map(|value| value.clamp(0.0, 1.0)))
        .collect::<Vec<_>>();

    if positions[0].is_none() {
        positions[0] = Some(0.0);
    }
    if positions[last_index].is_none() {
        positions[last_index] = Some(1.0);
    }

    let mut idx = 0usize;
    while idx <= last_index {
        if positions[idx].is_some() {
            idx += 1;
            continue;
        }

        let start_idx = idx.saturating_sub(1);
        let start_position = positions[start_idx].unwrap_or(0.0);
        let mut end_idx = idx + 1;
        while end_idx <= last_index && positions[end_idx].is_none() {
            end_idx += 1;
        }
        let end_position = positions
            .get(end_idx)
            .and_then(|position| *position)
            .unwrap_or(1.0);
        let span = end_idx.saturating_sub(start_idx).max(1);
        for fill_idx in idx..end_idx {
            let local = (fill_idx - start_idx) as f32 / span as f32;
            positions[fill_idx] = Some(start_position + (end_position - start_position) * local);
        }
        idx = end_idx;
    }

    let mut positions = positions
        .into_iter()
        .enumerate()
        .map(|(idx, position)| {
            position.unwrap_or_else(|| {
                if last_index == 0 {
                    0.0
                } else {
                    idx as f32 / last_index as f32
                }
            })
        })
        .collect::<Vec<_>>();
    for idx in 1..positions.len() {
        if positions[idx] < positions[idx - 1] {
            positions[idx] = positions[idx - 1];
        }
    }
    raw_stops
        .into_iter()
        .zip(positions)
        .map(|((color, _), position)| GradientStop { color, position })
        .collect()
}

fn parse_linear_gradient_angle(value: &str) -> Option<f32> {
    let value = value.trim().to_ascii_lowercase();
    if let Some(raw) = value.strip_suffix("deg") {
        return raw.trim().parse::<f32>().ok();
    }
    match value.as_str() {
        "to top" => Some(0.0),
        "to right" => Some(90.0),
        "to bottom" => Some(180.0),
        "to left" => Some(270.0),
        "to top right" | "to right top" => Some(45.0),
        "to bottom right" | "to right bottom" => Some(135.0),
        "to bottom left" | "to left bottom" => Some(225.0),
        "to top left" | "to left top" => Some(315.0),
        _ => None,
    }
}

fn parse_radial_gradients(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Vec<RadialGradient> {
    split_top_level(value, &[','])
        .into_iter()
        .map(str::trim)
        .filter(|layer| layer.starts_with("radial-gradient("))
        .filter_map(|layer| parse_radial_gradient_layer(layer, length_context))
        .collect()
}

fn parse_radial_gradient_layer(
    layer: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<RadialGradient> {
    let length_context = length_context.into();
    let body = strip_css_function(layer, "radial-gradient")?;
    let args = split_css_function_args(body);
    let mut center_x = 0.5;
    let mut center_y = 0.5;
    let mut first_arg_is_geometry = false;
    if let Some(first) = args.first() {
        if let Some((x, y)) = parse_radial_gradient_center(first) {
            center_x = x;
            center_y = y;
            first_arg_is_geometry = true;
        }
    }
    let color = args
        .iter()
        .find_map(|arg| extract_css_color(arg).filter(|color| color.a > 0.0))?;
    let radius = args
        .iter()
        .enumerate()
        .rev()
        .filter(|(idx, _)| !first_arg_is_geometry || *idx != 0)
        .find_map(|(_, arg)| {
            split_css_whitespace(arg)
                .into_iter()
                .rev()
                .find_map(|part| parse_radial_gradient_stop_length(&part, length_context))
        })
        .unwrap_or_else(|| CssLength::percent(0.45));
    Some(RadialGradient {
        color,
        center_x,
        center_y,
        radius,
    })
}

fn parse_radial_gradient_stop_length(
    value: &str,
    length_context: LengthContext,
) -> Option<CssLength> {
    let value = value.trim();
    if matches!(
        value,
        "closest-side" | "closest-corner" | "farthest-side" | "farthest-corner"
    ) {
        return match value {
            "closest-side" => Some(CssLength::percent(0.35)),
            "closest-corner" => Some(CssLength::percent(0.5)),
            "farthest-side" => Some(CssLength::percent(0.75)),
            "farthest-corner" => Some(CssLength::percent(1.0)),
            _ => None,
        };
    }
    parse_css_length_with_rem(value, length_context)
}

fn parse_radial_gradient_center(value: &str) -> Option<(f32, f32)> {
    let lower = value.to_ascii_lowercase();
    let (_, after_at) = lower.split_once(" at ")?;
    let parts = split_css_whitespace(after_at);
    let first = parts.first()?;
    let second = parts.get(1);
    Some((
        parse_position_component(first, true)?,
        second
            .and_then(|part| parse_position_component(part, false))
            .unwrap_or(0.5),
    ))
}

fn parse_position_component(value: &str, horizontal: bool) -> Option<f32> {
    match value.trim() {
        "left" if horizontal => Some(0.0),
        "right" if horizontal => Some(1.0),
        "top" if !horizontal => Some(0.0),
        "bottom" if !horizontal => Some(1.0),
        "center" => Some(0.5),
        value => parse_percent_unit(value),
    }
}

fn is_zero_rect_clip(value: &str) -> bool {
    let Some(body) = strip_css_function(value.trim(), "rect") else {
        return false;
    };
    let parts = if body.contains(',') {
        split_top_level(body, &[','])
            .into_iter()
            .map(|part| part.trim().to_string())
            .collect::<Vec<_>>()
    } else {
        split_css_whitespace(body)
    };
    parts.len() == 4
        && parts.iter().all(|part| {
            matches!(
                part.trim(),
                "0" | "0px" | "0pt" | "0rem" | "0em" | "0%" | "0.0" | "0.0px"
            )
        })
}

#[cfg(test)]
fn parse_grid_template_columns(value: &str) -> (Option<usize>, Option<CssLength>, Vec<GridTrack>) {
    parse_grid_template_columns_with_context(value, 12.0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GridColumnPlacement {
    start: Option<usize>,
    end: Option<usize>,
    span: usize,
}

fn parse_grid_column_placement(value: &str) -> Option<GridColumnPlacement> {
    let value = value.trim().to_ascii_lowercase();
    if value.is_empty() || matches!(value.as_str(), "auto" | "initial" | "inherit" | "unset") {
        return None;
    }
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact == "1 / -1" || compact == "1/-1" {
        return Some(GridColumnPlacement {
            start: Some(1),
            end: None,
            span: usize::MAX,
        });
    }

    let parts = split_top_level(&value, &['/']);
    let start = parts
        .first()
        .and_then(|part| parse_grid_positive_line(part.trim()));
    let explicit_end = parts
        .get(1)
        .and_then(|part| parse_grid_positive_line(part.trim()));
    let explicit_span = parts
        .iter()
        .filter_map(|part| parse_grid_line_span(part.trim()))
        .max();
    let span = match (start, explicit_end, explicit_span) {
        (Some(start), Some(end), _) if end > start => end - start,
        (_, _, Some(span)) => span,
        _ => 1,
    };

    Some(GridColumnPlacement {
        start,
        end: explicit_end,
        span,
    })
}

fn parse_grid_line_span(part: &str) -> Option<usize> {
    let part = part.trim();
    if !part.starts_with("span") {
        return None;
    }
    let span = part
        .strip_prefix("span ")
        .or_else(|| part.strip_prefix("span("))
        .unwrap_or(part)
        .trim()
        .trim_end_matches(')');
    span.split_whitespace()
        .next()
        .and_then(|token| token.parse::<usize>().ok())
        .filter(|span| *span > 0)
}

fn parse_grid_positive_line(part: &str) -> Option<usize> {
    let part = part.trim();
    if part.starts_with("span") || matches!(part, "auto" | "-1") {
        return None;
    }
    part.split_whitespace()
        .next()
        .and_then(|token| token.parse::<usize>().ok())
        .filter(|line| *line > 0)
}

fn parse_grid_template_columns_with_context(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> (Option<usize>, Option<CssLength>, Vec<GridTrack>) {
    let length_context = length_context.into();
    let mut columns = None;
    let mut min_width = None;
    let mut tracks = Vec::new();
    if let Some(body) = strip_css_function(value.trim(), "repeat") {
        let args = split_css_function_args(body);
        if let Some(first) = args.first() {
            columns = first.trim().parse::<usize>().ok();
        }
        if let Some(second) = args.get(1) {
            min_width = parse_minmax_min_width(second, length_context);
            if let Some(count) = columns {
                if let Some(track) = parse_grid_track(second, length_context) {
                    for _ in 0..count {
                        tracks.push(track.clone());
                    }
                }
            }
        }
    } else {
        let track_tokens = split_css_whitespace(value);
        if !track_tokens.is_empty() {
            columns = Some(track_tokens.len());
        }
        min_width = track_tokens
            .iter()
            .find_map(|track| parse_minmax_min_width(track, length_context));
        for track_token in track_tokens {
            if let Some(track) = parse_grid_track(&track_token, length_context) {
                tracks.push(track);
            }
        }
    }
    (columns, min_width, tracks)
}

fn parse_grid_track(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<GridTrack> {
    let length_context = length_context.into();
    let value = value.trim();
    if value.eq_ignore_ascii_case("auto") {
        return Some(GridTrack::Fr {
            factor: 1.0,
            min: None,
        });
    }
    if let Some(fr) = parse_fr_unit(value) {
        return Some(GridTrack::Fr {
            factor: fr,
            min: None,
        });
    }
    if let Some(body) = strip_css_function(value, "minmax") {
        let args = split_css_function_args(body);
        let min = args
            .first()
            .and_then(|min| parse_css_length_with_rem(min, length_context));
        let max_fr = args.get(1).and_then(|max| parse_fr_unit(max));
        if let Some(fr) = max_fr {
            return Some(GridTrack::Fr { factor: fr, min });
        }
        return min.map(GridTrack::Length);
    }
    parse_css_length_with_rem(value, length_context).map(GridTrack::Length)
}

fn parse_fr_unit(value: &str) -> Option<f32> {
    value
        .trim()
        .strip_suffix("fr")?
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|factor| *factor > 0.0)
}

fn parse_minmax_min_width(
    value: &str,
    length_context: impl Into<LengthContext> + Copy,
) -> Option<CssLength> {
    let length_context = length_context.into();
    let body = strip_css_function(value.trim(), "minmax")?;
    let args = split_css_function_args(body);
    args.first()
        .and_then(|min| parse_css_length_with_rem(min, length_context))
}

fn strip_css_function<'a>(value: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name}(");
    value.strip_prefix(&prefix)?.strip_suffix(')')
}

fn split_css_function_args(value: &str) -> Vec<String> {
    split_top_level(value, &[','])
        .into_iter()
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect()
}

fn split_css_whitespace(value: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0_i32;
    let mut start = None;
    for (idx, ch) in value.char_indices() {
        match ch {
            '(' => {
                depth += 1;
                start.get_or_insert(idx);
            }
            ')' => {
                depth -= 1;
                start.get_or_insert(idx);
            }
            ch if ch.is_whitespace() && depth == 0 => {
                if let Some(start_idx) = start.take() {
                    let part = value[start_idx..idx].trim();
                    if !part.is_empty() {
                        parts.push(part.to_string());
                    }
                }
            }
            _ => {
                start.get_or_insert(idx);
            }
        }
    }
    if let Some(start_idx) = start {
        let part = value[start_idx..].trim();
        if !part.is_empty() {
            parts.push(part.to_string());
        }
    }
    parts
}

fn split_top_level<'a>(value: &'a str, delimiters: &[char]) -> Vec<&'a str> {
    let mut parts = Vec::new();
    let mut depth = 0_i32;
    let mut start = 0usize;
    for (idx, ch) in value.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ch if depth == 0 && delimiters.contains(&ch) => {
                parts.push(&value[start..idx]);
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&value[start..]);
    parts
}

fn matching_function_end(value: &str) -> Option<usize> {
    let mut depth = 0_i32;
    for (idx, ch) in value.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_hex_color(value: &str) -> Option<Color> {
    let hex = value.strip_prefix('#')?;
    let expand = |ch: char| -> Option<u8> {
        let digit = ch.to_digit(16)? as u8;
        Some(digit * 17)
    };
    let (r, g, b, a) = match hex.len() {
        3 => {
            let mut chars = hex.chars();
            (
                expand(chars.next()?)?,
                expand(chars.next()?)?,
                expand(chars.next()?)?,
                255,
            )
        }
        4 => {
            let mut chars = hex.chars();
            (
                expand(chars.next()?)?,
                expand(chars.next()?)?,
                expand(chars.next()?)?,
                expand(chars.next()?)?,
            )
        }
        6 => (
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
            255,
        ),
        8 => (
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
            u8::from_str_radix(&hex[6..8], 16).ok()?,
        ),
        _ => return None,
    };
    Some(Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: a as f32 / 255.0,
    })
}

fn parse_named_color(value: &str) -> Option<Color> {
    fn rgb8(r: u8, g: u8, b: u8) -> Color {
        Color {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }

    Some(match value {
        "aliceblue" => rgb8(240, 248, 255),
        "antiquewhite" => rgb8(250, 235, 215),
        "aqua" | "cyan" => rgb8(0, 255, 255),
        "aquamarine" => rgb8(127, 255, 212),
        "azure" => rgb8(240, 255, 255),
        "beige" => rgb8(245, 245, 220),
        "bisque" => rgb8(255, 228, 196),
        "blanchedalmond" => rgb8(255, 235, 205),
        "blueviolet" => rgb8(138, 43, 226),
        "brown" => rgb8(165, 42, 42),
        "burlywood" => rgb8(222, 184, 135),
        "cadetblue" => rgb8(95, 158, 160),
        "chartreuse" => rgb8(127, 255, 0),
        "chocolate" => rgb8(210, 105, 30),
        "coral" => rgb8(255, 127, 80),
        "cornflowerblue" => rgb8(100, 149, 237),
        "cornsilk" => rgb8(255, 248, 220),
        "crimson" => rgb8(220, 20, 60),
        "darkblue" => rgb8(0, 0, 139),
        "darkcyan" => rgb8(0, 139, 139),
        "darkgoldenrod" => rgb8(184, 134, 11),
        "darkgray" | "darkgrey" => rgb8(169, 169, 169),
        "darkgreen" => rgb8(0, 100, 0),
        "darkorange" => rgb8(255, 140, 0),
        "darkred" => rgb8(139, 0, 0),
        "darkslateblue" => rgb8(72, 61, 139),
        "darkslategray" | "darkslategrey" => rgb8(47, 79, 79),
        "deeppink" => rgb8(255, 20, 147),
        "deepskyblue" => rgb8(0, 191, 255),
        "dimgray" | "dimgrey" => rgb8(105, 105, 105),
        "dodgerblue" => rgb8(30, 144, 255),
        "firebrick" => rgb8(178, 34, 34),
        "floralwhite" => rgb8(255, 250, 240),
        "forestgreen" => rgb8(34, 139, 34),
        "fuchsia" | "magenta" => rgb8(255, 0, 255),
        "gainsboro" => rgb8(220, 220, 220),
        "ghostwhite" => rgb8(248, 248, 255),
        "gold" => rgb8(255, 215, 0),
        "goldenrod" => rgb8(218, 165, 32),
        "greenyellow" => rgb8(173, 255, 47),
        "honeydew" => rgb8(240, 255, 240),
        "hotpink" => rgb8(255, 105, 180),
        "indianred" => rgb8(205, 92, 92),
        "indigo" => rgb8(75, 0, 130),
        "ivory" => rgb8(255, 255, 240),
        "khaki" => rgb8(240, 230, 140),
        "lavender" => rgb8(230, 230, 250),
        "lavenderblush" => rgb8(255, 240, 245),
        "lawngreen" => rgb8(124, 252, 0),
        "lemonchiffon" => rgb8(255, 250, 205),
        "lightblue" => rgb8(173, 216, 230),
        "lightcoral" => rgb8(240, 128, 128),
        "lightcyan" => rgb8(224, 255, 255),
        "lightgoldenrodyellow" => rgb8(250, 250, 210),
        "lightgray" | "lightgrey" => rgb8(211, 211, 211),
        "lightgreen" => rgb8(144, 238, 144),
        "lightpink" => rgb8(255, 182, 193),
        "lightsalmon" => rgb8(255, 160, 122),
        "lightskyblue" => rgb8(135, 206, 250),
        "lightslategray" | "lightslategrey" => rgb8(119, 136, 153),
        "lime" => rgb8(0, 255, 0),
        "limegreen" => rgb8(50, 205, 50),
        "linen" => rgb8(250, 240, 230),
        "maroon" => rgb8(128, 0, 0),
        "midnightblue" => rgb8(25, 25, 112),
        "mintcream" => rgb8(245, 255, 250),
        "mistyrose" => rgb8(255, 228, 225),
        "moccasin" => rgb8(255, 228, 181),
        "navy" => rgb8(0, 0, 128),
        "oldlace" => rgb8(253, 245, 230),
        "olive" => rgb8(128, 128, 0),
        "olivedrab" => rgb8(107, 142, 35),
        "orange" => rgb8(255, 165, 0),
        "orangered" => rgb8(255, 69, 0),
        "orchid" => rgb8(218, 112, 214),
        "palegoldenrod" => rgb8(238, 232, 170),
        "palegreen" => rgb8(152, 251, 152),
        "paleturquoise" => rgb8(175, 238, 238),
        "papayawhip" => rgb8(255, 239, 213),
        "peachpuff" => rgb8(255, 218, 185),
        "peru" => rgb8(205, 133, 63),
        "pink" => rgb8(255, 192, 203),
        "plum" => rgb8(221, 160, 221),
        "powderblue" => rgb8(176, 224, 230),
        "purple" => rgb8(128, 0, 128),
        "rebeccapurple" => rgb8(102, 51, 153),
        "rosybrown" => rgb8(188, 143, 143),
        "royalblue" => rgb8(65, 105, 225),
        "saddlebrown" => rgb8(139, 69, 19),
        "salmon" => rgb8(250, 128, 114),
        "sandybrown" => rgb8(244, 164, 96),
        "seagreen" => rgb8(46, 139, 87),
        "seashell" => rgb8(255, 245, 238),
        "sienna" => rgb8(160, 82, 45),
        "silver" => rgb8(192, 192, 192),
        "skyblue" => rgb8(135, 206, 235),
        "slateblue" => rgb8(106, 90, 205),
        "slategray" | "slategrey" => rgb8(112, 128, 144),
        "snow" => rgb8(255, 250, 250),
        "springgreen" => rgb8(0, 255, 127),
        "steelblue" => rgb8(70, 130, 180),
        "tan" => rgb8(210, 180, 140),
        "teal" => rgb8(0, 128, 128),
        "thistle" => rgb8(216, 191, 216),
        "tomato" => rgb8(255, 99, 71),
        "turquoise" => rgb8(64, 224, 208),
        "violet" => rgb8(238, 130, 238),
        "wheat" => rgb8(245, 222, 179),
        "yellow" => rgb8(255, 255, 0),
        "yellowgreen" => rgb8(154, 205, 50),
        "darkkhaki" => rgb8(189, 183, 107),
        "darkmagenta" => rgb8(139, 0, 139),
        "darkolivegreen" => rgb8(85, 107, 47),
        "darkorchid" => rgb8(153, 50, 204),
        "darksalmon" => rgb8(233, 150, 122),
        "darkseagreen" => rgb8(143, 188, 143),
        "darkturquoise" => rgb8(0, 206, 209),
        "darkviolet" => rgb8(148, 0, 211),
        "lightseagreen" => rgb8(32, 178, 170),
        "lightsteelblue" => rgb8(176, 196, 222),
        "lightyellow" => rgb8(255, 255, 224),
        "mediumaquamarine" => rgb8(102, 205, 170),
        "mediumblue" => rgb8(0, 0, 205),
        "mediumorchid" => rgb8(186, 85, 211),
        "mediumpurple" => rgb8(147, 112, 219),
        "mediumseagreen" => rgb8(60, 179, 113),
        "mediumslateblue" => rgb8(123, 104, 238),
        "mediumspringgreen" => rgb8(0, 250, 154),
        "mediumturquoise" => rgb8(72, 209, 204),
        "mediumvioletred" => rgb8(199, 21, 133),
        "navajowhite" => rgb8(255, 222, 173),
        "palevioletred" => rgb8(219, 112, 147),
        "whitesmoke" => rgb8(245, 245, 245),
        _ => return None,
    })
}

fn parse_rgb_color(value: &str) -> Option<Color> {
    let raw_body = value
        .strip_prefix("rgba(")
        .or_else(|| value.strip_prefix("rgb("))?
        .strip_suffix(')')?;
    let body = raw_body.trim();
    if body.to_ascii_lowercase().starts_with("from ") {
        return parse_relative_rgb_color(body);
    }

    let body = body.replace(',', " ").replace('/', " ");
    let parts = body.split_whitespace().collect::<Vec<_>>();
    let channels = parts
        .iter()
        .take(3)
        .copied()
        .map(parse_rgb_channel)
        .collect::<Option<Vec<_>>>()?;
    if channels.len() != 3 {
        return None;
    }
    let alpha = parts
        .get(3)
        .and_then(|value| parse_alpha_channel(value))
        .unwrap_or(1.0);
    Some(Color {
        r: channels[0],
        g: channels[1],
        b: channels[2],
        a: alpha,
    })
}

fn parse_relative_rgb_color(body: &str) -> Option<Color> {
    let normalized = body.replace('/', " / ").replace(',', " ");
    let tokens = split_css_whitespace(&normalized);
    if tokens.len() < 5 || !tokens.first()?.eq_ignore_ascii_case("from") {
        return None;
    }
    let base = Color::from_css(tokens.get(1)?.trim())?;
    let slash_index = tokens.iter().position(|token| token == "/");
    let channel_end = slash_index.unwrap_or(tokens.len());
    if channel_end < 5 {
        return None;
    }
    let channels = &tokens[2..channel_end];
    if channels.len() < 3 {
        return None;
    }
    let alpha = slash_index
        .and_then(|idx| tokens.get(idx + 1))
        .and_then(|value| parse_relative_alpha_channel(value, base))
        .unwrap_or(base.a);

    Some(Color {
        r: parse_relative_rgb_channel(&channels[0], base, 'r')?,
        g: parse_relative_rgb_channel(&channels[1], base, 'g')?,
        b: parse_relative_rgb_channel(&channels[2], base, 'b')?,
        a: alpha,
    })
}

fn parse_relative_rgb_channel(value: &str, base: Color, fallback_channel: char) -> Option<f32> {
    let value = value.trim();
    match value.to_ascii_lowercase().as_str() {
        "r" => Some(base.r),
        "g" => Some(base.g),
        "b" => Some(base.b),
        "none" => Some(0.0),
        _ => parse_rgb_channel(value)
            .or_else(|| parse_simple_relative_rgb_calc(value, base, fallback_channel)),
    }
}

fn parse_relative_alpha_channel(value: &str, base: Color) -> Option<f32> {
    let value = value.trim();
    match value.to_ascii_lowercase().as_str() {
        "alpha" | "a" => Some(base.a),
        "none" => Some(1.0),
        _ => parse_alpha_channel(value).or_else(|| parse_simple_relative_alpha_calc(value, base)),
    }
}

fn parse_simple_relative_rgb_calc(value: &str, base: Color, fallback_channel: char) -> Option<f32> {
    let body = strip_css_function(value.trim(), "calc")?.trim();
    let channel_value = match fallback_channel {
        'r' => base.r * 255.0,
        'g' => base.g * 255.0,
        'b' => base.b * 255.0,
        _ => return None,
    };
    parse_simple_channel_calc(body, fallback_channel, channel_value, 255.0)
        .map(|v| (v / 255.0).clamp(0.0, 1.0))
}

fn parse_simple_relative_alpha_calc(value: &str, base: Color) -> Option<f32> {
    let body = strip_css_function(value.trim(), "calc")?.trim();
    parse_simple_channel_calc(body, 'a', base.a, 1.0)
        .or_else(|| parse_simple_channel_calc(body, 'α', base.a, 1.0))
        .map(|v| v.clamp(0.0, 1.0))
}

fn parse_simple_channel_calc(
    body: &str,
    channel: char,
    channel_value: f32,
    scale: f32,
) -> Option<f32> {
    let normalized = body
        .replace('*', " * ")
        .replace('+', " + ")
        .replace('-', " - ");
    let parts = normalized.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        [name] if channel_name_matches(name, channel) => Some(channel_value),
        [name, "*", factor] | [factor, "*", name] if channel_name_matches(name, channel) => factor
            .parse::<f32>()
            .ok()
            .map(|factor| channel_value * factor),
        [name, "+", amount] if channel_name_matches(name, channel) => {
            parse_channel_calc_amount(amount, scale).map(|amount| channel_value + amount)
        }
        [name, "-", amount] if channel_name_matches(name, channel) => {
            parse_channel_calc_amount(amount, scale).map(|amount| channel_value - amount)
        }
        _ => None,
    }
}

fn channel_name_matches(value: &str, channel: char) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    match channel {
        'r' | 'g' | 'b' => lower == channel.to_string(),
        'a' | 'α' => lower == "a" || lower == "alpha",
        _ => false,
    }
}

fn parse_channel_calc_amount(value: &str, scale: f32) -> Option<f32> {
    if let Some(percent) = value.trim().strip_suffix('%') {
        return percent
            .trim()
            .parse::<f32>()
            .ok()
            .map(|value| value * scale / 100.0);
    }
    value.trim().parse::<f32>().ok()
}

fn parse_rgb_channel(value: &str) -> Option<f32> {
    if let Some(percent) = value.strip_suffix('%') {
        return percent
            .trim()
            .parse::<f32>()
            .ok()
            .map(|v| (v / 100.0).clamp(0.0, 1.0));
    }
    value
        .trim()
        .parse::<f32>()
        .ok()
        .map(|v| (v / 255.0).clamp(0.0, 1.0))
}

fn parse_alpha_channel(value: &str) -> Option<f32> {
    if let Some(percent) = value.strip_suffix('%') {
        return percent
            .trim()
            .parse::<f32>()
            .ok()
            .map(|v| (v / 100.0).clamp(0.0, 1.0));
    }
    value.trim().parse::<f32>().ok().map(|v| v.clamp(0.0, 1.0))
}

fn parse_hsl_color(value: &str) -> Option<Color> {
    let body = value
        .strip_prefix("hsla(")
        .or_else(|| value.strip_prefix("hsl("))?
        .strip_suffix(')')?
        .replace(',', " ")
        .replace('/', " ");
    let channels = body.split_whitespace().collect::<Vec<_>>();
    if channels.len() < 3 {
        return None;
    }
    let hue = parse_hue_degrees(channels[0])?;
    let saturation = parse_percent_unit(channels[1])?;
    let lightness = parse_percent_unit(channels[2])?;
    let alpha = channels
        .get(3)
        .and_then(|value| parse_alpha_channel(value))
        .unwrap_or(1.0);
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let hue_sector = (hue / 60.0).rem_euclid(6.0);
    let x = chroma * (1.0 - (hue_sector.rem_euclid(2.0) - 1.0).abs());
    let (r1, g1, b1) = match hue_sector {
        h if (0.0..1.0).contains(&h) => (chroma, x, 0.0),
        h if (1.0..2.0).contains(&h) => (x, chroma, 0.0),
        h if (2.0..3.0).contains(&h) => (0.0, chroma, x),
        h if (3.0..4.0).contains(&h) => (0.0, x, chroma),
        h if (4.0..5.0).contains(&h) => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = lightness - chroma / 2.0;
    Some(Color {
        r: (r1 + m).clamp(0.0, 1.0),
        g: (g1 + m).clamp(0.0, 1.0),
        b: (b1 + m).clamp(0.0, 1.0),
        a: alpha,
    })
}

fn parse_hwb_color(value: &str) -> Option<Color> {
    let body = value
        .strip_prefix("hwb(")?
        .strip_suffix(')')?
        .replace(',', " ")
        .replace('/', " ");
    let channels = body.split_whitespace().collect::<Vec<_>>();
    if channels.len() < 3 {
        return None;
    }
    let hue = parse_hue_degrees(channels[0])?;
    let whiteness = parse_percent_unit(channels[1])?;
    let blackness = parse_percent_unit(channels[2])?;
    let alpha = channels
        .get(3)
        .and_then(|value| parse_alpha_channel(value))
        .unwrap_or(1.0);
    Some(hwb_to_srgb(hue, whiteness, blackness, alpha))
}

fn hwb_to_srgb(hue: f32, whiteness: f32, blackness: f32, alpha: f32) -> Color {
    let sum = whiteness + blackness;
    if sum >= 1.0 {
        let gray = if sum > 0.0 { whiteness / sum } else { 0.0 };
        return Color {
            r: gray,
            g: gray,
            b: gray,
            a: alpha,
        };
    }

    let chroma = 1.0 - blackness - whiteness;
    let hue_sector = (hue / 60.0).rem_euclid(6.0);
    let x = 1.0 - (hue_sector.rem_euclid(2.0) - 1.0).abs();
    let (r1, g1, b1) = match hue_sector {
        h if (0.0..1.0).contains(&h) => (1.0, x, 0.0),
        h if (1.0..2.0).contains(&h) => (x, 1.0, 0.0),
        h if (2.0..3.0).contains(&h) => (0.0, 1.0, x),
        h if (3.0..4.0).contains(&h) => (0.0, x, 1.0),
        h if (4.0..5.0).contains(&h) => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };
    Color {
        r: (r1 * chroma + whiteness).clamp(0.0, 1.0),
        g: (g1 * chroma + whiteness).clamp(0.0, 1.0),
        b: (b1 * chroma + whiteness).clamp(0.0, 1.0),
        a: alpha,
    }
}

fn parse_lab_color(value: &str) -> Option<Color> {
    let body = value.strip_prefix("lab(")?.strip_suffix(')')?;
    let channels = body
        .replace('/', " ")
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if channels.len() < 3 {
        return None;
    }
    let lightness = parse_lab_lightness(&channels[0])?;
    let a = parse_lab_axis(&channels[1])?;
    let b = parse_lab_axis(&channels[2])?;
    let alpha = channels
        .get(3)
        .and_then(|value| parse_alpha_channel(value))
        .unwrap_or(1.0);

    Some(lab_to_srgb(lightness, a, b, alpha))
}

fn parse_lch_color(value: &str) -> Option<Color> {
    let body = value.strip_prefix("lch(")?.strip_suffix(')')?;
    let channels = body
        .replace('/', " ")
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if channels.len() < 3 {
        return None;
    }
    let lightness = parse_lab_lightness(&channels[0])?;
    let chroma = parse_lch_chroma(&channels[1])?;
    let hue = parse_hue_degrees(&channels[2])?.to_radians();
    let alpha = channels
        .get(3)
        .and_then(|value| parse_alpha_channel(value))
        .unwrap_or(1.0);
    let a = chroma * hue.cos();
    let b = chroma * hue.sin();

    Some(lab_to_srgb(lightness, a, b, alpha))
}

fn parse_lab_lightness(value: &str) -> Option<f32> {
    let value = value.trim();
    if let Some(percent) = value.strip_suffix('%') {
        return percent
            .trim()
            .parse::<f32>()
            .ok()
            .map(|value| value.clamp(0.0, 100.0));
    }
    value
        .parse::<f32>()
        .ok()
        .map(|value| value.clamp(0.0, 100.0))
}

fn parse_lab_axis(value: &str) -> Option<f32> {
    let value = value.trim();
    if let Some(percent) = value.strip_suffix('%') {
        return percent
            .trim()
            .parse::<f32>()
            .ok()
            .map(|value| (value / 100.0) * 125.0);
    }
    value.parse::<f32>().ok()
}

fn parse_lch_chroma(value: &str) -> Option<f32> {
    let value = value.trim();
    if let Some(percent) = value.strip_suffix('%') {
        return percent
            .trim()
            .parse::<f32>()
            .ok()
            .map(|value| ((value / 100.0) * 150.0).max(0.0));
    }
    value.parse::<f32>().ok().map(|value| value.max(0.0))
}

fn lab_to_srgb(lightness: f32, a: f32, b: f32, alpha: f32) -> Color {
    let fy = (lightness + 16.0) / 116.0;
    let fx = fy + a / 500.0;
    let fz = fy - b / 200.0;
    let x_d50 = 0.964_22 * lab_inv_f(fx);
    let y_d50 = lab_inv_f(fy);
    let z_d50 = 0.825_21 * lab_inv_f(fz);

    // CSS Lab/LCH are D50-referenced. Convert to D65 before the sRGB matrix.
    let x = 0.955_576_6 * x_d50 - 0.023_039_3 * y_d50 + 0.063_163_6 * z_d50;
    let y = -0.028_289_5 * x_d50 + 1.009_941_6 * y_d50 + 0.021_007_7 * z_d50;
    let z = 0.012_298_2 * x_d50 - 0.020_483 * y_d50 + 1.329_909_8 * z_d50;

    Color {
        r: linear_srgb_to_srgb(3.240_454_2 * x - 1.537_138_5 * y - 0.498_531_4 * z),
        g: linear_srgb_to_srgb(-0.969_266 * x + 1.876_010_8 * y + 0.041_556 * z),
        b: linear_srgb_to_srgb(0.055_643_4 * x - 0.204_025_9 * y + 1.057_225_2 * z),
        a: alpha,
    }
}

fn srgb_to_lab(color: Color) -> (f32, f32, f32) {
    let r = srgb_to_linear(color.r);
    let g = srgb_to_linear(color.g);
    let b = srgb_to_linear(color.b);
    let x_d65 = 0.412_456_4 * r + 0.357_576_1 * g + 0.180_437_5 * b;
    let y_d65 = 0.212_672_9 * r + 0.715_152_2 * g + 0.072_175 * b;
    let z_d65 = 0.019_333_9 * r + 0.119_192 * g + 0.950_304_1 * b;

    // CSS Lab/LCH are D50-referenced. Convert D65 sRGB XYZ to D50.
    let x_d50 = 1.047_811_2 * x_d65 + 0.022_886_6 * y_d65 - 0.050_127 * z_d65;
    let y_d50 = 0.029_542_4 * x_d65 + 0.990_484_4 * y_d65 - 0.017_049_1 * z_d65;
    let z_d50 = -0.009_234_5 * x_d65 + 0.015_043_6 * y_d65 + 0.752_131_6 * z_d65;

    let fx = lab_f(x_d50 / 0.964_22);
    let fy = lab_f(y_d50);
    let fz = lab_f(z_d50 / 0.825_21);
    (116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz))
}

fn lab_inv_f(value: f32) -> f32 {
    const EPSILON: f32 = 216.0 / 24_389.0;
    const KAPPA: f32 = 24_389.0 / 27.0;
    let cube = value * value * value;
    if cube > EPSILON {
        cube
    } else {
        (116.0 * value - 16.0) / KAPPA
    }
}

fn lab_f(value: f32) -> f32 {
    const EPSILON: f32 = 216.0 / 24_389.0;
    const KAPPA: f32 = 24_389.0 / 27.0;
    if value > EPSILON {
        value.cbrt()
    } else {
        (KAPPA * value + 16.0) / 116.0
    }
}

fn parse_oklch_color(value: &str) -> Option<Color> {
    let body = value.strip_prefix("oklch(")?.strip_suffix(')')?;
    let channels = body
        .replace('/', " ")
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if channels.len() < 3 {
        return None;
    }
    let alpha = channels
        .get(3)
        .and_then(|value| parse_alpha_channel(value))
        .unwrap_or(1.0);
    let lightness = if let Some(percent) = channels[0].strip_suffix('%') {
        percent.trim().parse::<f32>().ok()? / 100.0
    } else {
        channels[0].trim().parse::<f32>().ok()?
    }
    .clamp(0.0, 1.0);
    let chroma = channels[1].trim().parse::<f32>().ok()?.max(0.0);
    let hue = parse_hue_degrees(&channels[2])?.to_radians();
    let a = chroma * hue.cos();
    let b = chroma * hue.sin();

    Some(oklab_to_srgb(lightness, a, b, alpha))
}

fn parse_oklab_color(value: &str) -> Option<Color> {
    let body = value.strip_prefix("oklab(")?.strip_suffix(')')?;
    let channels = body
        .replace('/', " ")
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if channels.len() < 3 {
        return None;
    }
    let alpha = channels
        .get(3)
        .and_then(|value| parse_alpha_channel(value))
        .unwrap_or(1.0);
    let lightness = if let Some(percent) = channels[0].strip_suffix('%') {
        percent.trim().parse::<f32>().ok()? / 100.0
    } else {
        channels[0].trim().parse::<f32>().ok()?
    }
    .clamp(0.0, 1.0);
    let a = parse_oklab_axis(&channels[1])?;
    let b = parse_oklab_axis(&channels[2])?;

    Some(oklab_to_srgb(lightness, a, b, alpha))
}

fn parse_oklab_axis(value: &str) -> Option<f32> {
    if let Some(percent) = value.trim().strip_suffix('%') {
        return percent
            .trim()
            .parse::<f32>()
            .ok()
            .map(|value| value / 100.0);
    }
    value.trim().parse::<f32>().ok()
}

fn oklab_to_srgb(lightness: f32, a: f32, b: f32, alpha: f32) -> Color {
    let l_prime = lightness + 0.396_337_78 * a + 0.215_803_76 * b;
    let m_prime = lightness - 0.105_561_346 * a - 0.063_854_17 * b;
    let s_prime = lightness - 0.089_484_18 * a - 1.291_485_5 * b;

    let l = l_prime.powi(3);
    let m = m_prime.powi(3);
    let s = s_prime.powi(3);

    Color {
        r: linear_srgb_to_srgb(4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s),
        g: linear_srgb_to_srgb(-1.268_438 * l + 2.609_757_4 * m - 0.341_319_4 * s),
        b: linear_srgb_to_srgb(-0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s),
        a: alpha,
    }
}

fn srgb_to_oklab(color: Color) -> (f32, f32, f32) {
    let r = srgb_to_linear(color.r);
    let g = srgb_to_linear(color.g);
    let b = srgb_to_linear(color.b);
    let l = 0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b;
    let m = 0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b;
    let s = 0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b;
    let l_prime = l.cbrt();
    let m_prime = m.cbrt();
    let s_prime = s.cbrt();
    (
        0.210_454_26 * l_prime + 0.793_617_8 * m_prime - 0.004_072_047 * s_prime,
        1.977_998_5 * l_prime - 2.428_592_2 * m_prime + 0.450_593_7 * s_prime,
        0.025_904_037 * l_prime + 0.782_771_77 * m_prime - 0.808_675_77 * s_prime,
    )
}

fn parse_css_color_function(value: &str) -> Option<Color> {
    let body = value.strip_prefix("color(")?.strip_suffix(')')?;
    let normalized = body.replace('/', " / ");
    let parts = split_css_whitespace(&normalized);
    if parts.len() < 4 {
        return None;
    }
    let space = parts.first()?.to_ascii_lowercase();
    let slash_index = parts
        .iter()
        .position(|part| part == "/")
        .unwrap_or(parts.len());
    if slash_index < 4 {
        return None;
    }
    let channels = &parts[1..slash_index];
    let r = parse_color_function_channel(&channels[0])?;
    let g = parse_color_function_channel(&channels[1])?;
    let b = parse_color_function_channel(&channels[2])?;
    let alpha = parts
        .get(slash_index + 1)
        .and_then(|value| parse_alpha_channel(value))
        .unwrap_or(1.0);

    match space.as_str() {
        "srgb" | "srgb-linear" => Some(Color {
            r: r.clamp(0.0, 1.0),
            g: g.clamp(0.0, 1.0),
            b: b.clamp(0.0, 1.0),
            a: alpha,
        }),
        "display-p3" => Some(display_p3_to_srgb(r, g, b, alpha)),
        _ => None,
    }
}

fn parse_color_function_channel(value: &str) -> Option<f32> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("none") {
        return Some(0.0);
    }
    if let Some(percent) = value.strip_suffix('%') {
        return percent
            .trim()
            .parse::<f32>()
            .ok()
            .map(|value| value / 100.0);
    }
    value.parse::<f32>().ok()
}

fn display_p3_to_srgb(r: f32, g: f32, b: f32, alpha: f32) -> Color {
    let r = srgb_to_linear(r);
    let g = srgb_to_linear(g);
    let b = srgb_to_linear(b);

    let x = 0.486_570_95 * r + 0.265_667_7 * g + 0.198_217_29 * b;
    let y = 0.228_974_57 * r + 0.691_738_5 * g + 0.079_286_91 * b;
    let z = 0.045_113_38 * g + 1.043_944_4 * b;

    Color {
        r: linear_srgb_to_srgb(3.240_454_2 * x - 1.537_138_5 * y - 0.498_531_4 * z),
        g: linear_srgb_to_srgb(-0.969_266 * x + 1.876_010_8 * y + 0.041_556 * z),
        b: linear_srgb_to_srgb(0.055_643_4 * x - 0.204_025_9 * y + 1.057_225_2 * z),
        a: alpha.clamp(0.0, 1.0),
    }
}

fn parse_color_mix(value: &str) -> Option<Color> {
    let body = value.strip_prefix("color-mix(")?.strip_suffix(')')?;
    let parts = split_top_level(body, &[',']);
    if parts.len() < 3 {
        return None;
    }
    let space = parse_color_mix_space(parts[0].trim())?;

    let (first_color, first_weight) = parse_color_mix_stop(parts[1].trim())?;
    let (second_color, second_weight) = parse_color_mix_stop(parts[2].trim())?;
    let (mut first_weight, mut second_weight) = match (first_weight, second_weight) {
        (Some(first), Some(second)) => (first, second),
        (Some(first), None) => (first, 1.0 - first),
        (None, Some(second)) => (1.0 - second, second),
        (None, None) => (0.5, 0.5),
    };
    first_weight = first_weight.max(0.0);
    second_weight = second_weight.max(0.0);
    let total = first_weight + second_weight;
    if total <= f32::EPSILON {
        return Some(Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        });
    }
    first_weight /= total;
    second_weight /= total;

    Some(mix_colors_in_space(
        first_color,
        first_weight,
        second_color,
        second_weight,
        space,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColorMixSpace {
    Srgb,
    Lab,
    Oklab,
}

fn parse_color_mix_space(value: &str) -> Option<ColorMixSpace> {
    let space = value
        .trim()
        .strip_prefix("in ")?
        .split_whitespace()
        .next()?;
    match space {
        "srgb" | "srgb-linear" | "xyz" | "xyz-d50" | "xyz-d65" => Some(ColorMixSpace::Srgb),
        "lab" | "lch" => Some(ColorMixSpace::Lab),
        "oklab" | "oklch" => Some(ColorMixSpace::Oklab),
        _ => None,
    }
}

fn mix_colors_in_space(
    first_color: Color,
    first_weight: f32,
    second_color: Color,
    second_weight: f32,
    space: ColorMixSpace,
) -> Color {
    let alpha = first_color.a * first_weight + second_color.a * second_weight;
    if alpha <= f32::EPSILON {
        return Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        };
    }

    let mix_channel = |first: f32, second: f32| {
        (first * first_color.a * first_weight + second * second_color.a * second_weight) / alpha
    };
    let mut color = match space {
        ColorMixSpace::Srgb => Color {
            r: mix_channel(first_color.r, second_color.r).clamp(0.0, 1.0),
            g: mix_channel(first_color.g, second_color.g).clamp(0.0, 1.0),
            b: mix_channel(first_color.b, second_color.b).clamp(0.0, 1.0),
            a: alpha,
        },
        ColorMixSpace::Lab => {
            let first = srgb_to_lab(first_color);
            let second = srgb_to_lab(second_color);
            lab_to_srgb(
                mix_channel(first.0, second.0),
                mix_channel(first.1, second.1),
                mix_channel(first.2, second.2),
                alpha,
            )
        }
        ColorMixSpace::Oklab => {
            let first = srgb_to_oklab(first_color);
            let second = srgb_to_oklab(second_color);
            oklab_to_srgb(
                mix_channel(first.0, second.0),
                mix_channel(first.1, second.1),
                mix_channel(first.2, second.2),
                alpha,
            )
        }
    };
    color.a = alpha.clamp(0.0, 1.0);
    color
}

fn parse_color_mix_stop(value: &str) -> Option<(Color, Option<f32>)> {
    let mut parts = split_css_whitespace(value);
    let weight = parts.last().and_then(|part| {
        if part.ends_with('%') {
            parse_percent_unit(part)
        } else {
            None
        }
    });
    if weight.is_some() {
        parts.pop();
    }
    let color_value = parts.join(" ");
    Some((Color::from_css(color_value.trim())?, weight))
}

fn parse_light_dark_color(value: &str) -> Option<Color> {
    let body = value.strip_prefix("light-dark(")?.strip_suffix(')')?;
    let parts = split_top_level(body, &[',']);
    let light = parts.first()?.trim();
    Color::from_css(light)
}

fn linear_srgb_to_srgb(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    if value <= 0.003_130_8 {
        12.92 * value
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
    .clamp(0.0, 1.0)
}

fn srgb_to_linear(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn parse_hue_degrees(value: &str) -> Option<f32> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("none") {
        return Some(0.0);
    }
    let lower = value.to_ascii_lowercase();
    let (raw, multiplier) = if let Some(raw) = lower.strip_suffix("deg") {
        (raw, 1.0)
    } else if let Some(raw) = lower.strip_suffix("grad") {
        (raw, 0.9)
    } else if let Some(raw) = lower.strip_suffix("rad") {
        (raw, 180.0 / std::f32::consts::PI)
    } else if let Some(raw) = lower.strip_suffix("turn") {
        (raw, 360.0)
    } else {
        (lower.as_str(), 1.0)
    };
    raw.trim()
        .parse::<f32>()
        .ok()
        .map(|angle| (angle * multiplier).rem_euclid(360.0))
}

fn parse_percent_unit(value: &str) -> Option<f32> {
    value
        .trim()
        .strip_suffix('%')?
        .parse::<f32>()
        .ok()
        .map(|percent| (percent / 100.0).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_document;

    #[test]
    fn parses_float_and_clear_flow_properties() {
        let document = parse_document(
            r#"<div class="left"></div><div class="right"></div><div class="clear"></div>"#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![
            ".left { float: left; } .right { float: inline-end; direction: ltr; } .clear { clear: both; }"
                .to_string(),
        ]);

        let left = ComputedStyle::for_node(
            &document,
            &stylesheet,
            document.query_selector(".left").expect("left exists"),
        );
        let right = ComputedStyle::for_node(
            &document,
            &stylesheet,
            document.query_selector(".right").expect("right exists"),
        );
        let clear = ComputedStyle::for_node(
            &document,
            &stylesheet,
            document.query_selector(".clear").expect("clear exists"),
        );

        assert_eq!(left.float, FloatSide::Left);
        assert_eq!(right.float, FloatSide::Right);
        assert_eq!(clear.clear, ClearSide::Both);
    }

    #[test]
    fn parses_custom_page_dimensions_and_landscape_orientation() {
        let stylesheet = Stylesheet::from_css_chunks(vec![
            "@page { size: 220pt 140pt; margin: 10pt; }".to_string(),
        ]);
        let page = stylesheet.page_options(PageOptions::letter());
        assert!((page.width_pt - 220.0).abs() < 0.01);
        assert!((page.height_pt - 140.0).abs() < 0.01);
        assert!((page.margin_top_pt - 10.0).abs() < 0.01);

        let landscape =
            Stylesheet::from_css_chunks(vec!["@page { size: A4 landscape; }".to_string()])
                .page_options(PageOptions::letter());
        assert!((landscape.width_pt - PageOptions::A4_HEIGHT_PT).abs() < 0.01);
        assert!((landscape.height_pt - PageOptions::A4_WIDTH_PT).abs() < 0.01);
    }

    #[test]
    fn expands_is_and_where_selector_lists_across_all_arguments() {
        let expanded = expand_selector_list_functions(
            ".card :is(.unused, .target) > :where(.missing, .label)",
        );

        assert_eq!(
            expanded,
            vec![
                ".card .unused > .missing",
                ".card .unused > .label",
                ".card .target > .missing",
                ".card .target > .label",
            ]
        );
    }

    #[test]
    fn keeps_selector_function_commas_out_of_top_level_selector_split() {
        let selectors = split_top_level(".a, :is(.b, .c), .d", &[','])
            .into_iter()
            .map(str::trim)
            .collect::<Vec<_>>();

        assert_eq!(selectors, vec![".a", ":is(.b, .c)", ".d"]);
    }

    #[test]
    fn matches_modern_complex_not_selector_lists() {
        let document = parse_document(
            r#"
            <section class="card">
                <div class="enabled">
                    <p class="target allowed">Allowed</p>
                </div>
                <div class="disabled">
                    <p class="target denied">Denied</p>
                </div>
                <p class="target blocked">Blocked</p>
            </section>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target { color: rgb(220 38 38); }
            .card :not(.disabled .target, .blocked) { color: rgb(37 99 235); }
            "#
        .to_string()]);

        let allowed = document.query_selector(".allowed").expect("allowed exists");
        let denied = document.query_selector(".denied").expect("denied exists");
        let blocked = document.query_selector(".blocked").expect("blocked exists");

        let allowed_style = ComputedStyle::for_node(&document, &stylesheet, allowed);
        let denied_style = ComputedStyle::for_node(&document, &stylesheet, denied);
        let blocked_style = ComputedStyle::for_node(&document, &stylesheet, blocked);

        assert!((allowed_style.color.r - 37.0 / 255.0).abs() < 0.01);
        assert!((allowed_style.color.b - 235.0 / 255.0).abs() < 0.01);
        assert!((denied_style.color.r - 220.0 / 255.0).abs() < 0.01);
        assert!((blocked_style.color.r - 220.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn parses_fractional_and_fixed_grid_tracks() {
        let (columns, _, tracks) = parse_grid_template_columns("1.3fr 0.7fr");

        assert_eq!(columns, Some(2));
        assert_eq!(
            tracks,
            vec![
                GridTrack::Fr {
                    factor: 1.3,
                    min: None
                },
                GridTrack::Fr {
                    factor: 0.7,
                    min: None
                }
            ]
        );

        let (columns, _, tracks) = parse_grid_template_columns("1fr 320px");
        assert_eq!(columns, Some(2));
        assert!(
            matches!(tracks.first(), Some(GridTrack::Fr { factor, .. }) if (*factor - 1.0).abs() < f32::EPSILON)
        );
        assert!(matches!(tracks.get(1), Some(GridTrack::Length(_))));
    }

    #[test]
    fn parses_grid_template_rows_tracks() {
        let document =
            parse_document(r#"<section class="target">Grid rows</section>"#).expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target {
                display: grid;
                grid-template-rows: 24px minmax(32px, auto) 1fr;
            }
            "#
        .to_string()]);
        let target = document
            .query_selector(".target")
            .expect("target element exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert_eq!(
            style.grid_row_tracks,
            vec![
                GridTrack::Length(CssLength::points(18.0)),
                GridTrack::Length(CssLength::points(24.0)),
                GridTrack::Fr {
                    factor: 1.0,
                    min: None
                },
            ]
        );
    }

    #[test]
    fn parses_repeat_fractional_grid_tracks() {
        let (columns, _, tracks) = parse_grid_template_columns("repeat(3, 1fr)");

        assert_eq!(columns, Some(3));
        assert_eq!(tracks.len(), 3);
        assert!(
            tracks
                .iter()
                .all(|track| matches!(track, GridTrack::Fr { factor, .. } if (*factor - 1.0).abs() < f32::EPSILON))
        );
    }

    #[test]
    fn parses_grid_column_span_for_framework_grid_utilities() {
        assert_eq!(
            parse_grid_column_placement("span 2 / span 2").map(|placement| placement.span),
            Some(2)
        );
        assert_eq!(
            parse_grid_column_placement("auto / span 5").map(|placement| placement.span),
            Some(5)
        );
        assert_eq!(
            parse_grid_column_placement("1 / -1").map(|placement| placement.span),
            Some(usize::MAX)
        );
        assert_eq!(
            parse_grid_column_placement("2 / 4"),
            Some(GridColumnPlacement {
                start: Some(2),
                end: Some(4),
                span: 2
            })
        );

        let document =
            parse_document(r#"<section class="target">Span</section>"#).expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target { display: grid; grid-column-start: 2; grid-column-end: 5; }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert_eq!(style.grid_column_start, Some(2));
        assert_eq!(style.grid_column_end, Some(5));
        assert_eq!(style.grid_column_span, 3);
    }

    #[test]
    fn parses_grid_row_shorthand_and_longhands() {
        let document = parse_document(
            r#"
            <section class="shorthand">Row shorthand</section>
            <section class="longhand">Row longhand</section>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .shorthand { display: grid; grid-row: 2 / 4; }
            .longhand { display: grid; grid-row-start: 3; grid-row-end: 6; }
            "#
        .to_string()]);
        let shorthand = document
            .query_selector(".shorthand")
            .expect("shorthand target exists");
        let longhand = document
            .query_selector(".longhand")
            .expect("longhand target exists");
        let shorthand_style = ComputedStyle::for_node(&document, &stylesheet, shorthand);
        let longhand_style = ComputedStyle::for_node(&document, &stylesheet, longhand);

        assert_eq!(shorthand_style.grid_row_start, Some(2));
        assert_eq!(shorthand_style.grid_row_end, Some(4));
        assert_eq!(shorthand_style.grid_row_span, 2);
        assert_eq!(longhand_style.grid_row_start, Some(3));
        assert_eq!(longhand_style.grid_row_end, Some(6));
        assert_eq!(longhand_style.grid_row_span, 3);
    }

    #[test]
    fn preserves_custom_property_case_until_var_resolution() {
        let document =
            parse_document(r#"<section class="target">Case-sensitive variable</section>"#)
                .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            :root {
                --Brand: rgb(37 99 235);
                --brand: rgb(220 38 38);
            }
            .target { color: var(--Brand); }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.color.r - 37.0 / 255.0).abs() < 0.01);
        assert!((style.color.g - 99.0 / 255.0).abs() < 0.01);
        assert!((style.color.b - 235.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn resolves_var_function_case_insensitively() {
        let document =
            parse_document(r#"<section class="target">Uppercase variable function</section>"#)
                .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            :root { --brand: rgb(37 99 235); }
            .target { color: VAR(--brand); }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.color.r - 37.0 / 255.0).abs() < 0.01);
        assert!((style.color.g - 99.0 / 255.0).abs() < 0.01);
        assert!((style.color.b - 235.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn inherited_unitless_line_height_scales_with_child_font_size() {
        let document = parse_document(
            r#"
            <section class="parent">
                <p class="child">Scaled line height</p>
            </section>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .parent { font-size: 10px; line-height: 1.5; }
            .child { font-size: 20px; }
            "#
        .to_string()]);
        let child = document.query_selector(".child").expect("child exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, child);

        assert!((style.font_size - 15.0).abs() < 0.01);
        assert!((style.line_height - 22.5).abs() < 0.01);
    }

    #[test]
    fn self_referential_custom_properties_do_not_loop_forever() {
        let mut variables = BTreeMap::new();
        variables.insert(
            "--p-primary-color".to_string(),
            "var(--p-primary-color)".to_string(),
        );
        variables.insert("--a".to_string(), "var(--b)".to_string());
        variables.insert("--b".to_string(), "var(--a)".to_string());

        assert_eq!(resolve_css_value("var(--p-primary-color)", &variables), "");
        assert_eq!(resolve_css_value("var(--a)", &variables), "");
    }

    #[test]
    fn matches_has_selector_when_descendant_exists() {
        let document = parse_document(
            r#"
            <section class="card"><span class="marker">yes</span></section>
            <section class="card plain">no</section>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .card { color: rgb(220 38 38); }
            .card:has(.marker) { color: rgb(37 99 235); }
            "#
        .to_string()]);
        let marked = document
            .query_selector(".card")
            .expect("marked card exists");
        let plain = document
            .query_selector(".plain")
            .expect("plain card exists");

        let marked_style = ComputedStyle::for_node(&document, &stylesheet, marked);
        let plain_style = ComputedStyle::for_node(&document, &stylesheet, plain);

        assert!((marked_style.color.b - 235.0 / 255.0).abs() < 0.01);
        assert!((plain_style.color.r - 220.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn matches_relative_has_child_and_sibling_selectors() {
        let document = parse_document(
            r#"
            <section class="card">
                <span class="direct">Direct</span>
                <p class="anchor">Anchor</p>
                <p class="adjacent">Adjacent</p>
                <p class="later">Later</p>
            </section>
            <section class="plain">
                <article><span class="direct">Nested</span></article>
                <p class="plain-anchor">Anchor</p>
                <em>Interruption</em>
                <p class="adjacent">Not adjacent</p>
            </section>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .card:has(> .direct) { color: rgb(37 99 235); }
            .anchor:has(+ .adjacent) { background-color: rgb(220 252 231); }
            .anchor:has(~ .later) { border-color: rgb(14 165 233); }
            .plain:has(> .direct) { color: rgb(220 38 38); }
            .plain-anchor:has(+ .adjacent) { background-color: rgb(254 226 226); }
            "#
        .to_string()]);
        let card = document.query_selector(".card").expect("card exists");
        let plain = document.query_selector(".plain").expect("plain exists");
        let anchor = document.query_selector(".anchor").expect("anchor exists");
        let plain_anchor = document
            .query_selector(".plain-anchor")
            .expect("plain anchor exists");
        let card_style = ComputedStyle::for_node(&document, &stylesheet, card);
        let plain_style = ComputedStyle::for_node(&document, &stylesheet, plain);
        let anchor_style = ComputedStyle::for_node(&document, &stylesheet, anchor);
        let plain_anchor_style = ComputedStyle::for_node(&document, &stylesheet, plain_anchor);

        assert!((card_style.color.r - 37.0 / 255.0).abs() < 0.01);
        assert!((plain_style.color.r - Color::BLACK.r).abs() < 0.01);
        assert!(anchor_style.background.is_some());
        assert!((anchor_style.border_color.unwrap().b - 233.0 / 255.0).abs() < 0.01);
        assert!(plain_anchor_style.background.is_none());
    }

    #[test]
    fn matches_static_form_state_pseudo_classes_without_interactive_false_positives() {
        let document = parse_document(
            r#"
            <form class="states">
                <input class="disabled-control" disabled>
                <input class="enabled-control">
                <input class="checked-control" type="CHECKBOX" checked>
                <input class="default-checkbox-control" type="checkbox" checked>
                <select><option class="default-option-control" selected>Default option</option></select>
                <input class="non-default-control">
                <input class="required-control" required>
                <input class="optional-control">
                <input class="placeholder-control" placeholder="Search">
                <input class="readonly-control" readonly>
                <input class="dynamic-control">
                <a class="link-control" href="/invoice">Link</a>
                <span class="empty-control"></span>
                <span class="not-empty-control">text</span>
            </form>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .states input,
            .states a,
            .states span,
            .states option { color: rgb(220 38 38); }
            .disabled-control:disabled { color: rgb(37 99 235); }
            .enabled-control:enabled { color: rgb(37 99 235); }
            .checked-control:checked { color: rgb(37 99 235); }
            .default-checkbox-control:default { color: rgb(37 99 235); }
            option.default-option-control:default { color: rgb(37 99 235); }
            .non-default-control:default { color: rgb(34 197 94); }
            .required-control:required { color: rgb(37 99 235); }
            .optional-control:optional { color: rgb(37 99 235); }
            .placeholder-control:placeholder-shown { color: rgb(37 99 235); }
            .readonly-control:read-only { color: rgb(37 99 235); }
            .dynamic-control:hover,
            .dynamic-control:focus,
            .dynamic-control:focus-visible,
            .dynamic-control:active { color: rgb(34 197 94); }
            .link-control:any-link { color: rgb(37 99 235); }
            .empty-control:empty { color: rgb(37 99 235); }
            .not-empty-control:empty { color: rgb(34 197 94); }
            "#
        .to_string()]);

        for selector in [
            ".disabled-control",
            ".enabled-control",
            ".checked-control",
            ".default-checkbox-control",
            ".default-option-control",
            ".required-control",
            ".optional-control",
            ".placeholder-control",
            ".readonly-control",
            ".link-control",
            ".empty-control",
        ] {
            let id = document.query_selector(selector).expect("control exists");
            let style = ComputedStyle::for_node(&document, &stylesheet, id);
            assert!(
                (style.color.b - 235.0 / 255.0).abs() < 0.01,
                "{selector} should match its static pseudo-class"
            );
        }

        for selector in [
            ".dynamic-control",
            ".not-empty-control",
            ".non-default-control",
        ] {
            let id = document.query_selector(selector).expect("control exists");
            let style = ComputedStyle::for_node(&document, &stylesheet, id);
            assert!(
                (style.color.r - 220.0 / 255.0).abs() < 0.01,
                "{selector} should not be matched by inactive dynamic/false pseudo-classes"
            );
        }
    }

    #[test]
    fn skips_unsupported_pseudo_element_rules_instead_of_applying_them_to_elements() {
        let document = parse_document(r#"<input class="range" type="range">"#).expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .range { color: rgb(37 99 235); }
            .range::-webkit-slider-thumb { color: rgb(220 38 38); }
            .range::marker { background-color: rgb(220 38 38); }
            "#
        .to_string()]);
        let range = document.query_selector(".range").expect("range exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, range);

        assert!((style.color.b - 235.0 / 255.0).abs() < 0.01);
        assert!(style.background.is_none());
    }

    #[test]
    fn matches_an_plus_b_nth_child_selectors() {
        let document = parse_document(
            r#"
            <ul class="list">
                <li class="one">One</li>
                <li class="two">Two</li>
                <li class="three">Three</li>
                <li class="four">Four</li>
            </ul>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .list > li { color: rgb(220 38 38); }
            .list > li:nth-child(2n + 1) { color: rgb(37 99 235); }
            .list > li:nth-child(-n + 2) { border-left: 2px solid rgb(37 99 235); }
            .list > li:nth-last-child(2) { border-right: 2px solid rgb(37 99 235); }
            "#
        .to_string()]);
        let one = document.query_selector(".one").expect("one exists");
        let two = document.query_selector(".two").expect("two exists");
        let three = document.query_selector(".three").expect("three exists");

        let one_style = ComputedStyle::for_node(&document, &stylesheet, one);
        let two_style = ComputedStyle::for_node(&document, &stylesheet, two);
        let three_style = ComputedStyle::for_node(&document, &stylesheet, three);

        assert!((one_style.color.b - 235.0 / 255.0).abs() < 0.01);
        assert!((three_style.color.b - 235.0 / 255.0).abs() < 0.01);
        assert!((two_style.color.r - 220.0 / 255.0).abs() < 0.01);
        assert!((one_style.border_left_width - 1.5).abs() < 0.01);
        assert!((two_style.border_left_width - 1.5).abs() < 0.01);
        assert!((three_style.border_right_width - 1.5).abs() < 0.01);
    }

    #[test]
    fn matches_modern_nth_child_of_selector_lists() {
        let document = parse_document(
            r#"
            <ul class="filtered-list">
                <li class="row active">One</li>
                <li class="row archived">Two</li>
                <li class="row active target">Three</li>
                <li class="row active">Four</li>
                <li class="row featured">Five</li>
            </ul>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .filtered-list > li { color: rgb(220 38 38); }
            .filtered-list > li:nth-child(2 of .active, .featured) {
                color: rgb(37 99 235);
            }
            .filtered-list > li:nth-last-child(1 of .active:not(.archived), .featured) {
                border-right: 2px solid rgb(14 165 233);
            }
            "#
        .to_string()]);
        let archived = document
            .query_selector(".archived")
            .expect("archived exists");
        let target = document.query_selector(".target").expect("target exists");
        let featured = document
            .query_selector(".featured")
            .expect("featured exists");

        let archived_style = ComputedStyle::for_node(&document, &stylesheet, archived);
        let target_style = ComputedStyle::for_node(&document, &stylesheet, target);
        let featured_style = ComputedStyle::for_node(&document, &stylesheet, featured);

        assert!((target_style.color.b - 235.0 / 255.0).abs() < 0.01);
        assert!((archived_style.color.r - 220.0 / 255.0).abs() < 0.01);
        assert!((featured_style.border_right_width - 1.5).abs() < 0.01);
    }

    #[test]
    fn matches_of_type_structural_selectors() {
        let document = parse_document(
            r#"
            <section class="types">
                <h2>Title</h2>
                <p class="alpha">Alpha</p>
                <span>Separator</span>
                <p class="beta">Beta</p>
                <p class="gamma">Gamma</p>
            </section>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .types > p { color: rgb(220 38 38); }
            .types > p:first-of-type { color: rgb(37 99 235); }
            .types > p:nth-of-type(2n) { border-left: 2px solid rgb(37 99 235); }
            .types > p:last-of-type { background-color: rgb(219 234 254); }
            .types > p:nth-last-of-type(1) { border-right: 2px solid rgb(37 99 235); }
            "#
        .to_string()]);
        let alpha = document.query_selector(".alpha").expect("alpha exists");
        let beta = document.query_selector(".beta").expect("beta exists");
        let gamma = document.query_selector(".gamma").expect("gamma exists");

        let alpha_style = ComputedStyle::for_node(&document, &stylesheet, alpha);
        let beta_style = ComputedStyle::for_node(&document, &stylesheet, beta);
        let gamma_style = ComputedStyle::for_node(&document, &stylesheet, gamma);

        assert!((alpha_style.color.b - 235.0 / 255.0).abs() < 0.01);
        assert!((beta_style.border_left_width - 1.5).abs() < 0.01);
        assert!(gamma_style.background.is_some());
        assert!((gamma_style.border_right_width - 1.5).abs() < 0.01);
    }

    #[test]
    fn where_selector_has_zero_specificity() {
        let document =
            parse_document(r#"<section class="target">Specificity</section>"#).expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target { color: rgb(37 99 235); }
            :where(.target) { color: rgb(220 38 38); }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.color.r - 37.0 / 255.0).abs() < 0.01);
        assert!((style.color.g - 99.0 / 255.0).abs() < 0.01);
        assert!((style.color.b - 235.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn is_selector_uses_max_argument_specificity_for_cascade() {
        let document =
            parse_document(r#"<section class="target">Specificity</section>"#).expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            :is(#never, .target) { color: rgb(37 99 235); }
            .target { color: rgb(220 38 38); }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.color.r - 37.0 / 255.0).abs() < 0.01);
        assert!((style.color.g - 99.0 / 255.0).abs() < 0.01);
        assert!((style.color.b - 235.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn scopes_rules_to_scope_root() {
        let document = parse_document(
            r#"
            <section class="scope"><p class="target">scoped</p></section>
            <p class="target outside">outside</p>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target { color: rgb(220 38 38); }
            @scope (.scope) {
                .target { color: rgb(37 99 235); }
            }
            "#
        .to_string()]);
        let scoped = document
            .traverse_preorder()
            .into_iter()
            .find(|id| {
                matches!(
                    document.node(*id),
                    Some(Node::Element(element))
                        if element.classes().any(|class| class == "target")
                            && document
                                .parent_of(*id)
                                .and_then(|parent| document.node(parent))
                                .is_some_and(|node| matches!(
                                    node,
                                    Node::Element(parent)
                                        if parent.classes().any(|class| class == "scope")
                                ))
                )
            })
            .expect("scoped target");
        let outside = document.query_selector(".outside").expect("outside target");

        let scoped_style = ComputedStyle::for_node(&document, &stylesheet, scoped);
        let outside_style = ComputedStyle::for_node(&document, &stylesheet, outside);

        assert!((scoped_style.color.b - 235.0 / 255.0).abs() < 0.01);
        assert!((outside_style.color.r - 220.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn matches_modern_media_range_syntax() {
        let media = MediaContext {
            print: true,
            width_px: 640.0,
            height_px: 960.0,
        };

        assert!(media_query_matches("@media (width >= 40rem)", media));
        assert!(media_query_matches("@media (width <= 50rem)", media));
        assert!(!media_query_matches("@media (width < 30rem)", media));
        assert!(!media_query_matches("@media (30rem >= width)", media));
        assert!(media_query_matches("@media (height >= 50rem)", media));
        assert!(media_query_matches("@media (orientation: portrait)", media));
        assert!(!media_query_matches(
            "@media (orientation: landscape)",
            media
        ));
        assert!(media_query_matches(
            "@media (prefers-color-scheme: light)",
            media
        ));
        assert!(!media_query_matches(
            "@media (prefers-color-scheme: dark)",
            media
        ));
        assert!(media_query_matches(
            "@media (prefers-reduced-motion: no-preference)",
            media
        ));
        assert!(!media_query_matches(
            "@media (prefers-reduced-motion: reduce)",
            media
        ));
        assert!(media_query_matches(
            "@media not (prefers-color-scheme: dark)",
            media
        ));
        assert!(media_query_matches(
            "@media ((prefers-color-scheme: dark) or (width >= 30rem))",
            media
        ));
        assert!(!media_query_matches(
            "@media ((prefers-color-scheme: dark) or (width < 30rem))",
            media
        ));
        assert!(!media_query_matches(
            "@media not ((prefers-color-scheme: dark) or (width >= 30rem))",
            media
        ));
        assert!(media_query_matches(
            "@media (orientation: portrait) and ((prefers-color-scheme: dark) or (width >= 30rem))",
            media
        ));
        assert!(media_query_matches(
            "@media (hover: none) and (pointer: none) and (update: none) and (color-gamut: srgb)",
            media
        ));
        assert!(media_query_matches("@media (any-hover: none)", media));
        assert!(media_query_matches("@media (any-pointer: none)", media));
        assert!(!media_query_matches("@media (hover: hover)", media));
        assert!(!media_query_matches("@media (pointer: fine)", media));
        assert!(!media_query_matches("@media (update: fast)", media));
        assert!(!media_query_matches("@media (color-gamut: p3)", media));
    }

    #[test]
    fn flattens_supported_at_rules_case_insensitively() {
        let document = parse_document(r#"<section class="target">Uppercase at-rules</section>"#)
            .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            @MEDIA print {
                .target { color: rgb(220 38 38); }
            }
            @SUPPORTS (color: rgb(0 0 0)) {
                .target { background-color: rgb(219 234 254); }
            }
            @LAYER components {
                .target { border-left: 2px solid rgb(37 99 235); }
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.color.r - 220.0 / 255.0).abs() < 0.01);
        assert!(style.background.is_some());
        assert!((style.border_left_width - 1.5).abs() < 0.01);
    }

    #[test]
    fn attribute_selector_i_flag_matches_ascii_case_insensitively() {
        let document = parse_document(
            r#"
            <section class="target" data-state="Primary-Active" data-tags="Alpha Beta">
                Attribute flags
            </section>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            [data-state="primary-active" i] { color: rgb(37 99 235); }
            [data-state^="PRIMARY" i] { background-color: rgb(219 234 254); }
            [data-state$="ACTIVE" i] { border-left: 2px solid rgb(37 99 235); }
            [data-state*="ary-act" i] { border-right: 2px solid rgb(37 99 235); }
            [data-tags~="beta" i] { border-top: 2px solid rgb(37 99 235); }
            [data-state|="primary" i] { border-bottom: 2px solid rgb(37 99 235); }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.color.r - 37.0 / 255.0).abs() < 0.01);
        assert!(style.background.is_some());
        assert!((style.border_left_width - 1.5).abs() < 0.01);
        assert!((style.border_right_width - 1.5).abs() < 0.01);
        assert!((style.border_top_width - 1.5).abs() < 0.01);
        assert!((style.border_bottom_width - 1.5).abs() < 0.01);
    }

    #[test]
    fn attribute_selector_s_flag_preserves_case_sensitive_matching() {
        let document = parse_document(
            r#"<section class="target" data-state="Primary">Attribute flags</section>"#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            [data-state="primary" s] { color: rgb(220 38 38); }
            [data-state="Primary" s] { color: rgb(37 99 235); }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.color.r - 37.0 / 255.0).abs() < 0.01);
        assert!((style.color.g - 99.0 / 255.0).abs() < 0.01);
        assert!((style.color.b - 235.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn unlayered_rules_beat_later_layered_rules() {
        let document = parse_document(r#"<section class="target">Cascade layers</section>"#)
            .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target { color: rgb(37 99 235); }
            @layer components {
                .target { color: rgb(220 38 38); }
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.color.r - 37.0 / 255.0).abs() < 0.01);
        assert!((style.color.g - 99.0 / 255.0).abs() < 0.01);
        assert!((style.color.b - 235.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn named_layer_order_beats_source_order() {
        let document = parse_document(r#"<section class="target">Cascade layers</section>"#)
            .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            @layer reset, components;
            @layer components {
                .target { color: rgb(37 99 235); }
            }
            @layer reset {
                .target { color: rgb(220 38 38); }
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.color.r - 37.0 / 255.0).abs() < 0.01);
        assert!((style.color.g - 99.0 / 255.0).abs() < 0.01);
        assert!((style.color.b - 235.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn important_layer_order_reverses_normal_layer_order() {
        let document = parse_document(r#"<section class="target">Cascade layers</section>"#)
            .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            @layer reset, components;
            .target { color: rgb(15 23 42) !important; }
            @layer components {
                .target { color: rgb(220 38 38) !important; }
            }
            @layer reset {
                .target { color: rgb(37 99 235) !important; }
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.color.r - 37.0 / 255.0).abs() < 0.01);
        assert!((style.color.g - 99.0 / 255.0).abs() < 0.01);
        assert!((style.color.b - 235.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn parses_light_dark_as_light_color_for_print() {
        let color =
            Color::from_css("light-dark(rgb(37 99 235), rgb(15 23 42))").expect("light-dark color");

        assert!((color.r - 37.0 / 255.0).abs() < 0.01);
        assert!((color.g - 99.0 / 255.0).abs() < 0.01);
        assert!((color.b - 235.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn resolves_viewport_units_against_page_size() {
        let document = parse_document(r#"<section class="target">Viewport units</section>"#)
            .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            @page { size: A4; }
            .target {
                width: 50vw;
                max-width: 25dvi;
                min-height: 25dvh;
                max-height: 10lvb;
                padding: 2vmin 1vmax;
                margin-top: calc(1vh + 2px);
                transform: translateX(10svw);
                grid-template-columns: minmax(20vw, 1fr) 10vmin;
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.width.as_ref().unwrap().resolve(0.0) - 297.5).abs() < 0.01);
        assert!((style.max_width.as_ref().unwrap().resolve(0.0) - 148.75).abs() < 0.01);
        assert!((style.min_height.as_ref().unwrap().resolve(0.0) - 210.5).abs() < 0.01);
        assert!((style.max_height.as_ref().unwrap().resolve(0.0) - 84.2).abs() < 0.01);
        assert!((style.padding_top - 11.9).abs() < 0.01);
        assert!((style.padding_right - 8.42).abs() < 0.01);
        assert!((style.margin_top - 9.92).abs() < 0.01);
        assert!((style.transform_translate_x.as_ref().unwrap().resolve(0.0) - 59.5).abs() < 0.01);
        assert!(matches!(
            style.grid_tracks.first(),
            Some(GridTrack::Fr { min: Some(length), .. }) if (length.resolve(0.0) - 119.0).abs() < 0.01
        ));
        assert!(matches!(
            style.grid_tracks.get(1),
            Some(GridTrack::Length(length)) if (length.resolve(0.0) - 59.5).abs() < 0.01
        ));
    }

    #[test]
    fn resolves_env_length_fallbacks_for_static_print() {
        let direct = parse_css_length("env(safe-area-inset-left, 2px)")
            .expect("env fallback should parse as a length");
        let missing = parse_css_length("env(safe-area-inset-right)")
            .expect("env without fallback should conservatively resolve to zero");
        let nested = parse_css_length("env(safe-area-inset-bottom, calc(1rem + 2px))")
            .expect("env fallback should accept nested length functions");
        let calc = parse_css_length("calc(env(safe-area-inset-left, 2px) + 1rem)")
            .expect("env fallback should parse inside calc length expressions");

        assert!((direct.resolve(0.0) - 1.5).abs() < 0.01);
        assert!(missing.resolve(0.0).abs() < 0.01);
        assert!((nested.resolve(0.0) - 13.5).abs() < 0.01);
        assert!((calc.resolve(0.0) - 13.5).abs() < 0.01);
    }

    #[test]
    fn applies_env_length_fallbacks_through_computed_style() {
        let document = parse_document(r#"<section class="target">Env fallback</section>"#)
            .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target {
                padding-inline: calc(env(safe-area-inset-left, 2px) + 1rem);
                margin-block-start: env(safe-area-inset-top);
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.padding_left - 13.5).abs() < 0.01);
        assert!((style.padding_right - 13.5).abs() < 0.01);
        assert!(style.margin_top.abs() < 0.01);
    }

    #[test]
    fn resolves_em_units_against_current_font_size() {
        let document =
            parse_document(r#"<section class="target">Em units</section>"#).expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target {
                font-size: 20px;
                padding: 2em;
                width: calc(10em + 2px);
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.font_size - 15.0).abs() < 0.01);
        assert!((style.padding_top - 30.0).abs() < 0.01);
        assert!((style.width.as_ref().unwrap().resolve(0.0) - 151.5).abs() < 0.01);
    }

    #[test]
    fn resolves_lh_and_rlh_units_against_line_height_contexts() {
        let document = parse_document(r#"<section class="target">Line-height units</section>"#)
            .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target {
                font-size: 20px;
                line-height: 1.5;
                padding-top: 2lh;
                margin-top: 1rlh;
                width: calc(3lh + 1rlh);
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.font_size - 15.0).abs() < 0.01);
        assert!((style.line_height - 22.5).abs() < 0.01);
        assert!((style.padding_top - 45.0).abs() < 0.01);
        assert!((style.margin_top - 14.4).abs() < 0.01);
        assert!((style.width.as_ref().unwrap().resolve(0.0) - 81.9).abs() < 0.01);
    }

    #[test]
    fn resolves_modern_font_relative_units_against_current_font_size() {
        let document = parse_document(r#"<section class="target">Font-relative units</section>"#)
            .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target {
                font-size: 20px;
                padding-left: 2ex;
                padding-right: 1cap;
                min-width: 3ic;
                width: calc(2cap + 1ex);
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.font_size - 15.0).abs() < 0.01);
        assert!((style.padding_left - 15.0).abs() < 0.01);
        assert!((style.padding_right - 10.5).abs() < 0.01);
        assert!((style.min_width.as_ref().unwrap().resolve(0.0) - 45.0).abs() < 0.01);
        assert!((style.width.as_ref().unwrap().resolve(0.0) - 28.5).abs() < 0.01);
    }

    #[test]
    fn resolves_container_query_units_against_static_print_container_fallback() {
        let document = parse_document(r#"<section class="target">Container units</section>"#)
            .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target {
                inline-size: min(50cqw, 40rem);
                min-block-size: max(10cqh, 2rem);
                padding-inline: calc(2cqi + 1rem);
                margin-block-start: calc(1cqmin + 1cqmax);
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!(
            (style.width.as_ref().unwrap().resolve(10_000.0) - 306.0).abs() < 0.01,
            "50cqw of Letter width should resolve to 306pt and win over 40rem"
        );
        assert!((style.min_height.as_ref().unwrap().resolve(0.0) - 79.2).abs() < 0.01);
        assert!((style.padding_left - 24.24).abs() < 0.01);
        assert!((style.padding_right - 24.24).abs() < 0.01);
        assert!((style.margin_top - 14.04).abs() < 0.01);
    }

    #[test]
    fn resolves_rem_units_against_root_font_size() {
        let document =
            parse_document(r#"<section class="target">Rem units</section>"#).expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            html { font-size: 14px; }
            :root { --rhythm: 3.5rem; }
            .target {
                margin-bottom: var(--rhythm);
                padding: 2rem;
                width: calc(10rem + 2px);
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.margin_bottom - 36.75).abs() < 0.01);
        assert!((style.padding_top - 21.0).abs() < 0.01);
        assert!((style.width.as_ref().unwrap().resolve(0.0) - 106.5).abs() < 0.01);
    }

    #[test]
    fn resolves_ch_units_as_font_relative_lengths() {
        let document =
            parse_document(r#"<section class="target">Ch units</section>"#).expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target {
                font-size: 20px;
                max-width: 60ch;
                margin-left: calc(2ch + 1px);
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.font_size - 15.0).abs() < 0.01);
        assert!((style.max_width.as_ref().unwrap().resolve(0.0) - 504.0).abs() < 0.01);
        assert!((style.margin_left - 17.55).abs() < 0.01);
    }

    #[test]
    fn preserves_auto_inline_margins_for_flow_layout() {
        let document = parse_document(r#"<section class="target">Auto margins</section>"#)
            .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target {
                margin: 10px auto 20px;
                width: 300px;
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.margin_top - 7.5).abs() < 0.01);
        assert!((style.margin_bottom - 15.0).abs() < 0.01);
        assert!(style.margin_left_auto);
        assert!(style.margin_right_auto);
        assert_eq!(style.margin_left, 0.0);
        assert_eq!(style.margin_right, 0.0);
    }

    #[test]
    fn parses_logical_text_alignment_and_direction() {
        let document = parse_document(r#"<section class="target">Logical alignment</section>"#)
            .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target {
                direction: rtl;
                text-align: end;
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert_eq!(style.direction, TextDirection::Rtl);
        assert_eq!(style.text_align, TextAlign::End);
    }

    #[test]
    fn parses_text_align_justify_as_justification_mode() {
        let document = parse_document(r#"<section class="target">Justified text</section>"#)
            .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target { text-align: justify; }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert_eq!(style.text_align, TextAlign::Justify);
    }

    #[test]
    fn parses_text_align_last_and_text_align_all() {
        let document = parse_document(
            r#"<section class="parent"><p class="target">Last line alignment</p></section>"#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .parent {
                text-align-all: end;
                text-align-last: center;
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert_eq!(style.text_align, TextAlign::End);
        assert_eq!(style.text_align_last, Some(TextAlign::Center));
    }

    #[test]
    fn parses_table_cell_vertical_align_keywords() {
        let document = parse_document(r#"<table><tr><td class="target">Aligned</td></tr></table>"#)
            .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            td.target { vertical-align: middle; }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert_eq!(style.vertical_align, VerticalAlign::Middle);
    }

    #[test]
    fn parses_table_border_spacing_horizontal_and_vertical() {
        let document = parse_document(r#"<table class="target"><tr><td>Cell</td></tr></table>"#)
            .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target { border-collapse: separate; border-spacing: 10px 20px; }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert_eq!(style.border_collapse, BorderCollapse::Separate);
        assert!((style.border_spacing_horizontal - 7.5).abs() < 0.01);
        assert!((style.border_spacing_vertical - 15.0).abs() < 0.01);
    }

    #[test]
    fn parses_modern_flex_shorthand_and_basis() {
        let document = parse_document(
            r#"
            <section class="grow">Grow</section>
            <section class="fixed">Fixed</section>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .grow { flex: 2 1 0; }
            .fixed { flex: 0 0 8rem; }
            "#
        .to_string()]);
        let grow = document.query_selector(".grow").expect("grow exists");
        let fixed = document.query_selector(".fixed").expect("fixed exists");
        let grow_style = ComputedStyle::for_node(&document, &stylesheet, grow);
        let fixed_style = ComputedStyle::for_node(&document, &stylesheet, fixed);

        assert!((grow_style.flex_grow - 2.0).abs() < f32::EPSILON);
        assert!((grow_style.flex_shrink - 1.0).abs() < f32::EPSILON);
        assert!((grow_style.flex_basis.unwrap().resolve(200.0) - 0.0).abs() < 0.01);
        assert!((fixed_style.flex_grow - 0.0).abs() < f32::EPSILON);
        assert!((fixed_style.flex_shrink - 0.0).abs() < f32::EPSILON);
        assert!((fixed_style.flex_basis.unwrap().resolve(0.0) - 96.0).abs() < 0.01);
    }

    #[test]
    fn parses_flex_wrap_and_flow_shorthand() {
        let document = parse_document(
            r#"
            <section class="wrapped">Wrapped</section>
            <section class="flow">Flow</section>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .wrapped { flex-wrap: wrap; }
            .flow { flex-flow: column wrap-reverse; }
            "#
        .to_string()]);
        let wrapped = document.query_selector(".wrapped").expect("wrapped exists");
        let flow = document.query_selector(".flow").expect("flow exists");
        let wrapped_style = ComputedStyle::for_node(&document, &stylesheet, wrapped);
        let flow_style = ComputedStyle::for_node(&document, &stylesheet, flow);

        assert_eq!(wrapped_style.flex_wrap, FlexWrap::Wrap);
        assert_eq!(flow_style.flex_direction, FlexDirection::Column);
        assert_eq!(flow_style.flex_wrap, FlexWrap::Wrap);
    }

    #[test]
    fn parses_flex_order_align_self_and_max_block_size() {
        let document =
            parse_document(r#"<section class="target">Target</section>"#).expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target {
                order: -2;
                align-self: flex-end;
                max-height: 48px;
                max-block-size: 3rem;
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert_eq!(style.order, -2);
        assert_eq!(style.align_self, Some(AlignItems::End));
        assert!((style.max_height.as_ref().unwrap().resolve(0.0) - 36.0).abs() < 0.01);
    }

    #[test]
    fn parses_grid_box_alignment_properties() {
        let document = parse_document(
            r#"
            <section class="grid"><div class="child">Target</div></section>
            <section class="override">Override</section>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .grid { place-items: center start; }
            .child { justify-self: end; }
            .override { place-self: start center; }
            "#
        .to_string()]);
        let grid = document.query_selector(".grid").expect("grid exists");
        let child = document.query_selector(".child").expect("child exists");
        let override_node = document
            .query_selector(".override")
            .expect("override exists");
        let grid_style = ComputedStyle::for_node(&document, &stylesheet, grid);
        let child_style = ComputedStyle::for_node(&document, &stylesheet, child);
        let override_style = ComputedStyle::for_node(&document, &stylesheet, override_node);

        assert_eq!(grid_style.align_items, AlignItems::Center);
        assert_eq!(grid_style.justify_items, AlignItems::Start);
        assert_eq!(child_style.justify_self, Some(AlignItems::End));
        assert_eq!(override_style.align_self, Some(AlignItems::Start));
        assert_eq!(override_style.justify_self, Some(AlignItems::Center));
    }

    #[test]
    fn parses_text_overflow_ellipsis() {
        let document =
            parse_document(r#"<section class="target">Target</section>"#).expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target {
                overflow: hidden;
                white-space: nowrap;
                text-overflow: ellipsis;
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!(style.overflow_hidden);
        assert_eq!(style.white_space, WhiteSpace::NoWrap);
        assert_eq!(style.text_overflow, TextOverflow::Ellipsis);
    }

    #[test]
    fn parses_modern_text_wrap_properties() {
        let document = parse_document(
            r#"
            <section class="nowrap">No wrap</section>
            <section class="pre-line">Pre line</section>
            <section class="pretty">Pretty wrap</section>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .nowrap {
                text-wrap: nowrap;
                white-space-collapse: preserve;
            }
            .pre-line {
                white-space: pre-line;
            }
            .pretty {
                white-space: nowrap;
                text-wrap-mode: wrap;
                text-wrap-style: pretty;
                word-spacing: 0.25em;
            }
            "#
        .to_string()]);
        let nowrap = document.query_selector(".nowrap").expect("nowrap exists");
        let pre_line = document
            .query_selector(".pre-line")
            .expect("pre-line exists");
        let pretty = document.query_selector(".pretty").expect("pretty exists");

        assert_eq!(
            ComputedStyle::for_node(&document, &stylesheet, nowrap).white_space,
            WhiteSpace::NoWrap
        );
        assert_eq!(
            ComputedStyle::for_node(&document, &stylesheet, pre_line).white_space,
            WhiteSpace::PreLine
        );
        assert_eq!(
            ComputedStyle::for_node(&document, &stylesheet, pretty).white_space,
            WhiteSpace::Normal
        );
        assert_eq!(
            ComputedStyle::for_node(&document, &stylesheet, pretty).text_wrap_style,
            TextWrapStyle::Pretty
        );
        assert!(
            (ComputedStyle::for_node(&document, &stylesheet, pretty).word_spacing - 3.0).abs()
                < 0.01
        );
    }

    #[test]
    fn parses_text_wrap_balance_style() {
        let document = parse_document(r#"<section class="target">Balanced wrapping</section>"#)
            .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target {
                text-wrap: pretty;
                text-wrap-style: balance;
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert_eq!(style.white_space, WhiteSpace::Normal);
        assert_eq!(style.text_wrap_style, TextWrapStyle::Balance);
    }

    #[test]
    fn parses_tabular_numeric_font_features() {
        let document = parse_document(
            r#"
            <section class="variant">123</section>
            <section class="feature">456</section>
            <section class="reset">789</section>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .variant { font-variant-numeric: lining-nums tabular-nums; }
            .feature { font-feature-settings: "kern" 1, "tnum" 1; }
            .reset { font-variant-numeric: proportional-nums; }
            "#
        .to_string()]);
        let variant = document.query_selector(".variant").expect("variant exists");
        let feature = document.query_selector(".feature").expect("feature exists");
        let reset = document.query_selector(".reset").expect("reset exists");

        assert_eq!(
            ComputedStyle::for_node(&document, &stylesheet, variant).font_variant_numeric,
            FontVariantNumeric::TabularNums
        );
        assert_eq!(
            ComputedStyle::for_node(&document, &stylesheet, feature).font_variant_numeric,
            FontVariantNumeric::TabularNums
        );
        assert_eq!(
            ComputedStyle::for_node(&document, &stylesheet, reset).font_variant_numeric,
            FontVariantNumeric::Normal
        );
    }

    #[test]
    fn parses_modern_font_shorthand() {
        let document =
            parse_document(r#"<p class="target">Font shorthand</p>"#).expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target {
                font: italic 650 1.25rem/1.6 Georgia, serif;
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert_eq!(style.font_weight, FontWeight::Bold);
        assert_eq!(style.font_style, FontStyle::Italic);
        assert_eq!(style.font_face, FontFace::Serif);
        assert!((style.font_size - 15.0).abs() < 0.01);
        assert!((style.line_height - 24.0).abs() < 0.01);
        assert_eq!(style.line_height_multiplier, Some(1.6));
    }

    #[test]
    fn parses_and_inherits_font_style() {
        let document = parse_document(
            r#"
            <section class="parent">
                <span class="child">Inherited</span>
            </section>
            <span class="reset">Reset</span>
            <em class="ua">Emphasis</em>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .parent { font-style: oblique 12deg; }
            .reset { font-style: normal; }
            "#
        .to_string()]);
        let parent = document.query_selector(".parent").expect("parent exists");
        let child = document.query_selector(".child").expect("child exists");
        let reset = document.query_selector(".reset").expect("reset exists");
        let ua = document.query_selector(".ua").expect("ua exists");
        let parent_style = ComputedStyle::for_node(&document, &stylesheet, parent);
        let child_style =
            ComputedStyle::for_node_with_parent(&document, &stylesheet, child, Some(&parent_style));
        let reset_style = ComputedStyle::for_node(&document, &stylesheet, reset);
        let ua_style = ComputedStyle::for_node(&document, &stylesheet, ua);

        assert_eq!(parent_style.font_style, FontStyle::Oblique);
        assert_eq!(child_style.font_style, FontStyle::Oblique);
        assert_eq!(reset_style.font_style, FontStyle::Normal);
        assert_eq!(ua_style.font_style, FontStyle::Italic);
    }

    #[test]
    fn parses_text_decoration_and_semantic_defaults() {
        let document = parse_document(
            r#"
            <section class="both">Both</section>
            <section class="reset">Reset</section>
            <section class="modern">Modern decoration</section>
            <u class="ua-under">Under</u>
            <del class="ua-del">Deleted</del>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .both { text-decoration: underline line-through; }
            .reset { text-decoration-line: none; }
            .modern {
                font-size: 20px;
                text-decoration: underline rgb(37 99 235) 12%;
                text-underline-offset: 18%;
            }
            "#
        .to_string()]);
        let both = document.query_selector(".both").expect("both exists");
        let reset = document.query_selector(".reset").expect("reset exists");
        let modern = document.query_selector(".modern").expect("modern exists");
        let ua_under = document.query_selector(".ua-under").expect("u exists");
        let ua_del = document.query_selector(".ua-del").expect("del exists");
        let both_style = ComputedStyle::for_node(&document, &stylesheet, both);
        let reset_style = ComputedStyle::for_node(&document, &stylesheet, reset);
        let modern_style = ComputedStyle::for_node(&document, &stylesheet, modern);
        let ua_under_style = ComputedStyle::for_node(&document, &stylesheet, ua_under);
        let ua_del_style = ComputedStyle::for_node(&document, &stylesheet, ua_del);

        assert!(both_style.text_decoration.underline);
        assert!(both_style.text_decoration.line_through);
        assert_eq!(reset_style.text_decoration, TextDecoration::none());
        assert!(modern_style.text_decoration.underline);
        assert_eq!(
            modern_style.text_decoration_color,
            Some(Color {
                r: 37.0 / 255.0,
                g: 99.0 / 255.0,
                b: 235.0 / 255.0,
                a: 1.0,
            })
        );
        assert!((modern_style.text_decoration_thickness.unwrap() - 1.8).abs() < 0.01);
        assert!((modern_style.text_underline_offset.unwrap() - 2.7).abs() < 0.01);
        assert!(ua_under_style.text_decoration.underline);
        assert!(ua_del_style.text_decoration.line_through);
    }

    #[test]
    fn preserves_heavy_numeric_font_weight_bucket() {
        let document = parse_document(r#"<p class="target">Heavy weight</p>"#).expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target { font-weight: 850; }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert_eq!(style.font_weight, FontWeight::Heavy);
    }

    #[test]
    fn parses_and_inherits_list_style_type() {
        let document = parse_document(
            r#"
            <ul class="menu"><li class="none">Hidden marker</li></ul>
            <ol class="steps"><li class="decimal">Numbered marker</li></ol>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .menu { list-style: none; }
            .steps { list-style-type: decimal; }
            "#
        .to_string()]);
        let menu = document.query_selector(".menu").expect("menu exists");
        let none = document.query_selector(".none").expect("none exists");
        let steps = document.query_selector(".steps").expect("steps exists");
        let decimal = document.query_selector(".decimal").expect("decimal exists");
        let menu_style = ComputedStyle::for_node(&document, &stylesheet, menu);
        let none_style =
            ComputedStyle::for_node_with_parent(&document, &stylesheet, none, Some(&menu_style));
        let steps_style = ComputedStyle::for_node(&document, &stylesheet, steps);
        let decimal_style = ComputedStyle::for_node_with_parent(
            &document,
            &stylesheet,
            decimal,
            Some(&steps_style),
        );

        assert_eq!(menu_style.list_style_type, ListStyleType::None);
        assert_eq!(none_style.list_style_type, ListStyleType::None);
        assert_eq!(steps_style.list_style_type, ListStyleType::Decimal);
        assert_eq!(decimal_style.list_style_type, ListStyleType::Decimal);
    }

    #[test]
    fn all_standard_named_colors_match_the_reference_table() {
        let reference = include_str!("../tests/data/css-named-colors.txt");
        let mut names = BTreeSet::new();
        for line in reference
            .lines()
            .filter(|line| !line.starts_with('#') && !line.is_empty())
        {
            let (name, hex) = line.split_once(' ').unwrap();
            assert!(names.insert(name), "duplicate {name}");
            let expected = Color::from_css(hex).unwrap();
            assert_eq!(Color::from_css(name), Some(expected), "{name}");
            assert_eq!(
                Color::from_css(&name.to_ascii_uppercase()),
                Some(expected),
                "uppercase {name}"
            );
        }
        assert_eq!(names.len(), 148);
    }

    #[test]
    fn basic_named_colors_match_their_srgb_hex_values() {
        for (name, hex) in [
            ("red", "#ff0000"),
            ("blue", "#0000ff"),
            ("green", "#008000"),
            ("gray", "#808080"),
            ("grey", "#808080"),
        ] {
            assert_eq!(Color::from_css(name), Color::from_css(hex), "{name}");
        }
    }

    #[test]
    fn invalid_border_color_longhands_preserve_each_side() {
        let document = parse_document("<div class='target'>box</div>").unwrap();
        for properties in [
            [
                "border-top-color",
                "border-right-color",
                "border-bottom-color",
                "border-left-color",
            ],
            [
                "border-block-start-color",
                "border-inline-end-color",
                "border-block-end-color",
                "border-inline-start-color",
            ],
        ] {
            let invalid = properties
                .map(|property| format!("{property}:invalid;"))
                .join("");
            let stylesheet = Stylesheet::from_css_chunks(vec![format!(
                ".target {{border:2pt solid black;border-color:red blue green white;{invalid}}}"
            )]);
            let style = ComputedStyle::for_node(
                &document,
                &stylesheet,
                document.query_selector(".target").unwrap(),
            );
            assert_eq!(
                [
                    style.border_top_color,
                    style.border_right_color,
                    style.border_bottom_color,
                    style.border_left_color
                ],
                ["red", "blue", "green", "white"].map(Color::from_css)
            );
        }
    }

    #[test]
    fn expands_border_color_components_without_losing_previous_valid_colors() {
        for (value, expected) in [
            ("red", ["red"; 4]),
            ("red blue", ["red", "blue", "red", "blue"]),
            ("red blue green", ["red", "blue", "green", "blue"]),
            (
                "red blue green transparent",
                ["red", "blue", "green", "transparent"],
            ),
            (
                "rgb(255, 0, 0) currentColor",
                ["red", "purple", "red", "purple"],
            ),
            ("red invalid", ["black"; 4]),
            ("red blue green white black", ["black"; 4]),
        ] {
            let document = parse_document("<div class='target'>box</div>").unwrap();
            let stylesheet = Stylesheet::from_css_chunks(vec![format!(
                ".target {{color:purple;border:2pt solid black;border-color:{value}}}"
            )]);
            let style = ComputedStyle::for_node(
                &document,
                &stylesheet,
                document.query_selector(".target").unwrap(),
            );
            assert_eq!(
                [
                    style.border_top_color,
                    style.border_right_color,
                    style.border_bottom_color,
                    style.border_left_color
                ],
                expected.map(Color::from_css),
                "{value}"
            );
        }
    }

    #[test]
    fn border_width_longhands_share_keyword_and_invalid_value_handling() {
        for (property, affected) in [
            ("border-top-width", vec![0]),
            ("border-right-width", vec![1]),
            ("border-bottom-width", vec![2]),
            ("border-left-width", vec![3]),
            ("border-block-start-width", vec![0]),
            ("border-inline-end-width", vec![1]),
            ("border-block-end-width", vec![2]),
            ("border-inline-start-width", vec![3]),
            ("border-block-width", vec![0, 2]),
            ("border-inline-width", vec![3, 1]),
        ] {
            for (value, width) in [
                ("THICK", 3.75),
                ("0", 0.0),
                ("calc(2pt + 1pt)", 3.0),
                ("-1pt", 7.0),
                ("NaNpt", 7.0),
                ("10%", 7.0),
            ] {
                let document = parse_document("<div class='target'>box</div>").unwrap();
                let stylesheet = Stylesheet::from_css_chunks(vec![format!(
                    ".target {{border:7pt solid black;{property}:{value}}}"
                )]);
                let style = ComputedStyle::for_node(
                    &document,
                    &stylesheet,
                    document.query_selector(".target").unwrap(),
                );
                let mut expected = [7.0; 4];
                for side in &affected {
                    expected[*side] = width;
                }
                assert_eq!(
                    [
                        style.border_top_width,
                        style.border_right_width,
                        style.border_bottom_width,
                        style.border_left_width
                    ],
                    expected,
                    "{property}:{value}"
                );
            }
        }
        assert_eq!(
            parse_border_axis_widths("thin thick", 12.0),
            Some((0.75, 3.75))
        );
        assert_eq!(parse_border_axis_widths("thin -1pt", 12.0), None);
        assert_eq!(parse_border_axis_widths("thin medium thick", 12.0), None);
    }

    #[test]
    fn expands_border_width_components_and_rejects_invalid_lists() {
        for (value, expected) in [
            ("1pt", [1.0; 4]),
            ("1pt 2pt", [1.0, 2.0, 1.0, 2.0]),
            ("1pt 2pt 3pt", [1.0, 2.0, 3.0, 2.0]),
            ("1pt 2pt 3pt 0", [1.0, 2.0, 3.0, 0.0]),
            ("thin medium thick", [0.75, 2.25, 3.75, 2.25]),
            ("calc(1pt + 2pt) 4pt", [3.0, 4.0, 3.0, 4.0]),
            ("1pt -2pt", [7.0; 4]),
            ("1pt 20%", [7.0; 4]),
            ("1pt 2pt 3pt 4pt 5pt", [7.0; 4]),
            ("NaNpt", [7.0; 4]),
        ] {
            let document = parse_document("<div class='target'>box</div>").unwrap();
            let stylesheet = Stylesheet::from_css_chunks(vec![format!(
                ".target {{border:7pt solid black;border-width:{value}}}"
            )]);
            let style = ComputedStyle::for_node(
                &document,
                &stylesheet,
                document.query_selector(".target").unwrap(),
            );
            assert_eq!(
                [
                    style.border_top_width,
                    style.border_right_width,
                    style.border_bottom_width,
                    style.border_left_width
                ],
                expected,
                "{value}"
            );
        }
    }

    #[test]
    fn parses_border_side_longhands_and_radius_shorthand() {
        let document =
            parse_document(r#"<section class="target">Borders</section>"#).expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target {
                border-style: dashed;
                border-block-width: 2px 4px;
                border-inline-width: 6px 8px;
                border-block-color: rebeccapurple skyblue;
                border-inline-color: tomato gold;
                border-radius: 12px 4px / 6px 2px;
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.border_top_width - 1.5).abs() < 0.01);
        assert!((style.border_bottom_width - 3.0).abs() < 0.01);
        assert!((style.border_left_width - 4.5).abs() < 0.01);
        assert!((style.border_right_width - 6.0).abs() < 0.01);
        assert!((style.border_radius - 9.0).abs() < 0.01);
        assert_eq!(style.border_style, BorderLineStyle::Dashed);
        assert!(style.border_top_color.is_some());
        assert!(style.border_right_color.is_some());
        assert!(style.border_bottom_color.is_some());
        assert!(style.border_left_color.is_some());
    }

    #[test]
    fn parses_outline_shorthand_offset_and_current_color() {
        let document =
            parse_document(r#"<section class="target">Outline</section>"#).expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target {
                color: rgb(37 99 235);
                outline: thick dashed currentColor;
                outline-offset: 4px;
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.outline_width - 3.75).abs() < 0.01);
        assert!((style.outline_offset - 3.0).abs() < 0.01);
        assert_eq!(style.outline_style, BorderLineStyle::Dashed);
        assert_eq!(style.outline_color, Some(style.color));
    }

    #[test]
    fn parses_radial_gradient_length_stops() {
        let document = parse_document(r#"<section class="target">Radial gradient</section>"#)
            .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target {
                background:
                    radial-gradient(circle at 10% 20%,
                        rgb(37 99 235) 0 22rem,
                        transparent 36rem);
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);
        let radial = style.background_radials.first().expect("radial gradient");

        assert!((radial.center_x - 0.10).abs() < 0.01);
        assert!((radial.center_y - 0.20).abs() < 0.01);
        assert!((radial.radius.resolve(1000.0) - 432.0).abs() < 0.01);
    }

    #[test]
    fn parses_background_clip_property_and_shorthand_box() {
        let document = parse_document(
            r#"
            <section class="explicit">Explicit clip</section>
            <section class="shorthand">Shorthand clip</section>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .explicit {
                background-clip: padding-box;
                background-color: rgb(255 255 255);
            }
            .shorthand {
                background: linear-gradient(90deg, red, blue) content-box;
            }
            "#
        .to_string()]);
        let explicit = document
            .query_selector(".explicit")
            .expect("explicit target exists");
        let shorthand = document
            .query_selector(".shorthand")
            .expect("shorthand target exists");
        let explicit_style = ComputedStyle::for_node(&document, &stylesheet, explicit);
        let shorthand_style = ComputedStyle::for_node(&document, &stylesheet, shorthand);

        assert_eq!(explicit_style.background_clip, BackgroundClip::PaddingBox);
        assert_eq!(shorthand_style.background_clip, BackgroundClip::ContentBox);
    }

    #[test]
    fn all_initial_resets_inherited_and_ua_style_but_keeps_custom_properties() {
        let document = parse_document(
            r#"
            <section class="parent">
                <strong class="target">Reset</strong>
            </section>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            :root { --brand: rgb(37 99 235); }
            .parent { color: rgb(220 38 38); font-size: 20px; }
            .target { all: initial; color: var(--brand); }
            "#
        .to_string()]);
        let parent = document.query_selector(".parent").expect("parent exists");
        let target = document.query_selector(".target").expect("target exists");
        let parent_style = ComputedStyle::for_node(&document, &stylesheet, parent);
        let style = ComputedStyle::for_node_with_parent(
            &document,
            &stylesheet,
            target,
            Some(&parent_style),
        );

        assert_eq!(style.font_weight, FontWeight::Normal);
        assert!((style.font_size - 12.0).abs() < 0.01);
        assert!((style.color.r - 37.0 / 255.0).abs() < 0.01);
        assert!((style.color.b - 235.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn accepts_print_safe_screen_only_framework_properties() {
        for property in [
            "cursor",
            "user-select",
            "pointer-events",
            "transition",
            "-webkit-transition",
            "transition-duration",
            "animation",
            "-webkit-animation",
            "will-change",
            "appearance",
            "-webkit-appearance",
            "backdrop-filter",
            "-webkit-backdrop-filter",
            "touch-action",
            "-webkit-user-select",
            "-webkit-font-smoothing",
            "-webkit-text-size-adjust",
            "-webkit-tap-highlight-color",
            "hyphens",
            "-webkit-hyphens",
            "-moz-hyphens",
            "-moz-orient",
            "text-rendering",
            "font-kerning",
            "font-optical-sizing",
            "font-synthesis",
            "color-scheme",
            "print-color-adjust",
            "-webkit-print-color-adjust",
            "container-type",
            "overscroll-behavior",
            "scroll-behavior",
            "accent-color",
            "caret-color",
            "resize",
            "background-attachment",
            "tab-size",
            "align-content",
            "place-content",
        ] {
            assert!(
                is_supported_property(property),
                "expected {property} to be accepted as a print-safe compatibility property"
            );
        }
    }

    #[test]
    fn parses_modern_place_content_into_grid_axes() {
        let document =
            parse_document(r#"<section class="grid">Grid</section>"#).expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .grid {
                display: grid;
                place-content: end center;
            }
            "#
        .to_string()]);
        let grid = document
            .query_selector(".grid")
            .expect("grid target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, grid);

        assert_eq!(style.align_content, JustifyContent::End);
        assert_eq!(style.justify_content, JustifyContent::Center);
    }

    #[test]
    fn evaluates_modern_supports_boolean_conditions() {
        assert!(supports_query_matches(
            "@supports (color: color-mix(in lab, red, blue))"
        ));
        assert!(supports_query_matches(
            "@supports (not (margin-trim: inline))"
        ));
        assert!(!supports_query_matches("@supports not (color: red)"));
        assert!(supports_query_matches(
            "@supports ((-webkit-hyphens: none) and (not (margin-trim: inline))) or ((-moz-orient: inline) and (not (color: rgb(from red r g b))))"
        ));
    }

    #[test]
    fn registered_custom_property_initial_values_feed_var_resolution() {
        let document = parse_document(r#"<section class="target">Registered variable</section>"#)
            .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            @property --card-alpha {
              syntax: "<percentage>";
              inherits: false;
              initial-value: 42%;
            }
            .target { opacity: var(--card-alpha); }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.opacity - 0.42).abs() < 0.01);
    }

    #[test]
    fn evaluates_modern_container_inline_size_queries_against_print_width() {
        let document = parse_document(r#"<section class="target">Container query</section>"#)
            .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target { color: rgb(220 38 38); }
            @container card (min-width: 900px) {
                .target { color: rgb(15 23 42); }
            }
            @container card (inline-size <= 50rem) {
                .target { color: rgb(37 99 235); }
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.color.r - 37.0 / 255.0).abs() < 0.01);
        assert!((style.color.g - 99.0 / 255.0).abs() < 0.01);
        assert!((style.color.b - 235.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn matches_child_adjacent_and_general_sibling_combinators() {
        let document = parse_document(
            r#"
            <section class="parent">
                <div class="direct">Direct</div>
                <article><div class="nested">Nested</div></article>
                <p class="lead">Lead</p>
                <p class="adjacent">Adjacent</p>
                <p class="later">Later</p>
            </section>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .parent > .direct { color: rgb(37 99 235); }
            .parent > .nested { color: rgb(220 38 38); }
            .lead + .adjacent { background-color: rgb(220 252 231); }
            .lead ~ .later { border-color: rgb(14 165 233); }
            "#
        .to_string()]);
        let direct = document.query_selector(".direct").expect("direct exists");
        let nested = document.query_selector(".nested").expect("nested exists");
        let adjacent = document
            .query_selector(".adjacent")
            .expect("adjacent exists");
        let later = document.query_selector(".later").expect("later exists");

        let direct_style = ComputedStyle::for_node(&document, &stylesheet, direct);
        let nested_style = ComputedStyle::for_node(&document, &stylesheet, nested);
        let adjacent_style = ComputedStyle::for_node(&document, &stylesheet, adjacent);
        let later_style = ComputedStyle::for_node(&document, &stylesheet, later);

        assert!((direct_style.color.r - 37.0 / 255.0).abs() < 0.01);
        assert!((direct_style.color.b - 235.0 / 255.0).abs() < 0.01);
        assert!((nested_style.color.r - Color::BLACK.r).abs() < 0.01);
        assert!(adjacent_style.background.is_some());
        assert!((later_style.border_color.unwrap().b - 233.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn matches_inherited_language_and_direction_pseudo_classes() {
        let document = parse_document(
            r#"
            <main lang="es-MX">
                <p class="spanish">Idioma heredado</p>
                <p class="english" lang="en-US">Overridden language</p>
                <section dir="rtl">
                    <span class="rtl-chip">Dirección heredada</span>
                    <span class="ltr-chip" dir="ltr">Direction override</span>
                </section>
            </main>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .spanish:lang(es) { color: rgb(37 99 235); }
            .english:lang(es) { color: rgb(220 38 38); }
            .rtl-chip:dir(rtl) { background-color: rgb(239 246 255); }
            .ltr-chip:dir(ltr) { border-color: rgb(14 165 233); border-width: 1px; }
            "#
        .to_string()]);
        let spanish = document.query_selector(".spanish").expect("spanish exists");
        let english = document.query_selector(".english").expect("english exists");
        let rtl = document.query_selector(".rtl-chip").expect("rtl exists");
        let ltr = document.query_selector(".ltr-chip").expect("ltr exists");

        let spanish_style = ComputedStyle::for_node(&document, &stylesheet, spanish);
        let english_style = ComputedStyle::for_node(&document, &stylesheet, english);
        let rtl_style = ComputedStyle::for_node(&document, &stylesheet, rtl);
        let ltr_style = ComputedStyle::for_node(&document, &stylesheet, ltr);

        assert!((spanish_style.color.r - 37.0 / 255.0).abs() < 0.01);
        assert!((spanish_style.color.b - 235.0 / 255.0).abs() < 0.01);
        assert!((english_style.color.r - Color::BLACK.r).abs() < 0.01);
        assert!(rtl_style.background.is_some());
        assert!((ltr_style.border_color.unwrap().b - 233.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn strips_comments_and_case_insensitive_important_flags() {
        let document = parse_document(r#"<section class="target">Commented CSS</section>"#)
            .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            /* Framework banners and build comments must not poison selectors. */
            .target {
                color: rgb(37 99 235) !IMPORTANT;
                background: linear-gradient(90deg, rgb(219 234 254), rgb(255 255 255)); /* keep */
            }
            .ignored { content: "literal /* not a comment */"; }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.color.r - 37.0 / 255.0).abs() < 0.01);
        assert!(style.background_gradient.is_some());
    }

    #[test]
    fn important_declarations_win_author_cascade_before_inline_normal() {
        let document = parse_document(
            r#"<section id="target" class="target" style="color: rgb(220 38 38)">Important</section>"#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            #target { color: rgb(37 99 235) !important; }
            .target { color: rgb(220 38 38); }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.color.r - 37.0 / 255.0).abs() < 0.01);
        assert!((style.color.g - 99.0 / 255.0).abs() < 0.01);
        assert!((style.color.b - 235.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn parses_multiple_box_shadow_layers_and_skips_inset_layers() {
        let document = parse_document(r#"<section class="target">Layered shadows</section>"#)
            .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target {
                box-shadow:
                    0 10px 40px #00000008,
                    inset 0 0 0 1px rgb(0 0 0 / 10%),
                    0 1px 3px rgb(0 0 0 / 5%);
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert_eq!(style.box_shadows.len(), 2);
        assert!((style.box_shadows[0].offset_y - 7.5).abs() < 0.01);
        assert!((style.box_shadows[0].blur - 30.0).abs() < 0.01);
        assert!((style.box_shadows[0].color.a - 8.0 / 255.0).abs() < 0.01);
        assert!((style.box_shadows[1].offset_y - 0.75).abs() < 0.01);
        assert!((style.box_shadows[1].blur - 2.25).abs() < 0.01);
        assert!((style.box_shadows[1].color.a - 0.05).abs() < 0.01);
    }

    #[test]
    fn parses_z_index_for_positioned_stacking() {
        let document =
            parse_document(r#"<section class="target">Layer</section>"#).expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target {
                position: relative;
                z-index: 12;
                isolation: isolate;
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert_eq!(style.position, Position::Relative);
        assert_eq!(style.z_index, Some(12));
    }

    #[test]
    fn parses_oklab_colors() {
        let color = Color::from_css("oklab(70% 0.02 -0.08 / 75%)").expect("oklab color");

        assert!((color.a - 0.75).abs() < 0.01);
        assert!(color.r >= 0.0 && color.r <= 1.0);
        assert!(color.g >= 0.0 && color.g <= 1.0);
        assert!(color.b >= 0.0 && color.b <= 1.0);
    }

    #[test]
    fn parses_css_lab_and_lch_colors() {
        let lab = Color::from_css("lab(62% 18 -28 / 70%)").expect("lab color");
        let lch = Color::from_css("lch(62% 34 308deg / 55%)").expect("lch color");

        assert!((lab.a - 0.70).abs() < 0.01);
        assert!((lch.a - 0.55).abs() < 0.01);
        assert!(lab.r >= 0.0 && lab.r <= 1.0);
        assert!(lab.g >= 0.0 && lab.g <= 1.0);
        assert!(lab.b >= 0.0 && lab.b <= 1.0);
        assert!(lch.r >= 0.0 && lch.r <= 1.0);
        assert!(lch.g >= 0.0 && lch.g <= 1.0);
        assert!(lch.b >= 0.0 && lch.b <= 1.0);
    }

    #[test]
    fn parses_modern_hwb_colors_and_hue_units() {
        let cyan = Color::from_css("hwb(0.5turn 10% 20% / 40%)").expect("hwb turn color");
        let yellow = Color::from_css("hsl(66.6667grad 100% 50%)").expect("hsl grad hue");
        let blue = Color::from_css("lch(62% 34 4.71238898rad / 55%)").expect("lch rad hue");

        assert!((cyan.r - 0.10).abs() < 0.01);
        assert!((cyan.g - 0.80).abs() < 0.01);
        assert!((cyan.b - 0.80).abs() < 0.01);
        assert!((cyan.a - 0.40).abs() < 0.01);
        assert!(yellow.r > 0.95);
        assert!(yellow.g > 0.95);
        assert!(yellow.b < 0.05);
        assert!((blue.a - 0.55).abs() < 0.01);
        assert!(supports_query_matches(
            "@supports (color: hwb(220 20% 10% / 50%))"
        ));
    }

    #[test]
    fn parses_modern_relative_and_wide_gamut_colors() {
        let relative = Color::from_css("rgb(from oklch(62% 0.18 250 / 80%) r g b / 45%)")
            .expect("relative rgb color");
        let relative_calc =
            Color::from_css("rgb(from rgb(120 80 40 / 70%) calc(r * 0.5) g b / alpha)")
                .expect("relative rgb calc color");
        let p3 = Color::from_css("color(display-p3 1 0.35 0 / 60%)").expect("display-p3 color");
        let srgb = Color::from_css("color(srgb 0.2 0.4 0.6 / 75%)").expect("srgb color");

        assert!((relative.a - 0.45).abs() < 0.01);
        assert!((relative_calc.r - (60.0 / 255.0)).abs() < 0.01);
        assert!((relative_calc.a - 0.70).abs() < 0.01);
        assert!((p3.a - 0.60).abs() < 0.01);
        assert!(p3.r >= 0.0 && p3.r <= 1.0);
        assert!(p3.g >= 0.0 && p3.g <= 1.0);
        assert!(p3.b >= 0.0 && p3.b <= 1.0);
        assert!((srgb.r - 0.2).abs() < 0.01);
        assert!((srgb.g - 0.4).abs() < 0.01);
        assert!((srgb.b - 0.6).abs() < 0.01);
        assert!((srgb.a - 0.75).abs() < 0.01);
        assert!(supports_query_matches(
            "@supports (color: rgb(from red r g b / 50%))"
        ));
    }

    #[test]
    fn parses_color_mix_modern_color_spaces_as_print_fallback() {
        let lab_mix = Color::from_css("color-mix(in lab, red 50%, blue)").expect("lab color-mix");
        let srgb_mix =
            Color::from_css("color-mix(in srgb, red 50%, blue)").expect("srgb color-mix");
        let oklch_mix =
            Color::from_css("color-mix(in oklch shorter hue, oklch(70% 0.12 260) 40%, black)")
                .expect("oklch color-mix");

        assert!(lab_mix.r >= 0.0 && lab_mix.r <= 1.0);
        assert!(lab_mix.g >= 0.0 && lab_mix.g <= 1.0);
        assert!(lab_mix.b >= 0.0 && lab_mix.b <= 1.0);
        assert!(
            (lab_mix.r - srgb_mix.r).abs()
                + (lab_mix.g - srgb_mix.g).abs()
                + (lab_mix.b - srgb_mix.b).abs()
                > 0.01,
            "lab color-mix should be mixed in Lab, not silently in sRGB"
        );
        assert!(oklch_mix.r >= 0.0 && oklch_mix.r <= 1.0);
        assert!(oklch_mix.g >= 0.0 && oklch_mix.g <= 1.0);
        assert!(oklch_mix.b >= 0.0 && oklch_mix.b <= 1.0);
    }

    #[test]
    fn parses_modern_transform_list_and_individual_properties() {
        let document =
            parse_document(r#"<section class="target">Transform</section>"#).expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target {
                translate: 0.25rem -0.15rem;
                scale: 98%;
                rotate: 0.01turn;
                transform-origin: left top;
                transform: translateX(0.4rem) translateY(-0.2rem) scale(0.98, 1.02) rotate(2deg);
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.transform_rotate_deg - 5.6).abs() < 0.01);
        assert!((style.transform_scale_x - 0.9604).abs() < 0.01);
        assert!((style.transform_scale_y - 0.9996).abs() < 0.01);
        assert!((style.transform_translate_x.unwrap().resolve(100.0) - 7.8).abs() < 0.01);
        assert!((style.transform_translate_y.unwrap().resolve(100.0) + 4.2).abs() < 0.01);
        assert!((style.transform_origin_x.resolve(100.0) - 0.0).abs() < 0.01);
        assert!((style.transform_origin_y.resolve(100.0) - 0.0).abs() < 0.01);
    }

    #[test]
    fn parses_aspect_ratio_property_and_variables() {
        let document = parse_document(r#"<section class="target">Aspect ratio</section>"#)
            .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            :root { --video-ratio: 16 / 9; }
            .target { aspect-ratio: var(--video-ratio); }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.aspect_ratio.unwrap() - (16.0 / 9.0)).abs() < 0.01);
    }

    #[test]
    fn parses_background_image_url_size_and_position() {
        let document = parse_document(r#"<section class="target">Background image</section>"#)
            .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target {
                background-image: url("assets/Photo-Case.JPG");
                background-size: cover;
                background-position: right bottom;
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert_eq!(
            style.background_image.as_deref(),
            Some("assets/Photo-Case.JPG")
        );
        assert_eq!(style.background_size, BackgroundSize::Cover);
        assert_eq!(style.background_position_x, 1.0);
        assert_eq!(style.background_position_y, 0.0);
    }

    #[test]
    fn parses_background_shorthand_url_geometry() {
        let document = parse_document(r#"<section class="target">Background shorthand</section>"#)
            .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target {
                background: url("assets/bg/photo.jpg") right bottom / cover no-repeat #fff;
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert_eq!(
            style.background_image.as_deref(),
            Some("assets/bg/photo.jpg")
        );
        assert_eq!(style.background_size, BackgroundSize::Cover);
        assert_eq!(style.background_position_x, 1.0);
        assert_eq!(style.background_position_y, 0.0);
    }

    #[test]
    fn parses_object_position_keywords_and_percentages() {
        let document =
            parse_document(r#"<img class="target" src="assets/photo.jpg">"#).expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target {
                object-fit: cover;
                object-position: right 25%;
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert_eq!(style.object_fit, ObjectFit::Cover);
        assert!((style.object_position_x - 1.0).abs() < 0.01);
        assert!((style.object_position_y - 0.75).abs() < 0.01);
    }

    #[test]
    fn parses_two_axis_percent_background_position() {
        let document = parse_document(r#"<section class="target">Background position</section>"#)
            .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target { background-position: 20% 80%; }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);

        assert!((style.background_position_x - 0.20).abs() < 0.01);
        assert!((style.background_position_y - 0.20).abs() < 0.01);
    }

    #[test]
    fn parses_text_shadow_with_current_em_units() {
        let document =
            parse_document(r#"<section class="target">Text shadow</section>"#).expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target {
                font-size: 20px;
                text-shadow: 0.1em 0.2em 0.3em rgb(15 23 42 / 70%);
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let style = ComputedStyle::for_node(&document, &stylesheet, target);
        let shadow = style.text_shadow.expect("text shadow parsed");

        assert!((shadow.offset_x - 1.5).abs() < 0.01);
        assert!((shadow.offset_y - 3.0).abs() < 0.01);
        assert!((shadow.blur - 4.5).abs() < 0.01);
        assert!((shadow.color.a - 0.70).abs() < 0.01);
    }

    #[test]
    fn aria_hidden_does_not_hide_visual_rendering() {
        let document = parse_document(
            r#"
            <svg class="icon" aria-hidden="true"></svg>
            <p class="hidden-attr" hidden>Hidden</p>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![String::new()]);
        let icon = document.query_selector(".icon").expect("icon exists");
        let hidden = document
            .query_selector(".hidden-attr")
            .expect("hidden exists");

        assert_ne!(
            ComputedStyle::for_node(&document, &stylesheet, icon).display,
            Display::None
        );
        assert_eq!(
            ComputedStyle::for_node(&document, &stylesheet, hidden).display,
            Display::None
        );
    }

    #[test]
    fn visibility_hidden_preserves_layout_display_state() {
        let document = parse_document(
            r#"
            <section class="hidden">
                <p class="child">Hidden but laid out</p>
            </section>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .hidden { visibility: hidden; display: block; }
            "#
        .to_string()]);
        let hidden = document.query_selector(".hidden").expect("hidden exists");
        let child = document.query_selector(".child").expect("child exists");
        let hidden_style = ComputedStyle::for_node(&document, &stylesheet, hidden);
        let child_style = ComputedStyle::for_node(&document, &stylesheet, child);

        assert_eq!(hidden_style.display, Display::Block);
        assert_eq!(hidden_style.visibility, Visibility::Hidden);
        assert_eq!(child_style.visibility, Visibility::Hidden);
    }

    #[test]
    fn zero_rect_clip_hides_accessible_helper_content() {
        let document = parse_document(
            r#"
            <section class="clipped">
                <span class="child">Screen-reader helper</span>
            </section>
            <section class="visible">Visible</section>
            "#,
        )
        .expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .clipped {
                position: absolute;
                clip: rect(0 0 0 0);
                width: 1px;
                height: 1px;
                overflow: hidden;
            }
            .visible { clip: auto; }
            "#
        .to_string()]);
        let clipped = document.query_selector(".clipped").expect("clipped exists");
        let child = document.query_selector(".child").expect("child exists");
        let visible = document.query_selector(".visible").expect("visible exists");

        assert_eq!(
            ComputedStyle::for_node(&document, &stylesheet, clipped).visibility,
            Visibility::Hidden
        );
        assert_eq!(
            ComputedStyle::for_node(&document, &stylesheet, child).visibility,
            Visibility::Hidden
        );
        assert_eq!(
            ComputedStyle::for_node(&document, &stylesheet, visible).visibility,
            Visibility::Visible
        );
    }

    #[test]
    fn pseudo_element_content_does_not_leak_to_real_element() {
        let document = parse_document(r#"<p class="target">Body</p>"#).expect("valid html");
        let stylesheet = Stylesheet::from_css_chunks(vec![r#"
            .target::before {
                content: "§ ";
                color: rgb(37 99 235);
            }
            .target::after {
                content: " ✓";
            }
            "#
        .to_string()]);
        let target = document.query_selector(".target").expect("target exists");
        let element_style = ComputedStyle::for_node(&document, &stylesheet, target);
        let before = stylesheet
            .pseudo_style_for_node(&document, target, PseudoElement::Before, &element_style)
            .expect("before style");
        let after = stylesheet
            .pseudo_style_for_node(&document, target, PseudoElement::After, &element_style)
            .expect("after style");

        assert_eq!(element_style.content, None);
        assert_eq!(before.content.as_deref(), Some("§ "));
        assert_eq!(after.content.as_deref(), Some(" ✓"));
        assert!((before.color.b - 235.0 / 255.0).abs() < 0.01);
    }
}
