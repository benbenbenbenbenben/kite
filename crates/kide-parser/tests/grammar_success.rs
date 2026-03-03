use kide_parser::grammar::{
    AggregateMember, ContextElement, DictValue, PrimitiveType, RuleBody, TypeRef,
};

#[test]
fn parses_happy_path_grammar_features() {
    let source = r#"
        // cmt
        context Sales {
          dictionary {
            "legacy term" => forbidden
            "old term" => "new term"
          }

          boundary {
            forbid Payments
            forbid Legacy
          }

          aggregate Order bound to "src/order.rs" symbol "Order::create" hash "abc123" {
            id: Int
            status: Status

            command CreateOrder(id: Int, status: Status) bound to "src/order.rs" symbol "Order::create" hash "def456"
            invariant MustHaveStatus {
              status must exist
            }
          }

          aggregate Customer bound to "src/customer.rs" {
            name: String
          }
        }
    "#;

    let program = kide_parser::parse(source).expect("expected parser success");
    assert_eq!(program.contexts.len(), 1);

    let context = &program.contexts[0];
    assert_eq!(context.name.text, "Sales");
    assert_eq!(context.elements.len(), 4);

    let ContextElement::Dictionary(dictionary) = &context.elements[0] else {
        panic!("expected dictionary element")
    };
    assert_eq!(dictionary.entries.len(), 2);
    assert_eq!(dictionary.entries[0].key.text, "\"legacy term\"");
    assert!(matches!(dictionary.entries[0].value, DictValue::Forbidden));
    assert_eq!(dictionary.entries[1].key.text, "\"old term\"");
    let DictValue::Text(preferred) = &dictionary.entries[1].value else {
        panic!("expected dictionary preferred text")
    };
    assert_eq!(preferred.text, "\"new term\"");

    let ContextElement::Boundary(boundary) = &context.elements[1] else {
        panic!("expected boundary element")
    };
    assert_eq!(boundary.entries.len(), 2);
    assert_eq!(boundary.entries[0].context.text, "Payments");
    assert_eq!(boundary.entries[1].context.text, "Legacy");

    let ContextElement::Aggregate(order) = &context.elements[2] else {
        panic!("expected aggregate element")
    };
    assert_eq!(order.name.text, "Order");
    let order_binding = order.binding.as_ref().expect("expected aggregate binding");
    assert_eq!(order_binding.target.text, "\"src/order.rs\"");
    let order_symbol = order_binding
        .symbol
        .as_ref()
        .expect("expected aggregate symbol");
    assert_eq!(order_symbol.symbol.text, "\"Order::create\"");
    let order_hash = order_binding
        .hash
        .as_ref()
        .expect("expected aggregate hash");
    assert_eq!(order_hash.hash.text, "\"abc123\"");

    assert_eq!(order.members.len(), 4);
    let AggregateMember::Field(id_field) = &order.members[0] else {
        panic!("expected primitive field")
    };
    assert_eq!(id_field.name.text, "id");
    assert!(matches!(
        id_field.ty,
        TypeRef::Primitive(PrimitiveType::Int)
    ));

    let AggregateMember::Field(status_field) = &order.members[1] else {
        panic!("expected named field")
    };
    assert_eq!(status_field.name.text, "status");
    let TypeRef::Named(status_type) = &status_field.ty else {
        panic!("expected named type")
    };
    assert_eq!(status_type.text, "Status");

    let AggregateMember::Command(command) = &order.members[2] else {
        panic!("expected command member")
    };
    assert_eq!(command.name.text, "CreateOrder");
    assert_eq!(command.params.len(), 2);
    assert_eq!(command.params[0].name.text, "id");
    assert!(matches!(
        command.params[0].ty,
        TypeRef::Primitive(PrimitiveType::Int)
    ));
    assert_eq!(command.params[1].name.text, "status");
    let TypeRef::Named(param_type) = &command.params[1].ty else {
        panic!("expected named command parameter")
    };
    assert_eq!(param_type.text, "Status");
    let RuleBody::Binding(command_binding) = &command.body else {
        panic!("expected command binding body")
    };
    assert_eq!(command_binding.target.text, "\"src/order.rs\"");
    assert_eq!(
        command_binding.symbol.as_ref().unwrap().symbol.text,
        "\"Order::create\""
    );
    assert_eq!(
        command_binding.hash.as_ref().unwrap().hash.text,
        "\"def456\""
    );

    let AggregateMember::Invariant(invariant) = &order.members[3] else {
        panic!("expected invariant member")
    };
    assert_eq!(invariant.name.text, "MustHaveStatus");
    let RuleBody::Block(block) = &invariant.body else {
        panic!("expected invariant block body")
    };
    assert_eq!(block.fragments.len(), 1);
    assert!(block.fragments[0].text.contains("status must exist"));

    let ContextElement::Aggregate(customer) = &context.elements[3] else {
        panic!("expected second aggregate")
    };
    assert_eq!(customer.name.text, "Customer");
    let customer_binding = customer.binding.as_ref().expect("expected binding");
    assert_eq!(customer_binding.target.text, "\"src/customer.rs\"");
    assert!(customer_binding.symbol.is_none());
    assert!(customer_binding.hash.is_none());
    let AggregateMember::Field(name_field) = &customer.members[0] else {
        panic!("expected customer field")
    };
    assert_eq!(name_field.name.text, "name");
    assert!(matches!(
        name_field.ty,
        TypeRef::Primitive(PrimitiveType::String)
    ));
}

#[test]
fn parses_comments_and_whitespace_extras() {
    let source = r#"
        context   Inventory   {
          // cmt
          dictionary   {
            "sku"   =>   "stock keeping unit"   // ok
          }

          aggregate   Item   {
            code: String
          }
        }
    "#;

    let program = kide_parser::parse(source).expect("expected parser success with extras");
    assert_eq!(program.contexts.len(), 1);
    let context = &program.contexts[0];
    assert_eq!(context.name.text, "Inventory");
    assert_eq!(context.elements.len(), 2);
}
