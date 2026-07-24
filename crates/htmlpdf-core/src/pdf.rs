use crate::css::{Color, FontFace, FontStyle, FontWeight, LinearGradient, TextDecoration};
use crate::layout::{
    estimate_text_width_with_spacing, is_boldish, Circle, CircleStroke, ClipRect, GradientRect,
    ImageFormat, ImageRect, LayoutItem, LayoutPage, Line, PathCommand, PngColorSpace, Polygon,
    RoundRect, StrokeRect, SvgPath, TrueTypeMetrics,
};
use crate::PageOptions;
use std::collections::BTreeSet;

const ALPHA_STATES: usize = 101;
const FONT_DESCENDANT_OBJECT_ID_START: usize = 27;
const FONT_TOUNICODE_OBJECT_ID_START: usize = 35;
const FONT_CIDTOGID_OBJECT_ID_START: usize = 43;
const FIRST_ALPHA_OBJECT_ID: usize = 51;
const FIRST_IMAGE_OBJECT_ID: usize = FIRST_ALPHA_OBJECT_ID + ALPHA_STATES;

const FONT_DESCRIPTOR_OBJECT_ID_START: usize = 11;
const FONT_FILE_OBJECT_ID_START: usize = 19;

#[derive(Debug)]
struct PdfImageResource<'a> {
    object_id: usize,
    alpha_object_id: Option<usize>,
    name: String,
    image: &'a ImageRect,
}

#[derive(Clone, Copy)]
struct PdfFontResource {
    object_id: usize,
    descriptor_id: usize,
    file_id: usize,
    descendant_id: usize,
    to_unicode_id: usize,
    cid_to_gid_id: usize,
    base_font: &'static str,
    font_face: FontFace,
    font_weight: FontWeight,
    fallback_subtype: &'static str,
    fallback_base_font: &'static str,
    path_candidates: &'static [&'static str],
}

pub fn write_pdf(pages: &[LayoutPage], _page: &PageOptions) -> Result<Vec<u8>, String> {
    if pages.is_empty() {
        return Err("cannot write PDF without pages".to_string());
    }

    let (image_resources, page_image_names, image_object_count) = collect_image_resources(pages);
    let font_resources = pdf_font_resources();
    let font_usage = collect_font_usage(pages);
    let mut objects: Vec<(usize, Vec<u8>)> = Vec::new();
    let catalog_id = 1usize;
    let pages_id = 2usize;

    objects.push((catalog_id, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()));

    let mut kids = String::new();
    for idx in 0..pages.len() {
        let page_id = page_object_id(idx, image_object_count);
        kids.push_str(&format!("{page_id} 0 R "));
    }
    objects.push((
        pages_id,
        format!("<< /Type /Pages /Kids [{kids}] /Count {} >>", pages.len()).into_bytes(),
    ));

    let empty_font_usage = BTreeSet::new();
    for (idx, font) in font_resources.iter().enumerate() {
        let used_cids = font_usage.get(idx).unwrap_or(&empty_font_usage);
        push_font_objects(&mut objects, *font, used_cids);
    }
    for alpha_idx in 0..=100usize {
        let object_id = alpha_object_id(alpha_idx);
        let alpha = alpha_idx as f32 / 100.0;
        objects.push((
            object_id,
            format!(
                "<< /Type /ExtGState /ca {} /CA {} >>",
                fmt(alpha),
                fmt(alpha)
            )
            .into_bytes(),
        ));
    }
    let ext_g_state_resources = ext_g_state_resources();
    for image in &image_resources {
        if let (Some(alpha_object_id), Some(alpha_mask)) =
            (image.alpha_object_id, image.image.alpha_mask.as_deref())
        {
            objects.push((alpha_object_id, alpha_mask_object(image.image, alpha_mask)));
        }
        objects.push((
            image.object_id,
            image_object(image.image, image.alpha_object_id)?,
        ));
    }

    for (idx, layout_page) in pages.iter().enumerate() {
        let page_id = page_object_id(idx, image_object_count);
        let content_id = content_object_id(idx, image_object_count);
        let stream = content_stream(layout_page, &page_image_names[idx]);
        let shading_resources = shading_resources(layout_page);
        let shading_resource_block = if shading_resources.is_empty() {
            String::new()
        } else {
            format!("/Shading << {shading_resources} >> ")
        };
        let xobject_resources =
            xobject_resources_for_page(&image_resources, &page_image_names[idx]);
        let xobject_resource_block = if xobject_resources.is_empty() {
            String::new()
        } else {
            format!("/XObject << {xobject_resources} >> ")
        };
        let page = layout_page.page;
        objects.push((
            page_id,
            format!(
                "<< /Type /Page /Parent {pages_id} 0 R /MediaBox [0 0 {} {}] /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R /F4 6 0 R /F5 7 0 R /F6 8 0 R /F7 9 0 R /F8 10 0 R >> /ExtGState << {ext_g_state_resources} >> {shading_resource_block}{xobject_resource_block}>> /Contents {content_id} 0 R >>",
                fmt(page.width_pt),
                fmt(page.height_pt),
            )
            .into_bytes(),
        ));
        let stream_object = format!(
            "<< /Length {} >>
stream
{}endstream",
            stream.len(),
            String::from_utf8_lossy(&stream)
        );
        objects.push((content_id, stream_object.into_bytes()));
    }

    serialize(objects)
}

fn alpha_object_id(alpha_idx: usize) -> usize {
    FIRST_ALPHA_OBJECT_ID + alpha_idx
}

fn page_object_id(index: usize, image_count: usize) -> usize {
    FIRST_IMAGE_OBJECT_ID + image_count + index * 2
}

fn content_object_id(index: usize, image_count: usize) -> usize {
    page_object_id(index, image_count) + 1
}

fn collect_font_usage(pages: &[LayoutPage]) -> Vec<BTreeSet<u32>> {
    let mut usage = vec![BTreeSet::new(); 8];
    for page in pages {
        for item in &page.items {
            if let LayoutItem::Text(text) = item {
                let idx = font_resource_index(text.font_face, text.font_weight);
                for ch in text.text.chars().filter(|ch| !ch.is_control()) {
                    usage[idx].insert(ch as u32);
                }
            }
        }
    }
    for used in &mut usage {
        used.insert(' ' as u32);
    }
    usage
}

fn font_resource_index(font_face: FontFace, font_weight: FontWeight) -> usize {
    match (font_face, font_weight) {
        (FontFace::Sans, FontWeight::Normal) => 0,
        (FontFace::Sans, FontWeight::Bold | FontWeight::Heavy) => 1,
        (FontFace::Serif, FontWeight::Normal) => 2,
        (FontFace::Serif, FontWeight::Bold | FontWeight::Heavy) => 3,
        (FontFace::Mono, FontWeight::Normal) => 4,
        (FontFace::Mono, FontWeight::Bold | FontWeight::Heavy) => 5,
        (FontFace::Lato, FontWeight::Normal) => 6,
        (FontFace::Lato, FontWeight::Bold | FontWeight::Heavy) => 7,
    }
}

fn pdf_font_resources() -> Vec<PdfFontResource> {
    vec![
        PdfFontResource {
            object_id: 3,
            descriptor_id: FONT_DESCRIPTOR_OBJECT_ID_START,
            file_id: FONT_FILE_OBJECT_ID_START,
            descendant_id: FONT_DESCENDANT_OBJECT_ID_START,
            to_unicode_id: FONT_TOUNICODE_OBJECT_ID_START,
            cid_to_gid_id: FONT_CIDTOGID_OBJECT_ID_START,
            base_font: "NotoSans-Regular",
            font_face: FontFace::Sans,
            font_weight: FontWeight::Normal,
            fallback_subtype: "Type1",
            fallback_base_font: "NotoSans-Regular",
            path_candidates: &[
                "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
                "/home/gnurub/.local/share/fonts/NotoSans-VariableFont_wdth,wght.ttf",
                "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            ],
        },
        PdfFontResource {
            object_id: 4,
            descriptor_id: FONT_DESCRIPTOR_OBJECT_ID_START + 1,
            file_id: FONT_FILE_OBJECT_ID_START + 1,
            descendant_id: FONT_DESCENDANT_OBJECT_ID_START + 1,
            to_unicode_id: FONT_TOUNICODE_OBJECT_ID_START + 1,
            cid_to_gid_id: FONT_CIDTOGID_OBJECT_ID_START + 1,
            base_font: "NotoSans-Bold",
            font_face: FontFace::Sans,
            font_weight: FontWeight::Bold,
            fallback_subtype: "Type1",
            fallback_base_font: "NotoSans-Bold",
            path_candidates: &[
                "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf",
                "/home/gnurub/.local/share/fonts/NotoSans-VariableFont_wdth,wght.ttf",
                "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
            ],
        },
        PdfFontResource {
            object_id: 5,
            descriptor_id: FONT_DESCRIPTOR_OBJECT_ID_START + 2,
            file_id: FONT_FILE_OBJECT_ID_START + 2,
            descendant_id: FONT_DESCENDANT_OBJECT_ID_START + 2,
            to_unicode_id: FONT_TOUNICODE_OBJECT_ID_START + 2,
            cid_to_gid_id: FONT_CIDTOGID_OBJECT_ID_START + 2,
            base_font: "LiberationSerif-Regular",
            font_face: FontFace::Serif,
            font_weight: FontWeight::Normal,
            fallback_subtype: "Type1",
            fallback_base_font: "Times-Roman",
            path_candidates: &["/usr/share/fonts/truetype/liberation/LiberationSerif-Regular.ttf"],
        },
        PdfFontResource {
            object_id: 6,
            descriptor_id: FONT_DESCRIPTOR_OBJECT_ID_START + 3,
            file_id: FONT_FILE_OBJECT_ID_START + 3,
            descendant_id: FONT_DESCENDANT_OBJECT_ID_START + 3,
            to_unicode_id: FONT_TOUNICODE_OBJECT_ID_START + 3,
            cid_to_gid_id: FONT_CIDTOGID_OBJECT_ID_START + 3,
            base_font: "LiberationSerif-Bold",
            font_face: FontFace::Serif,
            font_weight: FontWeight::Bold,
            fallback_subtype: "Type1",
            fallback_base_font: "Times-Bold",
            path_candidates: &["/usr/share/fonts/truetype/liberation/LiberationSerif-Bold.ttf"],
        },
        PdfFontResource {
            object_id: 7,
            descriptor_id: FONT_DESCRIPTOR_OBJECT_ID_START + 4,
            file_id: FONT_FILE_OBJECT_ID_START + 4,
            descendant_id: FONT_DESCENDANT_OBJECT_ID_START + 4,
            to_unicode_id: FONT_TOUNICODE_OBJECT_ID_START + 4,
            cid_to_gid_id: FONT_CIDTOGID_OBJECT_ID_START + 4,
            base_font: "LiberationMono-Regular",
            font_face: FontFace::Mono,
            font_weight: FontWeight::Normal,
            fallback_subtype: "Type1",
            fallback_base_font: "Courier",
            path_candidates: &["/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf"],
        },
        PdfFontResource {
            object_id: 8,
            descriptor_id: FONT_DESCRIPTOR_OBJECT_ID_START + 5,
            file_id: FONT_FILE_OBJECT_ID_START + 5,
            descendant_id: FONT_DESCENDANT_OBJECT_ID_START + 5,
            to_unicode_id: FONT_TOUNICODE_OBJECT_ID_START + 5,
            cid_to_gid_id: FONT_CIDTOGID_OBJECT_ID_START + 5,
            base_font: "LiberationMono-Bold",
            font_face: FontFace::Mono,
            font_weight: FontWeight::Bold,
            fallback_subtype: "Type1",
            fallback_base_font: "Courier-Bold",
            path_candidates: &["/usr/share/fonts/truetype/liberation/LiberationMono-Bold.ttf"],
        },
        PdfFontResource {
            object_id: 9,
            descriptor_id: FONT_DESCRIPTOR_OBJECT_ID_START + 6,
            file_id: FONT_FILE_OBJECT_ID_START + 6,
            descendant_id: FONT_DESCENDANT_OBJECT_ID_START + 6,
            to_unicode_id: FONT_TOUNICODE_OBJECT_ID_START + 6,
            cid_to_gid_id: FONT_CIDTOGID_OBJECT_ID_START + 6,
            base_font: "Lato-Regular",
            font_face: FontFace::Lato,
            font_weight: FontWeight::Normal,
            fallback_subtype: "Type1",
            fallback_base_font: "Lato-Regular",
            path_candidates: &["/usr/share/fonts/truetype/lato/Lato-Regular.ttf"],
        },
        PdfFontResource {
            object_id: 10,
            descriptor_id: FONT_DESCRIPTOR_OBJECT_ID_START + 7,
            file_id: FONT_FILE_OBJECT_ID_START + 7,
            descendant_id: FONT_DESCENDANT_OBJECT_ID_START + 7,
            to_unicode_id: FONT_TOUNICODE_OBJECT_ID_START + 7,
            cid_to_gid_id: FONT_CIDTOGID_OBJECT_ID_START + 7,
            base_font: "Lato-Bold",
            font_face: FontFace::Lato,
            font_weight: FontWeight::Bold,
            fallback_subtype: "Type1",
            fallback_base_font: "Lato-Bold",
            path_candidates: &["/usr/share/fonts/truetype/lato/Lato-Bold.ttf"],
        },
    ]
}

fn push_font_objects(
    objects: &mut Vec<(usize, Vec<u8>)>,
    font: PdfFontResource,
    used_cids: &BTreeSet<u32>,
) {
    if std::env::var_os("HTMLPDF_EMBED_FONTS").is_some() {
        if let Some(bytes) = font
            .path_candidates
            .iter()
            .find_map(|path| std::fs::read(path).ok())
        {
            if let Some(metrics) = TrueTypeMetrics::parse(&bytes) {
                objects.push((font.file_id, font_file_object(&bytes)));
                objects.push((font.descriptor_id, font_descriptor_object(font)));
                objects.push((
                    font.cid_to_gid_id,
                    cid_to_gid_map_object(used_cids, &metrics),
                ));
                objects.push((font.to_unicode_id, to_unicode_cmap_object(font, used_cids)));
                objects.push((
                    font.descendant_id,
                    cid_font_object(font, used_cids, &metrics),
                ));
                objects.push((font.object_id, type0_font_object(font)));
                return;
            }
        }
    }
    objects.push((font.object_id, fallback_font_object(font)));
}

fn type0_font_object(font: PdfFontResource) -> Vec<u8> {
    format!(
        "<< /Type /Font /Subtype /Type0 /BaseFont /{} /Encoding /Identity-H /DescendantFonts [{} 0 R] /ToUnicode {} 0 R >>",
        font.base_font,
        font.descendant_id,
        font.to_unicode_id,
    )
    .into_bytes()
}

fn cid_font_object(
    font: PdfFontResource,
    used_cids: &BTreeSet<u32>,
    metrics: &TrueTypeMetrics,
) -> Vec<u8> {
    format!(
        "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor {} 0 R /W [{}] /CIDToGIDMap {} 0 R >>",
        font.base_font,
        font.descriptor_id,
        cid_widths(used_cids, metrics),
        font.cid_to_gid_id,
    )
    .into_bytes()
}

fn fallback_font_object(font: PdfFontResource) -> Vec<u8> {
    format!(
        "<< /Type /Font /Subtype /{} /BaseFont /{} /Encoding /WinAnsiEncoding >>",
        font.fallback_subtype, font.fallback_base_font
    )
    .into_bytes()
}

fn font_descriptor_object(font: PdfFontResource) -> Vec<u8> {
    let flags = match font.font_face {
        FontFace::Mono => 33,
        FontFace::Serif => 34,
        FontFace::Sans | FontFace::Lato => 32,
    };
    format!(
        "<< /Type /FontDescriptor /FontName /{} /Flags {flags} /FontBBox [-1000 -400 2500 1200] /ItalicAngle 0 /Ascent 1069 /Descent -293 /CapHeight 714 /StemV {} /FontFile2 {} 0 R >>",
        font.base_font,
        if is_boldish(font.font_weight) { 120 } else { 80 },
        font.file_id
    )
    .into_bytes()
}

fn font_file_object(bytes: &[u8]) -> Vec<u8> {
    let mut object = format!(
        "<< /Length {} /Length1 {} >>\nstream\n",
        bytes.len(),
        bytes.len()
    )
    .into_bytes();
    object.extend_from_slice(bytes);
    object.extend_from_slice(b"\nendstream");
    object
}

fn cid_widths(used_cids: &BTreeSet<u32>, metrics: &TrueTypeMetrics) -> String {
    used_cids
        .iter()
        .filter_map(|cid| {
            char::from_u32(*cid).and_then(|ch| {
                metrics
                    .glyph_width_1000(ch)
                    .map(|width| format!("{cid} [{}]", width.round() as i32))
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn cid_to_gid_map_object(used_cids: &BTreeSet<u32>, metrics: &TrueTypeMetrics) -> Vec<u8> {
    let max_cid = used_cids
        .iter()
        .copied()
        .filter(|cid| *cid <= u16::MAX as u32)
        .max()
        .unwrap_or(' ' as u32) as usize;
    let mut map = vec![0u8; (max_cid + 1) * 2];
    for cid in used_cids {
        if *cid > u16::MAX as u32 {
            continue;
        }
        let Some(ch) = char::from_u32(*cid) else {
            continue;
        };
        let Some(gid) = metrics.glyph_id(ch) else {
            continue;
        };
        let offset = *cid as usize * 2;
        map[offset] = (gid >> 8) as u8;
        map[offset + 1] = (gid & 0xff) as u8;
    }
    stream_object(&map)
}

fn to_unicode_cmap_object(font: PdfFontResource, used_cids: &BTreeSet<u32>) -> Vec<u8> {
    let mut cmap = String::new();
    cmap.push_str("/CIDInit /ProcSet findresource begin\n");
    cmap.push_str("12 dict begin\nbegincmap\n");
    cmap.push_str("/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n");
    cmap.push_str(&format!("/CMapName /{}-ToUnicode def\n", font.base_font));
    cmap.push_str("/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n");
    let entries = used_cids
        .iter()
        .copied()
        .filter(|cid| *cid <= u16::MAX as u32)
        .collect::<Vec<_>>();
    for chunk in entries.chunks(100) {
        cmap.push_str(&format!("{} beginbfchar\n", chunk.len()));
        for cid in chunk {
            cmap.push_str(&format!(
                "<{cid:04X}> <{}>\n",
                utf16be_hex(char::from_u32(*cid).unwrap_or('\u{FFFD}'))
            ));
        }
        cmap.push_str("endbfchar\n");
    }
    cmap.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
    stream_object(cmap.as_bytes())
}

fn collect_image_resources(
    pages: &[LayoutPage],
) -> (Vec<PdfImageResource<'_>>, Vec<Vec<String>>, usize) {
    let mut resources = Vec::new();
    let mut names_by_page = Vec::with_capacity(pages.len());
    let mut next_object_id = FIRST_IMAGE_OBJECT_ID;
    for page in pages {
        let mut page_names = Vec::new();
        for item in &page.items {
            if let LayoutItem::Image(image) = item {
                let name = format!("Im{}", resources.len());
                let object_id = next_object_id;
                next_object_id += 1;
                let alpha_object_id = if image.alpha_mask.is_some() {
                    let id = next_object_id;
                    next_object_id += 1;
                    Some(id)
                } else {
                    None
                };
                page_names.push(name.clone());
                resources.push(PdfImageResource {
                    object_id,
                    alpha_object_id,
                    name,
                    image,
                });
            }
        }
        names_by_page.push(page_names);
    }
    (
        resources,
        names_by_page,
        next_object_id.saturating_sub(FIRST_IMAGE_OBJECT_ID),
    )
}

fn xobject_resources_for_page(
    resources: &[PdfImageResource<'_>],
    page_image_names: &[String],
) -> String {
    let mut out = String::new();
    for name in page_image_names {
        if let Some(resource) = resources.iter().find(|resource| &resource.name == name) {
            out.push_str(&format!("/{name} {} 0 R ", resource.object_id));
        }
    }
    out
}

fn image_object(image: &ImageRect, alpha_object_id: Option<usize>) -> Result<Vec<u8>, String> {
    let filter = match image.format {
        ImageFormat::Jpeg => "/DCTDecode",
        ImageFormat::Png { .. } => "/FlateDecode",
        ImageFormat::Raw { .. } => "",
    };
    let color_space = match image.format {
        ImageFormat::Jpeg => "/DeviceRGB",
        ImageFormat::Png {
            color_space: PngColorSpace::DeviceGray,
            ..
        } => "/DeviceGray",
        ImageFormat::Png {
            color_space: PngColorSpace::DeviceRgb,
            ..
        } => "/DeviceRGB",
        ImageFormat::Raw {
            color_space: PngColorSpace::DeviceGray,
            ..
        } => "/DeviceGray",
        ImageFormat::Raw {
            color_space: PngColorSpace::DeviceRgb,
            ..
        } => "/DeviceRGB",
    };
    let bits_per_component = match image.format {
        ImageFormat::Jpeg => 8,
        ImageFormat::Png {
            bits_per_component, ..
        }
        | ImageFormat::Raw {
            bits_per_component, ..
        } => bits_per_component,
    };
    let filter_block = if filter.is_empty() {
        String::new()
    } else {
        format!(" /Filter {filter}")
    };
    let decode_params = match image.format {
        ImageFormat::Jpeg | ImageFormat::Raw { .. } => String::new(),
        ImageFormat::Png { components, .. } => format!(
            " /DecodeParms << /Predictor 15 /Colors {components} /BitsPerComponent {bits_per_component} /Columns {} >>",
            image.intrinsic_width_px
        ),
    };
    let smask = alpha_object_id
        .map(|id| format!(" /SMask {id} 0 R"))
        .unwrap_or_default();
    let mut object = format!(
        "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace {color_space} /BitsPerComponent {bits_per_component}{filter_block}{decode_params}{smask} /Length {} >>\nstream\n",
        image.intrinsic_width_px,
        image.intrinsic_height_px,
        image.data.len()
    )
    .into_bytes();
    object.extend_from_slice(&image.data);
    object.extend_from_slice(b"\nendstream");
    Ok(object)
}

fn alpha_mask_object(image: &ImageRect, alpha_mask: &[u8]) -> Vec<u8> {
    let mut object = format!(
        "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceGray /BitsPerComponent 8 /Length {} >>\nstream\n",
        image.intrinsic_width_px,
        image.intrinsic_height_px,
        alpha_mask.len()
    )
    .into_bytes();
    object.extend_from_slice(alpha_mask);
    object.extend_from_slice(b"\nendstream");
    object
}

fn ext_g_state_resources() -> String {
    let mut resources = String::new();
    for alpha_idx in 0..=100usize {
        resources.push_str(&format!(
            "/GS{alpha_idx} {} 0 R ",
            alpha_object_id(alpha_idx)
        ));
    }
    resources
}

fn shading_resources(page: &LayoutPage) -> String {
    let mut resources = String::new();
    let mut shading_index = 0usize;
    for item in &page.items {
        if let LayoutItem::LinearGradient(rect) = item {
            if native_linear_shading_supported(&rect.gradient) {
                resources.push_str(&format!(
                    "/Sh{shading_index} {} ",
                    linear_shading_dictionary(rect)
                ));
                shading_index += 1;
            }
        }
    }
    resources
}

fn content_stream(page: &LayoutPage, image_names: &[String]) -> Vec<u8> {
    let mut out = String::new();
    out.push_str(&format!("% page {}\n", page.number));
    let mut shading_index = 0usize;
    let mut image_index = 0usize;
    for item in &page.items {
        match item {
            LayoutItem::BeginClip(clip) => push_clip_rect(&mut out, clip),
            LayoutItem::EndClip => out.push_str("Q\n"),
            LayoutItem::Rect(rect) => {
                out.push_str(&format!(
                    "q {} {} {} {} rg {} {} {} {} re f Q\n",
                    alpha_graphics_state(rect.color.a),
                    fmt(rect.color.r),
                    fmt(rect.color.g),
                    fmt(rect.color.b),
                    fmt(rect.x),
                    fmt(rect.y),
                    fmt(rect.width),
                    fmt(rect.height),
                ));
            }
            LayoutItem::RoundRect(rect) => push_round_rect(&mut out, rect),
            LayoutItem::StrokeRect(rect) => push_stroke_rect(&mut out, rect),
            LayoutItem::LinearGradient(rect) => {
                if native_linear_shading_supported(&rect.gradient) {
                    push_native_linear_gradient(&mut out, rect, shading_index);
                    shading_index += 1;
                } else {
                    push_linear_gradient(&mut out, rect);
                }
            }
            LayoutItem::Circle(circle) => push_circle(&mut out, circle),
            LayoutItem::CircleStroke(circle) => push_circle_stroke(&mut out, circle),
            LayoutItem::Line(line) => push_line(&mut out, line),
            LayoutItem::Polygon(poly) => push_polygon(&mut out, poly),
            LayoutItem::SvgPath(path) => push_svg_path(&mut out, path),
            LayoutItem::Image(image) => {
                if let Some(name) = image_names.get(image_index) {
                    push_image(&mut out, image, name);
                }
                image_index += 1;
            }
            LayoutItem::Text(text) => {
                let font = pdf_font_name(text.font_face, text.font_weight);
                if let Some(shadow) = text.text_shadow {
                    let shadow_color = shadow.color.with_opacity(text.color.a * shadow.color.a);
                    push_text_run(
                        &mut out,
                        font,
                        text,
                        text.x + shadow.offset_x,
                        text.y - shadow.offset_y,
                        shadow_color,
                    );
                }
                push_text_run(&mut out, font, text, text.x, text.y, text.color);
            }
        }
    }
    out.into_bytes()
}

fn push_text_run(
    out: &mut String,
    font: &str,
    text: &crate::layout::TextRun,
    x: f32,
    y: f32,
    color: Color,
) {
    let (a, b, c, d) = text_transform_matrix(text.rotation_deg, text.font_style);
    let text_object = if std::env::var_os("HTMLPDF_EMBED_FONTS").is_some() {
        pdf_cid_hex_string(&text.text)
    } else {
        pdf_text_string(&text.text)
    };
    out.push_str(&format!(
        "q {} BT /{} {} Tf {} Tc {} Tw {} {} {} rg {} {} {} {} {} {} Tm {} Tj ET Q\n",
        alpha_graphics_state(color.a),
        font,
        fmt(text.font_size),
        fmt(text.letter_spacing),
        fmt(text.word_spacing),
        fmt(color.r),
        fmt(color.g),
        fmt(color.b),
        fmt(a),
        fmt(b),
        fmt(c),
        fmt(d),
        fmt(x),
        fmt(y),
        text_object,
    ));
    push_text_decoration(out, text, x, y, color);
}

fn text_transform_matrix(rotation_deg: f32, font_style: FontStyle) -> (f32, f32, f32, f32) {
    let radians = -rotation_deg.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();
    let skew = match font_style {
        FontStyle::Normal => 0.0,
        FontStyle::Italic | FontStyle::Oblique => -12.0_f32.to_radians().tan(),
    };
    (cos, sin, (skew * cos) - sin, (skew * sin) + cos)
}

fn push_text_decoration(
    out: &mut String,
    text: &crate::layout::TextRun,
    x: f32,
    y: f32,
    color: Color,
) {
    if text.text_decoration == TextDecoration::none() {
        return;
    }
    let width = estimate_text_width_with_spacing(
        &text.text,
        text.font_size,
        text.font_face,
        text.font_weight,
        text.letter_spacing,
        text.word_spacing,
    );
    if width <= 0.0 {
        return;
    }
    let (a, b, c, d) = text_transform_matrix(text.rotation_deg, text.font_style);
    let stroke_width = text
        .text_decoration_thickness
        .unwrap_or_else(|| (text.font_size / 14.0).clamp(0.45, 1.4))
        .clamp(0.1, text.font_size.max(0.1));
    let decoration_color = text.text_decoration_color.unwrap_or(color);
    if text.text_decoration.underline {
        let underline_offset = text
            .text_underline_offset
            .map(|offset| -offset)
            .unwrap_or_else(|| -text.font_size * 0.12);
        push_text_decoration_line(
            out,
            x,
            y,
            width,
            underline_offset,
            (a, b, c, d),
            stroke_width,
            decoration_color.with_opacity(color.a),
        );
    }
    if text.text_decoration.line_through {
        push_text_decoration_line(
            out,
            x,
            y,
            width,
            text.font_size * 0.32,
            (a, b, c, d),
            stroke_width,
            decoration_color.with_opacity(color.a),
        );
    }
}

fn push_text_decoration_line(
    out: &mut String,
    x: f32,
    y: f32,
    width: f32,
    offset_y: f32,
    matrix: (f32, f32, f32, f32),
    stroke_width: f32,
    color: Color,
) {
    let (a, b, c, d) = matrix;
    let x1 = x + c * offset_y;
    let y1 = y + d * offset_y;
    let x2 = x1 + a * width;
    let y2 = y1 + b * width;
    out.push_str(&format!(
        "q {} {} w {} {} {} RG {} {} m {} {} l S Q\n",
        alpha_graphics_state(color.a),
        fmt(stroke_width),
        fmt(color.r),
        fmt(color.g),
        fmt(color.b),
        fmt(x1),
        fmt(y1),
        fmt(x2),
        fmt(y2),
    ));
}

fn pdf_font_name(font_face: FontFace, font_weight: FontWeight) -> &'static str {
    match (font_face, font_weight) {
        (FontFace::Sans, FontWeight::Normal) => "F1",
        (FontFace::Sans, FontWeight::Bold | FontWeight::Heavy) => "F2",
        (FontFace::Serif, FontWeight::Normal) => "F3",
        (FontFace::Serif, FontWeight::Bold | FontWeight::Heavy) => "F4",
        (FontFace::Mono, FontWeight::Normal) => "F5",
        (FontFace::Mono, FontWeight::Bold | FontWeight::Heavy) => "F6",
        (FontFace::Lato, FontWeight::Normal) => "F7",
        (FontFace::Lato, FontWeight::Bold | FontWeight::Heavy) => "F8",
    }
}

fn alpha_graphics_state(alpha: f32) -> String {
    let alpha_idx = (alpha.clamp(0.0, 1.0) * 100.0).round() as usize;
    format!("/GS{alpha_idx} gs")
}

fn native_linear_shading_supported(gradient: &LinearGradient) -> bool {
    gradient.stops.len() <= 2
}

fn push_native_linear_gradient(out: &mut String, rect: &GradientRect, shading_index: usize) {
    out.push_str(&format!(
        "q {} {} {} {} re W n {} /Sh{shading_index} sh Q\n",
        fmt(rect.x),
        fmt(rect.y),
        fmt(rect.width),
        fmt(rect.height),
        alpha_graphics_state(linear_gradient_alpha(&rect.gradient)),
    ));
}

fn linear_shading_dictionary(rect: &GradientRect) -> String {
    let (x0, y0, x1, y1) = linear_gradient_coords(rect);
    let (start, end) = linear_gradient_endpoint_colors(&rect.gradient);
    format!(
        "<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [{} {} {} {}] /Domain [0 1] /Function << /FunctionType 2 /Domain [0 1] /C0 [{} {} {}] /C1 [{} {} {}] /N 1 >> /Extend [true true] >>",
        fmt(x0),
        fmt(y0),
        fmt(x1),
        fmt(y1),
        fmt(start.r),
        fmt(start.g),
        fmt(start.b),
        fmt(end.r),
        fmt(end.g),
        fmt(end.b),
    )
}

fn linear_gradient_endpoint_colors(gradient: &LinearGradient) -> (Color, Color) {
    let start = gradient
        .stops
        .first()
        .map(|stop| stop.color)
        .unwrap_or(gradient.start);
    let end = gradient
        .stops
        .last()
        .map(|stop| stop.color)
        .unwrap_or(gradient.end);
    (start, end)
}

fn linear_gradient_alpha(gradient: &LinearGradient) -> f32 {
    let (start, end) = linear_gradient_endpoint_colors(gradient);
    ((start.a + end.a) / 2.0).clamp(0.0, 1.0)
}

fn linear_gradient_coords(rect: &GradientRect) -> (f32, f32, f32, f32) {
    let angle = rect.gradient.angle_deg.rem_euclid(360.0).to_radians();
    let vx = angle.sin();
    let vy = -angle.cos();
    let cx = rect.x + rect.width / 2.0;
    let cy = rect.y + rect.height / 2.0;
    let half = ((rect.width * vx).abs() + (rect.height * vy).abs()) / 2.0;
    (
        cx - vx * half,
        cy - vy * half,
        cx + vx * half,
        cy + vy * half,
    )
}

fn push_linear_gradient(out: &mut String, rect: &GradientRect) {
    let angle = rect.gradient.angle_deg.rem_euclid(360.0);
    if angle == 90.0 || angle == 270.0 {
        let steps = 96usize;
        let strip_width = rect.width / steps as f32;
        for idx in 0..steps {
            let base_t = idx as f32 / (steps - 1) as f32;
            let t = if angle == 90.0 { base_t } else { 1.0 - base_t };
            let x = rect.x + idx as f32 * strip_width;
            let width = if idx == steps - 1 {
                rect.x + rect.width - x
            } else {
                strip_width + 0.25
            };
            push_gradient_cell(out, rect, x, rect.y, width, rect.height, t);
        }
        return;
    }
    if angle == 0.0 || angle == 180.0 {
        let steps = 96usize;
        let strip_height = rect.height / steps as f32;
        for idx in 0..steps {
            let base_t = idx as f32 / (steps - 1) as f32;
            let t = if angle == 180.0 { 1.0 - base_t } else { base_t };
            let y = rect.y + idx as f32 * strip_height;
            let height = if idx == steps - 1 {
                rect.y + rect.height - y
            } else {
                strip_height + 0.25
            };
            push_gradient_cell(out, rect, rect.x, y, rect.width, height, t);
        }
        return;
    }

    let radians = angle.to_radians();
    let vx = radians.sin();
    let vy = -radians.cos();
    let corner_projections = [0.0, vx, vy, vx + vy];
    let min_projection = corner_projections
        .iter()
        .copied()
        .fold(f32::INFINITY, f32::min);
    let max_projection = corner_projections
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);
    let span = (max_projection - min_projection).max(0.0001);

    if low_alpha_gradient(&rect.gradient) {
        let steps = 128usize;
        let strip_width = rect.width / steps as f32;
        for idx in 0..steps {
            let nx = (idx as f32 + 0.5) / steps as f32;
            let t = ((nx * vx + 0.5 * vy - min_projection) / span).clamp(0.0, 1.0);
            let x = rect.x + idx as f32 * strip_width;
            let width = if idx == steps - 1 {
                rect.x + rect.width - x
            } else {
                strip_width + 0.35
            };
            push_gradient_cell(out, rect, x, rect.y, width, rect.height, t);
        }
        return;
    }

    let cells = 72usize;
    let cell_width = rect.width / cells as f32;
    let cell_height = rect.height / cells as f32;
    for row in 0..cells {
        for col in 0..cells {
            let nx = (col as f32 + 0.5) / cells as f32;
            let ny = 1.0 - ((row as f32 + 0.5) / cells as f32);
            let t = ((nx * vx + ny * vy - min_projection) / span).clamp(0.0, 1.0);
            let x = rect.x + col as f32 * cell_width;
            let y = rect.y + row as f32 * cell_height;
            let width = if col == cells - 1 {
                rect.x + rect.width - x
            } else {
                cell_width + 0.25
            };
            let height = if row == cells - 1 {
                rect.y + rect.height - y
            } else {
                cell_height + 0.25
            };
            push_gradient_cell(out, rect, x, y, width, height, t);
        }
    }
}

fn low_alpha_gradient(gradient: &crate::css::LinearGradient) -> bool {
    gradient
        .stops
        .iter()
        .map(|stop| stop.color.a)
        .chain([gradient.start.a, gradient.end.a])
        .fold(0.0_f32, f32::max)
        < 0.25
}

fn push_gradient_cell(
    out: &mut String,
    rect: &GradientRect,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    t: f32,
) {
    let color = gradient_color(&rect.gradient, t);
    out.push_str(&format!(
        "q {} {} {} {} rg {} {} {} {} re f Q\n",
        alpha_graphics_state(color.a),
        fmt(color.r),
        fmt(color.g),
        fmt(color.b),
        fmt(x),
        fmt(y),
        fmt(width),
        fmt(height),
    ));
}

fn gradient_color(gradient: &crate::css::LinearGradient, t: f32) -> crate::css::Color {
    let t = t.clamp(0.0, 1.0);
    let stops = &gradient.stops;
    if stops.len() < 2 {
        return lerp_color(gradient.start, gradient.end, t);
    }
    if t <= stops[0].position {
        return stops[0].color;
    }
    for pair in stops.windows(2) {
        let start = pair[0];
        let end = pair[1];
        if t <= end.position {
            let span = (end.position - start.position).max(0.0001);
            let local_t = ((t - start.position) / span).clamp(0.0, 1.0);
            return lerp_color(start.color, end.color, local_t);
        }
    }
    stops.last().map(|stop| stop.color).unwrap_or(gradient.end)
}

fn lerp_color(start: crate::css::Color, end: crate::css::Color, t: f32) -> crate::css::Color {
    crate::css::Color {
        r: start.r + (end.r - start.r) * t,
        g: start.g + (end.g - start.g) * t,
        b: start.b + (end.b - start.b) * t,
        a: start.a + (end.a - start.a) * t,
    }
}

fn push_round_rect(out: &mut String, rect: &RoundRect) {
    let r = rect
        .radius
        .min(rect.width / 2.0)
        .min(rect.height / 2.0)
        .max(0.0);
    out.push_str(&format!(
        "q {} {} {} {} rg ",
        alpha_graphics_state(rect.color.a),
        fmt(rect.color.r),
        fmt(rect.color.g),
        fmt(rect.color.b),
    ));
    push_round_rect_path(out, rect.x, rect.y, rect.width, rect.height, r);
    out.push_str("f Q\n");
}

fn push_stroke_rect(out: &mut String, rect: &StrokeRect) {
    out.push_str(&format!(
        "q {} {} {} {} RG {} w {} ",
        alpha_graphics_state(rect.color.a),
        fmt(rect.color.r),
        fmt(rect.color.g),
        fmt(rect.color.b),
        fmt(rect.stroke_width),
        dash_pattern(rect.dash),
    ));
    if rect.radius > 0.0 {
        push_round_rect_path(out, rect.x, rect.y, rect.width, rect.height, rect.radius);
    } else {
        out.push_str(&format!(
            "{} {} {} {} re\n",
            fmt(rect.x),
            fmt(rect.y),
            fmt(rect.width),
            fmt(rect.height),
        ));
    }
    out.push_str("S Q\n");
}

fn push_image(out: &mut String, image: &ImageRect, name: &str) {
    if image.width <= 0.0 || image.height <= 0.0 {
        return;
    }
    out.push_str(&format!(
        "q {} 0 0 {} {} {} cm /{} Do Q\n",
        fmt(image.width),
        fmt(image.height),
        fmt(image.x),
        fmt(image.y),
        name,
    ));
}

fn push_clip_rect(out: &mut String, clip: &ClipRect) {
    out.push('q');
    out.push(' ');
    if clip.radius > 0.0 {
        push_round_rect_path(out, clip.x, clip.y, clip.width, clip.height, clip.radius);
    } else {
        out.push_str(&format!(
            "{} {} {} {} re\n",
            fmt(clip.x),
            fmt(clip.y),
            fmt(clip.width),
            fmt(clip.height),
        ));
    }
    out.push_str("W n\n");
}

fn push_round_rect_path(out: &mut String, x: f32, y: f32, width: f32, height: f32, radius: f32) {
    let r = radius.min(width / 2.0).min(height / 2.0).max(0.0);
    let c = 0.552_284_8 * r;
    let w = width;
    let h = height;
    out.push_str(&format!(
        "{} {} m {} {} l {} {} {} {} {} {} c {} {} l {} {} {} {} {} {} c {} {} l {} {} {} {} {} {} c {} {} l {} {} {} {} {} {} c h\n",
        fmt(x + r),
        fmt(y),
        fmt(x + w - r),
        fmt(y),
        fmt(x + w - r + c),
        fmt(y),
        fmt(x + w),
        fmt(y + r - c),
        fmt(x + w),
        fmt(y + r),
        fmt(x + w),
        fmt(y + h - r),
        fmt(x + w),
        fmt(y + h - r + c),
        fmt(x + w - r + c),
        fmt(y + h),
        fmt(x + w - r),
        fmt(y + h),
        fmt(x + r),
        fmt(y + h),
        fmt(x + r - c),
        fmt(y + h),
        fmt(x),
        fmt(y + h - r + c),
        fmt(x),
        fmt(y + h - r),
        fmt(x),
        fmt(y + r),
        fmt(x),
        fmt(y + r - c),
        fmt(x + r - c),
        fmt(y),
        fmt(x + r),
        fmt(y),
    ));
}

fn push_circle(out: &mut String, circle: &Circle) {
    let c = 0.552_284_8 * circle.r;
    let x = circle.cx;
    let y = circle.cy;
    let r = circle.r;
    out.push_str(&format!(
        "q {} {} {} {} rg {} {} m {} {} {} {} {} {} c {} {} {} {} {} {} c {} {} {} {} {} {} c {} {} {} {} {} {} c f Q\n",
        alpha_graphics_state(circle.color.a),
        fmt(circle.color.r),
        fmt(circle.color.g),
        fmt(circle.color.b),
        fmt(x + r),
        fmt(y),
        fmt(x + r),
        fmt(y + c),
        fmt(x + c),
        fmt(y + r),
        fmt(x),
        fmt(y + r),
        fmt(x - c),
        fmt(y + r),
        fmt(x - r),
        fmt(y + c),
        fmt(x - r),
        fmt(y),
        fmt(x - r),
        fmt(y - c),
        fmt(x - c),
        fmt(y - r),
        fmt(x),
        fmt(y - r),
        fmt(x + c),
        fmt(y - r),
        fmt(x + r),
        fmt(y - c),
        fmt(x + r),
        fmt(y),
    ));
}

fn push_line(out: &mut String, line: &Line) {
    out.push_str(&format!(
        "q {} {} {} {} RG {} w {} {} {} {} m {} {} l S Q\n",
        alpha_graphics_state(line.color.a),
        fmt(line.color.r),
        fmt(line.color.g),
        fmt(line.color.b),
        fmt(line.width),
        dash_pattern(line.dash),
        if line.line_cap_round { "1 J" } else { "0 J" },
        fmt(line.x1),
        fmt(line.y1),
        fmt(line.x2),
        fmt(line.y2),
    ));
}

fn dash_pattern(dash: Option<(f32, f32)>) -> String {
    dash.map(|(dash, gap)| format!("[{} {}] 0 d", fmt(dash), fmt(gap)))
        .unwrap_or_else(|| "[] 0 d".to_string())
}

fn push_circle_stroke(out: &mut String, circle: &CircleStroke) {
    let c = 0.552_284_8 * circle.r;
    let x = circle.cx;
    let y = circle.cy;
    let r = circle.r;
    out.push_str(&format!(
        "q {} {} {} {} RG {} w {} {} m {} {} {} {} {} {} c {} {} {} {} {} {} c {} {} {} {} {} {} c {} {} {} {} {} {} c S Q\n",
        alpha_graphics_state(circle.color.a),
        fmt(circle.color.r),
        fmt(circle.color.g),
        fmt(circle.color.b),
        fmt(circle.width),
        fmt(x + r),
        fmt(y),
        fmt(x + r),
        fmt(y + c),
        fmt(x + c),
        fmt(y + r),
        fmt(x),
        fmt(y + r),
        fmt(x - c),
        fmt(y + r),
        fmt(x - r),
        fmt(y + c),
        fmt(x - r),
        fmt(y),
        fmt(x - r),
        fmt(y - c),
        fmt(x - c),
        fmt(y - r),
        fmt(x),
        fmt(y - r),
        fmt(x + c),
        fmt(y - r),
        fmt(x + r),
        fmt(y - c),
        fmt(x + r),
        fmt(y),
    ));
}

fn push_polygon(out: &mut String, poly: &Polygon) {
    let Some((first_x, first_y)) = poly.points.first().copied() else {
        return;
    };
    out.push_str(&format!(
        "q {} {} {} {} rg {} {} m ",
        alpha_graphics_state(poly.color.a),
        fmt(poly.color.r),
        fmt(poly.color.g),
        fmt(poly.color.b),
        fmt(first_x),
        fmt(first_y),
    ));
    for (x, y) in poly.points.iter().skip(1) {
        out.push_str(&format!("{} {} l ", fmt(*x), fmt(*y)));
    }
    out.push_str("h f Q\n");
}

fn push_svg_path(out: &mut String, path: &SvgPath) {
    if path.commands.is_empty() || (path.fill.is_none() && path.stroke.is_none()) {
        return;
    }

    let alpha = path
        .fill
        .or(path.stroke)
        .map(|color| color.a)
        .unwrap_or(1.0);
    out.push_str(&format!("q {} ", alpha_graphics_state(alpha)));
    if let Some(fill) = path.fill {
        out.push_str(&format!(
            "{} {} {} rg ",
            fmt(fill.r),
            fmt(fill.g),
            fmt(fill.b)
        ));
    }
    if let Some(stroke) = path.stroke {
        out.push_str(&format!(
            "{} {} {} RG {} w {} {} ",
            fmt(stroke.r),
            fmt(stroke.g),
            fmt(stroke.b),
            fmt(path.stroke_width.max(0.1)),
            if path.line_cap_round { "1 J" } else { "0 J" },
            if path.line_cap_round { "1 j" } else { "0 j" }
        ));
    }
    for command in &path.commands {
        match *command {
            PathCommand::MoveTo(x, y) => {
                out.push_str(&format!("{} {} m ", fmt(x), fmt(y)));
            }
            PathCommand::LineTo(x, y) => {
                out.push_str(&format!("{} {} l ", fmt(x), fmt(y)));
            }
            PathCommand::CubicTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                out.push_str(&format!(
                    "{} {} {} {} {} {} c ",
                    fmt(x1),
                    fmt(y1),
                    fmt(x2),
                    fmt(y2),
                    fmt(x),
                    fmt(y)
                ));
            }
            PathCommand::Close => out.push_str("h "),
        }
    }
    match (path.fill.is_some(), path.stroke.is_some()) {
        (true, true) => out.push_str("B Q\n"),
        (true, false) => out.push_str("f Q\n"),
        (false, true) => out.push_str("S Q\n"),
        (false, false) => out.push_str("Q\n"),
    }
}

fn serialize(mut objects: Vec<(usize, Vec<u8>)>) -> Result<Vec<u8>, String> {
    objects.sort_by_key(|(id, _)| *id);
    let max_id = objects.iter().map(|(id, _)| *id).max().unwrap_or(0);

    let mut out = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");
    let mut offsets = vec![None::<usize>; max_id + 1];

    for (id, body) in objects {
        if id >= offsets.len() {
            return Err(format!("object id {id} exceeds xref table"));
        }
        offsets[id] = Some(out.len());
        out.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
        out.extend_from_slice(&body);
        out.extend_from_slice(b"\nendobj\n");
    }

    let xref_offset = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", max_id + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        if let Some(offset) = offset {
            out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        } else {
            out.extend_from_slice(b"0000000000 65535 f \n");
        }
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            max_id + 1,
            xref_offset
        )
        .as_bytes(),
    );
    Ok(out)
}

fn stream_object(bytes: &[u8]) -> Vec<u8> {
    let mut object = format!("<< /Length {} >>\nstream\n", bytes.len()).into_bytes();
    object.extend_from_slice(bytes);
    object.extend_from_slice(b"\nendstream");
    object
}

fn pdf_text_string(text: &str) -> String {
    format!("({})", escape_pdf_winansi_literal(text))
}

fn pdf_cid_hex_string(text: &str) -> String {
    let mut out = String::from("<");
    for ch in text.chars().filter(|ch| !ch.is_control()) {
        let cid = ch as u32;
        if cid <= u16::MAX as u32 {
            out.push_str(&format!("{cid:04X}"));
        } else {
            out.push_str("FFFD");
        }
    }
    out.push('>');
    out
}

fn utf16be_hex(ch: char) -> String {
    let mut units = [0u16; 2];
    ch.encode_utf16(&mut units)
        .iter()
        .map(|unit| format!("{unit:04X}"))
        .collect::<String>()
}

fn escape_pdf_winansi_literal(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        match ch {
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push(' '),
            ch if ch.is_ascii() && !ch.is_control() => out.push(ch),
            ch => {
                let byte = winansi_byte(ch).unwrap_or(b'?');
                out.push_str(&format!("\\{byte:03o}"));
            }
        }
    }
    out
}

fn winansi_byte(ch: char) -> Option<u8> {
    match ch {
        '\u{00a0}' => Some(0x20),
        '€' => Some(0x80),
        '‚' => Some(0x82),
        'ƒ' => Some(0x83),
        '„' => Some(0x84),
        '…' => Some(0x85),
        '†' => Some(0x86),
        '‡' => Some(0x87),
        'ˆ' => Some(0x88),
        '‰' => Some(0x89),
        'Š' => Some(0x8a),
        '‹' => Some(0x8b),
        'Œ' => Some(0x8c),
        'Ž' => Some(0x8e),
        '‘' => Some(0x91),
        '’' => Some(0x92),
        '“' => Some(0x93),
        '”' => Some(0x94),
        '•' => Some(0x95),
        '●' => Some(0x95),
        '−' => Some(b'-'),
        '–' => Some(0x96),
        '—' => Some(0x97),
        '˜' => Some(0x98),
        '™' => Some(0x99),
        'š' => Some(0x9a),
        '›' => Some(0x9b),
        'œ' => Some(0x9c),
        'ž' => Some(0x9e),
        'Ÿ' => Some(0x9f),
        ch if ('\u{00a1}'..='\u{00ff}').contains(&ch) => Some(ch as u8),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cid_hex_text_keeps_unicode_codepoints_for_embedded_fonts() {
        assert_eq!(pdf_cid_hex_string("A€•"), "<004120AC2022>");
    }

    #[test]
    fn to_unicode_cmap_maps_used_cids_back_to_text() {
        let mut used = BTreeSet::new();
        used.insert('A' as u32);
        used.insert('€' as u32);
        let font = pdf_font_resources()[0];
        let cmap = String::from_utf8(to_unicode_cmap_object(font, &used)).expect("valid cmap");

        assert!(cmap.contains("<0041> <0041>"));
        assert!(cmap.contains("<20AC> <20AC>"));
    }

    #[test]
    fn text_decoration_uses_modern_color_thickness_and_offset() {
        let text = crate::layout::TextRun {
            x: 72.0,
            y: 720.0,
            text: "Modern underline".to_string(),
            font_size: 12.0,
            color: Color::BLACK,
            font_weight: FontWeight::Normal,
            font_style: FontStyle::Normal,
            font_face: FontFace::Sans,
            letter_spacing: 0.0,
            word_spacing: 0.0,
            text_decoration: TextDecoration {
                underline: true,
                line_through: false,
            },
            text_decoration_color: Some(Color {
                r: 37.0 / 255.0,
                g: 99.0 / 255.0,
                b: 235.0 / 255.0,
                a: 1.0,
            }),
            text_decoration_thickness: Some(2.25),
            text_underline_offset: Some(3.0),
            rotation_deg: 0.0,
            text_shadow: None,
        };
        let mut out = String::new();

        push_text_decoration(&mut out, &text, text.x, text.y, text.color);

        assert!(out.contains("2.25 w"));
        assert!(out.contains("0.15 0.39 0.92 RG"));
        assert!(out.contains("72 717 m"));
    }
}

fn fmt(value: f32) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    if (rounded - rounded.trunc()).abs() < 0.001 {
        format!("{}", rounded.trunc() as i32)
    } else {
        format!("{rounded:.2}")
    }
}
