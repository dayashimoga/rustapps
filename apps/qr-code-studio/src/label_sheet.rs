#![forbid(unsafe_code)]
//! Printable Label Sheet Generator (Avery 5160, Avery L7160, Custom Grids)
//!
//! Generates vector PDF 1.4 documents and print-ready CSS HTML sheets completely offline
//! for manufacturing asset tags, inventory labels, badges, and warehouse stickers.

use crate::models::{BatchQrItem, QrStyleConfig};
use crate::qr_engine::generate_qr_matrix;
use std::fmt::Write as FmtWrite;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LabelSheetConfig {
    pub sheet_type: String,
    pub columns: u32,
    pub rows: u32,
    pub margin_top_mm: f32,
    pub margin_left_mm: f32,
    pub label_width_mm: f32,
    pub label_height_mm: f32,
    pub show_border: bool,
    pub page_format: String,
}

impl Default for LabelSheetConfig {
    fn default() -> Self {
        Self::avery_5160()
    }
}

impl LabelSheetConfig {
    /// Standard Avery 5160 / 8160 (US Letter: 3 cols x 10 rows = 30 labels/sheet)
    pub fn avery_5160() -> Self {
        Self {
            sheet_type: "Avery5160".to_string(),
            columns: 3,
            rows: 10,
            margin_top_mm: 12.7,
            margin_left_mm: 4.8,
            label_width_mm: 66.7,
            label_height_mm: 25.4,
            show_border: true,
            page_format: "Letter".to_string(),
        }
    }

    /// Standard Avery L7160 (A4: 3 cols x 7 rows = 21 labels/sheet)
    pub fn avery_l7160() -> Self {
        Self {
            sheet_type: "AveryL7160".to_string(),
            columns: 3,
            rows: 7,
            margin_top_mm: 15.1,
            margin_left_mm: 7.2,
            label_width_mm: 63.5,
            label_height_mm: 38.1,
            show_border: true,
            page_format: "A4".to_string(),
        }
    }
}

pub struct LabelSheetGenerator;

impl LabelSheetGenerator {
    const MM_TO_PT: f32 = 72.0 / 25.4; // 1 mm = 2.83464567 pt

    /// Generates a valid, self-contained PDF 1.4 document containing vector QR labels
    pub fn generate_pdf(
        items: &[BatchQrItem],
        style: &QrStyleConfig,
        config: &LabelSheetConfig,
    ) -> Result<Vec<u8>, String> {
        let (page_w_pt, page_h_pt) = match config.page_format.to_ascii_lowercase().as_str() {
            "a4" => (595.28f32, 841.89f32),
            _ => (612.0f32, 792.0f32), // Letter default
        };

        let labels_per_page = (config.columns * config.rows) as usize;
        if labels_per_page == 0 {
            return Err("Label sheet columns and rows must be greater than zero".to_string());
        }

        let num_pages = items.len().div_ceil(labels_per_page).max(1);

        // PDF Objects builder
        let mut pdf = Vec::new();
        let mut obj_offsets: Vec<usize> = Vec::new();

        // 1. Header
        pdf.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

        // Object 1: Catalog
        obj_offsets.push(pdf.len());
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        // Object 2: Pages (Kids will reference objects 4, 6, 8, ...)
        let pages_obj_idx = 2;
        let mut kids_refs = String::new();
        for p in 0..num_pages {
            let page_obj_id = 4 + p * 2;
            let _ = write!(kids_refs, "{} 0 R ", page_obj_id);
        }

        obj_offsets.push(pdf.len());
        let pages_entry = format!(
            "2 0 obj\n<< /Type /Pages /Kids [ {}] /Count {} >>\nendobj\n",
            kids_refs.trim(),
            num_pages
        );
        pdf.extend_from_slice(pages_entry.as_bytes());

        // Object 3: Standard Helvetica Font
        obj_offsets.push(pdf.len());
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>\nendobj\n",
        );

        // Precompute matrices for items
        let mut matrices = Vec::with_capacity(items.len());
        for item in items {
            let ecc = style.ecc_level;
            let mat = generate_qr_matrix(item.payload.as_bytes(), ecc)?;
            matrices.push((item, mat));
        }

        // Render each page
        for p in 0..num_pages {
            let page_obj_id = 4 + p * 2;
            let stream_obj_id = page_obj_id + 1;

            // Generate Content Stream
            let mut stream_data = String::new();
            let start_idx = p * labels_per_page;
            let end_idx = (start_idx + labels_per_page).min(items.len());

            for (slot, (item, matrix)) in matrices[start_idx..end_idx].iter().enumerate() {
                let col = (slot as u32) % config.columns;
                let row = (slot as u32) / config.columns;

                // Coordinates in PDF (origin is bottom-left)
                let x_mm = config.margin_left_mm + (col as f32) * config.label_width_mm;
                let y_top_mm = config.margin_top_mm + (row as f32) * config.label_height_mm;
                let y_bottom_mm = y_top_mm + config.label_height_mm;

                let x_pt = x_mm * Self::MM_TO_PT;
                let y_pt = page_h_pt - (y_bottom_mm * Self::MM_TO_PT);
                let w_pt = config.label_width_mm * Self::MM_TO_PT;
                let h_pt = config.label_height_mm * Self::MM_TO_PT;

                // Draw label border if requested
                if config.show_border {
                    let _ = writeln!(
                        stream_data,
                        "q 0.85 0.85 0.85 RG 0.5 w {:.2} {:.2} {:.2} {:.2} re S Q",
                        x_pt, y_pt, w_pt, h_pt
                    );
                }

                // Draw QR Code inside label slot
                let qr_pad_pt = 4.0f32;
                let qr_size_pt = (h_pt - qr_pad_pt * 2.0).min(w_pt * 0.42);
                let qr_x = x_pt + qr_pad_pt;
                let qr_y = y_pt + (h_pt - qr_size_pt) / 2.0;

                let dim = matrix.dimension;
                let mod_size = qr_size_pt / (dim as f32);

                let _ = writeln!(stream_data, "q 0 0 0 rg");
                for r in 0..dim {
                    for c in 0..dim {
                        if matrix.get(r, c) {
                            let mx = qr_x + (c as f32) * mod_size;
                            // PDF Y is bottom-up
                            let my = qr_y + ((dim - 1 - r) as f32) * mod_size;
                            let _ = writeln!(
                                stream_data,
                                "{:.2} {:.2} {:.2} {:.2} re f",
                                mx, my, mod_size, mod_size
                            );
                        }
                    }
                }
                stream_data.push_str("Q\n");

                // Draw Label Title and Payload Text
                let text_x = qr_x + qr_size_pt + 8.0;
                let text_y_title = y_pt + h_pt * 0.62;
                let text_y_sub = y_pt + h_pt * 0.38;

                let clean_label = sanitize_pdf_string(&item.label);
                let clean_payload = sanitize_pdf_string(&item.payload);
                let max_len = 28;
                let truncated_payload = if clean_payload.len() > max_len {
                    format!("{}...", &clean_payload[..max_len])
                } else {
                    clean_payload
                };

                let _ = writeln!(
                    stream_data,
                    "BT /F1 9 Tf 0.1 0.1 0.1 rg {:.2} {:.2} Td ({}) Tj ET",
                    text_x, text_y_title, clean_label
                );
                let _ = writeln!(
                    stream_data,
                    "BT /F1 7 Tf 0.4 0.4 0.4 rg {:.2} {:.2} Td ({}) Tj ET",
                    text_x, text_y_sub, truncated_payload
                );
            }

            // Write Page Object
            obj_offsets.push(pdf.len());
            let page_obj = format!(
                "{} 0 obj\n<< /Type /Page /Parent {} 0 R /MediaBox [ 0 0 {:.2} {:.2} ] /Contents {} 0 R /Resources << /Font << /F1 3 0 R >> >> >>\nendobj\n",
                page_obj_id, pages_obj_idx, page_w_pt, page_h_pt, stream_obj_id
            );
            pdf.extend_from_slice(page_obj.as_bytes());

            // Write Stream Object
            obj_offsets.push(pdf.len());
            let stream_bytes = stream_data.as_bytes();
            let stream_hdr = format!(
                "{} 0 obj\n<< /Length {} >>\nstream\n",
                stream_obj_id,
                stream_bytes.len()
            );
            pdf.extend_from_slice(stream_hdr.as_bytes());
            pdf.extend_from_slice(stream_bytes);
            pdf.extend_from_slice(b"\nendstream\nendobj\n");
        }

        // Xref Table
        let xref_offset = pdf.len();
        let total_objs = obj_offsets.len() + 1;
        let mut xref = format!("xref\n0 {}\n0000000000 65535 f \n", total_objs);
        for off in &obj_offsets {
            let _ = writeln!(xref, "{:010} 00000 n ", off);
        }
        pdf.extend_from_slice(xref.as_bytes());

        // Trailer
        let trailer = format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            total_objs, xref_offset
        );
        pdf.extend_from_slice(trailer.as_bytes());

        Ok(pdf)
    }

    /// Generates a print-ready HTML/CSS label sheet for instant in-browser printing
    pub fn generate_html(
        items: &[BatchQrItem],
        style: &QrStyleConfig,
        config: &LabelSheetConfig,
    ) -> Result<String, String> {
        let mut html = String::with_capacity(16384);
        html.push_str(r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>Printable Label Sheet</title><style>
@page { size: auto; margin: 0; }
body { margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; }
.sheet { display: grid; box-sizing: border-box; page-break-after: always; "#);
        let _ = write!(
            html,
            "grid-template-columns: repeat({}, {:.1}mm); grid-template-rows: repeat({}, {:.1}mm); padding-top: {:.1}mm; padding-left: {:.1}mm;",
            config.columns, config.label_width_mm, config.rows, config.label_height_mm, config.margin_top_mm, config.margin_left_mm
        );
        html.push_str(r#"}
.label { box-sizing: border-box; display: flex; align-items: center; padding: 3mm; overflow: hidden; "#);
        if config.show_border {
            html.push_str("border: 1px dashed #cbd5e1; ");
        }
        html.push_str(r#"}
.qr-box { flex: 0 0 auto; width: 20mm; height: 20mm; margin-right: 3mm; }
.qr-box svg { width: 100%; height: 100%; display: block; }
.text-box { flex: 1 1 auto; overflow: hidden; }
.title { font-size: 11px; font-weight: bold; color: #0f172a; white-space: nowrap; text-overflow: ellipsis; overflow: hidden; }
.payload { font-size: 8px; color: #64748b; word-break: break-all; margin-top: 2px; }
</style></head><body>"#);

        let labels_per_page = (config.columns * config.rows) as usize;
        let num_pages = items.len().div_ceil(labels_per_page).max(1);

        for p in 0..num_pages {
            html.push_str(r#"<div class="sheet">"#);
            let start = p * labels_per_page;
            let end = (start + labels_per_page).min(items.len());

            for item in &items[start..end] {
                let mat = generate_qr_matrix(item.payload.as_bytes(), style.ecc_level)?;
                let svg = crate::qr_engine::render_svg(&mat, style);

                html.push_str(r#"<div class="label"><div class="qr-box">"#);
                html.push_str(&svg);
                html.push_str(r#"</div><div class="text-box"><div class="title">"#);
                html.push_str(&html_escape(&item.label));
                html.push_str(r#"</div><div class="payload">"#);
                html.push_str(&html_escape(&item.payload));
                html.push_str(r#"</div></div></div>"#);
            }
            html.push_str("</div>");
        }

        html.push_str("</body></html>");
        Ok(html)
    }
}

fn sanitize_pdf_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\\' => out.push_str("\\\\"),
            c if c.is_ascii_graphic() || c == ' ' => out.push(c),
            _ => out.push(' '),
        }
    }
    out
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::QrEccLevel;

    #[test]
    fn test_pdf_label_sheet_generation() {
        let items = vec![
            BatchQrItem {
                payload: "https://item1.internal".to_string(),
                label: "Asset Tag #001".to_string(),
                payload_type: None,
            },
            BatchQrItem {
                payload: "https://item2.internal".to_string(),
                label: "Asset Tag #002".to_string(),
                payload_type: None,
            },
        ];

        let style = QrStyleConfig {
            ecc_level: QrEccLevel::High,
            ..Default::default()
        };

        let config = LabelSheetConfig::avery_5160();
        let pdf_bytes = LabelSheetGenerator::generate_pdf(&items, &style, &config).unwrap();

        assert!(!pdf_bytes.is_empty());
        assert!(pdf_bytes.starts_with(b"%PDF-1.4"));
        assert!(pdf_bytes.ends_with(b"%%EOF\n") || pdf_bytes.windows(5).any(|w| w == b"%%EOF"));

        let html = LabelSheetGenerator::generate_html(&items, &style, &config).unwrap();
        assert!(html.contains("Asset Tag #001"));
        assert!(html.contains("<svg"));
    }
}
