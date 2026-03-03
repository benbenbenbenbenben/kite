use rust_sitter::{Rule, Spanned};

#[derive(Debug, Rule)]
#[language]
#[extras(re(r"\s+"), re(r"//[^\n]*"))]
pub struct Program {
    pub contexts: Vec<Context>,
}

#[derive(Debug, Rule)]
pub struct Context {
    #[text("context")]
    _context_kw: (),
    pub name: Spanned<Identifier>,
    #[text("{")]
    _open: (),
    pub elements: Vec<ContextElement>,
    #[text("}")]
    _close: (),
}

#[derive(Debug, Rule)]
pub enum ContextElement {
    Dictionary(Dictionary),
    Boundary(Boundary),
    Aggregate(Aggregate),
}

#[derive(Debug, Rule)]
pub struct Dictionary {
    #[text("dictionary")]
    _dictionary_kw: (),
    #[text("{")]
    _open: (),
    pub entries: Vec<DictEntry>,
    #[text("}")]
    _close: (),
}

#[derive(Debug, Rule)]
pub struct Boundary {
    #[text("boundary")]
    _boundary_kw: (),
    #[text("{")]
    _open: (),
    pub entries: Vec<BoundaryEntry>,
    #[text("}")]
    _close: (),
}

#[derive(Debug, Rule)]
pub struct BoundaryEntry {
    #[text("forbid")]
    _forbid_kw: (),
    pub context: Spanned<Identifier>,
}

#[derive(Debug, Rule)]
pub struct DictEntry {
    pub key: Spanned<StringLiteral>,
    #[text("=>")]
    _arrow: (),
    pub value: DictValue,
}

#[derive(Debug, Rule)]
pub enum DictValue {
    Text(StringLiteral),
    #[leaf("forbidden")]
    Forbidden,
}

#[derive(Debug, Rule)]
pub struct Aggregate {
    #[text("aggregate")]
    _aggregate_kw: (),
    pub name: Spanned<Identifier>,
    pub binding: Option<Binding>,
    #[text("{")]
    _open: (),
    pub members: Vec<AggregateMember>,
    #[text("}")]
    _close: (),
}

#[derive(Debug, Rule)]
pub enum AggregateMember {
    Field(Field),
    Command(Command),
    Invariant(Invariant),
}

#[derive(Debug, Rule)]
pub struct Field {
    pub name: Identifier,
    #[text(":")]
    _colon: (),
    pub ty: TypeRef,
}

#[derive(Debug, Rule)]
pub enum TypeRef {
    Primitive(PrimitiveType),
    Named(Identifier),
}

#[derive(Debug, Rule)]
pub enum PrimitiveType {
    #[leaf("String")]
    String,
    #[leaf("Int")]
    Int,
    #[leaf("Decimal")]
    Decimal,
    #[leaf("Boolean")]
    Boolean,
    #[leaf("Date")]
    Date,
    #[leaf("Timestamp")]
    Timestamp,
    #[leaf("Void")]
    Void,
}

#[derive(Debug, Rule)]
pub struct Command {
    #[text("command")]
    _command_kw: (),
    pub name: Spanned<Identifier>,
    #[text("(")]
    _open: (),
    #[sep_by(",")]
    pub params: Vec<Parameter>,
    #[text(")")]
    _close: (),
    pub body: RuleBody,
}

#[derive(Debug, Rule)]
pub struct Invariant {
    #[text("invariant")]
    _invariant_kw: (),
    pub name: Spanned<Identifier>,
    pub body: RuleBody,
}

#[derive(Debug, Rule)]
pub enum RuleBody {
    Binding(Binding),
    Block(Block),
}

#[derive(Debug, Rule)]
pub struct Binding {
    #[text("bound")]
    _bound_kw: (),
    #[text("to")]
    _to_kw: (),
    pub target: Spanned<StringLiteral>,
    pub symbol: Option<BindingSymbol>,
    pub hash: Option<BindingHash>,
}

#[derive(Debug, Rule)]
pub struct BindingSymbol {
    #[text("symbol")]
    _symbol_kw: (),
    pub symbol: Spanned<StringLiteral>,
}

#[derive(Debug, Rule)]
pub struct BindingHash {
    #[text("hash")]
    _hash_kw: (),
    pub hash: Spanned<StringLiteral>,
}

#[derive(Debug, Rule)]
pub struct Parameter {
    pub name: Identifier,
    #[text(":")]
    _colon: (),
    pub ty: TypeRef,
}

#[derive(Debug, Rule)]
pub struct Block {
    #[text("{")]
    _open: (),
    pub fragments: Vec<BlockFragment>,
    #[text("}")]
    _close: (),
}

#[derive(Debug, Rule)]
pub struct BlockFragment {
    #[leaf(re(r#"[^{}]+"#))]
    pub text: String,
}

#[derive(Debug, Rule)]
pub struct Identifier {
    #[leaf(re(r"[a-zA-Z_][a-zA-Z0-9_]*"))]
    pub text: String,
}

#[derive(Debug, Rule)]
pub struct StringLiteral {
    #[leaf(re(r#""([^"\\]|\\.)*""#))]
    pub text: String,
}
