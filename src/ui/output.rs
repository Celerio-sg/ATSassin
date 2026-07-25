use anyhow::Result;
use std::fs;

pub struct OutputEngine;

impl OutputEngine {
    pub fn export_markdown(text: &str, path: &std::path::Path) -> Result<()> {
        fs::write(path, text)?;
        Ok(())
    }

    pub fn export_html(text: &str, path: &std::path::Path) -> Result<()> {
        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>ATSassin Export</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; line-height: 1.6; }}
        h1 {{ color: #333; }}
        .section {{ margin-bottom: 20px; }}
    </style>
</head>
<body>
    <pre>{}</pre>
</body>
</html>"#,
            text.replace("&", "&amp;")
                .replace("<", "&lt;")
                .replace(">", "&gt;")
        );
        fs::write(path, html)?;
        Ok(())
    }

    pub fn export_pdf(text: &str, path: &std::path::Path) -> Result<()> {
        let pdf_bytes = generate_pdf(text)?;
        fs::write(path, pdf_bytes)?;
        Ok(())
    }

    pub fn verify_ats_parseability(text: &str) -> Vec<String> {
        let mut issues = Vec::new();
        let lower = text.to_lowercase();

        if !lower.contains('@') {
            issues.push("Missing email address".to_string());
        }
        if !regex::Regex::new(r"\+?\d[\d\s\-()]{8,}")
            .unwrap()
            .is_match(&lower)
        {
            issues.push("Missing phone number".to_string());
        }
        if !lower.contains("experience") && !lower.contains("work history") {
            issues.push("Missing experience section".to_string());
        }
        if !lower.contains("education") {
            issues.push("Missing education section".to_string());
        }
        if !lower.contains("skill") {
            issues.push("Missing skills section".to_string());
        }

        let bullet_chars = text.matches('-').count() + text.matches('•').count();
        if bullet_chars < 3 {
            issues
                .push("Low bullet-point density; ATS parsers prefer structured lists".to_string());
        }

        if text.len() > 20000 {
            issues
                .push("Document too long; ATS parsers often truncate after 1-2 pages".to_string());
        }

        issues
    }
}

fn generate_pdf(text: &str) -> Result<Vec<u8>> {
    let sanitized = text
        .replace("\\", "\\\\")
        .replace("(", "\\(")
        .replace(")", "\\)")
        .replace("\r\n", "\n")
        .replace("\r", "\n");
    let lines: Vec<&str> = sanitized.lines().collect();
    let mut content = String::new();
    let mut y = 800.0;
    for line in &lines {
        if y < 50.0 {
            break;
        }
        content.push_str(&format!(
            "BT /F1 10 Tf 50 {} Td ({}) Tj ET\n",
            y,
            line.replace("\t", "    ")
        ));
        y -= 14.0;
    }

    let objects = vec![
        b"%PDF-1.4\n".to_vec(),
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_string().into_bytes(),
        "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_string().into_bytes(),
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n".to_string().into_bytes(),
        format!("4 0 obj\n<< /Length {} >>\nstream\n{}endstream\nendobj\n", content.len(), content).into_bytes(),
        "5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n".to_string().into_bytes(),
    ];

    let mut pdf = Vec::new();
    let mut offsets = Vec::new();
    for obj in &objects {
        offsets.push(pdf.len());
        pdf.extend_from_slice(obj);
    }

    let xref_offset = pdf.len();
    pdf.extend_from_slice(b"xref\n");
    pdf.extend_from_slice(format!("0 {}\n", objects.len() + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    pdf.extend_from_slice(b"trailer\n");
    pdf.extend_from_slice(format!("<< /Size {} /Root 1 0 R >>\n", objects.len() + 1).as_bytes());
    pdf.extend_from_slice(b"startxref\n");
    pdf.extend_from_slice(format!("{}\n", xref_offset).as_bytes());
    pdf.extend_from_slice(b"%%EOF\n");

    Ok(pdf)
}
