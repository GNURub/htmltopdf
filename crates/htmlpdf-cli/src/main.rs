use htmlpdf_core::{
    render_html_to_pages, write_pages_to_pdf, JsMode, LayoutPage, PageOptions, RenderMode,
    RenderOptions,
};
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    if let Err(err) = run() {
        eprintln!("htmlpdf: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return Ok(());
    }

    match args.first().map(String::as_str) {
        Some("render") => render_command(&args[1..]),
        Some(command) => Err(format!("unknown command: {command}")),
        None => Err("missing command".to_string()),
    }
}

fn render_command(args: &[String]) -> Result<(), String> {
    let mut inputs: Vec<PathBuf> = Vec::new();
    let mut output: Option<PathBuf> = None;
    let mut options = RenderOptions::default();
    let mut i = 0usize;

    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                let Some(path) = args.get(i) else {
                    return Err("--output requires a path".to_string());
                };
                output = Some(PathBuf::from(path));
            }
            "--page" => {
                i += 1;
                let Some(page) = args.get(i) else {
                    return Err("--page requires A4 or Letter".to_string());
                };
                options.page = match page.to_ascii_lowercase().as_str() {
                    "a4" => PageOptions::a4(),
                    "letter" => PageOptions::letter(),
                    other => return Err(format!("unsupported page size: {other}")),
                };
            }
            "--margin" => {
                i += 1;
                let Some(value) = args.get(i) else {
                    return Err("--margin requires a point value".to_string());
                };
                let margin = value
                    .parse::<f32>()
                    .map_err(|_| format!("invalid margin: {value}"))?;
                options.page = options.page.with_uniform_margin(margin);
            }
            "--js" => {
                i += 1;
                let Some(value) = args.get(i) else {
                    return Err("--js requires on or off".to_string());
                };
                options.js = match value.as_str() {
                    "on" | "limited" => JsMode::Limited,
                    "off" => JsMode::Off,
                    other => return Err(format!("unsupported --js value: {other}")),
                };
            }
            "--timeout-ms" => {
                i += 1;
                let Some(value) = args.get(i) else {
                    return Err("--timeout-ms requires a number".to_string());
                };
                options.timeout_ms = value
                    .parse::<u64>()
                    .map_err(|_| format!("invalid timeout: {value}"))?;
            }
            "--render-mode" => {
                i += 1;
                let Some(value) = args.get(i) else {
                    return Err("--render-mode requires generic or demo-fixture".to_string());
                };
                options.render_mode = match value.as_str() {
                    "generic" => RenderMode::Generic,
                    "demo-fixture" => RenderMode::DemoFixture,
                    other => return Err(format!("unsupported --render-mode value: {other}")),
                };
            }
            value if value.starts_with('-') => return Err(format!("unknown option: {value}")),
            value => {
                inputs.push(PathBuf::from(value));
            }
        }
        i += 1;
    }

    if inputs.is_empty() {
        return Err("render requires at least one input HTML file".to_string());
    }

    if inputs.len() == 1 {
        let input = &inputs[0];
        let output = output.unwrap_or_else(|| PathBuf::from("out.pdf"));
        render_one(input, &output, &options)?;
        println!("wrote {}", output.display());
        return Ok(());
    }

    let output_dir = output.unwrap_or_else(|| PathBuf::from("out"));
    if output_dir.extension().is_some() {
        render_many_to_one_pdf(&inputs, &output_dir, &options)?;
        println!("wrote {}", output_dir.display());
        return Ok(());
    }
    fs::create_dir_all(&output_dir)
        .map_err(|err| format!("failed to create {}: {err}", output_dir.display()))?;
    for input in inputs {
        let stem = input
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| format!("invalid input file name: {}", input.display()))?;
        let output = output_dir.join(format!("{stem}.pdf"));
        render_one(&input, &output, &options)?;
        println!("wrote {}", output.display());
    }
    Ok(())
}

fn render_many_to_one_pdf(
    inputs: &[PathBuf],
    output: &PathBuf,
    options: &RenderOptions,
) -> Result<(), String> {
    let mut all_pages = Vec::new();
    let mut output_page = None::<PageOptions>;
    for input in inputs {
        let (mut pages, page) = render_pages_for_input(input, options)?;
        if output_page.is_none() {
            output_page = Some(page);
        }
        for page in &mut pages {
            page.number = all_pages.len() + 1;
        }
        all_pages.extend(pages);
    }

    let page = output_page.ok_or_else(|| "no pages rendered".to_string())?;
    let pdf = write_pages_to_pdf(&all_pages, &page).map_err(|err| err.to_string())?;
    fs::write(output, pdf).map_err(|err| format!("failed to write {}: {err}", output.display()))
}

fn render_one(input: &PathBuf, output: &PathBuf, options: &RenderOptions) -> Result<(), String> {
    let (pages, page) = render_pages_for_input(input, options)?;
    let pdf = write_pages_to_pdf(&pages, &page).map_err(|err| err.to_string())?;
    fs::write(output, pdf).map_err(|err| format!("failed to write {}: {err}", output.display()))
}

fn render_pages_for_input(
    input: &PathBuf,
    options: &RenderOptions,
) -> Result<(Vec<LayoutPage>, PageOptions), String> {
    let html = fs::read_to_string(input)
        .map_err(|err| format!("failed to read {}: {err}", input.display()))?;
    let mut options = options.clone();
    options.base_url = Some(
        input
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_string_lossy()
            .to_string(),
    );
    render_html_to_pages(&html, &options).map_err(|err| err.to_string())
}

fn print_help() {
    println!(
        "htmlpdf\n\nUSAGE:\n  htmlpdf render <index.html> -o <out.pdf> [--page A4|Letter] [--margin <pt>] [--js on|off] [--timeout-ms <ms>] [--render-mode generic|demo-fixture]\n  htmlpdf render <files...> -o <output-dir> [options]\n  htmlpdf render <files...> -o <combined.pdf> [options]\n\nThe default render mode is generic. demo-fixture is only for repository visual-regression fixtures, not production rendering."
    );
}
