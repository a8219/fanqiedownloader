//! TXT / HTML / ZIP exporters.

use std::io::{Cursor, Write};

/// A resolved chapter: title + plain-text body.
#[derive(Debug, Clone)]
pub struct ChapterText {
    pub title: String,
    pub content: String,
}

/// Plain text with UTF-8 BOM (Windows-friendly).
pub fn export_txt(book_name: &str, chapters: &[ChapterText]) -> Vec<u8> {
    let mut body = String::with_capacity(4096);
    body.push_str(&format!("《{book_name}》\n\n"));
    for ch in chapters {
        body.push_str(&format!("{}\n\n{}\n\n", ch.title, ch.content));
    }

    let mut out = Vec::with_capacity(body.len() + 3);
    out.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    out.extend_from_slice(body.as_bytes());
    out
}

/// Single-file HTML.
pub fn export_html(book_name: &str, chapters: &[ChapterText]) -> Vec<u8> {
    let mut html = String::with_capacity(8192);
    html.push_str("<!DOCTYPE html>\n<html lang=\"zh-CN\"><head><meta charset=\"utf-8\">");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">");
    html.push_str(&format!("<title>{}</title>", esc(book_name)));
    html.push_str(
        "<style>body{max-width:760px;margin:0 auto;padding:24px 18px;\
         font:17px/1.85 -apple-system,\"PingFang SC\",sans-serif;color:#222}\
         h1{font-size:22px}h2{font-size:18px;margin-top:2em}\
         p{margin:.6em 0}</style></head><body>",
    );
    html.push_str(&format!("<h1>{}</h1>", esc(book_name)));

    for ch in chapters {
        html.push_str(&format!("<h2 id=\"{}\">{}</h2>", esc(&ch.title), esc(&ch.title)));
        for line in ch.content.lines() {
            if !line.trim().is_empty() {
                html.push_str(&format!("<p>{}</p>\n", esc(line)));
            }
        }
    }
    html.push_str("</body></html>");
    html.into_bytes()
}

/// ZIP archive: one HTML file per chapter + index.
pub fn export_zip(book_name: &str, chapters: &[ChapterText]) -> anyhow::Result<Vec<u8>> {
    let cursor = Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(cursor);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);

    zip.start_file("index.html", opts)?;
    writeln!(
        zip,
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>{}</title></head>\
         <body><h1>{}</h1><ul>",
        esc(book_name),
        esc(book_name)
    )?;
    for (i, ch) in chapters.iter().enumerate() {
        writeln!(
            zip,
            "<li><a href=\"chapter_{:05}.html\">{}</a></li>",
            i + 1,
            esc(&ch.title)
        )?;
    }
    writeln!(zip, "</ul></body></html>")?;

    for (i, ch) in chapters.iter().enumerate() {
        zip.start_file(format!("chapter_{:05}.html", i + 1), opts)?;
        writeln!(
            zip,
            "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>{}</title>\
             <style>body{{max-width:760px;margin:0 auto;padding:24px 18px;\
             font:17px/1.85 sans-serif}}</style></head><body><h2>{}</h2>",
            esc(&ch.title),
            esc(&ch.title)
        )?;
        for line in ch.content.lines() {
            if !line.trim().is_empty() {
                writeln!(zip, "<p>{}</p>", esc(line))?;
            }
        }
        writeln!(zip, "</body></html>")?;
    }

    let cursor = zip.finish()?;
    Ok(cursor.into_inner())
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("""),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}
