use crate::{
    grammar_registry::{AdapterRuntime as GrammarAdapterRuntime, GrammarRegistry},
    prisma_adapter, rust_adapter, typescript_adapter, ViolationSpan,
};
use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde_json::json;
use std::path::{Path, PathBuf};

pub struct AdapterRuntimeEngine<'a> {
    grammar_registry: &'a GrammarRegistry,
    base_dir: &'a Path,
}

impl<'a> AdapterRuntimeEngine<'a> {
    pub fn new(grammar_registry: &'a GrammarRegistry, base_dir: &'a Path) -> Self {
        Self {
            grammar_registry,
            base_dir,
        }
    }

    pub fn language_for_path(&self, path: &Path) -> Option<String> {
        self.grammar_registry
            .language_for_path(path)
            .map(str::to_owned)
            .or_else(|| match path.extension().and_then(|ext| ext.to_str()) {
                Some("rs") => Some("rust".to_owned()),
                Some("ts") | Some("tsx") => Some("typescript".to_owned()),
                Some("prisma") => Some("prisma".to_owned()),
                _ => None,
            })
    }

    pub fn symbol_exists(
        &self,
        language: &str,
        target_path: &Path,
        source: &str,
        symbol: &str,
        query: &str,
    ) -> Result<Option<bool>> {
        match self.backend_for(language) {
            AdapterBackend::Builtin => self.builtin_symbol_exists(language, target_path, source, symbol, query),
            AdapterBackend::Wasm(config) => config
                .call("symbol_exists", request_payload(target_path, source, Some(symbol), Some(query)))?
                .map(deserialize_call_output::<bool>)
                .transpose(),
        }
    }

    pub fn find_symbol_span(
        &self,
        language: &str,
        target_path: &Path,
        source: &str,
        symbol: &str,
        query: &str,
    ) -> Result<Option<ViolationSpan>> {
        match self.backend_for(language) {
            AdapterBackend::Builtin => {
                self.builtin_find_symbol_span(language, target_path, source, symbol, query)
            }
            AdapterBackend::Wasm(config) => config
                .call(
                    "find_symbol_span",
                    request_payload(target_path, source, Some(symbol), Some(query)),
                )?
                .map(deserialize_call_output::<ViolationSpan>)
                .transpose(),
        }
    }

    pub fn nearest_symbol(
        &self,
        language: &str,
        target_path: &Path,
        source: &str,
        symbol: &str,
        query: &str,
    ) -> Result<Option<String>> {
        match self.backend_for(language) {
            AdapterBackend::Builtin => {
                self.builtin_nearest_symbol(language, target_path, source, symbol, query)
            }
            AdapterBackend::Wasm(config) => config
                .call(
                    "nearest_symbol",
                    request_payload(target_path, source, Some(symbol), Some(query)),
                )?
                .map(deserialize_call_output::<Option<String>>)
                .transpose()
                .map(|maybe| maybe.flatten()),
        }
    }

    pub fn symbol_arity(
        &self,
        language: &str,
        target_path: &Path,
        source: &str,
        symbol: &str,
        query: &str,
    ) -> Result<Option<usize>> {
        match self.backend_for(language) {
            AdapterBackend::Builtin => self.builtin_symbol_arity(language, target_path, source, symbol, query),
            AdapterBackend::Wasm(config) => config
                .call(
                    "symbol_arity",
                    request_payload(target_path, source, Some(symbol), Some(query)),
                )?
                .map(deserialize_call_output::<Option<usize>>)
                .transpose()
                .map(|maybe| maybe.flatten()),
        }
    }

    pub fn boundary_references(
        &self,
        language: &str,
        target_path: &Path,
        source: &str,
    ) -> Result<Option<Vec<String>>> {
        match self.backend_for(language) {
            AdapterBackend::Builtin => self.builtin_boundary_references(language, target_path, source),
            AdapterBackend::Wasm(config) => config
                .call("boundary_references", request_payload(target_path, source, None, None))?
                .map(deserialize_call_output::<Vec<String>>)
                .transpose(),
        }
    }

    fn backend_for(&self, language: &str) -> AdapterBackend {
        #[cfg(not(target_arch = "wasm32"))]
        let runtime = GrammarAdapterRuntime::Native;
        #[cfg(target_arch = "wasm32")]
        let runtime = GrammarAdapterRuntime::Wasm;

        let Some(manifest) = self.grammar_registry.adapter_for(language, runtime) else {
            return AdapterBackend::Builtin;
        };

        if !manifest.backend_kind.eq_ignore_ascii_case("wasm") {
            return AdapterBackend::Builtin;
        }

        let configured_path = PathBuf::from(&manifest.module);
        let module_path = if configured_path.is_absolute() {
            configured_path
        } else {
            self.base_dir.join(configured_path)
        };
        AdapterBackend::Wasm(WasmBackendConfig { module_path })
    }

    fn builtin_symbol_exists(
        &self,
        language: &str,
        target_path: &Path,
        source: &str,
        symbol: &str,
        query: &str,
    ) -> Result<Option<bool>> {
        let value = match language {
            "rust" => Some(rust_adapter::symbol_exists(source, symbol, query)?),
            "typescript" if is_tsx_path(target_path) => {
                Some(typescript_adapter::symbol_exists_tsx(source, symbol, query)?)
            }
            "typescript" => Some(typescript_adapter::symbol_exists(source, symbol, query)?),
            "prisma" => Some(prisma_adapter::symbol_exists(source, symbol, query)?),
            _ => None,
        };
        Ok(value)
    }

    fn builtin_find_symbol_span(
        &self,
        language: &str,
        target_path: &Path,
        source: &str,
        symbol: &str,
        query: &str,
    ) -> Result<Option<ViolationSpan>> {
        let value = match language {
            "rust" => rust_adapter::find_symbol_span(source, symbol, query)?,
            "typescript" if is_tsx_path(target_path) => {
                typescript_adapter::find_symbol_span_tsx(source, symbol, query)?
            }
            "typescript" => typescript_adapter::find_symbol_span(source, symbol, query)?,
            "prisma" => prisma_adapter::find_symbol_span(source, symbol, query)?,
            _ => None,
        };
        Ok(value)
    }

    fn builtin_nearest_symbol(
        &self,
        language: &str,
        target_path: &Path,
        source: &str,
        symbol: &str,
        query: &str,
    ) -> Result<Option<String>> {
        let value = match language {
            "rust" => rust_adapter::nearest_symbol(source, symbol, query)?,
            "typescript" if is_tsx_path(target_path) => {
                typescript_adapter::nearest_symbol_tsx(source, symbol, query)?
            }
            "typescript" => typescript_adapter::nearest_symbol(source, symbol, query)?,
            "prisma" => prisma_adapter::nearest_symbol(source, symbol, query)?,
            _ => None,
        };
        Ok(value)
    }

    fn builtin_symbol_arity(
        &self,
        language: &str,
        target_path: &Path,
        source: &str,
        symbol: &str,
        query: &str,
    ) -> Result<Option<usize>> {
        let value = match language {
            "rust" => rust_adapter::symbol_arity(source, symbol, query)?,
            "typescript" if is_tsx_path(target_path) => {
                typescript_adapter::symbol_arity_tsx(source, symbol, query)?
            }
            "typescript" => typescript_adapter::symbol_arity(source, symbol, query)?,
            "prisma" => prisma_adapter::symbol_arity(source, symbol, query)?,
            _ => None,
        };
        Ok(value)
    }

    fn builtin_boundary_references(
        &self,
        language: &str,
        target_path: &Path,
        source: &str,
    ) -> Result<Option<Vec<String>>> {
        let value = match language {
            "rust" => Some(rust_adapter::boundary_references(source)?),
            "typescript" if is_tsx_path(target_path) => {
                Some(typescript_adapter::boundary_references_tsx(source)?)
            }
            "typescript" => Some(typescript_adapter::boundary_references(source)?),
            _ => None,
        };
        Ok(value)
    }
}

enum AdapterBackend {
    Builtin,
    Wasm(WasmBackendConfig),
}

struct WasmBackendConfig {
    module_path: PathBuf,
}

impl WasmBackendConfig {
    fn call(&self, operation: &str, payload: serde_json::Value) -> Result<Option<String>> {
        let payload = serde_json::to_string(&payload)?;
        if !self.module_path.exists() {
            return Ok(None);
        }
        invoke_wasm_adapter(&self.module_path, operation, &payload)
    }
}

fn request_payload(
    target_path: &Path,
    source: &str,
    symbol: Option<&str>,
    query: Option<&str>,
) -> serde_json::Value {
    json!({
        "target_path": target_path.display().to_string(),
        "source": source,
        "symbol": symbol,
        "query": query,
    })
}

fn deserialize_call_output<T: DeserializeOwned>(output: String) -> Result<T> {
    serde_json::from_str(&output)
        .or_else(|_| {
            let wrapped: serde_json::Value = serde_json::from_str(&output)?;
            let Some(value) = wrapped.get("value") else {
                anyhow::bail!("adapter output did not contain value")
            };
            Ok(serde_json::from_value(value.clone())?)
        })
        .with_context(|| "failed to decode wasm adapter response".to_owned())
}

#[cfg(not(target_arch = "wasm32"))]
fn invoke_wasm_adapter(module_path: &Path, operation: &str, payload: &str) -> Result<Option<String>> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = match Command::new("wasmtime")
        .arg("run")
        .arg(module_path)
        .arg("--")
        .arg(operation)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return Ok(None),
    };

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(payload.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Ok(None);
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if stdout.is_empty() {
        Ok(None)
    } else {
        Ok(Some(stdout))
    }
}

#[cfg(target_arch = "wasm32")]
fn invoke_wasm_adapter(module_path: &Path, operation: &str, payload: &str) -> Result<Option<String>> {
    #[repr(C)]
    struct JsResultBuffer {
        ptr: *mut u8,
        len: usize,
    }

    unsafe extern "C" {
        fn kide_adapter_js_call(
            module_ptr: *const u8,
            module_len: usize,
            operation_ptr: *const u8,
            operation_len: usize,
            payload_ptr: *const u8,
            payload_len: usize,
            out: *mut JsResultBuffer,
        ) -> i32;
        fn kide_adapter_js_free(ptr: *mut u8, len: usize);
    }

    let module = module_path.display().to_string();
    let mut out = JsResultBuffer {
        ptr: std::ptr::null_mut(),
        len: 0,
    };
    let status = unsafe {
        kide_adapter_js_call(
            module.as_ptr(),
            module.len(),
            operation.as_ptr(),
            operation.len(),
            payload.as_ptr(),
            payload.len(),
            &mut out,
        )
    };
    if status != 0 || out.ptr.is_null() || out.len == 0 {
        return Ok(None);
    }

    let bytes = unsafe { std::slice::from_raw_parts(out.ptr, out.len) };
    let output = String::from_utf8_lossy(bytes).to_string();
    unsafe { kide_adapter_js_free(out.ptr, out.len) };
    Ok(Some(output))
}

fn is_tsx_path(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some("tsx")
}
