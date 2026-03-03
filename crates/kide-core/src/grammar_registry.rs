use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AdapterRuntime {
    Native,
    Wasm,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct AdapterRuntimeManifest {
    pub backend_kind: String,
    pub module: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct AdapterManifest {
    pub native: AdapterRuntimeManifest,
    pub wasm: Option<AdapterRuntimeManifest>,
    #[serde(default)]
    pub wasm_fallback_to_native: bool,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct GrammarManifest {
    pub language: String,
    pub version: Option<String>,
    pub grammar_dir: Option<String>,
    pub extensions: Option<Vec<String>>,
    pub queries: HashMap<String, String>,
    #[serde(default)]
    pub adapter: Option<AdapterManifest>,
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

        let Some(query_rel_path) = grammar.manifest.queries.get(rule) else {
            return Ok(None);
        };

        let query_path = grammar.root.join(query_rel_path);
        let query = std::fs::read_to_string(&query_path)
            .with_context(|| format!("failed to read query {}", query_path.display()))?;
        Ok(Some(query))
    }

    pub fn has_language(&self, language: &str) -> bool {
        self.grammars.contains_key(language)
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
            if manifest_match || language_matches_default_extension(language, extension) {
                return Some(language.as_str());
            }
        }
        None
    }

    #[allow(dead_code)]
    pub fn adapter_for(
        &self,
        language: &str,
        runtime: AdapterRuntime,
    ) -> Option<&AdapterRuntimeManifest> {
        let adapter = self.grammars.get(language)?.manifest.adapter.as_ref()?;
        match runtime {
            AdapterRuntime::Native => Some(&adapter.native),
            AdapterRuntime::Wasm => adapter.wasm.as_ref().or_else(|| {
                if adapter.wasm_fallback_to_native {
                    Some(&adapter.native)
                } else {
                    None
                }
            }),
        }
    }
}

fn language_matches_default_extension(language: &str, extension: &str) -> bool {
    match language {
        "rust" => extension == "rs",
        "typescript" => extension == "ts" || extension == "tsx",
        "prisma" => extension == "prisma",
        _ => false,
    }
}
