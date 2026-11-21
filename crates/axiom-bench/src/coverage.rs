//! The verification coverage report behind `axiom-coverage`: for each claim
//! kind, whether the Lean checker's soundness theorem is proved; the count of
//! `sorry` and the axioms the audits allow in `lean/` and `lean-engine/`; and
//! which engine functions in `axiom-core` have a Lean correctness theorem.
//!
//! Everything is read from the source tree, so the report needs neither the
//! network nor a Lean toolchain. The kernel audits (`make lean-audit` and
//! `make engine-check`) remain the authority on axioms; this report quotes
//! the list each audit enforces. Generation fails if any `sorry` is found.

use std::fmt::{self, Write as _};
use std::path::{Path, PathBuf};

use axiom_cli::error::IoFailure;
use thiserror::Error;

/// Which input or output an operation was about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverageRole {
    /// A Lean source file or directory.
    LeanSource,
    /// An `axiom-core` source file or directory.
    RustSource,
    /// The proof 3 configuration, `proof3.toml`.
    Proof3Config,
    /// An axiom audit script.
    AuditScript,
    /// The output directory.
    OutputDir,
    /// The Markdown report, `coverage.md`.
    Table,
    /// The CSV report, `coverage.csv`.
    Csv,
}

impl CoverageRole {
    /// Every role.
    pub const ALL: [Self; 7] = [
        Self::LeanSource,
        Self::RustSource,
        Self::Proof3Config,
        Self::AuditScript,
        Self::OutputDir,
        Self::Table,
        Self::Csv,
    ];
}

impl fmt::Display for CoverageRole {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::LeanSource => "Lean source",
            Self::RustSource => "engine source",
            Self::Proof3Config => "proof 3 configuration",
            Self::AuditScript => "axiom audit",
            Self::OutputDir => "output directory",
            Self::Table => "table",
            Self::Csv => "CSV",
        })
    }
}

/// A failure that ends report generation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CoverageError {
    /// An input could not be read or an output could not be written.
    #[error("cannot access the {0}: {1}")]
    Io(CoverageRole, IoFailure),
    /// An audit script names no allowed axioms.
    #[error("an axiom audit names no allowed axioms")]
    NoAllowedAxioms,
    /// The Lean sources contain at least one `sorry`.
    #[error("the Lean sources contain {0} sorry")]
    SorryFound(usize),
}

impl CoverageError {
    /// A stable kebab-case name for the error.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Io(..) => "coverage-io-failed",
            Self::NoAllowedAxioms => "no-allowed-axioms",
            Self::SorryFound(_) => "sorry-found",
        }
    }

    /// The line printed on the error stream:
    /// `axiom-coverage: <code>: <message>`.
    #[must_use]
    pub fn report(&self) -> String {
        format!("axiom-coverage: {}: {self}", self.code())
    }
}

/// One claim kind of the checker and the theorem that makes it sound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KindRow {
    /// The claim kind as the certificate names it.
    pub kind: &'static str,
    /// The Lean function that checks it.
    pub checker: &'static str,
    /// The soundness theorem for that function.
    pub theorem: &'static str,
    /// What the theorem establishes.
    pub scope: &'static str,
    /// Whether the theorem is declared in the checker's sources.
    pub proved: bool,
}

/// The `sorry` count and the allowed axioms of one Lean project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRow {
    /// The project directory, `lean` or `lean-engine`.
    pub project: &'static str,
    /// The make target whose audit enforces the axioms.
    pub audit: &'static str,
    /// The number of Lean files read, outside `.lake`.
    pub files: usize,
    /// The number of `sorry` outside comments.
    pub sorry: usize,
    /// The number of `axiom` declarations outside comments.
    pub declared_axioms: usize,
    /// The axioms the audit allows, in the order it lists them.
    pub allowed_axioms: Vec<String>,
}

/// How far proof 3 reaches one engine function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionStatus {
    /// Its Lean translation has a correctness theorem.
    Proved,
    /// It delegates to a proved kernel function; the conversion around the
    /// call is not translated.
    ViaKernel,
    /// Proof 3 models it as `Rat` and trusts the model.
    TrustedModel,
    /// No Lean theorem covers it.
    Uncovered,
}

impl fmt::Display for FunctionStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Proved => "proved",
            Self::ViaKernel => "via kernel",
            Self::TrustedModel => "trusted model",
            Self::Uncovered => "uncovered",
        })
    }
}

/// One function of `axiom-core`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionRow {
    /// The source file's module, such as `kernel`.
    pub module: String,
    /// The function, qualified by its type for a method: `Probability::pow`.
    pub function: String,
    /// How far proof 3 reaches it.
    pub status: FunctionStatus,
    /// The theorems that cover it, empty when none does.
    pub theorems: Vec<String>,
}

/// The whole report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coverage {
    /// One row per claim kind.
    pub kinds: Vec<KindRow>,
    /// One row per Lean project.
    pub projects: Vec<ProjectRow>,
    /// One row per engine function.
    pub functions: Vec<FunctionRow>,
}

impl Coverage {
    /// The number of engine functions with the given status.
    #[must_use]
    pub fn count(&self, status: FunctionStatus) -> usize {
        self.functions
            .iter()
            .filter(|row| row.status == status)
            .count()
    }
}

/// The claim kinds `AxiomCheck.Check.checkClaim` handles, each with its
/// checker and soundness theorem. `checkClaim_sound` and `check_sound`
/// combine them for a whole certificate.
const KINDS: [(&str, &str, &str, &str); 7] = [
    (
        "Mass",
        "checkRow",
        "checkRow_sound",
        "the row's probabilities sum to one",
    ),
    (
        "FixedPoint",
        "checkFixedPoint",
        "checkFixedPoint_sound",
        "the value is the chain's steady state",
    ),
    (
        "Contraction",
        "checkContraction",
        "checkContraction_sound",
        "the factor is the chain's contraction factor",
    ),
    (
        "ConvergenceBound",
        "checkConvergenceBound",
        "checkConvergenceBound_sound",
        "after the stated steps every start is within the tolerance",
    ),
    (
        "Termination",
        "checkTermination",
        "checkTermination_sound",
        "the recursion terminates with the stated probability and expectation",
    ),
    (
        "ParametricContraction",
        "checkParametric",
        "checkParametric_sound",
        "the chain contracts for every parameter in the intervals",
    ),
    (
        "Estimate",
        "checkHoeffding",
        "checkHoeffding_sound",
        "the sample count meets Hoeffding's bound; the value is not checked",
    ),
];

/// The two Lean projects, each with its audit script and make target.
const PROJECTS: [(&str, &str); 2] = [("lean", "lean-audit"), ("lean-engine", "engine-check")];

/// Reads the source tree at `root` and builds the report.
///
/// # Errors
///
/// [`CoverageError::SorryFound`] if any Lean file outside `.lake` contains a
/// `sorry`, [`CoverageError::NoAllowedAxioms`] if an audit lists no axioms,
/// and [`CoverageError::Io`] if an input cannot be read.
pub fn generate(root: &Path) -> Result<Coverage, CoverageError> {
    let mut projects = Vec::new();
    for (project, audit) in PROJECTS {
        projects.push(project_row(root, project, audit)?);
    }
    let found: usize = projects.iter().map(|row| row.sorry).sum();
    if found > 0 {
        return Err(CoverageError::SorryFound(found));
    }

    let checker = lean_text(&root.join("lean/AxiomCheck"))?;
    let kinds = KINDS
        .iter()
        .map(|&(kind, checker_fn, theorem, scope)| KindRow {
            kind,
            checker: checker_fn,
            theorem,
            scope,
            proved: declares_theorem(&checker, theorem),
        })
        .collect();

    let correctness = read(
        &root.join("lean-engine/AxiomEngine/Correctness.lean"),
        CoverageRole::LeanSource,
    )?;
    let config = read(&root.join("proof3.toml"), CoverageRole::Proof3Config)?;
    let functions = function_rows(&root.join("crates/axiom-core/src"), &correctness, &config)?;

    Ok(Coverage {
        kinds,
        projects,
        functions,
    })
}

/// Writes `coverage.md` and `coverage.csv` into `out`.
///
/// # Errors
///
/// [`CoverageError::Io`] if the directory or a file cannot be written.
pub fn write(coverage: &Coverage, out: &Path) -> Result<(), CoverageError> {
    std::fs::create_dir_all(out).map_err(|error| io(CoverageRole::OutputDir, &error))?;
    std::fs::write(out.join("coverage.md"), markdown(coverage))
        .map_err(|error| io(CoverageRole::Table, &error))?;
    std::fs::write(out.join("coverage.csv"), csv(coverage))
        .map_err(|error| io(CoverageRole::Csv, &error))
}

fn io(role: CoverageRole, error: &std::io::Error) -> CoverageError {
    CoverageError::Io(role, IoFailure::from(error))
}

fn read(path: &Path, role: CoverageRole) -> Result<String, CoverageError> {
    std::fs::read_to_string(path).map_err(|error| io(role, &error))
}

fn project_row(
    root: &Path,
    project: &'static str,
    audit: &'static str,
) -> Result<ProjectRow, CoverageError> {
    let files = lean_files(&root.join(project))?;
    let mut sorry = 0;
    let mut declared_axioms = 0;
    for file in &files {
        let code = strip_comments(&read(file, CoverageRole::LeanSource)?);
        sorry += count_sorry(&code);
        declared_axioms += code
            .lines()
            .filter(|line| {
                let line = line.trim_start();
                line.starts_with("axiom ") || line.starts_with("private axiom ")
            })
            .count();
    }
    let script = read(
        &root.join(project).join("scripts/AxiomAudit.lean"),
        CoverageRole::AuditScript,
    )?;
    let allowed_axioms = allowed_axioms(&script);
    if allowed_axioms.is_empty() {
        return Err(CoverageError::NoAllowedAxioms);
    }
    Ok(ProjectRow {
        project,
        audit,
        files: files.len(),
        sorry,
        declared_axioms,
        allowed_axioms,
    })
}

/// Every `.lean` file under `dir`, skipping `.lake`, in sorted order.
fn lean_files(dir: &Path) -> Result<Vec<PathBuf>, CoverageError> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries =
            std::fs::read_dir(&current).map_err(|error| io(CoverageRole::LeanSource, &error))?;
        for entry in entries {
            let path = entry
                .map_err(|error| io(CoverageRole::LeanSource, &error))?
                .path();
            if path.file_name().is_some_and(|name| name == ".lake") {
                continue;
            }
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "lean") {
                found.push(path);
            }
        }
    }
    found.sort();
    Ok(found)
}

/// Every Lean file under `dir`, comments removed, joined.
fn lean_text(dir: &Path) -> Result<String, CoverageError> {
    let mut text = String::new();
    for file in lean_files(dir)? {
        text.push_str(&strip_comments(&read(&file, CoverageRole::LeanSource)?));
        text.push('\n');
    }
    Ok(text)
}

/// Lean source with `--` line comments and nested `/- -/` block comments,
/// doc comments included, replaced by spaces. Line breaks are kept.
#[must_use]
pub fn strip_comments(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut depth = 0usize;
    let mut i = 0;
    while i < chars.len() {
        let here = chars[i];
        let next = chars.get(i + 1).copied();
        if depth == 0 && here == '-' && next == Some('-') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if here == '/' && next == Some('-') {
            depth += 1;
            i += 2;
            continue;
        }
        if depth > 0 && here == '-' && next == Some('/') {
            depth -= 1;
            i += 2;
            continue;
        }
        if depth > 0 {
            if here == '\n' {
                out.push('\n');
            }
        } else {
            out.push(here);
        }
        i += 1;
    }
    out
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '\'' || c == '.'
}

/// The positions at which `word` occurs in `code` as a whole identifier.
fn word_positions<'a>(code: &'a str, word: &'a str) -> impl Iterator<Item = usize> + 'a {
    code.match_indices(word).filter_map(move |(at, _)| {
        let before = code[..at].chars().next_back();
        let after = code[at + word.len()..].chars().next();
        let whole = !before.is_some_and(is_ident) && !after.is_some_and(is_ident);
        whole.then_some(at)
    })
}

/// The number of `sorry` tokens in comment-free Lean code.
#[must_use]
pub fn count_sorry(code: &str) -> usize {
    word_positions(code, "sorry").count()
}

/// Whether comment-free Lean code declares `theorem <name>`.
#[must_use]
pub fn declares_theorem(code: &str, name: &str) -> bool {
    theorems(code).iter().any(|(declared, _)| declared == name)
}

/// Every `theorem` in comment-free Lean code, with its statement: the text
/// between its name and the first `:=`.
fn theorems(code: &str) -> Vec<(String, String)> {
    word_positions(code, "theorem")
        .filter_map(|at| {
            let rest = code[at + "theorem".len()..].trim_start();
            let name: String = rest.chars().take_while(|&c| is_ident(c)).collect();
            if name.is_empty() {
                return None;
            }
            let after = &rest[name.len()..];
            let statement = after.split(":=").next().unwrap_or(after);
            Some((name, statement.to_owned()))
        })
        .collect()
}

/// The axioms an audit script allows: the names quoted with double
/// backticks in its `allowedAxioms` definition.
#[must_use]
pub fn allowed_axioms(script: &str) -> Vec<String> {
    let code = strip_comments(script);
    let Some(line) = code
        .lines()
        .find(|line| line.trim_start().starts_with("def allowedAxioms"))
    else {
        return Vec::new();
    };
    line.split("``")
        .skip(1)
        .filter_map(|part| {
            let name: String = part.chars().take_while(|&c| is_ident(c)).collect();
            (!name.is_empty()).then_some(name)
        })
        .collect()
}

/// The functions a Rust source file defines outside `#[cfg(test)]`, each
/// qualified by the type of the inherent `impl` block it sits in.
#[must_use]
pub fn rust_functions(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut owner: Option<String> = None;
    for line in source.lines() {
        if line.trim_start().starts_with("#[cfg(test)]") {
            break;
        }
        if let Some(rest) = line.strip_prefix("impl") {
            owner = rest
                .split_whitespace()
                .last()
                .filter(|last| *last == "{")
                .and_then(|_| rest.split_whitespace().rev().nth(1))
                .map(|name| name.split('<').next().unwrap_or(name).to_owned());
            continue;
        }
        if line == "}" {
            owner = None;
            continue;
        }
        let mut rest = line.trim_start();
        for prefix in ["pub(crate) ", "pub ", "const "] {
            rest = rest.strip_prefix(prefix).unwrap_or(rest);
        }
        let Some(after) = rest.strip_prefix("fn ") else {
            continue;
        };
        let name: String = after
            .chars()
            .take_while(|&c| c.is_alphanumeric() || c == '_')
            .collect();
        if name.is_empty() {
            continue;
        }
        found.push(match &owner {
            Some(owner) => format!("{owner}::{name}"),
            None => name,
        });
    }
    found
}

/// The `(kernel function, public wrapper)` pairs `proof3.toml` lists.
fn wrappers(config: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let mut kernel: Option<String> = None;
    for line in config.lines() {
        let line = line.trim();
        if let Some(value) = quoted(line, "rust") {
            kernel = value
                .strip_prefix("axiom_core::kernel::")
                .map(str::to_owned);
        } else if let Some(value) = quoted(line, "public")
            && let Some(name) = kernel.take()
        {
            pairs.push((name, value.to_owned()));
        }
    }
    pairs
}

fn quoted<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.strip_prefix(key)?
        .trim_start()
        .strip_prefix('=')?
        .trim()
        .strip_prefix('"')?
        .strip_suffix('"')
}

fn function_rows(
    src: &Path,
    correctness: &str,
    config: &str,
) -> Result<Vec<FunctionRow>, CoverageError> {
    let statements = theorems(&strip_comments(correctness));
    let covering = |kernel_fn: &str| -> Vec<String> {
        let target = format!("kernel.{kernel_fn}");
        statements
            .iter()
            .filter(|(_, statement)| word_positions(statement, &target).next().is_some())
            .map(|(name, _)| name.clone())
            .collect()
    };
    let wrappers = wrappers(config);

    let mut modules = Vec::new();
    for entry in std::fs::read_dir(src).map_err(|error| io(CoverageRole::RustSource, &error))? {
        let path = entry
            .map_err(|error| io(CoverageRole::RustSource, &error))?
            .path();
        let Some(module) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        if path.extension().is_some_and(|ext| ext == "rs") && module != "lib" {
            modules.push((module.to_owned(), path.clone()));
        }
    }
    modules.sort();

    let mut rows = Vec::new();
    for (module, path) in modules {
        let source = read(&path, CoverageRole::RustSource)?;
        for function in rust_functions(&source) {
            let wrapped = wrappers
                .iter()
                .find(|(_, public)| *public == function)
                .map(|(kernel_fn, _)| covering(kernel_fn));
            let (status, theorems) = if module == "kernel" {
                let found = covering(&function);
                if found.is_empty() {
                    (FunctionStatus::Uncovered, found)
                } else {
                    (FunctionStatus::Proved, found)
                }
            } else if module == "exact_rational" {
                (FunctionStatus::TrustedModel, Vec::new())
            } else if let Some(found) = wrapped.filter(|found| !found.is_empty()) {
                (FunctionStatus::ViaKernel, found)
            } else {
                (FunctionStatus::Uncovered, Vec::new())
            };
            rows.push(FunctionRow {
                module: module.clone(),
                function,
                status,
                theorems,
            });
        }
    }
    Ok(rows)
}

fn csv_field(text: &str) -> String {
    if text.contains([',', '"', '\n']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_owned()
    }
}

/// The report as CSV: one header and one row per claim kind, Lean project
/// and engine function, told apart by the `section` column.
#[must_use]
pub fn csv(coverage: &Coverage) -> String {
    let mut out = String::from("section,item,status,lean,detail\n");
    let mut row = |fields: [&str; 5]| {
        let line: Vec<String> = fields.iter().map(|field| csv_field(field)).collect();
        out.push_str(&line.join(","));
        out.push('\n');
    };
    for kind in &coverage.kinds {
        let status = if kind.proved {
            "proved"
        } else {
            "computation only"
        };
        row(["kind", kind.kind, status, kind.theorem, kind.scope]);
    }
    for project in &coverage.projects {
        let detail = format!(
            "{} files; {} sorry; {} declared axioms; enforced by make {}",
            project.files, project.sorry, project.declared_axioms, project.audit
        );
        row([
            "project",
            project.project,
            if project.sorry == 0 {
                "no sorry"
            } else {
                "sorry"
            },
            &project.allowed_axioms.join(" "),
            &detail,
        ]);
    }
    for function in &coverage.functions {
        let item = format!("{}::{}", function.module, function.function);
        row([
            "function",
            &item,
            &function.status.to_string(),
            &function.theorems.join(" "),
            "",
        ]);
    }
    out
}

/// The report as Markdown.
#[must_use]
pub fn markdown(coverage: &Coverage) -> String {
    let mut out = String::from(
        "# Verification coverage\n\n\
         Generated by `make coverage` from the Lean and Rust sources. \
         Generation fails if any `sorry` is found outside comments. \
         The allowed axioms are those each kernel audit permits, and the \
         audit fails on any other axiom.\n\n\
         ## Claim kinds\n\n\
         | Kind | Checker | Soundness | Theorem | What it proves |\n\
         |---|---|---|---|---|\n",
    );
    for kind in &coverage.kinds {
        let status = if kind.proved {
            "proved"
        } else {
            "computation only"
        };
        let _ = writeln!(
            out,
            "| {} | `{}` | {} | `{}` | {} |",
            kind.kind, kind.checker, status, kind.theorem, kind.scope
        );
    }
    let proved = coverage.kinds.iter().filter(|kind| kind.proved).count();
    let _ = write!(
        out,
        "\n{proved} of {} claim kinds have a proved soundness theorem.\n\n\
         ## Sorry and axioms\n\n\
         | Project | Lean files | Sorry | Declared axioms | Allowed axioms | Audit |\n\
         |---|---|---|---|---|---|\n",
        coverage.kinds.len()
    );
    for project in &coverage.projects {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | `make {}` |",
            project.project,
            project.files,
            project.sorry,
            project.declared_axioms,
            project.allowed_axioms.join(", "),
            project.audit
        );
    }
    let _ = write!(
        out,
        "\n## Implementation coverage\n\n\
         `proved`: the Lean translation has a correctness theorem in \
         `lean-engine/AxiomEngine/Correctness.lean`. `via kernel`: the function \
         delegates to a proved kernel function, but the conversion around the \
         call is not translated. `trusted model`: proof 3 models it as `Rat`.\n\n\
         {} proved, {} via kernel, {} trusted model, {} uncovered, of {} functions.\n\n\
         | Module | Function | Status | Theorems |\n\
         |---|---|---|---|\n",
        coverage.count(FunctionStatus::Proved),
        coverage.count(FunctionStatus::ViaKernel),
        coverage.count(FunctionStatus::TrustedModel),
        coverage.count(FunctionStatus::Uncovered),
        coverage.functions.len()
    );
    for function in &coverage.functions {
        let theorems: Vec<String> = function
            .theorems
            .iter()
            .map(|name| format!("`{name}`"))
            .collect();
        let _ = writeln!(
            out,
            "| {} | `{}` | {} | {} |",
            function.module,
            function.function,
            function.status,
            theorems.join(", ")
        );
    }
    out
}
