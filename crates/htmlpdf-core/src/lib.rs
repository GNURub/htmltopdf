//! Lightweight HTML/CSS/limited-JS to PDF renderer.
//!
//! This crate deliberately is **not** a browser. It targets controlled
//! document workloads where predictable output, small deployment size, and low
//! cold-start latency matter more than supporting every web platform API.

mod css;
mod dom;
mod js;
mod layout;
mod parser;
mod pdf;

pub use css::{Color, ComputedStyle};
pub use dom::{Document, Node, NodeId};
pub use layout::{LayoutItem, LayoutPage};
use std::path::{Path, PathBuf};

/// Options for one render invocation.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub page: PageOptions,
    pub js: JsMode,
    pub timeout_ms: u64,
    pub base_url: Option<String>,
    /// Allow document-selected local stylesheets and images. Disable for HTTP input.
    /// Built-in font discovery is independent of this policy.
    pub allow_local_assets: bool,
    pub render_mode: RenderMode,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            page: PageOptions::letter(),
            js: JsMode::Limited,
            timeout_ms: 250,
            base_url: None,
            allow_local_assets: true,
            render_mode: RenderMode::Generic,
        }
    }
}

/// Page geometry in PDF points.
#[derive(Debug, Clone, Copy)]
pub struct PageOptions {
    pub width_pt: f32,
    pub height_pt: f32,
    pub margin_top_pt: f32,
    pub margin_right_pt: f32,
    pub margin_bottom_pt: f32,
    pub margin_left_pt: f32,
}

impl PageOptions {
    /// Reject non-finite geometry and pages with no printable area.
    pub fn validate(&self) -> Result<(), RenderError> {
        let values = [
            self.width_pt,
            self.height_pt,
            self.margin_top_pt,
            self.margin_right_pt,
            self.margin_bottom_pt,
            self.margin_left_pt,
        ];
        if values
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
            || self.width_pt <= self.margin_left_pt + self.margin_right_pt
            || self.height_pt <= self.margin_top_pt + self.margin_bottom_pt
        {
            return Err(RenderError::InvalidInput(
                "page geometry must be finite and nonnegative, with positive printable width and height".into(),
            ));
        }
        Ok(())
    }
    const CHROMIUM_DEFAULT_PRINT_MARGIN_PT: f32 = 28.35;
    pub const A4_WIDTH_PT: f32 = 595.0;
    pub const A4_HEIGHT_PT: f32 = 842.0;

    pub fn a4() -> Self {
        Self {
            width_pt: Self::A4_WIDTH_PT,
            height_pt: Self::A4_HEIGHT_PT,
            margin_top_pt: Self::CHROMIUM_DEFAULT_PRINT_MARGIN_PT,
            margin_right_pt: Self::CHROMIUM_DEFAULT_PRINT_MARGIN_PT,
            margin_bottom_pt: Self::CHROMIUM_DEFAULT_PRINT_MARGIN_PT,
            margin_left_pt: Self::CHROMIUM_DEFAULT_PRINT_MARGIN_PT,
        }
    }

    pub fn letter() -> Self {
        Self {
            width_pt: 612.0,
            height_pt: 792.0,
            margin_top_pt: Self::CHROMIUM_DEFAULT_PRINT_MARGIN_PT,
            margin_right_pt: Self::CHROMIUM_DEFAULT_PRINT_MARGIN_PT,
            margin_bottom_pt: Self::CHROMIUM_DEFAULT_PRINT_MARGIN_PT,
            margin_left_pt: Self::CHROMIUM_DEFAULT_PRINT_MARGIN_PT,
        }
    }

    pub fn with_uniform_margin(mut self, margin_pt: f32) -> Self {
        self.margin_top_pt = margin_pt;
        self.margin_right_pt = margin_pt;
        self.margin_bottom_pt = margin_pt;
        self.margin_left_pt = margin_pt;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsMode {
    Off,
    Limited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    /// Use the generic HTML/CSS/limited-JS pipeline.
    Generic,
    /// Enable repository demo fixture compositors for visual regression only.
    ///
    /// This is intentionally opt-in. The public renderer must not silently
    /// special-case one customer's HTML, because the open-source product goal is
    /// arbitrary document HTML, not a curated set of hardcoded templates.
    DemoFixture,
}

#[derive(Debug)]
pub enum RenderError {
    InvalidInput(String),
    JavaScript(String),
    Pdf(String),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput(message) => write!(f, "invalid input: {message}"),
            Self::JavaScript(message) => write!(f, "javascript error: {message}"),
            Self::Pdf(message) => write!(f, "pdf error: {message}"),
        }
    }
}

impl std::error::Error for RenderError {}

/// Render a complete HTML document to a PDF byte buffer.
pub fn render_html_to_pdf(html: &str, options: &RenderOptions) -> Result<Vec<u8>, RenderError> {
    let (pages, page) = render_html_to_pages(html, options)?;
    pdf::write_pdf(&pages, &page).map_err(RenderError::Pdf)
}

/// Render a complete HTML document to layout pages without serializing them.
///
/// This is useful for adapters such as the CLI when several HTML inputs need
/// to be concatenated into one PDF while still preserving each input's local
/// stylesheet base URL.
pub fn render_html_to_pages(
    html: &str,
    options: &RenderOptions,
) -> Result<(Vec<LayoutPage>, PageOptions), RenderError> {
    options.page.validate()?;
    let mut document = parser::parse_document(html)?;

    if options.js == JsMode::Limited {
        js::run_limited_scripts(&mut document, options.timeout_ms)?;
    }

    let stylesheet = css::Stylesheet::from_css_chunks(load_css_chunks(&document, options)?);
    let mut effective_options = options.clone();
    effective_options.page = stylesheet.page_options(options.page);
    effective_options.page.validate()?;
    let pages = layout::layout_document(&document, &stylesheet, &effective_options);
    Ok((pages, effective_options.page))
}

/// Serialize already-laid-out pages to a PDF byte buffer.
pub fn write_pages_to_pdf(
    pages: &[LayoutPage],
    page: &PageOptions,
) -> Result<Vec<u8>, RenderError> {
    page.validate()?;
    for layout_page in pages {
        layout_page.page.validate()?;
    }
    pdf::write_pdf(pages, page).map_err(RenderError::Pdf)
}

fn load_css_chunks(
    document: &Document,
    options: &RenderOptions,
) -> Result<Vec<String>, RenderError> {
    let mut chunks = Vec::new();
    for source in document.style_sources() {
        match source {
            dom::StyleSource::Inline(css) => chunks.push(css),
            dom::StyleSource::Linked { href, media } => {
                if !linked_stylesheet_applies_to_print(media.as_deref()) {
                    continue;
                }
                if !options.allow_local_assets {
                    return Err(RenderError::InvalidInput(
                        "linked stylesheets are disabled for this render".into(),
                    ));
                }
                if let Some(path) = resolve_local_asset(options.base_url.as_deref(), &href) {
                    let css = std::fs::read_to_string(&path).map_err(|err| {
                        RenderError::InvalidInput(format!(
                            "failed to read linked stylesheet {}: {err}",
                            path.display()
                        ))
                    })?;
                    chunks.push(css);
                }
            }
        }
    }
    Ok(chunks)
}

fn linked_stylesheet_applies_to_print(media: Option<&str>) -> bool {
    let Some(media) = media else {
        return true;
    };
    let media = media.trim().to_ascii_lowercase();
    media.is_empty()
        || media
            .split(',')
            .map(str::trim)
            .any(|query| query == "all" || query.contains("print"))
}

fn resolve_local_asset(base_url: Option<&str>, href: &str) -> Option<PathBuf> {
    let href = href.trim();
    if href.is_empty()
        || href.starts_with('#')
        || href.starts_with("data:")
        || href.starts_with("http://")
        || href.starts_with("https://")
        || href.starts_with("//")
    {
        return None;
    }
    let href = href.split(['?', '#']).next().unwrap_or(href);
    let href_path = Path::new(href);
    if href_path.is_absolute() {
        return Some(href_path.to_path_buf());
    }
    let base = base_url.map(normalize_base_path)?;
    Some(base.join(href_path))
}

fn normalize_base_path(base_url: &str) -> PathBuf {
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

#[cfg(test)]
mod tests {
    #[test]
    fn invalid_page_geometry_is_rejected_before_layout() {
        for width in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            let mut options = super::RenderOptions::default();
            options.page.width_pt = width;
            assert!(super::render_html_to_pdf("<p>text</p>", &options).is_err());
        }
        let options = super::RenderOptions::default();
        assert!(super::render_html_to_pdf(
            "<style>@page { size: 100pt 100pt; margin: 60pt; }</style><p>text</p>",
            &options
        )
        .is_err());
        let mut page = super::PageOptions::letter();
        page.margin_top_pt = -1.0;
        assert!(super::write_pages_to_pdf(&[], &page).is_err());
    }
    use super::*;

    #[test]
    fn renders_minimal_pdf() {
        let pdf = render_html_to_pdf("<h1>Hello</h1><p>World</p>", &RenderOptions::default())
            .unwrap_or_else(|err| panic!("render failed: {err}"));
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.ends_with(b"%%EOF\n"));
    }

    #[test]
    fn limited_js_mutates_text_content() {
        let html = r##"
            <h1 id="title">Before</h1>
            <script>document.querySelector("#title").textContent = "After";</script>
        "##;
        let mut doc = parser::parse_document(html).unwrap_or_else(|err| panic!("parse: {err}"));
        js::run_limited_scripts(&mut doc, 250).unwrap_or_else(|err| panic!("js: {err}"));
        let text = doc.text_content(doc.root());
        assert!(text.contains("After"));
        assert!(!text.contains("Before"));
    }
}
