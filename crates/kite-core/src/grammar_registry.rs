use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct BoundaryReferenceQuery {
    pub source: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QueriesManifest {
    pub symbol_exists: Option<String>,
    pub boundary_references: Option<BoundaryReferenceQuery>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct GrammarManifest {
    pub language: String,
    pub version: Option<String>,
    pub wasm_file: Option<String>,
    pub tsx_wasm_file: Option<String>,
    pub extensions: Option<Vec<String>>,
    pub display_name: Option<String>,
    pub queries: Option<QueriesManifest>,
}

#[derive(Debug)]
pub struct LoadedGrammar {
    pub root: PathBuf,
    pub manifest: GrammarManifest,
}

#[derive(Debug, Default)]
pub struct GrammarRegistry {
    grammars: HashMap<String, LoadedGrammar>,
}

impl GrammarRegistry {
    pub fn load(grammar_root: &Path) -> Result<Self> {
        let mut grammars = HashMap::new();
        for entry in std::fs::read_dir(grammar_root)
            .with_context(|| format!("failed to read grammar root {}", grammar_root.display()))?
        {
            let entry = entry?;
            let grammar_dir = entry.path();
            if !grammar_dir.is_dir() {
                continue;
            }

            let manifest_path = grammar_dir.join("manifest.toml");
            if !manifest_path.exists() {
                continue;
            }

            let manifest_str = std::fs::read_to_string(&manifest_path)
                .with_context(|| format!("failed to read {}", manifest_path.display()))?;
            let manifest: GrammarManifest = toml::from_str(&manifest_str)
                .with_context(|| format!("failed to parse {}", manifest_path.display()))?;
            grammars.insert(
                manifest.language.clone(),
                LoadedGrammar {
                    root: grammar_dir,
                    manifest,
                },
            );
        }

        Ok(Self { grammars })
    }

    pub fn query_for(&self, language: &str, rule: &str) -> Result<Option<String>> {
        let Some(grammar) = self.grammars.get(language) else {
            return Ok(None);
        };

        let Some(queries) = &grammar.manifest.queries else {
            return Ok(None);
        };

        let query_rel_path = match rule {
            "symbol_exists" => queries.symbol_exists.as_deref(),
            _ => None,
        };

        let Some(query_rel_path) = query_rel_path else {
            return Ok(None);
        };

        let query_path = grammar.root.join(query_rel_path);
        let query = std::fs::read_to_string(&query_path)
            .with_context(|| format!("failed to read query {}", query_path.display()))?;
        Ok(Some(query))
    }

    pub fn boundary_references_query(&self, language: &str) -> Option<String> {
        let grammar = self.grammars.get(language)?;
        grammar
            .manifest
            .queries
            .as_ref()?
            .boundary_references
            .as_ref()
            .map(|brq| brq.source.clone())
    }

    pub fn has_language(&self, language: &str) -> bool {
        self.grammars.contains_key(language)
    }

    pub fn wasm_bytes(&self, language: &str) -> Result<Option<Vec<u8>>> {
        let Some(grammar) = self.grammars.get(language) else {
            return Ok(None);
        };
        let Some(wasm_file) = &grammar.manifest.wasm_file else {
            return Ok(None);
        };
        let wasm_path = grammar.root.join(wasm_file);
        if !wasm_path.exists() {
            return Ok(None);
        }
        let bytes = std::fs::read(&wasm_path)
            .with_context(|| format!("failed to read wasm file {}", wasm_path.display()))?;
        Ok(Some(bytes))
    }

    pub fn tsx_wasm_bytes(&self, language: &str) -> Result<Option<Vec<u8>>> {
        let Some(grammar) = self.grammars.get(language) else {
            return Ok(None);
        };
        let Some(wasm_file) = &grammar.manifest.tsx_wasm_file else {
            return Ok(None);
        };
        let wasm_path = grammar.root.join(wasm_file);
        if !wasm_path.exists() {
            return Ok(None);
        }
        let bytes = std::fs::read(&wasm_path)
            .with_context(|| format!("failed to read tsx wasm file {}", wasm_path.display()))?;
        Ok(Some(bytes))
    }

    pub fn language_for_path<'a>(&'a self, path: &Path) -> Option<&'a str> {
        let extension = path.extension().and_then(|ext| ext.to_str())?;
        for (language, grammar) in &self.grammars {
            let manifest_match = grammar
                .manifest
                .extensions
                .as_deref()
                .is_some_and(|extensions| {
                    extensions
                        .iter()
                        .any(|candidate| candidate.trim_start_matches('.') == extension)
                });
            if manifest_match {
                return Some(language.as_str());
            }
        }
        None
    }

    pub fn display_name(&self, language: &str) -> &str {
        self.grammars
            .get(language)
            .and_then(|g| g.manifest.display_name.as_deref())
            .unwrap_or("bound")
    }
}
