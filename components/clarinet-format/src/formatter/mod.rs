pub mod helpers;
pub mod ignored;

use std::iter::Peekable;

use clarity::vm::functions::{define::DefineFunctions, NativeFunctions};
use clarity::vm::representations::{PreSymbolicExpression, PreSymbolicExpressionType};
use helpers::{name_and_args, t};
use ignored::ignored_exprs;
use std::fmt;

pub enum Indentation {
    Space(usize),
    Tab,
}

impl fmt::Display for Indentation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Indentation::Space(count) => write!(f, "{}", " ".repeat(*count)),
            Indentation::Tab => write!(f, "\t"),
        }
    }
}

// or/and with > N comparisons will be split across multiple lines
// (or
//   true
//   (is-eq 1 1)
//   false
// )
const BOOLEAN_BREAK_LIMIT: usize = 2;

// commented blocks with this string included will not be formatted
const FORMAT_IGNORE_SYNTAX: &str = "@format-ignore";

pub struct Settings {
    pub indentation: Indentation,
    pub max_line_length: usize,
}

impl Settings {
    pub fn new(indentation: Indentation, max_line_length: usize) -> Self {
        Settings {
            indentation,
            max_line_length,
        }
    }
}
impl Default for Settings {
    fn default() -> Settings {
        Settings {
            indentation: Indentation::Space(2),
            max_line_length: 80,
        }
    }
}

pub struct ClarityFormatter {
    settings: Settings,
}
impl ClarityFormatter {
    pub fn new(settings: Settings) -> Self {
        Self { settings }
    }
    /// formatting for files to ensure a newline at the end
    pub fn format_file(&self, source: &str) -> String {
        let pse = clarity::vm::ast::parser::v2::parse(source).unwrap();
        let agg = Aggregator::new(&self.settings, &pse, source);
        let result = agg.generate();

        // make sure the file ends with a newline
        result.trim_end_matches('\n').to_string() + "\n"
    }
    /// Alias `format_file` to `format`
    pub fn format(&self, source: &str) -> String {
        self.format_file(source)
    }
    /// for range formatting within editors
    pub fn format_section(&self, source: &str) -> String {
        let pse = clarity::vm::ast::parser::v2::parse(source).unwrap();
        // TODO: range formatting should specify to the aggregator that we're
        // starting mid-source and thus should pre-populate
        // `previous_indentation` for format_source_exprs
        let agg = Aggregator::new(&self.settings, &pse, source);
        agg.generate()
    }
}
/// Aggregator does the heavy lifting and generates the final output string.
/// all the formatting methods live within this struct.
pub struct Aggregator<'a> {
    settings: &'a Settings,
    pse: &'a [PreSymbolicExpression],
    source: &'a str,
}

impl<'a> Aggregator<'a> {
    pub fn new(settings: &'a Settings, pse: &'a [PreSymbolicExpression], source: &'a str) -> Self {
        Aggregator {
            settings,
            pse,
            source,
        }
    }
    pub fn generate(&self) -> String {
        self.format_source_exprs(self.pse, "")
    }
    fn format_source_exprs(
        &self,
        expressions: &[PreSymbolicExpression],
        previous_indentation: &str,
    ) -> String {
        // use peekable to handle trailing comments nicely
        let mut iter = expressions.iter().peekable();
        let mut result = "".to_owned(); // Accumulate results here

        while let Some(expr) = iter.next() {
            let trailing_comment = get_trailing_comment(expr, &mut iter);
            let cur = self.display_pse(expr, previous_indentation);
            if cur.contains(FORMAT_IGNORE_SYNTAX) {
                result.push_str(&cur);
                if let Some(next) = iter.peek() {
                    if let Some(block) = next.match_list() {
                        iter.next();
                        result.push('\n');
                        result.push_str(&ignored_exprs(block, self.source));
                    }
                }
                continue;
            }
            if let Some(list) = expr.match_list() {
                if let Some(atom_name) = list.split_first().and_then(|(f, _)| f.match_atom()) {
                    let formatted = if let Some(native) = NativeFunctions::lookup_by_name(atom_name)
                    {
                        match native {
                            NativeFunctions::Let => self.format_let(list, previous_indentation),
                            NativeFunctions::Begin => self.format_begin(list, previous_indentation),
                            NativeFunctions::Match => self.format_match(list, previous_indentation),
                            NativeFunctions::TupleCons => {
                                // if the kv map is defined with (tuple (c 1)) then we strip the
                                // ClarityName("tuple") out first and convert it to key/value syntax
                                self.format_key_value(&list[1..], previous_indentation)
                            }
                            NativeFunctions::If => self.format_if(list, previous_indentation),
                            NativeFunctions::And | NativeFunctions::Or => {
                                self.format_booleans(list, previous_indentation)
                            }
                            // everything else that's not special cased
                            NativeFunctions::Add
                            | NativeFunctions::Subtract
                            | NativeFunctions::Multiply
                            | NativeFunctions::Divide
                            | NativeFunctions::CmpGeq
                            | NativeFunctions::CmpLeq
                            | NativeFunctions::CmpLess
                            | NativeFunctions::CmpGreater
                            | NativeFunctions::ToInt
                            | NativeFunctions::ToUInt
                            | NativeFunctions::Modulo
                            | NativeFunctions::Power
                            | NativeFunctions::Sqrti
                            | NativeFunctions::Log2
                            | NativeFunctions::BitwiseXor
                            | NativeFunctions::Not
                            | NativeFunctions::Equals
                            | NativeFunctions::Map
                            | NativeFunctions::Fold
                            | NativeFunctions::Append
                            | NativeFunctions::Concat
                            | NativeFunctions::AsMaxLen
                            | NativeFunctions::Len
                            | NativeFunctions::ElementAt
                            | NativeFunctions::ElementAtAlias
                            | NativeFunctions::IndexOf
                            | NativeFunctions::IndexOfAlias
                            | NativeFunctions::BuffToIntLe
                            | NativeFunctions::BuffToUIntLe
                            | NativeFunctions::BuffToIntBe
                            | NativeFunctions::BuffToUIntBe
                            | NativeFunctions::IsStandard
                            | NativeFunctions::PrincipalDestruct
                            | NativeFunctions::PrincipalConstruct
                            | NativeFunctions::StringToInt
                            | NativeFunctions::StringToUInt
                            | NativeFunctions::IntToAscii
                            | NativeFunctions::IntToUtf8
                            | NativeFunctions::ListCons
                            | NativeFunctions::FetchVar
                            | NativeFunctions::SetVar
                            | NativeFunctions::FetchEntry
                            | NativeFunctions::SetEntry
                            | NativeFunctions::InsertEntry
                            | NativeFunctions::DeleteEntry
                            | NativeFunctions::TupleGet
                            | NativeFunctions::TupleMerge
                            | NativeFunctions::Hash160
                            | NativeFunctions::Sha256
                            | NativeFunctions::Sha512
                            | NativeFunctions::Sha512Trunc256
                            | NativeFunctions::Keccak256
                            | NativeFunctions::Secp256k1Recover
                            | NativeFunctions::Secp256k1Verify
                            | NativeFunctions::Print
                            | NativeFunctions::ContractCall
                            | NativeFunctions::AsContract
                            | NativeFunctions::ContractOf
                            | NativeFunctions::PrincipalOf
                            | NativeFunctions::AtBlock
                            | NativeFunctions::GetBlockInfo
                            | NativeFunctions::GetBurnBlockInfo
                            | NativeFunctions::ConsError
                            | NativeFunctions::ConsOkay
                            | NativeFunctions::ConsSome
                            | NativeFunctions::DefaultTo
                            | NativeFunctions::Asserts
                            | NativeFunctions::UnwrapRet
                            | NativeFunctions::UnwrapErrRet
                            | NativeFunctions::Unwrap
                            | NativeFunctions::UnwrapErr
                            | NativeFunctions::TryRet
                            | NativeFunctions::IsOkay
                            | NativeFunctions::IsNone
                            | NativeFunctions::IsErr
                            | NativeFunctions::IsSome
                            | NativeFunctions::Filter
                            | NativeFunctions::GetTokenBalance
                            | NativeFunctions::GetAssetOwner
                            | NativeFunctions::TransferToken
                            | NativeFunctions::TransferAsset
                            | NativeFunctions::MintAsset
                            | NativeFunctions::MintToken
                            | NativeFunctions::GetTokenSupply
                            | NativeFunctions::BurnToken
                            | NativeFunctions::BurnAsset
                            | NativeFunctions::GetStxBalance
                            | NativeFunctions::StxTransfer
                            | NativeFunctions::StxTransferMemo
                            | NativeFunctions::StxBurn
                            | NativeFunctions::StxGetAccount
                            | NativeFunctions::BitwiseAnd
                            | NativeFunctions::BitwiseOr
                            | NativeFunctions::BitwiseNot
                            | NativeFunctions::BitwiseLShift
                            | NativeFunctions::BitwiseRShift
                            | NativeFunctions::BitwiseXor2
                            | NativeFunctions::Slice
                            | NativeFunctions::ToConsensusBuff
                            | NativeFunctions::FromConsensusBuff
                            | NativeFunctions::ReplaceAt
                            | NativeFunctions::GetStacksBlockInfo
                            | NativeFunctions::GetTenureInfo => {
                                let inner_content =
                                    self.to_inner_content(list, previous_indentation);

                                // There's an annoying thing happening here
                                // Since we do expr.match_list() up above we only have the contents
                                // We should ideally be able to format!("{}", format_source_exprs(.., &[expr.clone()]))
                                // but that stack overflows so we manually print out the inner contents
                                format!(
                                    "{}{}",
                                    inner_content,
                                    if let Some(comment) = trailing_comment {
                                        format!(
                                            " {}",
                                            &self.display_pse(comment, previous_indentation)
                                        )
                                    } else if iter.peek().is_some() {
                                        " ".to_string()
                                    } else {
                                        "".to_string()
                                    }
                                )
                            }
                        }
                    } else if let Some(define) = DefineFunctions::lookup_by_name(atom_name) {
                        match define {
                            DefineFunctions::PublicFunction
                            | DefineFunctions::ReadOnlyFunction
                            | DefineFunctions::PrivateFunction => self.function(list),
                            DefineFunctions::Constant
                            | DefineFunctions::PersistedVariable
                            | DefineFunctions::NonFungibleToken => self.constant(list),
                            DefineFunctions::Map => self.format_map(list, previous_indentation),
                            DefineFunctions::UseTrait | DefineFunctions::ImplTrait => {
                                // these are the same as the following but need a trailing newline
                                format!(
                                    "({})\n",
                                    self.format_source_exprs(list, previous_indentation)
                                )
                            }
                            DefineFunctions::FungibleToken => {
                                self.fungible_token(list, previous_indentation)
                            }
                            DefineFunctions::Trait => self.define_trait(list, previous_indentation),
                        }
                    } else {
                        self.to_inner_content(list, previous_indentation)
                    };
                    result.push_str(t(&formatted));
                    continue;
                }
            }
            let current = self.display_pse(expr, previous_indentation);
            let mut between = " ";
            if let Some(next) = iter.peek() {
                if !is_same_line(expr, next) || is_comment(expr) {
                    between = "\n";
                }
            } else {
                // no next expression to space out
                between = "";
            }

            result.push_str(&format!("{current}{between}"));
        }
        result
    }

    fn define_trait(&self, exprs: &[PreSymbolicExpression], previous_indentation: &str) -> String {
        let mut acc = "(define-trait ".to_string();
        let indentation = &self.settings.indentation.to_string();
        acc.push_str(&self.format_source_exprs(&[exprs[1].clone()], previous_indentation));
        let mut iter = exprs[2..].iter().peekable();
        while let Some(expr) = iter.next() {
            let trailing = get_trailing_comment(expr, &mut iter);
            acc.push('\n');
            acc.push_str(indentation);
            acc.push_str(&self.format_source_exprs(&[expr.clone()], indentation));
            if let Some(comment) = trailing {
                acc.push(' ');
                acc.push_str(&self.display_pse(comment, previous_indentation));
            }
        }
        acc
    }

    fn fungible_token(
        &self,
        exprs: &[PreSymbolicExpression],
        previous_indentation: &str,
    ) -> String {
        let mut acc = "(define-fungible-token ".to_string();
        let mut iter = exprs[1..].iter().peekable();
        while let Some(expr) = iter.next() {
            let trailing = get_trailing_comment(expr, &mut iter);
            acc.push_str(&self.format_source_exprs(&[expr.clone()], previous_indentation));
            if iter.peek().is_some() {
                acc.push(' ');
            }
            if let Some(comment) = trailing {
                acc.push(' ');
                acc.push_str(&self.display_pse(comment, previous_indentation));
            }
        }
        acc.push(')');
        acc
    }
    fn constant(&self, exprs: &[PreSymbolicExpression]) -> String {
        let func_type = self.display_pse(exprs.first().unwrap(), "");
        let indentation = &self.settings.indentation.to_string();
        let mut acc = format!("({func_type} ");

        if let Some((name, args)) = name_and_args(exprs) {
            acc.push_str(&self.display_pse(name, ""));

            // Access the value from args
            if let Some(value) = args.first() {
                if let Some(list) = value.match_list() {
                    acc.push_str(&format!(
                        "\n{}({})",
                        indentation,
                        self.format_source_exprs(list, "")
                    ));
                    acc.push_str("\n)");
                } else {
                    // Handle non-list values (e.g., literals or simple expressions)
                    acc.push(' ');
                    acc.push_str(&self.display_pse(value, ""));
                    acc.push(')');
                }
            }

            acc.push('\n');
            acc
        } else {
            panic!("Expected a valid constant definition with (name value)")
        }
    }
    fn format_map(&self, exprs: &[PreSymbolicExpression], previous_indentation: &str) -> String {
        let mut acc = "(define-map ".to_string();
        let indentation = &self.settings.indentation.to_string();
        let space = format!("{}{}", previous_indentation, indentation);

        if let Some((name, args)) = name_and_args(exprs) {
            acc.push_str(&self.display_pse(name, previous_indentation));

            for arg in args.iter() {
                match &arg.pre_expr {
                    // this is hacked in to handle situations where the contents of
                    // map is a 'tuple'
                    PreSymbolicExpressionType::Tuple(list) => acc.push_str(&format!(
                        "\n{}{}",
                        space,
                        self.format_key_value_sugar(&list.to_vec(), &space)
                    )),
                    _ => acc.push_str(&format!(
                        "\n{}{}",
                        space,
                        self.format_source_exprs(&[arg.clone()], &space)
                    )),
                }
            }

            acc.push_str(&format!("\n{})\n", previous_indentation));
            acc
        } else {
            panic!("define-map without a name is invalid")
        }
    }

    // *begin* never on one line
    fn format_begin(&self, exprs: &[PreSymbolicExpression], previous_indentation: &str) -> String {
        let mut acc = "(begin".to_string();
        let indentation = &self.settings.indentation.to_string();
        let space = format!("{}{}", previous_indentation, indentation);

        let mut iter = exprs.get(1..).unwrap_or_default().iter().peekable();
        while let Some(expr) = iter.next() {
            let trailing = get_trailing_comment(expr, &mut iter);

            // begin body
            acc.push_str(&format!(
                "\n{}{}",
                space,
                self.format_source_exprs(&[expr.clone()], &space)
            ));
            if let Some(comment) = trailing {
                acc.push(' ');
                acc.push_str(&self.display_pse(comment, previous_indentation));
            }
        }
        acc.push_str(&format!("\n{})", previous_indentation));
        acc
    }

    // formats (and ..) and (or ...)
    // if given more than BOOLEAN_BREAK_LIMIT expressions it will break it onto new lines
    fn format_booleans(
        &self,
        exprs: &[PreSymbolicExpression],
        previous_indentation: &str,
    ) -> String {
        let func_type = self.display_pse(exprs.first().unwrap(), previous_indentation);
        let mut acc = format!("({func_type}");
        let indentation = &self.settings.indentation.to_string();
        let space = format!("{}{}", previous_indentation, indentation);
        let break_up =
            without_comments_len(&exprs[1..]) > BOOLEAN_BREAK_LIMIT || differing_lines(exprs);
        let mut iter = exprs.get(1..).unwrap_or_default().iter().peekable();
        if break_up {
            while let Some(expr) = iter.next() {
                let trailing = get_trailing_comment(expr, &mut iter);
                acc.push_str(&format!(
                    "\n{}{}",
                    space,
                    self.format_source_exprs(&[expr.clone()], &space)
                ));
                if let Some(comment) = trailing {
                    acc.push(' ');
                    acc.push_str(&self.display_pse(comment, previous_indentation));
                }
            }
        } else {
            while let Some(expr) = iter.next() {
                let trailing = get_trailing_comment(expr, &mut iter);
                acc.push(' ');
                acc.push_str(&self.format_source_exprs(&[expr.clone()], previous_indentation));
                if let Some(comment) = trailing {
                    acc.push(' ');
                    acc.push_str(&self.display_pse(comment, previous_indentation));
                    acc.push('\n');
                    acc.push_str(&space)
                }
            }
        }
        if break_up {
            acc.push_str(&format!("\n{}", previous_indentation));
        }
        acc.push(')');
        acc
    }

    fn format_if(&self, exprs: &[PreSymbolicExpression], previous_indentation: &str) -> String {
        let opening = exprs.first().unwrap();
        let func_type = self.display_pse(opening, previous_indentation);
        let indentation = &self.settings.indentation.to_string();
        let space = format!("{}{}", indentation, previous_indentation);

        let mut acc = format!("({func_type} ");
        let mut iter = exprs[1..].iter().peekable();
        let mut index = 0;

        while let Some(expr) = iter.next() {
            let trailing = get_trailing_comment(expr, &mut iter);
            if index > 0 {
                acc.push('\n');
                acc.push_str(&space);
            }
            acc.push_str(&self.format_source_exprs(&[expr.clone()], &space));
            if let Some(comment) = trailing {
                acc.push(' ');
                acc.push_str(&self.display_pse(comment, previous_indentation));
            }
            index += 1;
        }
        acc.push('\n');
        acc.push_str(previous_indentation);
        acc.push(')');

        acc
    }

    fn format_let(&self, exprs: &[PreSymbolicExpression], previous_indentation: &str) -> String {
        let mut acc = "(let (".to_string();
        let indentation = &self.settings.indentation.to_string();
        let space = format!("{}{}", previous_indentation, indentation);

        if let Some(args) = exprs[1].match_list() {
            let mut iter = args.iter().peekable();
            while let Some(arg) = iter.next() {
                let trailing = get_trailing_comment(arg, &mut iter);
                acc.push_str(&format!(
                    "\n{}{}",
                    space,
                    self.format_source_exprs(&[arg.clone()], &space)
                ));
                if let Some(comment) = trailing {
                    acc.push(' ');
                    acc.push_str(&self.display_pse(comment, previous_indentation));
                }
            }
        }
        // close the args paren
        acc.push_str(&format!("\n{})", previous_indentation));
        // start the let body
        for e in exprs.get(2..).unwrap_or_default() {
            acc.push_str(&format!(
                "\n{}{}",
                space,
                self.format_source_exprs(&[e.clone()], &space)
            ))
        }
        acc.push_str(&format!("\n{})", previous_indentation));
        acc
    }

    // * match *
    // always multiple lines
    fn format_match(&self, exprs: &[PreSymbolicExpression], previous_indentation: &str) -> String {
        let mut acc = "(match ".to_string();
        let indentation = &self.settings.indentation.to_string();
        let space = format!("{}{}", previous_indentation, indentation);

        // value to match on
        acc.push_str(&self.format_source_exprs(&[exprs[1].clone()], previous_indentation));
        // branches evenly spaced

        let mut iter = exprs[2..].iter().peekable();
        while let Some(branch) = iter.next() {
            let trailing = get_trailing_comment(branch, &mut iter);
            acc.push_str(&format!(
                "\n{}{}",
                space,
                self.format_source_exprs(&[branch.clone()], &space)
            ));
            if let Some(comment) = trailing {
                acc.push(' ');
                acc.push_str(&self.display_pse(comment, previous_indentation));
            }
        }
        acc.push_str(&format!("\n{})", previous_indentation));
        acc
    }

    fn format_list(&self, exprs: &[PreSymbolicExpression], previous_indentation: &str) -> String {
        let mut acc = "(".to_string();
        for (i, expr) in exprs[0..].iter().enumerate() {
            let value = self.format_source_exprs(&[expr.clone()], previous_indentation);
            if i < exprs.len() - 1 {
                acc.push_str(&value.to_string());
                acc.push(' ');
            } else {
                acc.push_str(&value.to_string());
            }
        }
        acc.push(')');
        t(&acc).to_string()
    }

    // used for { n1: 1 } syntax
    fn format_key_value_sugar(
        &self,
        exprs: &[PreSymbolicExpression],
        previous_indentation: &str,
    ) -> String {
        let indentation = &self.settings.indentation.to_string();
        let space = format!("{}{}", previous_indentation, indentation);
        let over_2_kvs = without_comments_len(exprs) > 2;
        let mut acc = "{".to_string();

        if over_2_kvs {
            acc.push('\n');
            let mut iter = exprs.iter().peekable();
            while let Some(key) = iter.next() {
                if is_comment(key) {
                    acc.push_str(&space);
                    acc.push_str(&self.display_pse(key, previous_indentation));
                    acc.push('\n');
                    continue;
                }
                let value = iter.next().unwrap();
                let trailing = get_trailing_comment(value, &mut iter);
                // Pass the current indentation level to nested formatting
                let key_str = self.format_source_exprs(&[key.clone()], &space);
                let value_str = self.format_source_exprs(&[value.clone()], &space);
                acc.push_str(&format!("{}{}: {},", space, key_str, value_str));

                if let Some(comment) = trailing {
                    acc.push(' ');
                    acc.push_str(&self.display_pse(comment, previous_indentation));
                }
                acc.push('\n');
            }
        } else {
            // for cases where we keep it on the same line with 1 k/v pair
            let fkey = self.display_pse(&exprs[0], previous_indentation);
            acc.push_str(&format!(
                " {fkey}: {} ",
                self.format_source_exprs(&[exprs[1].clone()], previous_indentation)
            ));
        }

        if over_2_kvs {
            acc.push_str(previous_indentation);
        }
        acc.push('}');
        acc
    }

    // used for (tuple (n1  1)) syntax
    // Note: Converted to a { a: 1 } style map
    // TODO: This should be rolled into format_key_value_sugar, but the PSE
    // structure is different so it would take some finagling
    fn format_key_value(
        &self,
        exprs: &[PreSymbolicExpression],
        previous_indentation: &str,
    ) -> String {
        let indentation = &self.settings.indentation.to_string();
        let space = format!("{}{}", previous_indentation, indentation);

        let mut acc = previous_indentation.to_string();
        acc.push('{');

        // for cases where we keep it on the same line with 1 k/v pair
        let multiline = exprs.len() > 1;
        if multiline {
            acc.push('\n');
            let mut iter = exprs.iter().peekable();
            while let Some(arg) = iter.next() {
                let trailing = get_trailing_comment(arg, &mut iter);
                let (key, value) = arg
                    .match_list()
                    .and_then(|list| list.split_first())
                    .unwrap();
                let fkey = self.display_pse(key, previous_indentation);

                acc.push_str(&format!(
                    "{space}{fkey}: {},",
                    self.format_source_exprs(value, previous_indentation)
                ));
                if let Some(comment) = trailing {
                    acc.push(' ');
                    acc.push_str(&self.display_pse(comment, previous_indentation));
                }
                acc.push('\n');
            }
            acc.push_str(previous_indentation);
        } else {
            // for cases where we keep it on the same line with 1 k/v pair
            let (key, value) = exprs[0]
                .match_list()
                .and_then(|list| list.split_first())
                .unwrap();
            let fkey = self.display_pse(key, previous_indentation);
            acc.push_str(&format!(
                " {fkey}: {} ",
                self.format_source_exprs(value, previous_indentation)
            ));
        }

        acc.push('}');
        acc
    }

    // This prints leaves of the PSE tree
    fn display_pse(&self, pse: &PreSymbolicExpression, previous_indentation: &str) -> String {
        match pse.pre_expr {
            PreSymbolicExpressionType::Atom(ref value) => t(value.as_str()).to_string(),
            PreSymbolicExpressionType::AtomValue(ref value) => value.to_string(),
            PreSymbolicExpressionType::List(ref items) => {
                self.format_list(items, previous_indentation)
            }
            PreSymbolicExpressionType::Tuple(ref items) => {
                self.format_key_value_sugar(items, previous_indentation)
            }
            PreSymbolicExpressionType::SugaredContractIdentifier(ref name) => {
                format!(".{}", name)
            }
            PreSymbolicExpressionType::SugaredFieldIdentifier(ref contract, ref field) => {
                format!(".{}.{}", contract, field)
            }
            PreSymbolicExpressionType::FieldIdentifier(ref trait_id) => {
                format!("'{}", trait_id)
            }
            PreSymbolicExpressionType::TraitReference(ref name) => name.to_string(),
            PreSymbolicExpressionType::Comment(ref text) => {
                if text.is_empty() {
                    ";;".to_string()
                } else {
                    format!(";; {}", t(text))
                }
            }
            PreSymbolicExpressionType::Placeholder(ref placeholder) => {
                placeholder.to_string() // Placeholder is for if parsing fails
            }
        }
    }

    // * functions

    // Top level define-<function> should have a line break above and after (except on first line)
    // options always on new lines
    // Functions Always on multiple lines, even if short
    fn function(&self, exprs: &[PreSymbolicExpression]) -> String {
        let func_type = self.display_pse(exprs.first().unwrap(), "");
        let indentation = &self.settings.indentation.to_string();
        let args_indent = format!("{}{}", indentation, indentation);

        let mut acc = format!("({func_type} (");

        // function name and arguments
        if let Some(def) = exprs.get(1).and_then(|f| f.match_list()) {
            if let Some((name, args)) = def.split_first() {
                acc.push_str(&self.display_pse(name, ""));

                let mut iter = args.iter().peekable();
                while let Some(arg) = iter.next() {
                    let trailing = get_trailing_comment(arg, &mut iter);
                    if arg.match_list().is_some() {
                        // expr args
                        acc.push_str(&format!(
                            "\n{}{}",
                            args_indent,
                            self.format_source_exprs(&[arg.clone()], &args_indent)
                        ))
                    } else {
                        // atom args
                        acc.push_str(&self.format_source_exprs(&[arg.clone()], &args_indent))
                    }
                    if let Some(comment) = trailing {
                        acc.push(' ');
                        acc.push_str(&self.display_pse(comment, ""));
                    }
                }
                if args.is_empty() {
                    acc.push(')')
                } else {
                    acc.push_str(&format!("\n{})", indentation))
                }
            } else {
                panic!("can't have a nameless function")
            }
        }

        // function body expressions
        for expr in exprs.get(2..).unwrap_or_default() {
            acc.push_str(&format!(
                "\n{}{}",
                indentation,
                self.format_source_exprs(&[expr.clone()], &self.settings.indentation.to_string(),)
            ))
        }
        acc.push_str("\n)\n\n");
        acc
    }

    // This code handles the line width wrapping and happens near the bottom of the
    // traversal
    fn to_inner_content(
        &self,
        list: &[PreSymbolicExpression],
        previous_indentation: &str,
    ) -> String {
        let mut result = String::new();
        let mut current_line_width = previous_indentation.len();
        let mut first_on_line = true;
        let mut broken_up = false;
        let indentation = self.settings.indentation.to_string();
        let base_indent = format!("{}{}", previous_indentation, indentation);

        // TODO: this should ignore comment length
        for expr in list.iter() {
            let indented = if first_on_line {
                &base_indent
            } else {
                previous_indentation
            };
            let formatted = self.format_source_exprs(&[expr.clone()], indented);
            let trimmed = t(&formatted);
            let expr_width = trimmed.len();

            if !first_on_line {
                // For subexpressions over max line length, add newline with increased indent
                if current_line_width + expr_width + 1 > self.settings.max_line_length {
                    result.push('\n');
                    result.push_str(&base_indent);
                    current_line_width = base_indent.len() + indentation.len();
                    broken_up = true;
                } else {
                    result.push(' ');
                    current_line_width += 1;
                }
            }

            if broken_up {
                // reformat with increased indent in the case we broke up the code on max width
                let formatted = self.format_source_exprs(&[expr.clone()], &base_indent);
                let trimmed = t(&formatted);
                result.push_str(trimmed);
            } else {
                result.push_str(trimmed);
            }

            current_line_width += expr_width;
            first_on_line = false;
            broken_up = false;
        }

        let break_lines = result.contains('\n') && {
            let lines: Vec<_> = result.lines().collect();
            if let Some(last_line) = lines.last() {
                if last_line.trim() == ")" || last_line.trim() == "}" {
                    // Don't add a newline if we're already looking at a closing paren/bracket
                    false
                } else {
                    true
                }
            } else {
                true
            }
        };
        let newlined = format!("\n{})", previous_indentation);
        format!("({}{}", result, if break_lines { &newlined } else { ")" })
    }
}

fn is_comment(pse: &PreSymbolicExpression) -> bool {
    matches!(pse.pre_expr, PreSymbolicExpressionType::Comment(_))
}

fn without_comments_len(exprs: &[PreSymbolicExpression]) -> usize {
    exprs.iter().filter(|expr| !is_comment(expr)).count()
}
// if the exprs are already broken onto different lines, return true
fn differing_lines(exprs: &[PreSymbolicExpression]) -> bool {
    !exprs
        .windows(2)
        .all(|window| window[0].span().start_line == window[1].span().start_line)
}

fn is_same_line(expr1: &PreSymbolicExpression, expr2: &PreSymbolicExpression) -> bool {
    expr1.span().start_line == expr2.span().start_line
}

// convenience function to return a possible comment PSE from a peekable iterator
fn get_trailing_comment<'a, I>(
    expr: &'a PreSymbolicExpression,
    iter: &mut Peekable<I>,
) -> Option<&'a PreSymbolicExpression>
where
    I: Iterator<Item = &'a PreSymbolicExpression>,
{
    // cloned() here because of the second mutable borrow on iter.next()
    match iter.peek().cloned() {
        Some(next) => {
            if is_comment(next) && is_same_line(expr, next) {
                iter.next();
                Some(next)
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests_formatter {
    use super::{ClarityFormatter, Settings};
    use crate::formatter::Indentation;
    #[allow(unused_imports)]
    use std::assert_eq;
    #[macro_export]
    macro_rules! assert_eq {
        ($($arg:tt)*) => {
            pretty_assertions::assert_eq!($($arg)*)
        }
    }
    use std::collections::HashMap;
    use std::fs;
    use std::path::Path;

    fn format_with_default(source: &str) -> String {
        let formatter = ClarityFormatter::new(Settings::default());
        formatter.format_section(source)
    }

    fn format_with(source: &str, settings: Settings) -> String {
        let formatter = ClarityFormatter::new(settings);
        formatter.format_section(source)
    }

    /// This is strictly for reading top metadata from golden tests
    fn from_metadata(metadata: &str) -> Settings {
        let mut max_line_length = 80;
        let mut indent = Indentation::Space(2);

        let metadata_map: HashMap<&str, &str> = metadata
            .split(',')
            .map(|pair| pair.trim())
            .filter_map(|kv| kv.split_once(':'))
            .map(|(k, v)| (k.trim(), v.trim()))
            .collect();

        if let Some(length) = metadata_map.get("max_line_length") {
            max_line_length = length.parse().unwrap_or(max_line_length);
        }

        if let Some(&indentation) = metadata_map.get("indentation") {
            indent = match indentation {
                "tab" => Indentation::Tab,
                value => {
                    if let Ok(spaces) = value.parse::<usize>() {
                        Indentation::Space(spaces)
                    } else {
                        Indentation::Space(2) // Fallback to default
                    }
                }
            };
        }

        Settings {
            max_line_length,
            indentation: indent,
        }
    }
    fn format_file_with_metadata(source: &str) -> String {
        let mut lines = source.lines();
        let metadata_line = lines.next().unwrap_or_default();
        let settings = from_metadata(metadata_line);

        let real_source = lines.collect::<Vec<&str>>().join("\n");
        let formatter = ClarityFormatter::new(settings);
        formatter.format_file(&real_source)
    }
    #[test]
    fn test_simplest_formatter() {
        let result = format_with_default(&String::from("(  ok    true )"));
        assert_eq!(result, "(ok true)");
    }

    #[test]
    fn test_fungible_token() {
        let src = "(define-fungible-token hello)";
        let result = format_with_default(&String::from(src));
        assert_eq!(result, src);

        let src = "(define-fungible-token hello u100)";
        let result = format_with_default(&String::from(src));
        assert_eq!(result, src);
    }

    #[test]
    fn test_manual_tuple() {
        let result = format_with_default(&String::from("(tuple (n1 1))"));
        assert_eq!(result, "{ n1: 1 }");
        let result = format_with_default(&String::from("(tuple (n1 1) (n2 2))"));
        assert_eq!(result, "{\n  n1: 1,\n  n2: 2,\n}");
    }

    #[test]
    fn test_function_formatter() {
        let result = format_with_default(&String::from("(define-private (my-func) (ok true))"));
        assert_eq!(result, "(define-private (my-func)\n  (ok true)\n)\n\n");
    }

    #[test]
    fn test_multi_function() {
        let src = "(define-public (my-func) (ok true))\n(define-public (my-func2) (ok true))";
        let result = format_with_default(&String::from(src));
        let expected = r#"(define-public (my-func)
  (ok true)
)

(define-public (my-func2)
  (ok true)
)

"#;
        assert_eq!(expected, result);
    }
    #[test]
    fn test_function_args_multiline() {
        let src = "(define-public (my-func (amount uint) (sender principal)) (ok true))";
        let result = format_with_default(&String::from(src));
        assert_eq!(
            result,
            "(define-public (my-func\n    (amount uint)\n    (sender principal)\n  )\n  (ok true)\n)\n\n"
        );
    }
    #[test]
    fn test_pre_comments_included() {
        let src = ";; this is a pre comment\n;; multi\n(ok true)";
        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
    }

    #[test]
    fn test_inline_comments_included() {
        let src = "(ok true) ;; this is an inline comment";
        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
    }

    #[test]
    fn test_booleans() {
        let src = "(or true false)";
        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
        let src = "(or true (is-eq 1 2) (is-eq 1 1))";
        let result = format_with_default(&String::from(src));
        let expected = "(or\n  true\n  (is-eq 1 2)\n  (is-eq 1 1)\n)";
        assert_eq!(expected, result);
    }

    #[test]
    fn test_booleans_with_comments() {
        let src = r#"(or
  true
  ;; pre comment
  (is-eq 1 2) ;; comment
  (is-eq 1 1) ;; b
)"#;
        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);

        let src = r#"(asserts!
  (or
    (is-eq merkle-root txid) ;; true, if the transaction is the only transaction
    (try! (verify-merkle-proof reversed-txid (reverse-buff32 merkle-root) proof))
  )
  (err ERR-INVALID-MERKLE-PROOF)
)"#;
        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
    }

    #[test]
    fn long_line_unwrapping() {
        let src = "(try! (unwrap! (complete-deposit-wrapper (get txid deposit) (get vout-index deposit) (get amount deposit) (get recipient deposit) (get burn-hash deposit) (get burn-height deposit) (get sweep-txid deposit)) (err (+ ERR_DEPOSIT_INDEX_PREFIX (+ u10 index)))))";
        let result = format_with_default(&String::from(src));
        let expected = r#"(try!
  (unwrap!
    (complete-deposit-wrapper (get txid deposit) (get vout-index deposit)
      (get amount deposit) (get recipient deposit) (get burn-hash deposit)
      (get burn-height deposit) (get sweep-txid deposit)
    )
    (err (+ ERR_DEPOSIT_INDEX_PREFIX (+ u10 index)))
  ))"#;
        assert_eq!(expected, result);

        // non-max-length sanity case
        let src = "(try! (unwrap! (something) (err SOME_ERR)))";
        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
    }

    #[test]
    fn test_map() {
        let src = "(define-map a uint {n1: (buff 20)})";
        let result = format_with_default(&String::from(src));
        assert_eq!(result, "(define-map a\n  uint\n  { n1: (buff 20) }\n)\n");
        let src = "(define-map something { name: (buff 48), a: uint } uint)";
        let result = format_with_default(&String::from(src));
        let expected = r#"(define-map something
  {
    name: (buff 48),
    a: uint,
  }
  uint
)
"#;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_let() {
        let src = "(let ((a 1) (b 2)) (+ a b))";
        let result = format_with_default(&String::from(src));
        let expected = "(let (\n  (a 1)\n  (b 2)\n)\n  (+ a b)\n)";
        assert_eq!(expected, result);
    }

    #[test]
    fn test_option_match() {
        let src = "(match opt value (ok (handle-new-value value)) (ok 1))";
        let result = format_with_default(&String::from(src));
        // "(match opt\n
        let expected = r#"(match opt
  value
  (ok (handle-new-value value))
  (ok 1)
)"#;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_response_match() {
        let src = "(match x value (ok (+ to-add value)) err-value (err err-value))";
        let result = format_with_default(&String::from(src));
        let expected = r#"(match x
  value
  (ok (+ to-add value))
  err-value
  (err err-value)
)"#;
        assert_eq!(result, expected);
    }
    #[test]
    fn test_key_value_sugar() {
        let src = "{name: (buff 48)}";
        let result = format_with_default(&String::from(src));
        assert_eq!(result, "{ name: (buff 48) }");
        let src = "{ name: (buff 48), a: uint }";
        let result = format_with_default(&String::from(src));
        assert_eq!(result, "{\n  name: (buff 48),\n  a: uint,\n}");
    }

    #[test]
    fn map_in_map() {
        let src = "(ok { a: b, ctx: { a: b, c: d }})";
        let result = format_with_default(src);
        let expected = r#"(ok {
  a: b,
  ctx: {
    a: b,
    c: d,
  },
})"#;
        std::assert_eq!(expected, result);
        let src = r#"(ok
  {
    varslice: (unwrap! (slice? txbuff slice-start target-index) (err ERR-OUT-OF-BOUNDS)),
    ctx: {
      txbuff: tx,
      index: (+ u1 ptr),
    },
  })"#;
        let result = format_with_default(src);
        assert_eq!(src, result);
    }

    #[test]
    fn old_tuple() {
        let src = r#"(tuple
  (a uint)
  (b uint) ;; comment
  (c bool)
)"#;
        let result = format_with_default(src);
        let expected = r#"{
  a: uint,
  b: uint, ;; comment
  c: bool,
}"#;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_indentation_levels() {
        let src = "(begin (let ((a 1) (b 2)) (ok true)))";
        let result = format_with_default(&String::from(src));
        let expected = r#"(begin
  (let (
    (a 1)
    (b 2)
  )
    (ok true)
  )
)"#;
        assert_eq!(result, expected);
    }
    #[test]
    fn test_let_comments() {
        let src = r#"(begin
  (let (
    (a 1) ;; something
    (b 2) ;; comment
  )
    (ok true)
  )
)"#;
        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
    }

    #[test]
    fn test_block_comments() {
        let src = ";;\n;; abc\n;;";
        let result = format_with_default(src);
        assert_eq!(src, result)
    }

    #[test]
    fn test_key_value_sugar_comment_midrecord() {
        let src = r#"{
  name: (buff 48),
  ;; comment
  owner: send-to, ;; trailing
}"#;
        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
    }

    #[test]
    fn test_basic_slice() {
        let src = "(slice? (1 2 3 4 5) u5 u9)";
        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
    }
    #[test]
    fn test_constant() {
        let src = "(define-constant something 1)\n";
        let result = format_with_default(&String::from(src));
        assert_eq!(result, "(define-constant something 1)\n");
        let src2 = "(define-constant something (1 2))\n";
        let result2 = format_with_default(&String::from(src2));
        assert_eq!(result2, "(define-constant something\n  (1 2)\n)\n");
    }

    #[test]
    fn test_begin_never_one_line() {
        let src = "(begin (ok true))";
        let result = format_with_default(&String::from(src));
        assert_eq!(result, "(begin\n  (ok true)\n)");
    }

    #[test]
    fn test_begin() {
        let src = "(begin (+ 1 1) ;; a\n (ok true))";
        let result = format_with_default(&String::from(src));
        assert_eq!(result, "(begin\n  (+ 1 1) ;; a\n  (ok true)\n)");
    }

    #[test]
    fn test_custom_tab_setting() {
        let src = "(begin (ok true))";
        let result = format_with(&String::from(src), Settings::new(Indentation::Space(4), 80));
        assert_eq!(result, "(begin\n    (ok true)\n)");
    }

    #[test]
    fn test_if() {
        let src = "(if (<= amount max-supply) (list ) (something amount))";
        let result = format_with_default(&String::from(src));
        let expected = "(if (<= amount max-supply)\n  (list)\n  (something amount)\n)";
        assert_eq!(result, expected);
    }
    #[test]
    fn test_ignore_formatting() {
        let src = ";; @format-ignore\n(    begin ( ok true))";
        let result = format_with(&String::from(src), Settings::new(Indentation::Space(4), 80));
        assert_eq!(src, result);

        let src = ";; @format-ignore\n(list\n  u64\n  u64 u64\n)";
        let result = format_with(&String::from(src), Settings::new(Indentation::Space(4), 80));
        assert_eq!(src, result);
    }

    #[test]
    fn test_index_of() {
        let src = "(index-of? (contract-call? .pool borroweable) asset)";
        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
    }
    #[test]
    fn test_traits() {
        let src = "(use-trait token-a-trait 'SPAXYA5XS51713FDTQ8H94EJ4V579CXMTRNBZKSF.token-a.token-trait)\n";
        let result = format_with(&String::from(src), Settings::new(Indentation::Space(4), 80));
        assert_eq!(src, result);
        let src = "(as-contract (contract-call? .tokens mint! u19))";
        let result = format_with(&String::from(src), Settings::new(Indentation::Space(4), 80));
        assert_eq!(src, result);
    }

    // this looks redundant, but a regression kept happening with ill-spaced
    // inner expressions. Likely this is a product of poorly handled nesting
    // logic
    #[test]
    fn spacing_for_inner_expr() {
        let src = "(something (- (/ b o) (/ (- balance-sender a) o)))";
        let result = format_with_default(src);
        assert_eq!(src, result)
    }
    #[test]
    fn closing_if_parens() {
        let src = "(something (if (true) (list) (list 1 2 3)))";
        let result = format_with_default(src);
        let expected = r#"(something (if (true)
  (list)
  (list 1 2 3)
))"#;
        assert_eq!(expected, result);
    }

    #[test]
    fn ok_map() {
        let src = "(ok { a: b, c: d })";
        let result = format_with_default(src);
        let expected = r#"(ok {
  a: b,
  c: d,
})"#;
        assert_eq!(expected, result);
    }

    #[test]
    fn if_let_if() {
        let src = r#"(if (true)
  (let (
    (a (if (true)
      (list)
      (list)
    ))
  )
    (list)
  )
  (list)
)"#;
        let result = format_with_default(src);
        std::assert_eq!(src, result);
    }

    #[test]
    #[ignore]
    fn define_trait_test() {
        // TODO: Not sure how this should be formatted
        let src = r#"(define-trait token-trait
  ((transfer? (principal principal uint) (response uint uint))
    (get-balance (principal) (response uint uint))
  )
)"#;
        let result = format_with_default(src);
        assert_eq!(src, result);
    }
    #[test]
    fn unwrap_wrapped_lines() {
        let src = r#"(new-available-ids
  (if (is-eq no-to-treasury u0)
    (var-get available-ids)
    (unwrap-panic
      (as-max-len? (concat (var-get available-ids) ids-to-treasury) u10000)
    )
  ))"#;
        let result = format_with_default(src);
        assert_eq!(src, result);
    }

    #[test]
    fn test_irl_contracts() {
        let golden_dir = "./tests/golden";
        let intended_dir = "./tests/golden-intended";

        // Iterate over files in the golden directory
        for entry in fs::read_dir(golden_dir).expect("Failed to read golden directory") {
            let entry = entry.expect("Failed to read directory entry");
            let path = entry.path();

            if path.is_file() {
                let src = fs::read_to_string(&path).expect("Failed to read source file");

                let file_name = path.file_name().expect("Failed to get file name");
                let intended_path = Path::new(intended_dir).join(file_name);

                let intended =
                    fs::read_to_string(&intended_path).expect("Failed to read intended file");

                // Apply formatting and compare
                let result = format_file_with_metadata(&src);
                pretty_assertions::assert_eq!(
                    result,
                    intended,
                    "Mismatch in file: {:?}",
                    file_name
                );
            }
        }
    }
}
