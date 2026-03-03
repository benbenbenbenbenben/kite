use kide_parser::grammar::{
    AggregateMember, ContextElement, DictValue, PrimitiveType, RuleBody, TypeRef,
};

#[test]
fn parses_escaped_string_literals_in_dictionary_and_bindings() {
    let source = r#"
        context Escapes {
          dictionary {
            "key \"with\" slash \\ end" => "value \"with\" slash \\ end"
          }

          aggregate Example bound to "src/\"order\"/impl.rs" hash "sha:\\abc\"xyz" {
            command Sync() bound to "cmd/\"sync\".rs" hash "cmd:\\def\"uvw"
          }
        }
    "#;

    let program = kide_parser::parse(source).expect("expected parser success");
    let context = &program.contexts[0];

    let ContextElement::Dictionary(dictionary) = &context.elements[0] else {
        panic!("expected dictionary")
    };
    assert_eq!(dictionary.entries.len(), 1);
    assert_eq!(
        dictionary.entries[0].key.text,
        r#""key \"with\" slash \\ end""#
    );
    let DictValue::Text(value) = &dictionary.entries[0].value else {
        panic!("expected text dictionary value")
    };
    assert_eq!(value.text, r#""value \"with\" slash \\ end""#);

    let ContextElement::Aggregate(aggregate) = &context.elements[1] else {
        panic!("expected aggregate")
    };
    let binding = aggregate
        .binding
        .as_ref()
        .expect("expected aggregate binding");
    assert_eq!(binding.target.text, r#""src/\"order\"/impl.rs""#);
    assert!(binding.symbol.is_none());
    assert_eq!(
        binding.hash.as_ref().unwrap().hash.text,
        r#""sha:\\abc\"xyz""#
    );

    let AggregateMember::Command(command) = &aggregate.members[0] else {
        panic!("expected command")
    };
    let RuleBody::Binding(command_binding) = &command.body else {
        panic!("expected binding body")
    };
    assert_eq!(command_binding.target.text, r#""cmd/\"sync\".rs""#);
    assert!(command_binding.symbol.is_none());
    assert_eq!(
        command_binding.hash.as_ref().unwrap().hash.text,
        r#""cmd:\\def\"uvw""#
    );
}

#[test]
fn parses_all_primitive_types_in_fields_and_params() {
    let source = r#"
        context Primitives {
          aggregate AllPrimitives {
            a: String
            b: Int
            c: Decimal
            d: Boolean
            e: Date
            f: Timestamp
            g: Void

            command UseAll(
              a: String,
              b: Int,
              c: Decimal,
              d: Boolean,
              e: Date,
              f: Timestamp,
              g: Void
            ) {
              side effects allowed
            }
          }
        }
    "#;

    let program = kide_parser::parse(source).expect("expected parser success");
    let ContextElement::Aggregate(aggregate) = &program.contexts[0].elements[0] else {
        panic!("expected aggregate")
    };

    assert_eq!(aggregate.members.len(), 8);
    let AggregateMember::Field(a) = &aggregate.members[0] else {
        panic!("expected field a")
    };
    assert!(matches!(a.ty, TypeRef::Primitive(PrimitiveType::String)));
    let AggregateMember::Field(b) = &aggregate.members[1] else {
        panic!("expected field b")
    };
    assert!(matches!(b.ty, TypeRef::Primitive(PrimitiveType::Int)));
    let AggregateMember::Field(c) = &aggregate.members[2] else {
        panic!("expected field c")
    };
    assert!(matches!(c.ty, TypeRef::Primitive(PrimitiveType::Decimal)));
    let AggregateMember::Field(d) = &aggregate.members[3] else {
        panic!("expected field d")
    };
    assert!(matches!(d.ty, TypeRef::Primitive(PrimitiveType::Boolean)));
    let AggregateMember::Field(e) = &aggregate.members[4] else {
        panic!("expected field e")
    };
    assert!(matches!(e.ty, TypeRef::Primitive(PrimitiveType::Date)));
    let AggregateMember::Field(f) = &aggregate.members[5] else {
        panic!("expected field f")
    };
    assert!(matches!(f.ty, TypeRef::Primitive(PrimitiveType::Timestamp)));
    let AggregateMember::Field(g) = &aggregate.members[6] else {
        panic!("expected field g")
    };
    assert!(matches!(g.ty, TypeRef::Primitive(PrimitiveType::Void)));

    let AggregateMember::Command(command) = &aggregate.members[7] else {
        panic!("expected command")
    };
    assert_eq!(command.params.len(), 7);
    assert!(matches!(
        command.params[0].ty,
        TypeRef::Primitive(PrimitiveType::String)
    ));
    assert!(matches!(
        command.params[1].ty,
        TypeRef::Primitive(PrimitiveType::Int)
    ));
    assert!(matches!(
        command.params[2].ty,
        TypeRef::Primitive(PrimitiveType::Decimal)
    ));
    assert!(matches!(
        command.params[3].ty,
        TypeRef::Primitive(PrimitiveType::Boolean)
    ));
    assert!(matches!(
        command.params[4].ty,
        TypeRef::Primitive(PrimitiveType::Date)
    ));
    assert!(matches!(
        command.params[5].ty,
        TypeRef::Primitive(PrimitiveType::Timestamp)
    ));
    assert!(matches!(
        command.params[6].ty,
        TypeRef::Primitive(PrimitiveType::Void)
    ));
}

#[test]
fn parses_hash_only_and_symbol_only_bindings() {
    let source = r#"
        context Bindings {
          aggregate HashOnly bound to "src/hash.rs" hash "h-only" {
            command SymbolOnly() bound to "src/symbol.rs" symbol "Type::symbol"
          }
        }
    "#;

    let program = kide_parser::parse(source).expect("expected parser success");
    let ContextElement::Aggregate(aggregate) = &program.contexts[0].elements[0] else {
        panic!("expected aggregate")
    };

    let aggregate_binding = aggregate
        .binding
        .as_ref()
        .expect("expected aggregate binding");
    assert_eq!(aggregate_binding.target.text, r#""src/hash.rs""#);
    assert!(aggregate_binding.symbol.is_none());
    assert_eq!(
        aggregate_binding.hash.as_ref().unwrap().hash.text,
        r#""h-only""#
    );

    let AggregateMember::Command(command) = &aggregate.members[0] else {
        panic!("expected command")
    };
    let RuleBody::Binding(command_binding) = &command.body else {
        panic!("expected binding body")
    };
    assert_eq!(command_binding.target.text, r#""src/symbol.rs""#);
    assert_eq!(
        command_binding.symbol.as_ref().unwrap().symbol.text,
        r#""Type::symbol""#
    );
    assert!(command_binding.hash.is_none());
}

#[test]
fn parses_command_with_empty_parameter_list() {
    let source = r#"
        context EmptyParams {
          aggregate Worker {
            command Ping() bound to "src/ping.rs"
          }
        }
    "#;

    let program = kide_parser::parse(source).expect("expected parser success");
    let ContextElement::Aggregate(aggregate) = &program.contexts[0].elements[0] else {
        panic!("expected aggregate")
    };
    let AggregateMember::Command(command) = &aggregate.members[0] else {
        panic!("expected command")
    };

    assert_eq!(command.name.text, "Ping");
    assert!(command.params.is_empty());
    let RuleBody::Binding(binding) = &command.body else {
        panic!("expected binding body")
    };
    assert_eq!(binding.target.text, r#""src/ping.rs""#);
    assert!(binding.symbol.is_none());
    assert!(binding.hash.is_none());
}

#[test]
fn parses_multiline_block_fragments() {
    let source = r#"
        context Blocks {
          aggregate RuleCarrier {
            invariant MultiLine {
first line of text
second line of text
third line of text
            }
          }
        }
    "#;

    let program = kide_parser::parse(source).expect("expected parser success");
    let ContextElement::Aggregate(aggregate) = &program.contexts[0].elements[0] else {
        panic!("expected aggregate")
    };
    let AggregateMember::Invariant(invariant) = &aggregate.members[0] else {
        panic!("expected invariant")
    };
    let RuleBody::Block(block) = &invariant.body else {
        panic!("expected block body")
    };

    assert_eq!(block.fragments.len(), 1);
    assert_eq!(
        block.fragments[0].text,
        "\nfirst line of text\nsecond line of text\nthird line of text\n            "
    );
}
