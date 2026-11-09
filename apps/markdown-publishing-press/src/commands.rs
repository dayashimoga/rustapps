//! IPC Command Dispatcher for Offline Academic & Technical Publishing Press.

use crate::models::{DocumentTemplate, TargetFormat};
use crate::service::PressService;
use serde_json::{json, Value};
use shared_licensing::{AppId, LicenseStatusReport, LicensingManager};
use std::sync::{Arc, OnceLock};

static SERVICE: OnceLock<Arc<PressService>> = OnceLock::new();
static LICENSING: OnceLock<LicensingManager> = OnceLock::new();

pub fn get_service() -> Arc<PressService> {
    SERVICE
        .get_or_init(|| Arc::new(PressService::new()))
        .clone()
}

pub fn get_licensing() -> &'static LicensingManager {
    LICENSING.get_or_init(|| {
        let data_dir = platform_common::AppPaths::from_app_name("markdown-publishing-press")
            .map(|p| p.data_dir)
            .unwrap_or_else(|_| std::path::PathBuf::from("data"));
        LicensingManager::new(AppId::MarkdownPublishingPress, &data_dir)
    })
}

/// Dispatches an IPC command formatted as JSON
pub fn dispatch_ipc_command(action: &str, payload: Value) -> Result<Value, String> {
    let service = get_service();
    let licensing = get_licensing();

    match action {
        "get_stats" => {
            let stats = service.get_stats();
            Ok(json!(stats))
        }

        "list_projects" => {
            let list = service.list_projects();
            Ok(json!(list))
        }

        "get_project" => {
            let pid = payload
                .get("project_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let project = service.get_project(pid);
            Ok(json!(project))
        }

        "get_artifact" => {
            let pid = payload
                .get("project_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let artifact = service.get_artifact(pid);
            Ok(json!(artifact))
        }

        "compile_project" => {
            let pid = payload
                .get("project_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing project_id".to_string())?;

            let template = payload
                .get("template")
                .and_then(|v| v.as_str())
                .map(|s| match s {
                    "acm_sigconf" => DocumentTemplate::AcmSigconf,
                    "springer_lncs" => DocumentTemplate::SpringerLncs,
                    "arxiv_preprint" => DocumentTemplate::ArxivPreprint,
                    "enterprise_sop" => DocumentTemplate::EnterpriseSop,
                    "book_chapter" => DocumentTemplate::BookChapter,
                    _ => DocumentTemplate::IeeeTwoColumn,
                });

            let format = payload
                .get("format")
                .and_then(|v| v.as_str())
                .map(|s| match s {
                    "epub3" => TargetFormat::Epub3,
                    "standalone_html5" => TargetFormat::StandaloneHtml5,
                    "typst_source" => TargetFormat::TypstSource,
                    _ => TargetFormat::PrintPdf,
                });

            let artifact = service.compile_project(pid, template, format)?;
            Ok(json!(artifact))
        }

        "save_project" => {
            let pid = payload
                .get("project_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let title = payload
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled Project");
            let authors = payload
                .get("authors")
                .and_then(|v| v.as_str())
                .unwrap_or("Anonymous");
            let abstract_text = payload
                .get("abstract_text")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let source_md = payload
                .get("source_markdown")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let bibtex_raw = payload
                .get("bibtex_raw")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let template = payload
                .get("template")
                .and_then(|v| v.as_str())
                .map(|s| match s {
                    "acm_sigconf" => DocumentTemplate::AcmSigconf,
                    "springer_lncs" => DocumentTemplate::SpringerLncs,
                    "arxiv_preprint" => DocumentTemplate::ArxivPreprint,
                    "enterprise_sop" => DocumentTemplate::EnterpriseSop,
                    "book_chapter" => DocumentTemplate::BookChapter,
                    _ => DocumentTemplate::IeeeTwoColumn,
                })
                .unwrap_or(DocumentTemplate::IeeeTwoColumn);

            let format = payload
                .get("target_format")
                .and_then(|v| v.as_str())
                .map(|s| match s {
                    "epub3" => TargetFormat::Epub3,
                    "standalone_html5" => TargetFormat::StandaloneHtml5,
                    "typst_source" => TargetFormat::TypstSource,
                    _ => TargetFormat::PrintPdf,
                })
                .unwrap_or(TargetFormat::PrintPdf);

            let project = service.save_project(
                pid,
                title,
                authors,
                abstract_text,
                template,
                format,
                source_md,
                bibtex_raw,
            )?;
            Ok(json!(project))
        }

        "sign_copyright_certificate" => {
            let pid = payload
                .get("project_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let cert = service.sign_copyright_certificate(pid)?;
            Ok(json!(cert))
        }

        "get_license_status" => {
            let report: LicenseStatusReport = licensing.status_report();
            Ok(json!(report))
        }

        "apply_license_key" => {
            let key = payload
                .get("license_key")
                .or_else(|| payload.get("key"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing license_key".to_string())?;

            let report = licensing.activate(key).map_err(|e| e.to_string())?;
            Ok(json!(report))
        }

        _ => Err(format!("Unknown action: {}", action)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_get_stats() {
        let res = dispatch_ipc_command("get_stats", json!({})).unwrap();
        assert!(res["total_projects"].as_u64().unwrap() >= 2);
    }

    #[test]
    fn test_ipc_list_and_get_project() {
        let list_res = dispatch_ipc_command("list_projects", json!({})).unwrap();
        let arr = list_res.as_array().unwrap();
        assert_eq!(arr.len(), 2);

        let p_id = arr[0]["project_id"].as_str().unwrap();
        let get_res = dispatch_ipc_command("get_project", json!({ "project_id": p_id })).unwrap();
        assert_eq!(get_res["project_id"].as_str().unwrap(), p_id);
    }

    #[test]
    fn test_ipc_compile_project() {
        let res = dispatch_ipc_command(
            "compile_project",
            json!({
                "project_id": "PROJECT-QUANTUM-ALGO",
                "template": "acm_sigconf",
                "format": "print_pdf"
            }),
        )
        .unwrap();
        assert_eq!(res["project_id"].as_str().unwrap(), "PROJECT-QUANTUM-ALGO");
        assert!(res["page_count"].as_u64().unwrap() >= 1);
        assert!(!res["document_blake3_hash"].as_str().unwrap().is_empty());
    }

    #[test]
    fn test_ipc_sign_copyright_certificate() {
        let res = dispatch_ipc_command(
            "sign_copyright_certificate",
            json!({ "project_id": "PROJECT-QUANTUM-ALGO" }),
        )
        .unwrap();
        assert_eq!(res["project_id"].as_str().unwrap(), "PROJECT-QUANTUM-ALGO");
        assert!(!res["ed25519_signature_hex"].as_str().unwrap().is_empty());
    }

    #[test]
    fn test_ipc_license_status() {
        let res = dispatch_ipc_command("get_license_status", json!({})).unwrap();
        assert!(res["tier"].is_string());
    }
}
