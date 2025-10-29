use crate::{
    expr::{
        get_origin, set_origin, Arg, Bind, Expr, ExprId, ExprKind, Lambda, ModPath,
        ModuleKind, Origin, Pattern, Sandbox, Sig, SigItem, StructurePattern, TryCatch,
        TypeDef,
    },
    typ::{FnArgType, FnType, TVar, Type},
};
use anyhow::{bail, Result};
use arcstr::{literal, ArcStr};
use combine::{
    attempt, between, chainl1, choice, eof, look_ahead, many, many1, none_of,
    not_followed_by, optional,
    parser::{
        char::{alpha_num, digit, space, string},
        combinator::recognize,
        range::{take_while, take_while1},
    },
    position, sep_by, sep_by1, skip_many,
    stream::{
        position::{self, SourcePosition},
        Range,
    },
    token, unexpected_any, value, EasyParser, ParseError, Parser, RangeStream,
};
use compact_str::CompactString;
use escaping::Escape;
use fxhash::FxHashSet;
use netidx::{
    path::Path,
    publisher::{Typ, Value},
    utils::Either,
};
use netidx_value::parser::{
    escaped_string, int, value as parse_value, VAL_ESC, VAL_MUST_ESC,
};
use parking_lot::RwLock;
use poolshark::local::LPooled;
use std::sync::LazyLock;
use triomphe::Arc;

#[cfg(test)]
mod test;

fn escape_generic(c: char) -> bool {
    c.is_control()
}

pub const GRAPHIX_MUST_ESC: [char; 4] = ['"', '\\', '[', ']'];
pub static GRAPHIX_ESC: LazyLock<Escape> = LazyLock::new(|| {
    Escape::new(
        '\\',
        &['"', '\\', '[', ']', '\n', '\r', '\t', '\0'],
        &[('\n', "n"), ('\r', "r"), ('\t', "t"), ('\0', "0")],
        Some(escape_generic),
    )
    .unwrap()
});
pub const RESERVED: LazyLock<FxHashSet<&str>> = LazyLock::new(|| {
    FxHashSet::from_iter([
        "true", "false", "ok", "null", "mod", "let", "select", "pub", "type", "fn",
        "cast", "if", "u32", "v32", "i32", "z32", "u64", "v64", "i64", "z64", "f32",
        "f64", "decimal", "datetime", "duration", "bool", "string", "bytes", "result",
        "null", "_", "?", "fn", "Array", "Map", "any", "Any", "use", "rec", "catch",
        "try",
    ])
});

fn spaces<I>() -> impl Parser<I, Output = ()>
where
    I: RangeStream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
    I::Range: Range,
{
    combine::parser::char::spaces().with(skip_many(attempt(
        string("//")
            .with(not_followed_by(token('/')))
            .with(skip_many(none_of(['\n'])))
            .with(combine::parser::char::spaces()),
    )))
}

fn doc_comment<I>() -> impl Parser<I, Output = Option<ArcStr>>
where
    I: RangeStream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
    I::Range: Range,
{
    combine::parser::char::spaces()
        .with(many(
            string("///")
                .with(many(none_of(['\n'])))
                .skip(combine::parser::char::spaces()),
        ))
        .map(
            |lines: LPooled<Vec<String>>| {
                if lines.len() == 0 {
                    None
                } else {
                    Some(ArcStr::from(lines.join("\n")))
                }
            },
        )
}

fn spstring<'a, I>(s: &'static str) -> impl Parser<I, Output = &'a str>
where
    I: RangeStream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
    I::Range: Range,
{
    spaces().with(string(s))
}

fn ident<I>(cap: bool) -> impl Parser<I, Output = ArcStr>
where
    I: RangeStream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
    I::Range: Range,
{
    recognize((
        take_while1(move |c: char| c.is_alphabetic() && cap == c.is_uppercase()),
        take_while(|c: char| c.is_alphanumeric() || c == '_'),
    ))
    .map(|s: CompactString| ArcStr::from(s.as_str()))
}

fn fname<I>() -> impl Parser<I, Output = ArcStr>
where
    I: RangeStream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
    I::Range: Range,
{
    ident(false).then(|s| {
        if RESERVED.contains(&s.as_str()) {
            unexpected_any("can't use keyword as a function or variable name").left()
        } else {
            value(s).right()
        }
    })
}

fn spfname<I>() -> impl Parser<I, Output = ArcStr>
where
    I: RangeStream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
    I::Range: Range,
{
    spaces().with(fname())
}

fn typname<I>() -> impl Parser<I, Output = ArcStr>
where
    I: RangeStream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
    I::Range: Range,
{
    ident(true).then(|s| {
        if RESERVED.contains(&s.as_str()) {
            unexpected_any("can't use keyword as a type name").left()
        } else {
            value(s).right()
        }
    })
}

fn sptypname<I>() -> impl Parser<I, Output = ArcStr>
where
    I: RangeStream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
    I::Range: Range,
{
    spaces().with(typname())
}

pub(crate) fn modpath<I>() -> impl Parser<I, Output = ModPath>
where
    I: RangeStream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
    I::Range: Range,
{
    sep_by1(fname(), string("::"))
        .map(|mut v: LPooled<Vec<ArcStr>>| ModPath(Path::from_iter(v.drain(..))))
}

fn spmodpath<I>() -> impl Parser<I, Output = ModPath>
where
    I: RangeStream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
    I::Range: Range,
{
    spaces().with(modpath())
}

fn typath<I>() -> impl Parser<I, Output = ModPath>
where
    I: RangeStream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
    I::Range: Range,
{
    sep_by1(choice((attempt(spfname()), sptypname())), string("::")).then(
        |mut parts: LPooled<Vec<ArcStr>>| {
            if parts.len() == 0 {
                unexpected_any("empty type path").left()
            } else {
                match parts.last().unwrap().chars().next() {
                    None => unexpected_any("empty name").left(),
                    Some(c) if c.is_lowercase() => {
                        unexpected_any("type names must be capitalized").left()
                    }
                    Some(_) => value(ModPath::from(parts.drain(..))).right(),
                }
            }
        },
    )
}

fn sptypath<I>() -> impl Parser<I, Output = ModPath>
where
    I: RangeStream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
    I::Range: Range,
{
    spaces().with(typath())
}

fn csep<I>() -> impl Parser<I, Output = char>
where
    I: RangeStream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
    I::Range: Range,
{
    attempt(spaces().with(token(',')))
}

fn sptoken<I>(t: char) -> impl Parser<I, Output = char>
where
    I: RangeStream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
    I::Range: Range,
{
    spaces().with(token(t))
}

parser! {
    fn interpolated[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        #[derive(Debug, Clone)]
        enum Intp {
            Lit(SourcePosition, String),
            Expr(Expr),
        }
        impl Intp {
            fn to_expr(self) -> Expr {
                match self {
                    Intp::Lit(pos, s) => Expr {
                        id: ExprId::new(),
                        ori: get_origin(),
                        pos,
                        kind: ExprKind::Constant(Value::from(s)),
                    },
                    Intp::Expr(s) => s,
                }
            }
        }
        (
            position(),
            between(
                token('"'),
                token('"'),
                many(choice((
                    attempt(between(token('['), sptoken(']'), expr()).map(Intp::Expr)),
                    (position(), escaped_string(&GRAPHIX_MUST_ESC, &GRAPHIX_ESC))
                    .then(|(pos, s)| {
                        if s.is_empty() {
                            unexpected_any("empty string").right()
                        } else {
                            value(Intp::Lit(pos, s)).left()
                        }
                    }),
                ))),
            ),
        )
            .map(|(pos, mut toks): (_, LPooled<Vec<Intp>>)| {
                let mut argvec = vec![];
                toks.drain(..)
                    .fold(None, |src, tok| -> Option<Expr> {
                        match (src, tok) {
                            (None, t @ Intp::Lit(_, _)) => Some(t.to_expr()),
                            (None, Intp::Expr(s)) => {
                                argvec.push(s);
                                Some(
                                    ExprKind::StringInterpolate {
                                        args: Arc::from_iter(argvec.clone().into_iter()),
                                    }
                                    .to_expr(pos),
                                )
                            }
                            (Some(src @ Expr { kind: ExprKind::Constant(_), .. }), s) => {
                                argvec.extend([src, s.to_expr()]);
                                Some(
                                    ExprKind::StringInterpolate {
                                        args: Arc::from_iter(argvec.clone().into_iter()),
                                    }
                                    .to_expr(pos),
                                )
                            }
                            (
                                Some(Expr {
                                    kind: ExprKind::StringInterpolate { args: _ },
                                    ..
                                }),
                                s,
                            ) => {
                                argvec.push(s.to_expr());
                                Some(
                                    ExprKind::StringInterpolate {
                                        args: Arc::from_iter(argvec.clone().into_iter()),
                                    }
                                    .to_expr(pos),
                                )
                            }
                            (Some(Expr { kind: ExprKind::Bind { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::StructWith { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Array { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Map { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::MapRef { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Any { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::StructRef { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::TupleRef { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Tuple { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Variant { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Struct { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Qop(_), .. }), _)
                                | (Some(Expr { kind: ExprKind::OrNever(_), .. }), _)
                                | (Some(Expr { kind: ExprKind::TryCatch(_), .. }), _)
                                | (Some(Expr { kind: ExprKind::Do { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Module { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Use { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Connect { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Ref { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Eq { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Ne { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Lt { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Gt { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Gte { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Lte { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::And { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Or { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Not { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Add { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Sub { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Mul { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Div { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Mod { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Select { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::TypeCast { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::TypeDef { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::ArrayRef { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::ArraySlice { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Apply { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::ByRef(_), .. }), _)
                                | (Some(Expr { kind: ExprKind::Deref(_), .. }), _)
                                | (Some(Expr { kind: ExprKind::Sample { .. }, .. }), _)
                                | (Some(Expr { kind: ExprKind::Lambda { .. }, .. }), _) => {
                                    unreachable!()
                                }
                        }
                    })
                    .unwrap_or_else(|| ExprKind::Constant(Value::from("")).to_expr(pos))
            })
    }
}

parser! {
    fn sig_item[I]()(I) -> SigItem
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        choice((
            attempt(spaces().with(typedef())).map(|e| match e.kind {
                ExprKind::TypeDef(td) => SigItem::TypeDef(td),
                _ => unreachable!()
            }),
            attempt(spstring("val").with(space()).with((spfname(), sptoken(':').with(typexp()))))
                .map(|(name, typ)| {
                    SigItem::Bind(name, typ)
                }),
            attempt(spstring("mod").with(space()).with((
                spfname().skip(sptoken(':')).skip(spstring("sig")),
                between(sptoken('{'), sptoken('}'),
                    sep_by1(sig_item(), attempt(sptoken(';'))))
            ))).map(|(name, mut items): (ArcStr, LPooled<Vec<SigItem>>)| {
                SigItem::Module(name, Sig(Arc::from_iter(items.drain(..))))
            })
        ))
    }
}

parser! {
    fn sandbox[I]()(I) -> Sandbox
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        choice((
            spstring("unrestricted").map(|_| Sandbox::Unrestricted),
            spstring("blacklist").with(between(
                sptoken('['), sptoken(']'),
                sep_by1(spaces().with(modpath()), csep())
            )).map(|l: Vec<ModPath>| Sandbox::Blacklist(Arc::from(l))),
            spstring("whitelist").with(between(
                sptoken('['), sptoken(']'),
                sep_by1(spaces().with(modpath()), csep())
            )).map(|l: Vec<ModPath>| Sandbox::Whitelist(Arc::from(l)))
        ))
        .skip(sptoken(';'))
    }
}

parser! {
    fn dynamic_module[I]()(I) -> ModuleKind
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        space().with(spstring("dynamic")).with(between(
            sptoken('{'), sptoken('}'),
            (
                spstring("sandbox").with(space()).with(sandbox()),
                spstring("sig").with(between(
                    sptoken('{'), sptoken('}'),
                    sep_by1(sig_item(), attempt(sptoken(';')))
                        .map(|i: Vec<SigItem>| Sig(Arc::from(i)))
                ))
                .skip(sptoken(';')),
                spstring("source").with(space()).with(expr())
            )
        )).map(|(sandbox, sig, source)| {
            ModuleKind::Dynamic { sandbox, sig, source: Arc::new(source) }
        })
    }
}

parser! {
    fn inline_module[I]()(I) -> ModuleKind
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        between(
            sptoken('{'), sptoken('}'),
            sep_by(expr(), attempt(sptoken(';')))
        )
        .map(|m: Vec<Expr>| ModuleKind::Inline(Arc::from(m)))
    }
}

parser! {
    fn module[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (
            position(),
            optional(string("pub").skip(space())).map(|o| o.is_some()),
            spstring("mod").with(space()).with(spfname()),
            optional(choice((
                attempt(inline_module()),
                attempt(dynamic_module())
            ))).map(|m| m.unwrap_or(ModuleKind::Unresolved))
        )
            .map(|(pos, export, name, value)| {
                ExprKind::Module { name, export, value }.to_expr(pos)
            })
    }
}

fn use_module<I>() -> impl Parser<I, Output = Expr>
where
    I: RangeStream<Token = char, Position = SourcePosition>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
    I::Range: Range,
{
    (position(), string("use").with(space()).with(spmodpath()))
        .map(|(pos, name)| ExprKind::Use { name }.to_expr(pos))
}

parser! {
    fn do_block[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (
            position(),
            between(token('{'), sptoken('}'), sep_by1(expr(), attempt(sptoken(';')))),
        )
            .then(|(pos, args): (_, Vec<Expr>)| {
                if args.len() < 2 {
                    unexpected_any("do must contain at least 2 expressions").left()
                } else {
                    value(ExprKind::Do { exprs: Arc::from(args) }.to_expr(pos)).right()
                }
            })
    }
}

parser! {
    fn array[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (position(), between(token('['), sptoken(']'), sep_by(expr(), csep()))).map(
            |(pos, mut args): (_, LPooled<Vec<Expr>>)| {
                ExprKind::Array { args: Arc::from_iter(args.drain(..)) }.to_expr(pos)
            },
        )
    }
}

parser! {
    fn apply_pexp[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        choice((
            attempt(spaces().with(qop(reference()))),
            between(sptoken('('), sptoken(')'), expr()),
        ))
    }
}

parser! {
    fn ref_pexp[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        choice((
            attempt(spaces().with(qop(reference()))),
            between(sptoken('('), sptoken(')'), expr()),
        ))
    }
}

parser! {
    fn structref[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (position(), ref_pexp().skip(sptoken('.')), spfname()).map(|(pos, source, field)| {
            ExprKind::StructRef { source: Arc::new(source), field }.to_expr(pos)
        })
    }
}

parser! {
    fn tupleref[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (position(), ref_pexp().skip(sptoken('.')), int::<_, usize>()).map(
            |(pos, source, field)| {
                ExprKind::TupleRef { source: Arc::new(source), field }.to_expr(pos)
            },
        )
    }
}

parser! {
    fn arrayref[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (
            position(),
            ref_pexp(),
            between(
                token('['),
                sptoken(']'),
                choice((
                    attempt(
                        (
                            position(),
                            spaces().with(optional(many1(digit()))).skip(spstring("..")),
                            spaces().with(optional(many1(digit()))),
                        )
                            .skip(look_ahead(sptoken(']'))),
                    )
                        .map(
                            |(pos, start, end): (
                                _,
                                Option<CompactString>,
                                Option<CompactString>,
                            )| {
                                let start = start.map(|i| Value::U64(i.parse().unwrap()));
                                let start = start.map(|e| ExprKind::Constant(e).to_expr(pos));
                                let end = end.map(|i| Value::U64(i.parse().unwrap()));
                                let end = end.map(|e| ExprKind::Constant(e).to_expr(pos));
                                Either::Left((start, end))
                            },
                        ),
                    attempt((
                        optional(attempt(expr())).skip(spstring("..")),
                        optional(attempt(expr())),
                    ))
                        .map(|(start, end)| Either::Left((start, end))),
                    attempt(expr()).map(|e| Either::Right(e)),
                )),
            ),
        )
            .map(|(pos, a, args)| match args {
                Either::Left((start, end)) => ExprKind::ArraySlice {
                    source: Arc::new(a),
                    start: start.map(Arc::new),
                    end: end.map(Arc::new),
                }
                .to_expr(pos),
                Either::Right(i) => {
                    ExprKind::ArrayRef { source: Arc::new(a), i: Arc::new(i) }.to_expr(pos)
                }
            })
    }
}

parser! {
    fn mapref[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (position(), ref_pexp(), between(sptoken('{'), sptoken('}'), expr())).map(|(pos, source, key)| {
            ExprKind::MapRef { source: Arc::new(source), key: Arc::new(key) }.to_expr(pos)
        })
    }
}

parser! {
    fn apply[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (
            position(),
            apply_pexp(),
            between(
                sptoken('('),
                sptoken(')'),
                sep_by(
                    choice((
                        attempt((sptoken('#').with(fname()).skip(token(':')), expr()))
                            .map(|(n, e)| (Some(n), e)),
                        attempt((
                            position(),
                            sptoken('#').with(fname()),
                        ))
                            .map(|(pos, n)| {
                                let e = ExprKind::Ref { name: [n.clone()].into() }.to_expr(pos);
                                (Some(n), e)
                            }),
                        expr().map(|e| (None, e)),
                    )),
                    csep(),
                ),
            ),
        )
            .then(|(pos, function, args): (_, Expr, Vec<(Option<ArcStr>, Expr)>)| {
                let mut anon = false;
                for (a, _) in &args {
                    if a.is_some() && anon {
                        return unexpected_any(
                            "labeled arguments must come before anonymous arguments",
                        )
                            .right();
                    }
                    anon |= a.is_none();
                }
                value((pos, function, args)).left()
            })
            .map(|(pos, function, args): (_, Expr, Vec<(Option<ArcStr>, Expr)>)| {
                ExprKind::Apply { function: Arc::new(function), args: Arc::from(args) }
                .to_expr(pos)
            })
    }
}

parser! {
    fn any[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (
            position(),
            string("any").with(between(sptoken('('), sptoken(')'), sep_by(expr(), csep()))),
        )
            .map(|(pos, args): (_, Vec<Expr>)| {
                ExprKind::Any { args: Arc::from(args) }.to_expr(pos)
            })
    }
}

fn typeprim<I>() -> impl Parser<I, Output = Typ>
where
    I: RangeStream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
    I::Range: Range,
{
    choice((
        attempt(spstring("u32").map(|_| Typ::U32)),
        attempt(spstring("v32").map(|_| Typ::V32)),
        attempt(spstring("i32").map(|_| Typ::I32)),
        attempt(spstring("z32").map(|_| Typ::Z32)),
        attempt(spstring("u64").map(|_| Typ::U64)),
        attempt(spstring("v64").map(|_| Typ::V64)),
        attempt(spstring("i64").map(|_| Typ::I64)),
        attempt(spstring("z64").map(|_| Typ::Z64)),
        attempt(spstring("f32").map(|_| Typ::F32)),
        attempt(spstring("f64").map(|_| Typ::F64)),
        attempt(spstring("decimal").map(|_| Typ::Decimal)),
        attempt(spstring("datetime").map(|_| Typ::DateTime)),
        attempt(spstring("duration").map(|_| Typ::Duration)),
        attempt(spstring("bool").map(|_| Typ::Bool)),
        attempt(spstring("string").map(|_| Typ::String)),
        attempt(spstring("bytes").map(|_| Typ::Bytes)),
        attempt(spstring("error").map(|_| Typ::Error)),
        attempt(spstring("array").map(|_| Typ::Array)),
        attempt(spstring("null").map(|_| Typ::Null)),
    ))
    .skip(not_followed_by(choice((alpha_num(), token('_')))))
}

parser! {
    fn fntype[I]()(I) -> FnType
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        spstring("fn")
            .with((
                optional(attempt(between(
                    token('<'),
                    sptoken('>'),
                    sep_by1((tvar().skip(sptoken(':')), typexp()), csep()),
                )))
                    .map(|cs: Option<LPooled<Vec<(TVar, Type)>>>| match cs {
                        Some(cs) => Arc::new(RwLock::new(cs)),
                        None => Arc::new(RwLock::new(LPooled::take())),
                    }),
                between(
                    token('('),
                    sptoken(')'),
                    sep_by(
                        choice((
                            attempt(
                                (
                                    spaces()
                                        .with(optional(token('?')).map(|o| o.is_some()))
                                        .skip(token('#')),
                                    fname().skip(token(':')),
                                    typexp(),
                                )
                                    .map(
                                        |(optional, name, typ)| {
                                            Either::Left(FnArgType {
                                                label: Some((name.into(), optional)),
                                                typ,
                                            })
                                        },
                                    ),
                            ),
                            attempt(
                                typexp()
                                    .map(|typ| Either::Left(FnArgType { label: None, typ })),
                            ),
                            attempt(
                                spstring("@args:").with(typexp()).map(|e| Either::Right(e)),
                            ),
                        )),
                        csep(),
                    ),
                ),
                spstring("->").with(typexp()),
                optional(attempt(space().with(spstring("throws")).with(space()).with(typexp())))
            ))
            .then(
                |(constraints, mut args, rtype, throws): (
                    Arc<RwLock<LPooled<Vec<(TVar, Type)>>>>,
                    Vec<Either<FnArgType, Type>>,
                    Type,
                    Option<Type>
                )| {
                    let vargs = match args.pop() {
                        None => None,
                        Some(Either::Right(t)) => Some(t),
                        Some(Either::Left(t)) => {
                            args.push(Either::Left(t));
                            None
                        }
                    };
                    if !args.iter().all(|a| a.is_left()) {
                        return unexpected_any(
                            "vargs must appear once at the end of the args",
                        )
                            .left();
                    }
                    let args = Arc::from_iter(args.into_iter().map(|t| match t {
                        Either::Left(t) => t,
                        Either::Right(_) => unreachable!(),
                    }));
                    let mut anon = false;
                    for a in args.iter() {
                        if anon && a.label.is_some() {
                            return unexpected_any(
                                "anonymous args must appear after labeled args",
                            )
                                .left();
                        }
                        anon |= a.label.is_none();
                    }
                    let throws = throws.unwrap_or(Type::Bottom);
                    value(FnType { args, vargs, rtype, constraints, throws }).right()
                },
            )
    }
}

fn tvar<I>() -> impl Parser<I, Output = TVar>
where
    I: RangeStream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
    I::Range: Range,
{
    sptoken('\'').with(fname()).map(TVar::empty_named)
}

parser! {
    fn typexp[I]()(I) -> Type
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        choice((
            attempt(sptoken('&').with(typexp()).map(|t| Type::ByRef(Arc::new(t)))),
            attempt(sptoken('_').map(|_| Type::Bottom)),
            attempt(
                between(sptoken('['), sptoken(']'), sep_by(typexp(), csep()))
                    .map(|mut ts: LPooled<Vec<Type>>| Type::flatten_set(ts.drain(..))),
            ),
            attempt(between(sptoken('('), sptoken(')'), sep_by1(typexp(), csep())).map(
                |mut exps: LPooled<Vec<Type>>| {
                    if exps.len() == 1 {
                        exps.pop().unwrap()
                    } else {
                        Type::Tuple(Arc::from_iter(exps.drain(..)))
                    }
                },
            )),
            attempt(
                between(
                    sptoken('{'),
                    sptoken('}'),
                    sep_by1((spfname().skip(sptoken(':')), typexp()), csep()),
                )
                    .then(|mut exps: LPooled<Vec<(ArcStr, Type)>>| {
                        let s = exps.iter().map(|(n, _)| n).collect::<LPooled<FxHashSet<_>>>();
                        if s.len() < exps.len() {
                            return unexpected_any("struct field names must be unique").left();
                        }
                        drop(s);
                        exps.sort_by_key(|(n, _)| n.clone());
                        value(Type::Struct(Arc::from_iter(exps.drain(..)))).right()
                    }),
            ),
            attempt(
                (
                    sptoken('`').with(typname()),
                    optional(attempt(between(
                        token('('),
                        sptoken(')'),
                        sep_by1(typexp(), csep()),
                    ))),
                )
                    .map(|(tag, typs): (ArcStr, Option<LPooled<Vec<Type>>>)| {
                        let mut t = match typs {
                            None => LPooled::take(),
                            Some(v) => v,
                        };
                        Type::Variant(tag.clone(), Arc::from_iter(t.drain(..)))
                    }),
            ),
            attempt(fntype().map(|f| Type::Fn(Arc::new(f)))),
            attempt(spstring("Array").with(between(sptoken('<'), sptoken('>'), typexp())))
                .map(|t| Type::Array(Arc::new(t))),
            attempt(spstring("Map").with(
                between(sptoken('<'), sptoken('>'),
                    (typexp().skip(sptoken(',')), typexp())
                )))
                .map(|(k, v)| Type::Map { key: Arc::new(k), value: Arc::new(v) }),
            attempt(spstring("Error").with(between(sptoken('<'), sptoken('>'), typexp())))
                .map(|t| Type::Error(Arc::new(t))),
            attempt((
                sptypath(),
                optional(attempt(between(
                    sptoken('<'),
                    sptoken('>'),
                    sep_by1(typexp(), csep()),
                ))),
            ))
                .map(|(n, params): (ModPath, Option<LPooled<Vec<Type>>>)| {
                    let params = params
                        .map(|mut a| Arc::from_iter(a.drain(..)))
                        .unwrap_or_else(|| Arc::from_iter([]));
                    Type::Ref { scope: ModPath::root(), name: n, params }
                }),
            attempt(spstring("Any")).map(|_| Type::Any),
            attempt(typeprim()).map(|typ| Type::Primitive(typ.into())),
            attempt(tvar()).map(|tv| Type::TVar(tv)),
        ))
    }
}

parser! {
    fn lambda_args[I]()(I) -> (LPooled<Vec<Arg>>, Option<Option<Type>>)
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        sep_by(
            (
                choice((
                    attempt(spaces().with(structure_pattern())).map(|p| (false, p)),
                    attempt(spaces().with(token('#').with(fname())))
                        .map(|b| (true, StructurePattern::Bind(b))),
                    attempt(spstring("@args"))
                        .map(|s| (false, StructurePattern::Bind(ArcStr::from(s)))),
                )),
                optional(attempt(sptoken(':').with(typexp()))),
                optional(attempt(sptoken('=').with(expr()))),
            ),
            csep(),
        )
            .then(|v: Vec<((bool, StructurePattern), Option<Type>, Option<Expr>)>| {
                let args = v
                    .into_iter()
                    .map(|((labeled, pattern), constraint, default)| {
                        if !labeled && default.is_some() {
                            bail!("labeled")
                        } else {
                            Ok(Arg { labeled: labeled.then_some(default), pattern, constraint })
                        }
                    })
                    .collect::<Result<LPooled<Vec<_>>>>();
                match args {
                    Ok(a) => value(a).right(),
                    Err(_) => {
                        unexpected_any("only labeled arguments may have a default value").left()
                    }
                }
            })
        // @args must be last
            .then(|mut v: LPooled<Vec<Arg>>| {
                match v.iter().enumerate().find(|(_, a)| match &a.pattern {
                    StructurePattern::Bind(n) if n == "@args" => true,
                    _ => false,
                }) {
                    None => value((v, None)).left(),
                    Some((i, _)) => {
                        if i == v.len() - 1 {
                            let a = v.pop().unwrap();
                            value((v, Some(a.constraint))).left()
                        } else {
                            unexpected_any("@args must be the last argument").right()
                        }
                    }
                }
            })
        // labeled before anonymous args
            .then(|(v, vargs): (LPooled<Vec<Arg>>, Option<Option<Type>>)| {
                let mut anon = false;
                for a in v.iter() {
                    if a.labeled.is_some() && anon {
                        return unexpected_any("labeled args must come before anon args").right();
                    }
                    anon |= a.labeled.is_none();
                }
                value((v, vargs)).left()
            })
    }
}

parser! {
    fn lambda[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (
            position(),
            attempt(sep_by((tvar().skip(sptoken(':')), typexp()), csep()))
                .map(|mut tvs: LPooled<Vec<(TVar, Type)>>| Arc::from_iter(tvs.drain(..))),
            between(sptoken('|'), sptoken('|'), lambda_args()),
            optional(attempt(spstring("->").with(typexp()))),
            optional(attempt(space().with(spstring("throws").with(space()).with(typexp())))),
            space().with(choice((
                attempt(sptoken('\'').with(fname()).skip(not_followed_by(sptoken(':'))))
                    .map(Either::Right),
                expr().map(|e| Either::Left(e)),
            ))),
        )
            .map(|(pos, constraints, (mut args, vargs), rtype, throws, body)| {
                let args = Arc::from_iter(args.drain(..));
                ExprKind::Lambda(Arc::new(Lambda { args, vargs, rtype, throws, constraints, body }))
                    .to_expr(pos)
            })
    }
}

parser! {
    fn letbind[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (
            position(),
            doc_comment(),
            optional(string("pub").skip(space())).map(|o| o.is_some()),
            spstring("let")
                .with(space())
                .with((
                    optional(attempt(spstring("rec").with(space()))),
                    structure_pattern(),
                    optional(attempt(sptoken(':').with(typexp())))
                ))
                .skip(spstring("=")),
            expr(),
        )
            .map(|(pos, doc, export, (rec, pattern, typ), value)| {
                let rec = rec.is_some();
                ExprKind::Bind(Arc::new(Bind { rec, doc, export, pattern, typ, value }))
                    .to_expr(pos)
            })
    }
}

parser! {
    fn connect[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (position(), optional(token('*')), spmodpath().skip(spstring("<-")), expr()).map(
            |(pos, deref, name, e)| {
                ExprKind::Connect { name, value: Arc::new(e), deref: deref.is_some() }
                .to_expr(pos)
            },
        )
    }
}

fn literal<I>() -> impl Parser<I, Output = Expr>
where
    I: RangeStream<Token = char, Position = SourcePosition>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
    I::Range: Range,
{
    (position(), parse_value(&VAL_MUST_ESC, &VAL_ESC).skip(not_followed_by(token('_'))))
        .then(|(pos, v)| match v {
            Value::String(_) => {
                unexpected_any("parse error in string interpolation").left()
            }
            v => value(ExprKind::Constant(v).to_expr(pos)).right(),
        })
}

fn reference<I>() -> impl Parser<I, Output = Expr>
where
    I: RangeStream<Token = char, Position = SourcePosition>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
    I::Range: Range,
{
    (position(), modpath()).map(|(pos, name)| ExprKind::Ref { name }.to_expr(pos))
}

parser! {
    fn deref_arith[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (position(), token('*').with(arith_term()))
            .map(|(pos, expr)| ExprKind::Deref(Arc::new(expr)).to_expr(pos))
    }
}

parser! {
    fn qop[I, P](p: P)(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range, P: Parser<I, Output = Expr>]
    {
        enum Op {
            Qop,
            OrNever,
        }
        (
            position(),
            p,
            optional(choice((
                attempt(sptoken('?')).map(|_| Op::Qop),
                attempt(sptoken('$')).map(|_| Op::OrNever)
            )))
        ).map(|(pos, e, qop)| match qop {
            None => e,
            Some(Op::Qop) => ExprKind::Qop(Arc::new(e)).to_expr(pos),
            Some(Op::OrNever) => ExprKind::OrNever(Arc::new(e)).to_expr(pos)
        })
    }
}

parser! {
    fn arith_term[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        choice((
            attempt(spaces().with(qop(deref_arith()))),
            attempt(spaces().with(raw_string())),
            attempt(spaces().with(array())),
            attempt(spaces().with(byref_arith())),
            attempt(spaces().with(tuple())),
            attempt(spaces().with(structure())),
            attempt(spaces().with(map())),
            attempt(spaces().with(variant())),
            attempt(spaces().with(structwith())),
            attempt(spaces().with(qop(arrayref()))),
            attempt(spaces().with(qop(tupleref()))),
            attempt(spaces().with(qop(structref()))),
            attempt(spaces().with(qop(mapref()))),
            attempt(spaces().with(qop(apply()))),
            attempt(spaces().with(qop(do_block()))),
            attempt(spaces().with(qop(select()))),
            attempt(spaces().with(qop(cast()))),
            attempt(spaces().with(qop(any()))),
            attempt(spaces().with(interpolated())),
            attempt(spaces().with(literal())),
            attempt(spaces().with(qop(reference()))),
            attempt(
                (position(), sptoken('!').with(arith()))
                    .map(|(pos, expr)| ExprKind::Not { expr: Arc::new(expr) }.to_expr(pos)),
            ),
            attempt(between(sptoken('('), sptoken(')'), arith())),
        ))
            .skip(spaces())
    }
}

parser! {
    fn arith[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        choice((
            attempt(chainl1(
                arith_term(),
                choice((
                    attempt(spstring("+")),
                    attempt(spstring("-")),
                    attempt(spstring("*")),
                    attempt(spstring("/")),
                    attempt(spstring("%")),
                    attempt(spstring("==")),
                    attempt(spstring("!=")),
                    attempt(spstring(">=")),
                    attempt(spstring("<=")),
                    attempt(spstring(">")),
                    attempt(spstring("<")),
                    attempt(spstring("&&")),
                    attempt(spstring("||")),
                    attempt(spstring("~")),
                ))
                    .map(|op: &str| match op {
                        "+" => |lhs: Expr, rhs: Expr| {
                            let pos = lhs.pos;
                            ExprKind::Add { lhs: Arc::new(lhs), rhs: Arc::new(rhs) }.to_expr(pos)
                        },
                        "-" => |lhs: Expr, rhs: Expr| {
                            let pos = lhs.pos;
                            ExprKind::Sub { lhs: Arc::new(lhs), rhs: Arc::new(rhs) }.to_expr(pos)
                        },
                        "*" => |lhs: Expr, rhs: Expr| {
                            let pos = lhs.pos;
                            ExprKind::Mul { lhs: Arc::new(lhs), rhs: Arc::new(rhs) }.to_expr(pos)
                        },
                        "/" => |lhs: Expr, rhs: Expr| {
                            let pos = lhs.pos;
                            ExprKind::Div { lhs: Arc::new(lhs), rhs: Arc::new(rhs) }.to_expr(pos)
                        },
                        "%" => |lhs: Expr, rhs: Expr| {
                            let pos = lhs.pos;
                            ExprKind::Mod { lhs: Arc::new(lhs), rhs: Arc::new(rhs) }.to_expr(pos)
                        },
                        "==" => |lhs: Expr, rhs: Expr| {
                            let pos = lhs.pos;
                            ExprKind::Eq { lhs: Arc::new(lhs), rhs: Arc::new(rhs) }.to_expr(pos)
                        },
                        "!=" => |lhs: Expr, rhs: Expr| {
                            let pos = lhs.pos;
                            ExprKind::Ne { lhs: Arc::new(lhs), rhs: Arc::new(rhs) }.to_expr(pos)
                        },
                        ">" => |lhs: Expr, rhs: Expr| {
                            let pos = lhs.pos;
                            ExprKind::Gt { lhs: Arc::new(lhs), rhs: Arc::new(rhs) }.to_expr(pos)
                        },
                        "<" => |lhs: Expr, rhs: Expr| {
                            let pos = lhs.pos;
                            ExprKind::Lt { lhs: Arc::new(lhs), rhs: Arc::new(rhs) }.to_expr(pos)
                        },
                        ">=" => |lhs: Expr, rhs: Expr| {
                            let pos = lhs.pos;
                            ExprKind::Gte { lhs: Arc::new(lhs), rhs: Arc::new(rhs) }.to_expr(pos)
                        },
                        "<=" => |lhs: Expr, rhs: Expr| {
                            let pos = lhs.pos;
                            ExprKind::Lte { lhs: Arc::new(lhs), rhs: Arc::new(rhs) }.to_expr(pos)
                        },
                        "&&" => |lhs: Expr, rhs: Expr| {
                            let pos = lhs.pos;
                            ExprKind::And { lhs: Arc::new(lhs), rhs: Arc::new(rhs) }.to_expr(pos)
                        },
                        "||" => |lhs: Expr, rhs: Expr| {
                            let pos = lhs.pos;
                            ExprKind::Or { lhs: Arc::new(lhs), rhs: Arc::new(rhs) }.to_expr(pos)
                        },
                        "~" => |lhs: Expr, rhs: Expr| {
                            let pos = lhs.pos;
                            ExprKind::Sample { lhs: Arc::new(lhs), rhs: Arc::new(rhs) }
                            .to_expr(pos)
                        },
                        _ => unreachable!(),
                    }),
            )),
            attempt((position(), sptoken('!').with(arith_term())))
                .map(|(pos, expr)| ExprKind::Not { expr: Arc::new(expr) }.to_expr(pos)),
            attempt(between(sptoken('('), sptoken(')'), arith())),
        ))
    }
}

parser! {
    fn slice_pattern[I]()(I) -> StructurePattern
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        macro_rules! all_left {
            ($pats:expr) => {{
                let mut err = false;
                let pats: Arc<[StructurePattern]> =
                    Arc::from_iter($pats.drain(..).map(|s| match s {
                        Either::Left(s) => s,
                        Either::Right(_) => {
                            err = true;
                            StructurePattern::Ignore
                        }
                    }));
                if err {
                    return unexpected_any("invalid pattern").left();
                }
                pats
            }};
        }
        (
            optional(attempt(spfname().skip(sptoken('@')))),
            between(
                sptoken('['),
                sptoken(']'),
                sep_by(
                    choice((
                        attempt(spstring("..")).map(|_| Either::Right(None)),
                        attempt(spfname().skip(spstring("..")))
                            .map(|n| Either::Right(Some(n))),
                        structure_pattern().map(|p| Either::Left(p)),
                    )),
                    csep(),
                ),
            ),
        )
            .then(
                |(all, mut pats): (
                    Option<ArcStr>,
                    LPooled<Vec<Either<StructurePattern, Option<ArcStr>>>>,
                )| {
                    if pats.len() == 0 {
                        value(StructurePattern::Slice { all, binds: Arc::from_iter([]) })
                            .right()
                    } else if pats.len() == 1 {
                        match pats.pop().unwrap() {
                            Either::Left(s) => value(StructurePattern::Slice {
                                all,
                                binds: Arc::from_iter([s]),
                            })
                                .right(),
                            Either::Right(_) => {
                                unexpected_any("invalid singular range match").left()
                            }
                        }
                    } else {
                        match (&pats[0], &pats[pats.len() - 1]) {
                            (Either::Right(_), Either::Right(_)) => {
                                unexpected_any("invalid pattern").left()
                            }
                            (Either::Right(_), Either::Left(_)) => {
                                let head = pats.remove(0).right().unwrap();
                                let suffix = all_left!(pats);
                                value(StructurePattern::SliceSuffix { all, head, suffix })
                                    .right()
                            }
                            (Either::Left(_), Either::Right(_)) => {
                                let tail = pats.pop().unwrap().right().unwrap();
                                let prefix = all_left!(pats);
                                value(StructurePattern::SlicePrefix { all, tail, prefix })
                                    .right()
                            }
                            (Either::Left(_), Either::Left(_)) => {
                                value(StructurePattern::Slice { all, binds: all_left!(pats) })
                                    .right()
                            }
                        }
                    }
                },
            )
    }
}

fn raw_string<I>() -> impl Parser<I, Output = Expr>
where
    I: RangeStream<Token = char, Position = SourcePosition>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
    I::Range: Range,
{
    static MUST_ESC: [char; 2] = ['\\', '\''];
    static ESC: LazyLock<Escape> =
        LazyLock::new(|| Escape::new('\\', &MUST_ESC, &[], None).unwrap());
    (position(), between(string("r\'"), token('\''), escaped_string(&MUST_ESC, &ESC)))
        .map(|(pos, s): (_, String)| {
            ExprKind::Constant(Value::String(s.into())).to_expr(pos)
        })
}

parser! {
    fn tuple_pattern[I]()(I) -> StructurePattern
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (
            optional(attempt(spfname().skip(sptoken('@')))),
            between(sptoken('('), sptoken(')'), sep_by1(structure_pattern(), csep())),
        )
            .then(|(all, mut binds): (Option<ArcStr>, LPooled<Vec<StructurePattern>>)| {
                if binds.len() < 2 {
                    unexpected_any("tuples must have at least 2 elements").left()
                } else {
                    value(StructurePattern::Tuple { all, binds: Arc::from_iter(binds.drain(..)) })
                        .right()
                }
            })
    }
}

parser! {
    fn variant_pattern[I]()(I) -> StructurePattern
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (
            optional(attempt(spfname().skip(sptoken('@')))),
            sptoken('`').with(typname()),
            optional(attempt(between(
                sptoken('('),
                sptoken(')'),
                sep_by1(structure_pattern(), csep()),
            ))),
        )
            .map(
                |(all, tag, binds): (
                    Option<ArcStr>,
                    ArcStr,
                    Option<LPooled<Vec<StructurePattern>>>,
                )| {
                    let mut binds = match binds {
                        None => LPooled::take(),
                        Some(a) => a,
                    };
                    StructurePattern::Variant { all, tag, binds: Arc::from_iter(binds.drain(..)) }
                },
            )
    }
}

parser! {
    fn struct_pattern[I]()(I) -> StructurePattern
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (
            optional(attempt(spfname().skip(sptoken('@')))),
            between(
                sptoken('{'),
                sptoken('}'),
                sep_by1(
                    choice((
                        attempt((spfname().skip(sptoken(':')), structure_pattern()))
                            .map(|(s, p)| (s, p, true)),
                        attempt(spfname()).map(|s| {
                            let p = StructurePattern::Bind(s.clone());
                            (s, p, true)
                        }),
                        spstring("..")
                            .map(|_| (literal!(""), StructurePattern::Ignore, false)),
                    )),
                    csep(),
                ),
            ),
        )
            .then(
                |(all, mut binds): (
                    Option<ArcStr>,
                    LPooled<Vec<(ArcStr, StructurePattern, bool)>>,
                )| {
                    let mut exhaustive = true;
                    binds.retain(|(_, _, ex)| {
                        exhaustive &= *ex;
                        *ex
                    });
                    binds.sort_by_key(|(s, _, _)| s.clone());
                    let s = binds.iter().map(|(s, _, _)| s).collect::<LPooled<FxHashSet<_>>>();
                    if s.len() < binds.len() {
                        unexpected_any("struct fields must be unique").left()
                    } else {
                        drop(s);
                        let binds = Arc::from_iter(binds.drain(..).map(|(s, p, _)| (s, p)));
                        value(StructurePattern::Struct { all, exhaustive, binds }).right()
                    }
                },
            )
    }
}

parser! {
    fn structure_pattern[I]()(I) -> StructurePattern
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        choice((
            attempt(slice_pattern()),
            attempt(tuple_pattern()),
            attempt(struct_pattern()),
            attempt(variant_pattern()),
            attempt(parse_value(&VAL_MUST_ESC, &VAL_ESC).skip(not_followed_by(token('_'))))
                .map(|v| StructurePattern::Literal(v)),
            attempt(sptoken('_')).map(|_| StructurePattern::Ignore),
            spfname().map(|name| StructurePattern::Bind(name)),
        ))
    }
}

parser! {
    fn pattern[I]()(I) -> Pattern
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (
            optional(attempt(typexp().skip(space().with(spstring("as "))))),
            structure_pattern(),
            optional(attempt(space().with(spstring("if").with(space()).with(expr())))),
        )
            .map(
                |(type_predicate, structure_predicate, guard): (
                    Option<Type>,
                    StructurePattern,
                    Option<Expr>,
                )| { Pattern { type_predicate, structure_predicate, guard } },
            )
    }
}

parser! {
    fn select[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (
            position(),
            string("select").with(space()).with((
                expr(),
                between(
                    sptoken('{'),
                    sptoken('}'),
                    sep_by1((pattern(), spstring("=>").with(expr())), csep()),
                ),
            )),
        )
            .map(|(pos, (arg, arms)): (_, (Expr, Vec<(Pattern, Expr)>))| {
                ExprKind::Select { arg: Arc::new(arg), arms: Arc::from(arms) }.to_expr(pos)
            })
    }
}

parser! {
    fn cast[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (
            position(),
            string("cast").with(between(token('<'), sptoken('>'), typexp())),
            between(sptoken('('), sptoken(')'), expr()),
        )
            .map(|(pos, typ, e)| ExprKind::TypeCast { expr: Arc::new(e), typ }.to_expr(pos))
    }
}

parser! {
    fn typedef[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (
            position(),
            string("type").with(sptypname()),
            optional(attempt(between(
                sptoken('<'),
                sptoken('>'),
                sep_by1((tvar(), optional(attempt(sptoken(':').with(typexp())))), csep()),
            ))),
            sptoken('=').with(typexp()),
        )
            .map(|(pos, name, params, typ)| {
                let params = params
                    .map(|mut ps: LPooled<Vec<(TVar, Option<Type>)>>| {
                        Arc::from_iter(ps.drain(..))
                    })
                    .unwrap_or_else(|| Arc::<[(TVar, Option<Type>)]>::from(Vec::new()));
                ExprKind::TypeDef(TypeDef { name, params, typ }).to_expr(pos)
            })
    }
}

parser! {
    fn tuple[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (position(), between(token('('), sptoken(')'), sep_by1(expr(), csep()))).then(
            |(pos, mut exprs): (_, LPooled<Vec<Expr>>)| {
                if exprs.len() < 2 {
                    unexpected_any("tuples must have at least 2 elements").left()
                } else {
                    value(ExprKind::Tuple { args: Arc::from_iter(exprs.drain(..)) }.to_expr(pos))
                        .right()
                }
            },
        )
    }
}

parser! {
    fn structure[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (
            position(),
            between(
                token('{'),
                sptoken('}'),
                sep_by1((spfname(), optional(attempt(sptoken(':')).with(expr()))), csep()),
            ),
        )
            .then(|(pos, mut exprs): (_, LPooled<Vec<(ArcStr, Option<Expr>)>>)| {
                let s = exprs.iter().map(|(n, _)| n).collect::<LPooled<FxHashSet<_>>>();
                if s.len() < exprs.len() {
                    return unexpected_any("struct fields must be unique").left();
                }
                drop(s);
                exprs.sort_by_key(|(n, _)| n.clone());
                let args = exprs.drain(..).map(|(n, e)| match e {
                    Some(e) => (n, e),
                    None => {
                        let e = ExprKind::Ref { name: [n.clone()].into() }.to_expr(pos);
                        (n, e)
                    }
                });
                value(ExprKind::Struct { args: Arc::from_iter(args) }.to_expr(pos)).right()
            })
    }
}

parser! {
    fn map[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (
            position(),
            between(
                token('{'),
                sptoken('}'),
                sep_by((expr(), spstring("=>").with(expr())), csep()),
            ),
        )
            .map(|(pos, mut args): (_, LPooled<Vec<(Expr, Expr)>>)| {
                ExprKind::Map { args: Arc::from_iter(args.drain(..)) }.to_expr(pos)
            })
    }
}

parser! {
    fn variant[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (
            position(),
            token('`').with(typname()),
            optional(attempt(between(token('('), sptoken(')'), sep_by1(expr(), csep())))),
        )
            .map(|(pos, tag, args): (_, ArcStr, Option<LPooled<Vec<Expr>>>)| {
                let mut args = match args {
                    None => LPooled::take(),
                    Some(a) => a,
                };
                ExprKind::Variant { tag, args: Arc::from_iter(args.drain(..)) }.to_expr(pos)
            })
    }
}

parser! {
    fn structwith[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (
            position(),
            between(
                token('{'),
                sptoken('}'),
                (
                    ref_pexp().skip(space()).skip(spstring("with")).skip(space()),
                    sep_by1((spfname(), optional(attempt(sptoken(':').with(expr())))), csep()),
                ),
            ),
        )
            .then(
                |(pos, (source, mut exprs)): (_, (Expr, LPooled<Vec<(ArcStr, Option<Expr>)>>))| {
                    let s = exprs.iter().map(|(n, _)| n).collect::<LPooled<FxHashSet<_>>>();
                    if s.len() < exprs.len() {
                        return unexpected_any("struct fields must be unique").left();
                    }
                    drop(s);
                    exprs.sort_by_key(|(n, _)| n.clone());
                    let exprs = exprs.drain(..).map(|(name, e)| match e {
                        Some(e) => (name, e),
                        None => {
                            let e = ExprKind::Ref { name: ModPath::from([name.clone()]) }.to_expr(pos);
                            (name, e)
                        }
                    });
                    let e = ExprKind::StructWith {
                        source: Arc::new(source),
                        replace: Arc::from_iter(exprs),
                    }
                    .to_expr(pos);
                    value(e).right()
                },
            )
    }
}

parser! {
    fn try_catch[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (
            position().skip(string("try")).skip(space()),
            sep_by1(expr(), attempt(sptoken(';'))),
            spstring("catch").with(
                between(
                    sptoken('('),
                    sptoken(')'),
                    (spfname(), optional(attempt(sptoken(':').with(typexp()))))
                )
            ),
            spstring("=>").with(expr())
        )
            .map(|(pos, mut exprs, (bind, constraint), handler):
                  (_, LPooled<Vec<Expr>>, _, _)|
            {
                ExprKind::TryCatch(Arc::new(TryCatch {
                    bind,
                    constraint,
                    exprs: Arc::from_iter(exprs.drain(..)),
                    handler: Arc::new(handler)
                })).to_expr(pos)
            })
    }
}

parser! {
    fn byref[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (position(), token('&').with(expr()))
            .map(|(pos, expr)| ExprKind::ByRef(Arc::new(expr)).to_expr(pos))
    }
}

parser! {
    fn byref_arith[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (position(), token('&').with(arith_term()))
            .map(|(pos, expr)| ExprKind::ByRef(Arc::new(expr)).to_expr(pos))
    }
}

parser! {
    fn deref[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        (position(), token('*').with(expr()))
            .map(|(pos, expr)| ExprKind::Deref(Arc::new(expr)).to_expr(pos))
    }
}

parser! {
    fn expr[I]()(I) -> Expr
    where [I: RangeStream<Token = char, Position = SourcePosition>, I::Range: Range]
    {
        choice((
            attempt(choice((
                attempt(spaces().with(try_catch())),
                attempt(spaces().with(module())),
                attempt(spaces().with(use_module())),
                attempt(spaces().with(typedef())),
                attempt(spaces().with(raw_string())),
                attempt(spaces().with(array())),
                attempt(spaces().with(byref())),
                attempt(spaces().with(connect())),
                attempt(spaces().with(arith())),
                attempt(spaces().with(qop(mapref()))),
                attempt(spaces().with(qop(arrayref()))),
                attempt(spaces().with(qop(tupleref()))),
                attempt(spaces().with(qop(structref()))),
                attempt(spaces().with(qop(deref()))),
                attempt(spaces().with(qop(apply()))),
                attempt(spaces().with(tuple())),
                attempt(spaces().with(between(token('('), sptoken(')'), expr()))),
            ))),
            attempt(spaces().with(structure())),
            attempt(spaces().with(map())),
            attempt(spaces().with(variant())),
            attempt(spaces().with(structwith())),
            attempt(spaces().with(qop(do_block()))),
            attempt(spaces().with(lambda())),
            attempt(spaces().with(letbind())),
            attempt(spaces().with(qop(select()))),
            attempt(spaces().with(qop(cast()))),
            attempt(spaces().with(qop(any()))),
            attempt(spaces().with(interpolated())),
            attempt(spaces().with(literal())),
            attempt(spaces().with(qop(reference())))
        ))
    }
}

/// Parse one or more expressions
///
/// followed by (optional) whitespace and then eof. At least one
/// expression is required otherwise this function will fail.
pub fn parse(ori: Origin) -> anyhow::Result<Arc<[Expr]>> {
    let ori = Arc::new(ori);
    set_origin(ori.clone());
    let mut r: LPooled<Vec<Option<Expr>>> = sep_by1(
        choice((expr().map(Some), look_ahead(spaces().with(eof())).map(|_| None))),
        attempt(sptoken(';')),
    )
    .skip(spaces())
    .skip(eof())
    .easy_parse(position::Stream::new(&*ori.text))
    .map(|(r, _)| r)
    .map_err(|e| anyhow::anyhow!(format!("{}", e)))?;
    Ok(Arc::from_iter(r.drain(..).filter_map(|e| e)))
}

/// Parse one and only one expression.
pub fn parse_one(s: &str) -> anyhow::Result<Expr> {
    expr()
        .skip(spaces())
        .skip(eof())
        .easy_parse(position::Stream::new(&*s))
        .map(|(r, _)| r)
        .map_err(|e| anyhow::anyhow!(format!("{e}")))
}

pub fn test_parse_mapref(s: &str) -> anyhow::Result<Expr> {
    mapref()
        .skip(spaces())
        .skip(eof())
        .easy_parse(position::Stream::new(&*s))
        .map(|(r, _)| r)
        .map_err(|e| anyhow::anyhow!(format!("{e}")))
}

/// Parse one fntype expression
pub fn parse_fn_type(s: &str) -> anyhow::Result<FnType> {
    fntype()
        .skip(spaces())
        .skip(eof())
        .easy_parse(position::Stream::new(s))
        .map(|(r, _)| r)
        .map_err(|e| anyhow::anyhow!(format!("{e}")))
}

/// Parse one type expression
pub fn parse_type(s: &str) -> anyhow::Result<Type> {
    typexp()
        .skip(spaces())
        .skip(eof())
        .easy_parse(position::Stream::new(s))
        .map(|(r, _)| r)
        .map_err(|e| anyhow::anyhow!(format!("{e}")))
}

pub(super) fn parse_modpath(s: &str) -> anyhow::Result<ModPath> {
    modpath()
        .skip(spaces())
        .skip(eof())
        .easy_parse(position::Stream::new(s))
        .map(|(r, _)| r)
        .map_err(|e| anyhow::anyhow!(format!("{e}")))
}
