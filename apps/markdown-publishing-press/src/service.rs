//! Service Layer for Offline Academic & Technical Publishing Press.
//!
//! Stores publishing projects, manages BibTeX references, executes offline
//! multi-template compilation, and issues cryptographically signed copyright certificates.

use crate::models::{
    CompilationArtifact, CopyrightCertificate, DocumentTemplate, PressStats, ProjectSummary,
    PublishingProject, TargetFormat,
};
use crate::publishing_engine::PublishingEngine;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use std::collections::HashMap;
use std::sync::RwLock;

pub struct PressService {
    projects: RwLock<HashMap<String, PublishingProject>>,
    artifacts: RwLock<HashMap<String, CompilationArtifact>>,
    signing_key: SigningKey,
}

impl Default for PressService {
    fn default() -> Self {
        Self::new()
    }
}

impl PressService {
    pub fn new() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);

        let service = Self {
            projects: RwLock::new(HashMap::new()),
            artifacts: RwLock::new(HashMap::new()),
            signing_key,
        };

        service.seed_sample_projects();
        service
    }

    fn seed_sample_projects(&self) {
        // Sample 1: Academic IEEE Paper on Post-Quantum Cryptography
        let bib_raw1 = r#"
@article{nist2024,
  author = {National Institute of Standards and Technology},
  title = {FIPS 203: Module-Lattice-Based Key-Encapsulation Mechanism Standard (ML-KEM)},
  journal = {NIST Federal Information Processing Standards},
  year = {2024},
  doi = {10.6028/NIST.FIPS.203}
}

@inproceedings{shor1994,
  author = {Peter W. Shor},
  title = {Algorithms for Quantum Computation: Discrete Logarithms and Factoring},
  booktitle = {35th Annual Symposium on Foundations of Computer Science (FOCS)},
  year = {1994},
  doi = {10.1109/SFCS.1994.365700}
}

@article{bernstein2017,
  author = {Daniel J. Bernstein and Tanja Lange},
  title = {Post-quantum cryptography: dealing with the fallout of physics},
  journal = {Communications of the ACM},
  year = {2017},
  doi = {10.1145/3126193}
}
"#;

        let markdown1 = r#"# Introduction
The imminent advent of fault-tolerant quantum computation threatens classical public-key cryptography based on integer factorization and discrete logarithms [@shor1994]. Recent standardization of lattice-based schemes by NIST [@nist2024] establishes a concrete roadmap for zero-trust industrial deployments.

## Threat Model in SCADA Networks
In mission-critical industrial SCADA environments, adversaries may intercept encrypted operational telemetry today and archive it for future quantum cryptanalysis—a vector known as "Harvest Now, Decrypt Later" [@bernstein2017].

![Lattice-Based Encapsulation Protocol Flow](figures/ml_kem_flow.png){#fig:lattice_flow}

## Mathematical Formulation
The security of Module-LWE relies on the hardness of finding vector $s \in R_q^k$ given matrix $A \in R_q^{k \times k}$ and noisy sample $b = A s + e \pmod{q}$. For error distribution $\chi_\eta$:

$$ \Pr[|\langle e, v \rangle| \ge \beta] \le 2 \exp\left(-\frac{\beta^2}{2 \sigma^2}\right) $$

## Experimental Verification & Performance
Our offline evaluation on an ARM Cortex-M4 microcontroller running at 168 MHz demonstrated ML-KEM-768 key encapsulation in 1,240 microseconds with 1,184 bytes public key overhead.
"#;

        let bib1 = PublishingEngine::parse_bibtex(bib_raw1);
        let proj1 = PublishingProject {
            project_id: "PROJECT-QUANTUM-ALGO".to_string(),
            title: "Post-Quantum Cryptographic Protocols in Air-Gapped Industrial Networks".to_string(),
            authors: "Dr. Elena Vance, Marcus Thorne, IEEE Senior Member".to_string(),
            abstract_text: "We evaluate the practical latency, memory overhead, and side-channel resilience of NIST FIPS 203 ML-KEM and FIPS 204 ML-DSA inside air-gapped RS-485 Modbus and CAN-FD industrial networks. Our benchmarks prove sub-millisecond handshake encapsulation without external cloud dependencies.".to_string(),
            template: DocumentTemplate::IeeeTwoColumn,
            target_format: TargetFormat::PrintPdf,
            source_markdown: markdown1.to_string(),
            bibtex_raw: bib_raw1.to_string(),
            bibliography: bib1,
            created_at_epoch_ms: chrono::Utc::now().timestamp_millis() - 86400000,
            updated_at_epoch_ms: chrono::Utc::now().timestamp_millis() - 3600000,
        };

        // Sample 2: Enterprise SOP on Shamir Secret Sharing Quorum
        let markdown2 = r#"# Purpose and Scope
This Standard Operating Procedure defines the mandatory air-gapped protocol for conducting a Shamir $M$-of-$N$ Root Certificate Authority ceremony within an isolated Class-3 SCIF environment.

## Security Classification & Roles
All participants must possess active Top Secret clearance. The ceremony requires an $M=3$ quorum of $N=5$ designated key custodians.

![Shamir Hardware Key Reconstitution Station](diagrams/shamir_station.svg){#fig:shamir_station}

## Step-by-Step Procedure
1. Verify physical air-gap integrity: disconnect all RF, Ethernet, and USB peripherals.
2. Initialize the hardened offline host using verified cryptographic media.
3. Key custodians insert their hardware YubiKey FIPS tokens in sequential order.
4. Execute polynomial interpolation over Galois Field $GF(2^8)$:

$$ f(x) = a_0 + \sum_{i=1}^{M-1} a_i x^i \pmod{P(x)} $$

5. Validate root certificate Blake3 fingerprint before signing production CRLs.
"#;

        let proj2 = PublishingProject {
            project_id: "PROJECT-ENTERPRISE-SOP".to_string(),
            title: "SOP-SEC-804: Shamir Secret Sharing Quorum Ceremony & Root CA Initialization".to_string(),
            authors: "Office of the CISO • Enterprise Cryptography Group".to_string(),
            abstract_text: "Mandatory operating procedure for executing quorum-authenticated key generation and split-key reconstitution for offline enterprise root certification authorities.".to_string(),
            template: DocumentTemplate::EnterpriseSop,
            target_format: TargetFormat::StandaloneHtml5,
            source_markdown: markdown2.to_string(),
            bibtex_raw: String::new(),
            bibliography: Vec::new(),
            created_at_epoch_ms: chrono::Utc::now().timestamp_millis() - 43200000,
            updated_at_epoch_ms: chrono::Utc::now().timestamp_millis() - 7200000,
        };

        // Pre-compile both projects
        let art1 = PublishingEngine::compile(&proj1, proj1.template, proj1.target_format);
        let art2 = PublishingEngine::compile(&proj2, proj2.template, proj2.target_format);

        if let Ok(mut lock_p) = self.projects.write() {
            lock_p.insert(proj1.project_id.clone(), proj1);
            lock_p.insert(proj2.project_id.clone(), proj2);
        }

        if let Ok(mut lock_a) = self.artifacts.write() {
            lock_a.insert(art1.project_id.clone(), art1);
            lock_a.insert(art2.project_id.clone(), art2);
        }
    }

    /// Evaluates metrics across all managed publishing projects
    pub fn get_stats(&self) -> PressStats {
        let lock_p = match self.projects.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        let lock_a = match self.artifacts.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        let mut total_pages = 0;
        let mut total_cites = 0;

        for p in lock_p.values() {
            total_cites += p.bibliography.len();
        }

        for a in lock_a.values() {
            total_pages += a.page_count;
        }

        PressStats {
            total_projects: lock_p.len(),
            total_compilations: lock_a.len(),
            total_pages_rendered: total_pages,
            total_citations_indexed: total_cites,
        }
    }

    /// Returns list of summaries for all loaded projects
    pub fn list_projects(&self) -> Vec<ProjectSummary> {
        let lock = match self.projects.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        let mut list: Vec<ProjectSummary> = lock
            .values()
            .map(|p| ProjectSummary {
                project_id: p.project_id.clone(),
                title: p.title.clone(),
                authors: p.authors.clone(),
                template: p.template,
                word_count: p.source_markdown.split_whitespace().count(),
                citations_count: p.bibliography.len(),
                updated_at_epoch_ms: p.updated_at_epoch_ms,
            })
            .collect();

        list.sort_by_key(|b| std::cmp::Reverse(b.updated_at_epoch_ms));
        list
    }

    /// Retrieves full project details by ID
    pub fn get_project(&self, project_id: &str) -> Option<PublishingProject> {
        let lock = match self.projects.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if project_id.is_empty() {
            return lock.values().next().cloned();
        }

        lock.get(project_id).cloned()
    }

    /// Retrieves latest compilation artifact by project ID
    pub fn get_artifact(&self, project_id: &str) -> Option<CompilationArtifact> {
        let lock = match self.artifacts.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if project_id.is_empty() {
            return lock.values().next().cloned();
        }

        lock.get(project_id).cloned()
    }

    /// Compiles a project into a publication artifact
    pub fn compile_project(
        &self,
        project_id: &str,
        template_override: Option<DocumentTemplate>,
        format_override: Option<TargetFormat>,
    ) -> Result<CompilationArtifact, String> {
        let project = self
            .get_project(project_id)
            .ok_or_else(|| format!("Project '{}' not found", project_id))?;

        let template = template_override.unwrap_or(project.template);
        let format = format_override.unwrap_or(project.target_format);

        let artifact = PublishingEngine::compile(&project, template, format);

        if let Ok(mut l) = self.artifacts.write() {
            l.insert(project_id.to_string(), artifact.clone());
        }

        Ok(artifact)
    }

    /// Updates or creates a publishing project
    #[allow(clippy::too_many_arguments)]
    pub fn save_project(
        &self,
        project_id: &str,
        title: &str,
        authors: &str,
        abstract_text: &str,
        template: DocumentTemplate,
        target_format: TargetFormat,
        source_markdown: &str,
        bibtex_raw: &str,
    ) -> Result<PublishingProject, String> {
        let pid = if project_id.is_empty() {
            format!("PROJ-{:08x}", rand::random::<u32>())
        } else {
            project_id.to_string()
        };

        let bib = PublishingEngine::parse_bibtex(bibtex_raw);
        let now = chrono::Utc::now().timestamp_millis();

        let project = PublishingProject {
            project_id: pid.clone(),
            title: title.to_string(),
            authors: authors.to_string(),
            abstract_text: abstract_text.to_string(),
            template,
            target_format,
            source_markdown: source_markdown.to_string(),
            bibtex_raw: bibtex_raw.to_string(),
            bibliography: bib,
            created_at_epoch_ms: now,
            updated_at_epoch_ms: now,
        };

        let artifact = PublishingEngine::compile(&project, template, target_format);

        if let Ok(mut lp) = self.projects.write() {
            lp.insert(pid.clone(), project.clone());
        }

        if let Ok(mut la) = self.artifacts.write() {
            la.insert(pid, artifact);
        }

        Ok(project)
    }

    /// Issues an Ed25519-signed Copyright Certificate for a publishing project
    pub fn sign_copyright_certificate(
        &self,
        project_id: &str,
    ) -> Result<CopyrightCertificate, String> {
        let project = self
            .get_project(project_id)
            .ok_or_else(|| format!("Project '{}' not found", project_id))?;

        let artifact = self.get_artifact(project_id).ok_or_else(|| {
            "No compiled artifact found for project; please compile first".to_string()
        })?;

        Ok(PublishingEngine::sign_copyright_certificate(
            &project,
            &artifact.document_blake3_hash,
            &self.signing_key,
        ))
    }
}
