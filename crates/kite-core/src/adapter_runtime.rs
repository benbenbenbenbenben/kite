use crate::grammar_registry::GrammarRegistry;
use crate::ViolationSpan;
use anyhow::Result;
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use crate::wasm_adapter;

#[cfg(target_arch = "wasm32")]
use {serde::de::DeserializeOwned, serde_json::json};

pub struct AdapterRuntimeEngine<'a> {
    grammar_registry: &'a GrammarRegistry,
    #[allow(dead_code)]
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
    }

    pub fn symbol_exists(
        &self,
        language: &str,
        target_path: &Path,
        source: &str,
        symbol: &str,
        query: &str,
    ) -> Result<Option<bool>> {
        let is_tsx = is_tsx_path(target_path);

        #[cfg(not(target_arch = "wasm32"))]
        {
            wasm_adapter::symbol_exists(
                self.grammar_registry,
                language,
                is_tsx,
                source,
                symbol,
                query,
            )
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.js_bridge_call(
                "symbol_exists",
                target_path,
                source,
                Some(symbol),
                Some(query),
            )
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
        let is_tsx = is_tsx_path(target_path);

        #[cfg(not(target_arch = "wasm32"))]
        {
            wasm_adapter::find_symbol_span(
                self.grammar_registry,
                language,
                is_tsx,
                source,
                symbol,
                query,
            )
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.js_bridge_call(
                "find_symbol_span",
                target_path,
                source,
                Some(symbol),
                Some(query),
            )
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
        let is_tsx = is_tsx_path(target_path);

        #[cfg(not(target_arch = "wasm32"))]
        {
            wasm_adapter::nearest_symbol(
                self.grammar_registry,
                language,
                is_tsx,
                source,
                symbol,
                query,
            )
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.js_bridge_call::<Option<String>>(
                "nearest_symbol",
                target_path,
                source,
                Some(symbol),
                Some(query),
            )
            .map(|maybe| maybe.flatten())
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
        let is_tsx = is_tsx_path(target_path);

        #[cfg(not(target_arch = "wasm32"))]
        {
            wasm_adapter::symbol_arity(
                self.grammar_registry,
                language,
                is_tsx,
                source,
                symbol,
                query,
            )
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.js_bridge_call::<Option<usize>>(
                "symbol_arity",
                target_path,
                source,
                Some(symbol),
                Some(query),
            )
            .map(|maybe| maybe.flatten())
        }
    }

    pub fn boundary_references(
        &self,
        language: &str,
        target_path: &Path,
        source: &str,
    ) -> Result<Option<Vec<String>>> {
        let is_tsx = is_tsx_path(target_path);

        #[cfg(not(target_arch = "wasm32"))]
        {
            wasm_adapter::boundary_references(self.grammar_registry, language, is_tsx, source)
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.js_bridge_call("boundary_references", target_path, source, None, None)
        }
    }

    pub fn list_symbols(
        &self,
        language: &str,
        target_path: &Path,
        source: &str,
        query: &str,
    ) -> Result<Vec<String>> {
        let is_tsx = is_tsx_path(target_path);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let symbols = wasm_adapter::captured_symbols(
                self.grammar_registry,
                language,
                is_tsx,
                source,
                query,
            )?;
            Ok(symbols.unwrap_or_default())
        }

        #[cfg(target_arch = "wasm32")]
        {
            let symbols: Option<Vec<String>> =
                self.js_bridge_call("list_symbols", target_path, source, None, Some(query))?;
            Ok(symbols.unwrap_or_default())
        }
    }
}

// JS bridge support for wasm32 targets
#[cfg(target_arch = "wasm32")]
impl<'a> AdapterRuntimeEngine<'a> {
    fn js_bridge_call<T: DeserializeOwned>(
        &self,
        operation: &str,
        target_path: &Path,
        source: &str,
        symbol: Option<&str>,
        query: Option<&str>,
    ) -> Result<Option<T>> {
        let payload = json!({
            "target_path": target_path.display().to_string(),
            "source": source,
            "symbol": symbol,
            "query": query,
        });
        let payload = serde_json::to_string(&payload)?;
        let module = target_path.display().to_string();
        let output = invoke_js_bridge(&module, operation, &payload)?;
        let Some(output) = output else {
            return Ok(None);
        };
        deserialize_call_output(&output).map(Some)
    }
}

#[cfg(target_arch = "wasm32")]
fn deserialize_call_output<T: DeserializeOwned>(output: &str) -> Result<T> {
    use anyhow::Context;
    serde_json::from_str(output)
        .or_else(|_| {
            let wrapped: serde_json::Value = serde_json::from_str(output)?;
            let Some(value) = wrapped.get("value") else {
                anyhow::bail!("adapter output did not contain value")
            };
            Ok(serde_json::from_value(value.clone())?)
        })
        .with_context(|| "failed to decode wasm adapter response".to_owned())
}

#[cfg(target_arch = "wasm32")]
fn invoke_js_bridge(module: &str, operation: &str, payload: &str) -> Result<Option<String>> {
    #[repr(C)]
    struct JsResultBuffer {
        ptr: *mut u8,
        len: usize,
    }

    unsafe extern "C" {
        fn kite_adapter_js_call(
            module_ptr: *const u8,
            module_len: usize,
            operation_ptr: *const u8,
            operation_len: usize,
            payload_ptr: *const u8,
            payload_len: usize,
            out: *mut JsResultBuffer,
        ) -> i32;
        fn kite_adapter_js_free(ptr: *mut u8, len: usize);
    }

    let mut out = JsResultBuffer {
        ptr: std::ptr::null_mut(),
        len: 0,
    };
    let status = unsafe {
        kite_adapter_js_call(
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
    unsafe { kite_adapter_js_free(out.ptr, out.len) };
    Ok(Some(output))
}

fn is_tsx_path(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some("tsx")
}
