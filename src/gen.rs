use std::io;
use std::io::Write;
use rand::{Rng, RngCore, Error};
use rand::seq::SliceRandom;
use regex_syntax::Parser;

use crate::Configuration;

struct Context<'t, T: Write, R: Rng> {
    conf: &'t Configuration,
    out: &'t mut T,
    rng: &'t mut R,
    depth: u32,
}

impl<'t, T: Write, R: Rng> Write for Context<'t, T, R> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.out.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.out.flush()
    }
}

impl<'t, T: Write, R: Rng> RngCore for Context<'t, T, R> {
    fn next_u32(&mut self) -> u32 {
        self.rng.next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.rng.next_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.rng.fill_bytes(dest)
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        self.rng.try_fill_bytes(dest)
    }
}

impl<'t, T: Write, R: Rng> Context<'t, T, R> {
    fn get_regex_parser(&self) -> Parser {
        regex_syntax::ParserBuilder::new().unicode(!self.conf.ascii_only).build()
    }

    fn write_debug(&mut self, s: &str) {
        if self.conf.debug {
            self.out.write(s.as_bytes()).unwrap();
        }
    }
}

pub fn document<'t, T: Write + 'static, R: Rng + 'static>(out: &mut T, rng: &mut R, conf: &Configuration) -> io::Result<usize> {
    let ctx: &mut Context<T, R> = &mut Context {
        conf,
        out,
        rng,
        depth: 0,
    };

    let result = nodes()(ctx);
    ctx.flush().unwrap();
    result
}

// nodes := linespace* (node nodes?)? linespace*
fn nodes<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        if ctx.depth > ctx.conf.depth_max {
            return Ok(0);
        }

        ctx.write_debug("<NODES>");

        ctx.depth += 1;
        let result = concat(vec![
            repeat(linespace(), 0, ctx.conf.blank_lines_max),
            repeat(node(), 0, ctx.conf.nodes_per_child_max),
            repeat(linespace(), 0, ctx.conf.blank_lines_max),
        ])(ctx);
        ctx.depth -= 1;

        ctx.write_debug("</NODES>");

        result
    })
}

// node := ('/-' node-space*)? type? identifier (node-space+ node-prop-or-arg)* (node-space* node-children ws*)? node-space* node-terminator
fn node<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        ctx.write_debug("<NODE>");
        let result = concat(vec![
            maybe(concat(vec![
                write_literal("/-"),
                repeat(node_space(), 0, ctx.conf.extra_space_max),
            ])),
            maybe(type_rule()),
            identifier(),
            repeat(concat(vec![
                repeat(node_space(), 1, ctx.conf.extra_space_max),
                node_prop_or_arg(),
            ]), 0, ctx.conf.props_or_args_max),
            maybe(concat(vec![
                repeat(node_space(), 0, ctx.conf.extra_space_max),
                node_children(),
                repeat(ws(), 0, ctx.conf.extra_space_max),
            ])),
            repeat(node_space(), 0, ctx.conf.extra_space_max),
            node_terminator(),
        ])(ctx);

        ctx.write_debug("</NODE>");
        result
    })
}

// node-prop-or-arg := ('/-' node-space*)? (prop | value)
fn node_prop_or_arg<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        ctx.write_debug("<NODE-PROP-OR-ARG>");

        let result = concat(vec![
            maybe(concat(vec![
                write_literal("/-"),
                repeat(node_space(), 0, ctx.conf.extra_space_max)
            ])),
            select(vec![prop(), value()])
        ])(ctx);

        ctx.write_debug("</NODE-PROP-OR-ARG>");

        result
    })
}

// node-children := ('/-' node-space*)? {' nodes '}'
fn node_children<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        ctx.write_debug("<NODE-CHILDREN>");

        let result = concat(vec![
            maybe(concat(vec![
                write_literal("/-"),
                repeat(node_space(), 0, ctx.conf.extra_space_max),
            ])),
            write_literal("{"),
            nodes(),
            write_literal("}"),
        ])(ctx);

        ctx.write_debug("</NODE-CHILDREN>");

        result
    })
}

// node-space := ws* escline ws* | ws+
fn node_space<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        ctx.write_debug("<NODE-SPACE>");

        let result = select(vec![
            concat(vec![
                repeat(ws(), 0, ctx.conf.extra_space_max),
                escline(),
                repeat(ws(), 0, ctx.conf.extra_space_max),
            ]),
            repeat(ws(), 1, ctx.conf.extra_space_max),
        ])(ctx);

        ctx.write_debug("</NODE-SPACE>");

        result
    })
}

// node-terminator := single-line-comment | newline | ';' | eof
fn node_terminator<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        ctx.write_debug("<NODE-TERMINATOR>");

        let result = select(vec![
            single_line_comment(),
            newline(),
            write_literal(";"),
        ])(ctx);

        ctx.write_debug("</NODE-TERMINATOR>");

        result
    })
}

// identifier := string | bare-identifier
fn identifier<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        ctx.write_debug("<IDENTIFIER>");

        let result = select(vec![string_rule(), bare_identifier()])(ctx);

        ctx.write_debug("</IDENTIFIER>");

        result
    })
}

// bare-identifier := ((identifier-char - digit - sign) identifier-char*| sign ((identifier-char - digit) identifier-char*)?) - keyword
fn bare_identifier<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        ctx.write_debug("<BARE-IDENTIFIER>");

        let result = select(vec![
            concat(vec![
                identifier_char_minus_digit_and_sign(),
                repeat(identifier_char(), 0, ctx.conf.identifier_len_max - 1),
            ]),
            concat(vec![
                sign(),
                maybe(concat(vec![
                    identifier_char_minus_digit(),
                    repeat(identifier_char(), 0, ctx.conf.identifier_len_max - 1),
                ])),
            ]),
        ])(ctx);

        ctx.write_debug("</BARE-IDENTIFIER>");

        result
    })
}

// identifier-char := unicode - linespace - [\/(){}<>;[]=,"]
//Hax: To avoid generating one of the keywords (true|false|null), we don't use 'u' or 'l'
fn identifier_char<T: Write + 'static, R: Rng + 'static>()
    -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {

    pick_ascii_or_utf8(
        write_rand_re("[-+0-9A-Za-km-tv-z]", 1),
        write_rand_re("[^ul\\\\/\\(\\){}<>;\\[\\]=,\"\
                          \u{000D}\u{000A}\u{000C}\u{0085}\u{2028}\
                          \u{2029}\u{0009}\u{0020}\u{00A0}\u{1680}\
                          \u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\
                          \u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\
                          \u{200A}\u{202F}\u{205F}\u{3000}\u{FEFF}]", 1)
    )
}

fn identifier_char_minus_digit<T: Write + 'static, R: Rng + 'static>()
    -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {

    pick_ascii_or_utf8(
        write_rand_re("[-+A-Za-km-tv-z]", 1),
        write_rand_re("[^ul0-9\\\\/\\(\\){}<>;\\[\\]=,\"\
                          \u{000D}\u{000A}\u{000C}\u{0085}\u{2028}\
                          \u{2029}\u{0009}\u{0020}\u{00A0}\u{1680}\
                          \u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\
                          \u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\
                          \u{200A}\u{202F}\u{205F}\u{3000}\u{FEFF}]", 1)
    )
}

fn identifier_char_minus_digit_and_sign<T: Write + 'static, R: Rng + 'static>()
    -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    pick_ascii_or_utf8(
        write_rand_re("[A-Za-km-tv-z]", 1),
        write_rand_re("[^-+ul0-9\\\\/\\(\\){}<>;\\[\\]=,\"\
                          \u{000D}\u{000A}\u{000C}\u{0085}\u{2028}\
                          \u{2029}\u{0009}\u{0020}\u{00A0}\u{1680}\
                          \u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\
                          \u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\
                          \u{200A}\u{202F}\u{205F}\u{3000}\u{FEFF}]", 1)
    )
}

// keyword := boolean | 'null'
fn keyword<T: Write, R: Rng>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        ctx.write_debug("<KEYWORD>");

        let result = select(vec![
            write_literal("true"),
            write_literal("false"),
            write_literal("null"),
        ])(ctx);

        ctx.write_debug("</KEYWORD>");

        result
    })
}

// prop := identifier '=' value
fn prop<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        ctx.write_debug("<PROP>");

        let result = concat(vec![
            identifier(),
            write_literal("="),
            value(),
        ])(ctx);

        ctx.write_debug("</PROP>");

        result
    })
}

// value := type? (string | number | keyword)
fn value<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        ctx.write_debug("<VALUE>");

        let result = concat(vec![
            maybe(type_rule()),
            select(vec![string_rule(), number(), keyword()]),
        ])(ctx);

        ctx.write_debug("</VALUE>");

        result
    })
}

// type := '(' identifier ')'
fn type_rule<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        ctx.write_debug("<TYPE>");

        let result = concat(vec![
            write_literal("("),
            identifier(),
            write_literal(")"),
        ])(ctx);

        ctx.write_debug("</TYPE>");

        result
    })
}

// string := raw-string | escaped-string
fn string_rule<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        ctx.write_debug("<STRING>");

        let result = select(vec![raw_string(), escaped_string()])(ctx);

        ctx.write_debug("</STRING>");

        result
    })
}

// escaped-string := '"' character* '"'
fn escaped_string<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        concat(vec![
            write_literal("\""),
            repeat(character(), 0, ctx.conf.string_len_max),
            write_literal("\""),
        ])(ctx)
    })
}

// character := '\' escape | [^\"]
fn character<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        select(vec![
            concat(vec![
                write_literal("\\"),
                escape(),
            ]),
            pick_ascii_or_utf8(
                write_rand_re("[a-zA-Z0-9 .,;!@\\#\\$%\\^&*()]", 1),
                write_rand_re("[^\\\\\"]", 1)
            ),
        ])(ctx)
    })
}

// escape := ["\\/bfnrt] | 'u{' hex-digit{1, 6} '}'
fn escape<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        select(vec![
            write_rand_re("[\"\\\\/bfnrt]", 1),
            concat(vec![
                write_literal("u{"),
                write_rand_unicode_hex(),
                write_literal("}"),
            ]),
        ])(ctx)
    })
}

// raw-string := 'r' raw-string-hash
fn raw_string<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        concat(vec![
            write_literal("r"),
            raw_string_hash(),
        ])(ctx)
    })
}

// raw-string-hash := '#' raw-string-hash '#' | raw-string-quotes
fn raw_string_hash<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        select(vec![
            concat(vec![
                write_literal("#"),
                raw_string_hash(),
                write_literal("#"),
            ]),
            raw_string_quotes(),
        ])(ctx)
    })
}

// raw-string-quotes := '"' .* '"'
fn raw_string_quotes<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        concat(vec![
            write_literal("\""),
            pick_ascii_or_utf8(
                write_rand_re("\\w*", ctx.conf.string_len_max),
                write_rand_re(".*", ctx.conf.string_len_max)),
            write_literal("\""),
        ])(ctx)
    })
}

// number := decimal | hex | octal | binary
fn number<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        ctx.write_debug("<NUMBER>");

        let result = select(vec![decimal(), hex(), octal(), binary()])(ctx);

        ctx.write_debug("</NUMBER>");

        result
    })
}

// decimal := sign? integer ('.' integer)? exponent?
fn decimal<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        concat(vec![
            maybe(sign()),
            integer(),
            maybe(concat(vec![
                write_literal("."),
                integer(),
            ])),
            maybe(exponent()),
        ])(ctx)
    })
}

// exponent := ('e' | 'E') sign? integer
fn exponent<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        concat(vec![
            select(vec![
                write_literal("e"),
                write_literal("E"),
            ]),
            maybe(sign()),
            integer(),
        ])(ctx)
    })
}

// integer := digit (digit | '_')*
fn integer<T: Write, R: Rng>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| write_rand_re("[0-9][0-9_]*", ctx.conf.num_len_max)(ctx))
}

// sign := '+' | '-'
fn sign<T: Write, R: Rng>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        select(vec![
            write_literal("+"),
            write_literal("-"),
        ])(ctx)
    })
}

// hex := sign? '0x' hex-digit (hex-digit | '_')*
fn hex<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        concat(vec![
            maybe(sign()),
            write_literal("0x"),
            write_rand_re("[0-9A-Fa-f][0-9A-Fa-f_]*", ctx.conf.num_len_max),
        ])(ctx)
    })
}

// octal := sign? '0o' [0-7] [0-7_]*
fn octal<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        concat(vec![
            maybe(sign()),
            write_literal("0o"),
            write_rand_re("[0-7][0-7_]*", ctx.conf.num_len_max),
        ])(ctx)
    })
}

// binary := sign? '0b' ('0' | '1') ('0' | '1' | '_')*
fn binary<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        concat(vec![
            maybe(sign()),
            write_literal("0b"),
            write_rand_re("[01][01_]*", ctx.conf.num_len_max),
        ])(ctx)
    })
}

// escline := '\\' ws* (single-line-comment | newline)
fn escline<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        ctx.write_debug("<ESCLINE>");

        let result = concat(vec![
            write_literal("\\"),
            repeat(ws(), 0, ctx.conf.extra_space_max),
            select(vec![single_line_comment(), newline()]),
        ])(ctx);

        ctx.write_debug("</ESCLINE>");

        result
    })
}

// linespace := newline | ws | single-line-comment
fn linespace<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        ctx.write_debug("<LINESPACE>");

        let result = select(vec![newline(), ws(), single_line_comment()])(ctx);

        ctx.write_debug("</LINESPACE>");

        result
    })
}

// newline := See Table (All line-break white_space)
fn newline<T: Write + 'static, R: Rng + 'static>()
    -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {

    pick_ascii_or_utf8(
        select(vec![
            write_literal("\u{000D}"),
            write_literal("\u{000A}"),
            write_literal("\u{000D}\u{000A}"),
        ]),
        select(vec![
            write_literal("\u{000D}"),
            write_literal("\u{000A}"),
            write_literal("\u{000D}\u{000A}"),
            write_literal("\u{0085}"),
            write_literal("\u{000C}"),
            write_literal("\u{2028}"),
            write_literal("\u{2029}"),
        ])
    )
}

// ws := bom | unicode-space | multi-line-comment
fn ws<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        ctx.write_debug("<WS>");

        let result = select(vec![bom(), unicode_space(), multi_line_comment()])(ctx);

        ctx.write_debug("</WS>");

        result
    })
}

// bom := '\u{FEFF}'
fn bom<T: Write, R: Rng>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| write_literal("\u{FEFF}")(ctx))
}

// unicode-space := See Table (All White_Space unicode characters which are not `newline`)
fn unicode_space<T: Write, R: Rng>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        select(vec![
            write_literal("\u{0009}"),
            write_literal("\u{0020}"),
            write_literal("\u{00A0}"),
            write_literal("\u{1680}"),
            write_literal("\u{2000}"),
            write_literal("\u{2001}"),
            write_literal("\u{2002}"),
            write_literal("\u{2003}"),
            write_literal("\u{2004}"),
            write_literal("\u{2005}"),
            write_literal("\u{2006}"),
            write_literal("\u{2007}"),
            write_literal("\u{2008}"),
            write_literal("\u{2009}"),
            write_literal("\u{200A}"),
            write_literal("\u{202F}"),
            write_literal("\u{205F}"),
            write_literal("\u{3000}"),
        ])(ctx)
    })
}

// single-line-comment := '//' ^newline+ (newline | eof)
fn single_line_comment<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        ctx.write_debug("<SINGLE-LINE-COMMENT>");

        let result = concat(vec![
            write_literal("//"),
            pick_ascii_or_utf8(
                write_rand_re("\\w+", ctx.conf.comment_len_max),
                write_rand_re("[^\u{000D}\u{000A}\u{000C}\u{0085}\u{2028}\u{2029}]+", ctx.conf.comment_len_max)
            ),
            newline(),
        ])(ctx);

        ctx.write_debug("</SINGLE-LINE-COMMENT>");

        result
    })
}

// multi-line-comment := '/*' commented-block
fn multi_line_comment<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|ctx| {
        ctx.write_debug("<MULTI-LINE-COMMENT>");

        let result = concat(vec![
            write_literal("/*"),
            commented_block(),
        ])(ctx);

        ctx.write_debug("</MULTI-LINE-COMMENT>");

        result
    })
}

// commented-block := '*/' | (multi-line-comment | '*' | '/' | [^*/]+) commented-block
fn commented_block<T: Write + 'static, R: Rng + 'static>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    return Box::new(|ctx| {
        select(vec![
            write_literal("*/"),
            concat(vec![
                pick_ascii_or_utf8(
                    select(vec![
                        write_rand_re("\\*\\w", 1),
                        write_rand_re("/\\w", 1),
                        write_rand_re("\\w+", ctx.conf.comment_len_max),
                        multi_line_comment(),
                    ]),
                    select(vec![
                        write_rand_re("\\*[^/]", 1),
                        write_rand_re("/[^*]", 1),
                        write_rand_re("[^*/]+", ctx.conf.comment_len_max),
                        multi_line_comment(),
                    ])
                ),
                commented_block(),
            ]),
        ])(ctx)
    });
}

fn write_literal<T: Write, R: Rng>(
    s: &str
) -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize> + '_> {
    Box::new(move |c| c.write(s.as_bytes()))
}

fn write_rand_unicode_hex<T: Write, R: Rng>() -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(|c| {
        write_literal(&format!("{:#x}", c.gen_range(1..=0x10FFFF))[2..])(c) //Need to slice off the '0x'
    })
}
fn pick_ascii_or_utf8<T: Write + 'static, R: Rng + 'static>(
    ascii: Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>>,
    unicode: Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>>,
) -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>> {
    Box::new(move |ctx| {
        return if ctx.conf.ascii_only {
            ascii(ctx)
        } else {
            unicode(ctx)
        }
    })
}

fn write_rand_re<T: Write, R: Rng>(
    pattern: &str, rep: u32,
) -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize> + '_> {
    Box::new(move |c| {
        let s = rand_re(c, pattern, rep);
        c.write(s.as_bytes())
    })
}

fn rand_re<T: Write, R: Rng>(ctx: &mut Context<T, R>, pattern: &str, rep: u32) -> String {
    let hir = ctx.get_regex_parser().parse(pattern).unwrap();
    let re = rand_regex::Regex::with_hir(hir, rep).unwrap();
    ctx.sample(re)
}

fn maybe<'t, T: Write + 't, R: Rng + 't>(
    func: Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>>,
) -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize> + 't> {
    Box::new(move |c| {
        let rnd: f32 = c.gen();
        return if rnd > 0.5 {
            func(c)
        } else {
            Ok(0)
        };
    })
}

fn repeat<'t, T: Write + 't, R: Rng + 't>(
    func: Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>>,
    min_times: u32, max_times: u32,
) -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize> + 't> {
    Box::new(move |c| {
        let times = c.gen_range(min_times..=max_times);
        let mut size = 0;
        for _ in 0..times {
            match func(c) {
                Err(e) => return Err(e),
                Ok(s) => size += s
            }
        }

        Ok(size)
    })
}

fn select<'t, T: Write + 't, R: Rng + 't>(
    options: Vec<Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>>>,
) -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize> + 't> {
    Box::new(move |c| {
        options.choose(c.rng).unwrap()(c)
    })
}

fn concat<'t, T: Write + 't, R: Rng + 't>(
    calls: Vec<Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize>>>,
) -> Box<dyn Fn(&mut Context<T, R>) -> io::Result<usize> + 't> {
    Box::new(move |c| {
        let mut size = 0;
        for call in calls.iter() {
            match call(c) {
                Err(e) => return Err(e),
                Ok(s) => size += s,
            }
        }

        Ok(size)
    })
}
