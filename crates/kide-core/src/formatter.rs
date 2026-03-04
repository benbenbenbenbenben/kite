use crate::type_ref_name;
use kide_parser::grammar::{AggregateMember, Binding, ContextElement, DictValue, RuleBody};

pub fn format_source(source: &str) -> anyhow::Result<String> {
    let ast = kide_parser::parse(source)?;
    let mut out = String::new();

    for (i, context) in ast.contexts.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        // Preserve comments that appear before this context
        // (We'll do a best-effort approach: emit the formatted AST)
        out.push_str(&format!("context {} {{\n", context.name.text));

        for element in &context.elements {
            match element {
                ContextElement::Dictionary(dict) => {
                    out.push_str("  dictionary {\n");
                    for entry in &dict.entries {
                        let key = &entry.key.text;
                        match &entry.value {
                            DictValue::Forbidden => {
                                out.push_str(&format!("    {} => forbidden\n", key));
                            }
                            DictValue::Text(text) => {
                                out.push_str(&format!("    {} => {}\n", key, text.text));
                            }
                        }
                    }
                    out.push_str("  }\n\n");
                }
                ContextElement::Boundary(boundary) => {
                    out.push_str("  boundary {\n");
                    for entry in &boundary.entries {
                        out.push_str(&format!("    forbid {}\n", entry.context.text));
                    }
                    out.push_str("  }\n\n");
                }
                ContextElement::Aggregate(agg) => {
                    out.push_str(&format!("  aggregate {}", agg.name.text));
                    if let Some(desc) = &agg.description {
                        out.push_str(&format!(" {}", desc.text));
                    }
                    if let Some(binding) = &agg.binding {
                        format_binding(&mut out, binding);
                    }
                    out.push_str(" {\n");

                    for member in &agg.members {
                        match member {
                            AggregateMember::Field(field) => {
                                let ty = type_ref_name(&field.ty);
                                out.push_str(&format!("    {}: {}\n", field.name.text, ty));
                            }
                            AggregateMember::Command(cmd) => {
                                let params: Vec<String> = cmd
                                    .params
                                    .iter()
                                    .map(|p| format!("{}: {}", p.name.text, type_ref_name(&p.ty)))
                                    .collect();
                                out.push_str(&format!(
                                    "    command {}({})",
                                    cmd.name.text,
                                    params.join(", ")
                                ));
                                if let Some(desc) = &cmd.description {
                                    out.push_str(&format!(" {}", desc.text));
                                }
                                format_rule_body(&mut out, &cmd.body);
                                out.push('\n');
                            }
                            AggregateMember::Invariant(inv) => {
                                out.push_str(&format!("    invariant {}", inv.name.text));
                                if let Some(desc) = &inv.description {
                                    out.push_str(&format!(" {}", desc.text));
                                }
                                format_rule_body(&mut out, &inv.body);
                                out.push('\n');
                            }
                        }
                    }
                    out.push_str("  }\n");
                }
            }
        }
        out.push_str("}\n");
    }

    Ok(out)
}

fn format_binding(out: &mut String, binding: &Binding) {
    out.push_str(&format!(" bound to {}", binding.target.text));
    if let Some(sym) = &binding.symbol {
        out.push_str(&format!(" symbol {}", sym.symbol.text));
    }
    if let Some(hash) = &binding.hash {
        out.push_str(&format!(" hash {}", hash.hash.text));
    }
}

fn format_rule_body(out: &mut String, body: &RuleBody) {
    match body {
        RuleBody::Binding(binding) => format_binding(out, binding),
        RuleBody::Block(block) => {
            out.push_str(" {");
            for fragment in &block.fragments {
                out.push_str(&fragment.text);
            }
            out.push('}');
        }
    }
}
