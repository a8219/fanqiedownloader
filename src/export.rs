//! Export utilities: TXT, HTML, EPUB, ZIP.

use std::io::{Write, Cursor};

pub struct ExportChapter {
    pub title: String,
    pub content: String,
}

/// Export chapters as plain text.
pub fn export_txt(book_name: &str, chapters: &[ExportChapter]) -> Vec<u8> {
    let mut out = format!("# {}\n\n", book_name);
    for ch in chapters {
        out.push_str(&format!("## {}\n\n{}\n\n", ch.title, ch.content));
    }
    let mut bytes = out.into_bytes();
    // Prepend UTF-8 BOM
    let mut result = vec![0xEF, 0xBB, 0xBF];
    result.append(&mut bytes);
    result
}

/// Export chapters as HTML.
pub fn export_html(book_name: &str, chapters: &[ExportChapter]) -> Vec<u8> {
    let mut html = format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>{}</title></head><body><h1>{}</h1>"#,
        esc_html(book_name), esc_html(book_name)
    );
    for ch in chapters {
        let content = ch.content.lines()
            .map(|l| format!("<p>{}</p>", esc_html(l)))
            .collect::<Vec<_>>()
            .join("\n");
        html.push_str(&format!("<section><h2>{}</h2>{}</section>", esc_html(&ch.title), content));
    }
    html.push_str("</body></html>");
    html.into_bytes()
}

/// Export chapters as ZIP containing individual HTML files.
pub fn export_zip(book_name: &str, chapters: &[ExportChapter]) -> anyhow::Result<Vec<u8>> {
    let buf = Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(buf);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // index.html
    zip.start_file("index.html", options)?;
    writeln!(zip, "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>{}</title></head><body><h1>{}</h1></body></html>", esc_html(book_name), esc_html(book_name))?;

    for (i, ch) in chapters.iter().enumerate() {
        let filename = format!("chapter_{:05}.html", i + 1);
        zip.start_file(&filename, options)?;
        let content = ch.content.lines()
            .map(|l| format!("<p>{}</p>", esc_html(l)))
            .collect::<Vec<_>>()
            .join("\n");
        writeln!(zip, "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>{}</title></head><body><h2>{}</h2>{}</body></html>", esc_html(&ch.title), esc_html(&ch.title), content)?;
    }

    let buf = zip.finish()?;
    Ok(buf.into_inner())
}

fn esc_html(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', """)
}
