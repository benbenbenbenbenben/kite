mod adapter_runtime;
mod grammar_registry;
mod wasm_adapter;

use adapter_runtime::AdapterRuntimeEngine;
use anyhow::{Context, Result};
use grammar_registry::GrammarRegistry;
use kide_parser::grammar::{
    Aggregate, AggregateMember, Binding, BindingHash, BindingSymbol, Boundary, BoundaryEntry,
    Command, Context as DomainContext, ContextElement, DictEntry, DictValue, Dictionary, Invariant,
    RuleBody,
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeSet, HashSet},
    path::{Path, PathBuf},
};

pub const CODE_BINDING_FILE_NOT_FOUND: &str = "BINDING_FILE_NOT_FOUND";
pub const CODE_BINDING_SYMBOL_UNVERIFIED_DEPENDENCY: &str = "BINDING_SYMBOL_UNVERIFIED_DEPENDENCY";
pub const CODE_BINDING_SYMBOL_NOT_FOUND: &str = "BINDING_SYMBOL_NOT_FOUND";
pub const CODE_BINDING_SYMBOL_UNSUPPORTED_LANGUAGE: &str = "BINDING_SYMBOL_UNSUPPORTED_LANGUAGE";
pub const CODE_BINDING_SYMBOL_QUERY_MISSING: &str = "BINDING_SYMBOL_QUERY_MISSING";
pub const CODE_COMMAND_BINDING_ARITY_MISMATCH: &str = "COMMAND_BINDING_ARITY_MISMATCH";
pub const CODE_COMMAND_BINDING_INTENT_SUSPICIOUS: &str = "COMMAND_BINDING_INTENT_SUSPICIOUS";
pub const CODE_DICTIONARY_TERM_FORBIDDEN: &str = "DICTIONARY_TERM_FORBIDDEN";
pub const CODE_DICTIONARY_TERM_PREFERRED: &str = "DICTIONARY_TERM_PREFERRED";
pub const CODE_DICTIONARY_DUPLICATE_KEY: &str = "DICTIONARY_DUPLICATE_KEY";
pub const CODE_CONTEXT_BOUNDARY_FORBIDDEN: &str = "CONTEXT_BOUNDARY_FORBIDDEN";
pub const CODE_CONTEXT_BOUNDARY_DUPLICATE_FORBID: &str = "CONTEXT_BOUNDARY_DUPLICATE_FORBID";
pub const CODE_CONTEXT_BOUNDARY_SELF_FORBID: &str = "CONTEXT_BOUNDARY_SELF_FORBID";
pub const CODE_BINDING_HASH_INVALID_FORMAT: &str = "BINDING_HASH_INVALID_FORMAT";
pub const CODE_BINDING_HASH_MISMATCH: &str = "BINDING_HASH_MISMATCH";
pub const CODE_BINDING_FILE_EMPTY: &str = "BINDING_FILE_EMPTY";
pub const CODE_BINDING_SYMBOL_MISSING: &str = "BINDING_SYMBOL_MISSING";
pub const DOCS_BINDING_FILE_NOT_FOUND: &str =
    "https://docs.kide.dev/diagnostics/binding-file-not-found";
pub const DOCS_BINDING_SYMBOL_UNVERIFIED_DEPENDENCY: &str =
    "https://docs.kide.dev/diagnostics/binding-symbol-unverified-dependency";
pub const DOCS_BINDING_SYMBOL_NOT_FOUND: &str =
    "https://docs.kide.dev/diagnostics/binding-symbol-not-found";
pub const DOCS_BINDING_SYMBOL_UNSUPPORTED_LANGUAGE: &str =
    "https://docs.kide.dev/diagnostics/binding-symbol-unsupported-language";
pub const DOCS_BINDING_SYMBOL_QUERY_MISSING: &str =
    "https://docs.kide.dev/diagnostics/binding-symbol-query-missing";
pub const DOCS_COMMAND_BINDING_ARITY_MISMATCH: &str =
    "https://docs.kide.dev/diagnostics/command-binding-arity-mismatch";
pub const DOCS_COMMAND_BINDING_INTENT_SUSPICIOUS: &str =
    "https://docs.kide.dev/diagnostics/command-binding-intent-suspicious";
pub const DOCS_DICTIONARY_TERM_FORBIDDEN: &str =
    "https://docs.kide.dev/diagnostics/dictionary-term-forbidden";
pub const DOCS_DICTIONARY_TERM_PREFERRED: &str =
    "https://docs.kide.dev/diagnostics/dictionary-term-preferred";
pub const DOCS_DICTIONARY_DUPLICATE_KEY: &str =
    "https://docs.kide.dev/diagnostics/dictionary-duplicate-key";
pub const DOCS_CONTEXT_BOUNDARY_FORBIDDEN: &str =
    "https://docs.kide.dev/diagnostics/context-boundary-forbidden";
pub const DOCS_CONTEXT_BOUNDARY_DUPLICATE_FORBID: &str =
    "https://docs.kide.dev/diagnostics/context-boundary-duplicate-forbid";
pub const DOCS_CONTEXT_BOUNDARY_SELF_FORBID: &str =
    "https://docs.kide.dev/diagnostics/context-boundary-self-forbid";
pub const DOCS_BINDING_HASH_INVALID_FORMAT: &str =
    "https://docs.kide.dev/diagnostics/binding-hash-invalid-format";
pub const DOCS_BINDING_HASH_MISMATCH: &str =
    "https://docs.kide.dev/diagnostics/binding-hash-mismatch";
pub const DOCS_BINDING_FILE_EMPTY: &str = "https://docs.kide.dev/diagnostics/binding-file-empty";
pub const DOCS_BINDING_SYMBOL_MISSING: &str =
    "https://docs.kide.dev/diagnostics/binding-symbol-missing";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationSeverity {
    Error,
    Warning,
    Information,
}

impl ViolationSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Information => "information",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ViolationSpan {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub severity: ViolationSeverity,
    pub code: &'static str,
    pub message: String,
    pub hint: Option<String>,
    pub docs_uri: Option<&'static str>,
    pub span: Option<ViolationSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckReport {
    pub contexts: usize,
    pub violations: Vec<Violation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionLocation {
    pub file_path: PathBuf,
    pub span: ViolationSpan,
}

impl CheckReport {
    pub fn has_errors(&self) -> bool {
        self.violations
            .iter()
            .any(|violation| violation.severity == ViolationSeverity::Error)
    }
}

pub fn check_source(source: &str) -> Result<CheckReport> {
    check_source_in_dir(source, Path::new("."))
}

pub fn check_source_in_dir(source: &str, base_dir: &Path) -> Result<CheckReport> {
    let ast = kide_parser::parse(source)?;
    let grammar_root = resolve_grammar_root(base_dir)?;
    let grammar_registry = GrammarRegistry::load(&grammar_root)?;
    let adapter_runtime = AdapterRuntimeEngine::new(&grammar_registry, base_dir);
    let violations = validate_program(&ast, base_dir, &grammar_registry, &adapter_runtime)?;
    Ok(CheckReport {
        contexts: ast.contexts.len(),
        violations,
    })
}

pub fn check_file(path: &Path) -> Result<CheckReport> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    check_source_in_dir(&source, base_dir)
}

pub fn definition_at(
    source: &str,
    base_dir: &Path,
    line: u32,
    column: u32,
) -> Result<Option<DefinitionLocation>> {
    let ast = kide_parser::parse(source)?;
    let grammar_root = resolve_grammar_root(base_dir)?;
    let grammar_registry = GrammarRegistry::load(&grammar_root)?;
    let adapter_runtime = AdapterRuntimeEngine::new(&grammar_registry, base_dir);
    let line = line as usize + 1;
    let column = column as usize + 1;

    for binding in bindings_in_program(&ast) {
        let target = unquote(&binding.target.text);
        let target_path = resolve_bound_path(base_dir, &target);

        let target_span = span_from_binding_target(binding);
        if position_in_span(line, column, &target_span) {
            return Ok(Some(DefinitionLocation {
                file_path: target_path,
                span: ViolationSpan {
                    start_line: 1,
                    start_column: 1,
                    end_line: 1,
                    end_column: 1,
                },
            }));
        }

        let Some(symbol_binding) = &binding.symbol else {
            continue;
        };
        let symbol_span = span_from_symbol_binding(symbol_binding);
        if !position_in_span(line, column, &symbol_span) {
            continue;
        }

        if !target_path.exists() {
            return Ok(None);
        }

        let symbol = unquote(&symbol_binding.symbol.text);
        let Some(language) = adapter_runtime.language_for_path(&target_path) else {
            return Ok(None);
        };
        let Some(query) = grammar_registry.query_for(&language, "symbol_exists")? else {
            return Ok(None);
        };
        let target_source = std::fs::read_to_string(&target_path)
            .with_context(|| format!("failed to read {}", target_path.display()))?;
        let symbol_span = adapter_runtime.find_symbol_span(
            &language,
            &target_path,
            &target_source,
            &symbol,
            &query,
        )?;
        if let Some(span) = symbol_span {
            return Ok(Some(DefinitionLocation {
                file_path: target_path,
                span,
            }));
        }
    }

    Ok(None)
}

fn validate_program(
    program: &kide_parser::Program,
    base_dir: &Path,
    grammar_registry: &GrammarRegistry,
    adapter_runtime: &AdapterRuntimeEngine<'_>,
) -> Result<Vec<Violation>> {
    let mut violations = Vec::new();
    for context in &program.contexts {
        validate_context(
            context,
            base_dir,
            grammar_registry,
            adapter_runtime,
            &mut violations,
        )?;
    }
    Ok(violations)
}

fn bindings_in_program(program: &kide_parser::Program) -> Vec<&Binding> {
    let mut bindings = Vec::new();
    for context in &program.contexts {
        bindings.extend(bindings_in_context(context));
    }
    bindings
}

fn bindings_in_context(context: &DomainContext) -> Vec<&Binding> {
    let mut bindings = Vec::new();
    for element in &context.elements {
        let ContextElement::Aggregate(aggregate) = element else {
            continue;
        };
        if let Some(binding) = &aggregate.binding {
            bindings.push(binding);
        }
        for member in &aggregate.members {
            match member {
                AggregateMember::Command(command) => {
                    if let RuleBody::Binding(binding) = &command.body {
                        bindings.push(binding);
                    }
                }
                AggregateMember::Invariant(invariant) => {
                    if let RuleBody::Binding(binding) = &invariant.body {
                        bindings.push(binding);
                    }
                }
                AggregateMember::Field(_) => {}
            }
        }
    }
    bindings
}

struct BoundSource {
    path: PathBuf,
    source: String,
    fallback_span: ViolationSpan,
}

fn validate_context(
    context: &DomainContext,
    base_dir: &Path,
    grammar_registry: &GrammarRegistry,
    adapter_runtime: &AdapterRuntimeEngine<'_>,
    violations: &mut Vec<Violation>,
) -> Result<()> {
    let bound_sources = collect_context_bound_sources(context, base_dir)?;
    for element in &context.elements {
        match element {
            ContextElement::Dictionary(dictionary) => {
                validate_dictionary(context, dictionary, &bound_sources, violations)
            }
            ContextElement::Boundary(boundary) => validate_boundary(
                context,
                boundary,
                &bound_sources,
                adapter_runtime,
                violations,
            ),
            ContextElement::Aggregate(aggregate) => validate_aggregate(
                &aggregate,
                base_dir,
                grammar_registry,
                adapter_runtime,
                violations,
            )?,
        }
    }
    Ok(())
}

fn collect_context_bound_sources(
    context: &DomainContext,
    base_dir: &Path,
) -> Result<Vec<BoundSource>> {
    let mut seen = HashSet::new();
    let mut bound_sources = Vec::new();
    for binding in bindings_in_context(context) {
        let target = unquote(&binding.target.text);
        let target_path = resolve_bound_path(base_dir, &target);
        if !target_path.exists() || !seen.insert(target_path.clone()) {
            continue;
        }
        let source = std::fs::read_to_string(&target_path)
            .with_context(|| format!("failed to read {}", target_path.display()))?;
        bound_sources.push(BoundSource {
            path: target_path,
            source,
            fallback_span: span_from_binding_target(binding),
        });
    }
    Ok(bound_sources)
}

fn validate_dictionary(
    context: &DomainContext,
    dictionary: &Dictionary,
    bound_sources: &[BoundSource],
    violations: &mut Vec<Violation>,
) {
    let mut seen_terms = HashSet::new();
    for entry in &dictionary.entries {
        let term = unquote(&entry.key.text);
        if term.is_empty() {
            continue;
        }
        if !seen_terms.insert(term.clone()) {
            violations.push(Violation {
                severity: ViolationSeverity::Error,
                code: CODE_DICTIONARY_DUPLICATE_KEY,
                message: format!(
                    "dictionary key '{}' appears more than once in the same dictionary block",
                    term
                ),
                hint: Some(format!("remove or merge duplicate key '{}'", term)),
                docs_uri: Some(DOCS_DICTIONARY_DUPLICATE_KEY),
                span: span_for_dictionary_entry(context, entry, None),
            });
        }
        for bound_source in bound_sources {
            if !contains_term_with_word_boundaries(&bound_source.source, &term) {
                continue;
            }
            match &entry.value {
                DictValue::Forbidden => violations.push(Violation {
                    severity: ViolationSeverity::Error,
                    code: CODE_DICTIONARY_TERM_FORBIDDEN,
                    message: format!(
                        "dictionary term '{}' is forbidden but appears in '{}'",
                        term,
                        bound_source.path.display()
                    ),
                    hint: Some(format!("remove '{}' from bound source files", term)),
                    docs_uri: Some(DOCS_DICTIONARY_TERM_FORBIDDEN),
                    span: span_for_dictionary_entry(
                        context,
                        entry,
                        Some(bound_source.fallback_span),
                    ),
                }),
                DictValue::Text(preferred) => {
                    let preferred = unquote(&preferred.text);
                    violations.push(Violation {
                        severity: ViolationSeverity::Warning,
                        code: CODE_DICTIONARY_TERM_PREFERRED,
                        message: format!(
                            "dictionary term '{}' appears in '{}' but preferred term is '{}'",
                            term,
                            bound_source.path.display(),
                            preferred
                        ),
                        hint: Some(format!("use '{}' instead of '{}'", preferred, term)),
                        docs_uri: Some(DOCS_DICTIONARY_TERM_PREFERRED),
                        span: span_for_dictionary_entry(
                            context,
                            entry,
                            Some(bound_source.fallback_span),
                        ),
                    });
                }
            }
        }
    }
}

fn validate_boundary(
    context: &DomainContext,
    boundary: &Boundary,
    bound_sources: &[BoundSource],
    adapter_runtime: &AdapterRuntimeEngine<'_>,
    violations: &mut Vec<Violation>,
) {
    let mut seen_forbidden_contexts = HashSet::new();
    let current_context = context.name.text.as_str();
    for entry in &boundary.entries {
        let forbidden_context = entry.context.text.as_str();
        if forbidden_context.is_empty() {
            continue;
        }
        if !seen_forbidden_contexts.insert(forbidden_context.to_owned()) {
            violations.push(Violation {
                severity: ViolationSeverity::Error,
                code: CODE_CONTEXT_BOUNDARY_DUPLICATE_FORBID,
                message: format!(
                    "boundary forbids context '{}' more than once in the same boundary block",
                    forbidden_context
                ),
                hint: Some(format!(
                    "remove duplicate 'forbid {}' entries in this boundary block",
                    forbidden_context
                )),
                docs_uri: Some(DOCS_CONTEXT_BOUNDARY_DUPLICATE_FORBID),
                span: span_for_boundary_entry(context, entry),
            });
        }
        if forbidden_context == current_context {
            violations.push(Violation {
                severity: ViolationSeverity::Warning,
                code: CODE_CONTEXT_BOUNDARY_SELF_FORBID,
                message: format!(
                    "boundary forbids current context '{}'; this rule is ineffective",
                    current_context
                ),
                hint: Some(format!(
                    "remove 'forbid {}' or replace it with another context name",
                    current_context
                )),
                docs_uri: Some(DOCS_CONTEXT_BOUNDARY_SELF_FORBID),
                span: span_for_boundary_entry(context, entry),
            });
        }
        for bound_source in bound_sources {
            if !has_boundary_forbidden_reference(bound_source, forbidden_context, adapter_runtime) {
                continue;
            }
            violations.push(Violation {
                severity: ViolationSeverity::Error,
                code: CODE_CONTEXT_BOUNDARY_FORBIDDEN,
                message: format!(
                    "boundary forbids context '{}' but it appears in '{}'",
                    forbidden_context,
                    bound_source.path.display()
                ),
                hint: Some(format!(
                    "remove references to '{}' from files bound in this context",
                    forbidden_context
                )),
                docs_uri: Some(DOCS_CONTEXT_BOUNDARY_FORBIDDEN),
                span: span_for_boundary_entry(context, entry),
            });
        }
    }
}

fn has_boundary_forbidden_reference(
    bound_source: &BoundSource,
    forbidden_context: &str,
    adapter_runtime: &AdapterRuntimeEngine<'_>,
) -> bool {
    if forbidden_context.is_empty() {
        return false;
    }
    match dependency_reference_present(bound_source, forbidden_context, adapter_runtime) {
        Some(found) => found,
        None => contains_term_with_word_boundaries(&bound_source.source, forbidden_context),
    }
}

fn dependency_reference_present(
    bound_source: &BoundSource,
    forbidden_context: &str,
    adapter_runtime: &AdapterRuntimeEngine<'_>,
) -> Option<bool> {
    let language = adapter_runtime.language_for_path(&bound_source.path)?;
    let references = adapter_runtime
        .boundary_references(&language, &bound_source.path, &bound_source.source)
        .ok()??;
    Some(
        references
            .iter()
            .any(|reference| contains_term_with_word_boundaries(reference, forbidden_context)),
    )
}

fn span_for_dictionary_entry(
    context: &DomainContext,
    entry: &DictEntry,
    fallback_span: Option<ViolationSpan>,
) -> Option<ViolationSpan> {
    let span = span_from_position(&entry.key.position);
    if span.start_line == 0 && span.end_line == 0 {
        fallback_span.or_else(|| Some(span_from_position(&context.name.position)))
    } else {
        Some(span)
    }
}

fn span_for_boundary_entry(
    context: &DomainContext,
    entry: &BoundaryEntry,
) -> Option<ViolationSpan> {
    let span = span_from_position(&entry.context.position);
    if span.start_line == 0 && span.end_line == 0 {
        Some(span_from_position(&context.name.position))
    } else {
        Some(span)
    }
}

fn contains_term_with_word_boundaries(source: &str, term: &str) -> bool {
    let mut offset = 0;
    while let Some(found) = source[offset..].find(term) {
        let start = offset + found;
        let end = start + term.len();
        if has_word_boundaries(source, start, end) {
            return true;
        }
        let step = source[start..]
            .chars()
            .next()
            .map(|character| character.len_utf8())
            .unwrap_or(1);
        offset = start + step;
    }
    false
}

fn has_word_boundaries(source: &str, start: usize, end: usize) -> bool {
    let left = source[..start].chars().next_back();
    let right = source[end..].chars().next();
    !is_term_character(left) && !is_term_character(right)
}

fn is_term_character(character: Option<char>) -> bool {
    character.is_some_and(|character| character.is_alphanumeric() || character == '_')
}

fn validate_aggregate(
    aggregate: &Aggregate,
    base_dir: &Path,
    grammar_registry: &GrammarRegistry,
    adapter_runtime: &AdapterRuntimeEngine<'_>,
    violations: &mut Vec<Violation>,
) -> Result<()> {
    if let Some(binding) = &aggregate.binding {
        validate_binding(
            binding,
            base_dir,
            grammar_registry,
            adapter_runtime,
            violations,
        )?;
    }

    for member in &aggregate.members {
        match member {
            AggregateMember::Command(command) => validate_command(
                command,
                base_dir,
                grammar_registry,
                adapter_runtime,
                violations,
            )?,
            AggregateMember::Invariant(invariant) => validate_invariant(
                invariant,
                base_dir,
                grammar_registry,
                adapter_runtime,
                violations,
            )?,
            AggregateMember::Field(_) => {}
        }
    }
    Ok(())
}

fn validate_command(
    command: &Command,
    base_dir: &Path,
    grammar_registry: &GrammarRegistry,
    adapter_runtime: &AdapterRuntimeEngine<'_>,
    violations: &mut Vec<Violation>,
) -> Result<()> {
    if let RuleBody::Binding(binding) = &command.body {
        validate_command_binding_intent(command, binding, violations);
        validate_binding(
            binding,
            base_dir,
            grammar_registry,
            adapter_runtime,
            violations,
        )?;
        validate_command_binding_arity(
            command,
            binding,
            base_dir,
            grammar_registry,
            adapter_runtime,
            violations,
        )?;
    }
    Ok(())
}

fn validate_command_binding_intent(
    command: &Command,
    binding: &Binding,
    violations: &mut Vec<Violation>,
) {
    const WRITE_PREFIXES: [&str; 8] = [
        "create", "add", "update", "set", "remove", "delete", "ship", "cancel",
    ];
    const READ_PREFIXES: [&str; 4] = ["get", "list", "find", "read"];

    let Some(symbol_binding) = &binding.symbol else {
        return;
    };
    if !starts_with_any_prefix(&command.name.text, &WRITE_PREFIXES) {
        return;
    }

    let symbol = unquote(&symbol_binding.symbol.text);
    let symbol_leaf = symbol_leaf_name(&symbol);
    if !starts_with_any_prefix(symbol_leaf, &READ_PREFIXES) {
        return;
    }

    violations.push(Violation {
        severity: ViolationSeverity::Warning,
        code: CODE_COMMAND_BINDING_INTENT_SUSPICIOUS,
        message: format!(
            "command '{}' looks write-oriented but bound symbol '{}' looks read-oriented",
            command.name.text, symbol
        ),
        hint: Some(
            "bind this command to a write-oriented symbol or rename the command/symbol so intents match"
                .to_owned(),
        ),
        docs_uri: Some(DOCS_COMMAND_BINDING_INTENT_SUSPICIOUS),
        span: Some(span_from_symbol_binding(symbol_binding)),
    });
}

fn validate_command_binding_arity(
    command: &Command,
    binding: &Binding,
    base_dir: &Path,
    grammar_registry: &GrammarRegistry,
    adapter_runtime: &AdapterRuntimeEngine<'_>,
    violations: &mut Vec<Violation>,
) -> Result<()> {
    let Some(symbol_binding) = &binding.symbol else {
        return Ok(());
    };

    let target = unquote(&binding.target.text);
    let target_path = resolve_bound_path(base_dir, &target);
    if !target_path.exists() {
        return Ok(());
    }

    let Some(language) = adapter_runtime.language_for_path(&target_path) else {
        return Ok(());
    };
    if !grammar_registry.has_language(&language) {
        return Ok(());
    }
    let Some(query) = grammar_registry.query_for(&language, "symbol_exists")? else {
        return Ok(());
    };

    let source = std::fs::read_to_string(&target_path)
        .with_context(|| format!("failed to read {}", target_path.display()))?;
    let symbol = unquote(&symbol_binding.symbol.text);
    let expected_arity =
        adapter_runtime.symbol_arity(&language, &target_path, &source, &symbol, &query)?;
    let Some(expected_arity) = expected_arity else {
        return Ok(());
    };

    let actual_arity = command.params.len();
    if actual_arity == expected_arity {
        return Ok(());
    }
    let language_name = language_display_name_from_registry(grammar_registry, &language);

    violations.push(Violation {
        severity: ViolationSeverity::Error,
        code: CODE_COMMAND_BINDING_ARITY_MISMATCH,
        message: format!(
            "command '{}' declares {} parameter(s), but {} symbol '{}' expects {} parameter(s)",
            command.name.text, actual_arity, language_name, symbol, expected_arity
        ),
        hint: Some(format!(
            "adjust command parameters to {} or bind to a {} symbol that accepts {} parameter(s)",
            expected_arity, language_name, actual_arity
        )),
        docs_uri: Some(DOCS_COMMAND_BINDING_ARITY_MISMATCH),
        span: Some(span_from_symbol_binding(symbol_binding)),
    });

    Ok(())
}

fn validate_invariant(
    invariant: &Invariant,
    base_dir: &Path,
    grammar_registry: &GrammarRegistry,
    adapter_runtime: &AdapterRuntimeEngine<'_>,
    violations: &mut Vec<Violation>,
) -> Result<()> {
    if let RuleBody::Binding(binding) = &invariant.body {
        validate_binding(
            binding,
            base_dir,
            grammar_registry,
            adapter_runtime,
            violations,
        )?;
    }
    Ok(())
}

fn validate_binding(
    binding: &Binding,
    base_dir: &Path,
    grammar_registry: &GrammarRegistry,
    adapter_runtime: &AdapterRuntimeEngine<'_>,
    violations: &mut Vec<Violation>,
) -> Result<()> {
    let target = unquote(&binding.target.text);
    let target_path = resolve_bound_path(base_dir, &target);
    let target_exists = target_path.exists();

    if !target_exists {
        violations.push(Violation {
            severity: ViolationSeverity::Error,
            code: CODE_BINDING_FILE_NOT_FOUND,
            message: format!("bound file '{}' does not exist", target_path.display()),
            hint: Some("create the file or update the bound path".to_owned()),
            docs_uri: Some(DOCS_BINDING_FILE_NOT_FOUND),
            span: Some(span_from_binding_target(binding)),
        });
    } else if let Ok(content) = std::fs::read_to_string(&target_path) {
        if content.trim().is_empty() {
            violations.push(Violation {
                severity: ViolationSeverity::Warning,
                code: CODE_BINDING_FILE_EMPTY,
                message: format!(
                    "bound file '{}' exists but is empty; implementation may be missing",
                    target_path.display()
                ),
                hint: Some("add implementation to the bound file or remove the binding".to_owned()),
                docs_uri: Some(DOCS_BINDING_FILE_EMPTY),
                span: Some(span_from_binding_target(binding)),
            });
        }
    }

    validate_binding_hash(binding, &target_path, target_exists, violations)?;

    if binding.symbol.is_none() {
        violations.push(Violation {
            severity: ViolationSeverity::Information,
            code: CODE_BINDING_SYMBOL_MISSING,
            message: format!(
                "binding to '{}' has no symbol clause; consider adding one for precise verification",
                target_path.display()
            ),
            hint: Some("add a 'symbol' clause to bind to a specific declaration".to_owned()),
            docs_uri: Some(DOCS_BINDING_SYMBOL_MISSING),
            span: Some(span_from_binding_target(binding)),
        });
    }

    if let Some(symbol_binding) = &binding.symbol {
        let symbol = unquote(&symbol_binding.symbol.text);

        if !target_exists {
            violations.push(Violation {
                severity: ViolationSeverity::Warning,
                code: CODE_BINDING_SYMBOL_UNVERIFIED_DEPENDENCY,
                message: format!(
                    "symbol '{}' could not be verified because bound file '{}' is missing",
                    symbol,
                    target_path.display()
                ),
                hint: Some("fix the missing bound file first, then re-run verification".to_owned()),
                docs_uri: Some(DOCS_BINDING_SYMBOL_UNVERIFIED_DEPENDENCY),
                span: Some(span_from_symbol_binding(symbol_binding)),
            });
            return Ok(());
        }

        let language = adapter_runtime.language_for_path(&target_path);
        let Some(language) = language else {
            violations.push(Violation {
                severity: ViolationSeverity::Warning,
                code: CODE_BINDING_SYMBOL_UNSUPPORTED_LANGUAGE,
                message: format!(
                    "symbol '{}' cannot be verified for unsupported file '{}'",
                    symbol,
                    target_path.display()
                ),
                hint: Some("use a supported language file or remove the symbol clause".to_owned()),
                docs_uri: Some(DOCS_BINDING_SYMBOL_UNSUPPORTED_LANGUAGE),
                span: Some(span_from_symbol_binding(symbol_binding)),
            });
            return Ok(());
        };

        if !grammar_registry.has_language(&language) {
            violations.push(Violation {
                severity: ViolationSeverity::Warning,
                code: CODE_BINDING_SYMBOL_UNSUPPORTED_LANGUAGE,
                message: format!(
                    "symbol '{}' cannot be verified because grammar '{}' is not registered",
                    symbol, language
                ),
                hint: Some("register the language grammar in grammars/grammars.toml".to_owned()),
                docs_uri: Some(DOCS_BINDING_SYMBOL_UNSUPPORTED_LANGUAGE),
                span: Some(span_from_symbol_binding(symbol_binding)),
            });
            return Ok(());
        }

        let query = grammar_registry.query_for(&language, "symbol_exists")?;
        let Some(query) = query else {
            violations.push(Violation {
                severity: ViolationSeverity::Warning,
                code: CODE_BINDING_SYMBOL_QUERY_MISSING,
                message: format!(
                    "symbol '{}' cannot be verified because query 'symbol_exists' is missing for '{}'",
                    symbol, language
                ),
                hint: Some("add a 'symbol_exists' query to the language grammar".to_owned()),
                docs_uri: Some(DOCS_BINDING_SYMBOL_QUERY_MISSING),
                span: Some(span_from_symbol_binding(symbol_binding)),
            });
            return Ok(());
        };

        let source = std::fs::read_to_string(&target_path)
            .with_context(|| format!("failed to read {}", target_path.display()))?;
        let symbol_found =
            adapter_runtime.symbol_exists(&language, &target_path, &source, &symbol, &query)?;
        let Some(symbol_found) = symbol_found else {
            violations.push(Violation {
                severity: ViolationSeverity::Warning,
                code: CODE_BINDING_SYMBOL_UNSUPPORTED_LANGUAGE,
                message: format!(
                    "symbol '{}' cannot be verified because adapter runtime is unavailable for '{}'",
                    symbol, language
                ),
                hint: Some("configure a builtin adapter or a reachable wasm adapter".to_owned()),
                docs_uri: Some(DOCS_BINDING_SYMBOL_UNSUPPORTED_LANGUAGE),
                span: Some(span_from_symbol_binding(symbol_binding)),
            });
            return Ok(());
        };

        if !symbol_found {
            let hint = nearest_symbol_hint(
                adapter_runtime,
                &language,
                &target_path,
                &source,
                &symbol,
                &query,
            )?
            .unwrap_or_else(|| {
                "check the symbol name and ensure it is declared in the bound file".to_owned()
            });
            violations.push(Violation {
                severity: ViolationSeverity::Error,
                code: CODE_BINDING_SYMBOL_NOT_FOUND,
                message: format!(
                    "symbol '{}' was not found in '{}'",
                    symbol,
                    target_path.display()
                ),
                hint: Some(hint),
                docs_uri: Some(DOCS_BINDING_SYMBOL_NOT_FOUND),
                span: Some(span_from_symbol_binding(symbol_binding)),
            });
        }
    }

    Ok(())
}

fn validate_binding_hash(
    binding: &Binding,
    target_path: &Path,
    target_exists: bool,
    violations: &mut Vec<Violation>,
) -> Result<()> {
    let Some(hash_binding) = &binding.hash else {
        return Ok(());
    };

    let expected_hash = unquote(&hash_binding.hash.text);
    if !is_valid_sha256_hex(&expected_hash) {
        violations.push(Violation {
            severity: ViolationSeverity::Error,
            code: CODE_BINDING_HASH_INVALID_FORMAT,
            message: format!(
                "hash '{}' is invalid; expected lowercase SHA-256 hex (64 characters)",
                expected_hash
            ),
            hint: Some("use format 'hash \"<64 lowercase hex SHA-256>\"'".to_owned()),
            docs_uri: Some(DOCS_BINDING_HASH_INVALID_FORMAT),
            span: span_for_hash_binding(binding, hash_binding),
        });
        return Ok(());
    }

    if !target_exists {
        return Ok(());
    }

    let source = std::fs::read_to_string(target_path)
        .with_context(|| format!("failed to read {}", target_path.display()))?;
    let actual_hash = sha256_hex(&source);
    if actual_hash == expected_hash {
        return Ok(());
    }

    violations.push(Violation {
        severity: ViolationSeverity::Error,
        code: CODE_BINDING_HASH_MISMATCH,
        message: format!(
            "hash mismatch for '{}': expected '{}', computed '{}'",
            target_path.display(),
            expected_hash,
            actual_hash
        ),
        hint: Some(
            "update the hash clause to the current file SHA-256 or restore file contents"
                .to_owned(),
        ),
        docs_uri: Some(DOCS_BINDING_HASH_MISMATCH),
        span: span_for_hash_binding(binding, hash_binding),
    });
    Ok(())
}

fn is_valid_sha256_hex(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256_hex(source: &str) -> String {
    format!("{:x}", Sha256::digest(source.as_bytes()))
}

fn span_from_binding_target(binding: &Binding) -> ViolationSpan {
    span_from_position(&binding.target.position)
}

fn span_for_hash_binding(binding: &Binding, hash_binding: &BindingHash) -> Option<ViolationSpan> {
    let span = span_from_hash_binding(hash_binding);
    if span.start_line == 0 && span.end_line == 0 {
        Some(span_from_binding_target(binding))
    } else {
        Some(span)
    }
}

fn nearest_symbol_hint(
    adapter_runtime: &AdapterRuntimeEngine<'_>,
    language: &str,
    target_path: &Path,
    source: &str,
    symbol: &str,
    query: &str,
) -> Result<Option<String>> {
    let nearest = adapter_runtime.nearest_symbol(language, target_path, source, symbol, query)?;
    Ok(nearest.map(|candidate| format!("did you mean '{}'?", candidate)))
}

fn span_from_symbol_binding(binding: &BindingSymbol) -> ViolationSpan {
    span_from_position(&binding.symbol.position)
}

fn span_from_hash_binding(binding: &BindingHash) -> ViolationSpan {
    span_from_position(&binding.hash.position)
}

fn span_from_position(position: &rust_sitter::Position) -> ViolationSpan {
    ViolationSpan {
        start_line: position.start.line,
        start_column: position.start.column,
        end_line: position.end.line,
        end_column: position.end.column,
    }
}

fn position_in_span(line: usize, column: usize, span: &ViolationSpan) -> bool {
    if line < span.start_line || line > span.end_line {
        return false;
    }
    if line == span.start_line && column < span.start_column {
        return false;
    }
    if line == span.end_line && column > span.end_column {
        return false;
    }
    true
}

fn resolve_bound_path(base_dir: &Path, target: &str) -> PathBuf {
    let target_path = Path::new(target);
    if target_path.is_absolute() {
        target_path.to_path_buf()
    } else {
        base_dir.join(target_path)
    }
}

fn unquote(input: &str) -> String {
    input
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .unwrap_or(input)
        .to_owned()
}

fn symbol_leaf_name(symbol: &str) -> &str {
    symbol
        .rsplit(|character| matches!(character, ':' | '.' | '#'))
        .next()
        .unwrap_or(symbol)
}

fn starts_with_any_prefix(name: &str, prefixes: &[&str]) -> bool {
    let lower_name = name.to_ascii_lowercase();
    prefixes.iter().any(|prefix| lower_name.starts_with(prefix))
}

fn language_display_name_from_registry(
    grammar_registry: &GrammarRegistry,
    language: &str,
) -> String {
    grammar_registry.display_name(language).to_owned()
}

pub(crate) fn nearest_symbol_name(target_symbol: &str, candidates: Vec<String>) -> Option<String> {
    let target_symbol = target_symbol.trim();
    if target_symbol.is_empty() {
        return None;
    }

    let normalized_target = target_symbol.to_lowercase();
    let target_len = normalized_target.chars().count();
    let mut best: Option<(usize, String)> = None;

    for candidate in BTreeSet::from_iter(candidates.into_iter().filter(|name| !name.is_empty())) {
        let normalized_candidate = candidate.to_lowercase();
        let candidate_len = normalized_candidate.chars().count();
        let distance = levenshtein_distance(&normalized_target, &normalized_candidate);
        let threshold = nearest_symbol_distance_threshold(target_len.max(candidate_len));
        if distance > threshold {
            continue;
        }
        match &best {
            Some((best_distance, best_candidate))
                if distance > *best_distance
                    || (distance == *best_distance && candidate >= *best_candidate) => {}
            _ => {
                best = Some((distance, candidate));
            }
        }
    }

    best.map(|(_, candidate)| candidate)
}

fn nearest_symbol_distance_threshold(max_len: usize) -> usize {
    match max_len {
        0..=4 => 2,
        5..=8 => 2,
        _ => 3,
    }
}

fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();

    if a_chars.is_empty() {
        return b_chars.len();
    }
    if b_chars.is_empty() {
        return a_chars.len();
    }

    let mut prev_row: Vec<usize> = (0..=b_chars.len()).collect();
    let mut curr_row = vec![0; b_chars.len() + 1];

    for (i, a_char) in a_chars.iter().enumerate() {
        curr_row[0] = i + 1;
        for (j, b_char) in b_chars.iter().enumerate() {
            let cost = if a_char == b_char { 0 } else { 1 };
            curr_row[j + 1] = (curr_row[j] + 1)
                .min(prev_row[j + 1] + 1)
                .min(prev_row[j] + cost);
        }
        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[b_chars.len()]
}

fn resolve_grammar_root(base_dir: &Path) -> Result<PathBuf> {
    if let Ok(path) = std::env::var("KIDE_GRAMMARS_DIR") {
        let candidate = PathBuf::from(path);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    for candidate in base_dir
        .ancestors()
        .map(|ancestor| ancestor.join("grammars"))
    {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    let workspace_default = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../grammars");
    if workspace_default.exists() {
        return Ok(workspace_default);
    }

    anyhow::bail!("no grammars directory found")
}
