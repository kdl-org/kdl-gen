use std::io;
use std::io::Write;
use rand::{thread_rng, Rng};
use crate::Configuration;

struct Context<'t, T: Write> {
    depth: u32,
    conf: Configuration,
    out: &'t mut T,
}

impl<'t, T: Write> Write for Context<'t, T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.out.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.out.flush()
    }
}

pub fn document<'t, T: Write>(out: &mut T, conf: Configuration) -> io::Result<usize> {
    let ctx: &mut Context<T> = &mut Context {
        out,
        conf,
        depth: 0,
    };

    nodes(ctx)
}

// nodes := linespace* (node nodes?)? linespace*
fn nodes<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    if ctx.depth > ctx.conf.depth_max {
        return Ok(0);
    }
    ctx.depth += 1;
    let result = concat(ctx, &[
        |c| repeat(c, 0, c.conf.blank_lines_max, linespace),
        |c| repeat(c, 0, c.conf.nodes_per_child_max, node),
        |c| repeat(c, 0, c.conf.blank_lines_max, linespace),
    ]);
    ctx.depth -= 1;

    result
}

// node := ('/-' node-space*)? type? identifier (node-space+ node-prop-or-arg)* (node-space* node-children ws*)? node-space* node-terminator
fn node<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    concat(ctx, &[
        |c| maybe(c, |c| concat(c, &[
            |c| write_literal(c, "/-"),
            |c| repeat(c, 0, c.conf.extra_space_max, node_space)
        ])),
        |c| maybe(c, type_rule),
        identifier,
        |c| repeat(c, 0, c.conf.props_or_args_max,
                   |c| concat(c, &[
                       |c| repeat(c, 1, c.conf.extra_space_max, node_space),
                       node_prop_or_arg
                   ])),
        |c| maybe(c, |c| concat(c, &[
            |c| repeat(c, 0, c.conf.extra_space_max, node_space),
            node_children,
            |c| repeat(c, 0, c.conf.extra_space_max, ws),
        ])),
        |c| repeat(c, 0, c.conf.extra_space_max, node_space),
        node_terminator
    ])
}

// node-prop-or-arg := ('/-' node-space*)? (prop | value)
fn node_prop_or_arg<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    concat(ctx, &[
        |c| maybe(c, |c| concat(c, &[
            |c| write_literal(c, "/-"),
            |c| repeat(c, 0, c.conf.extra_space_max, node_space)
        ])),
        |c| select(c, &[prop, value])
    ])
}

// node-children := ('/-' node-space*)? {' nodes '}'
fn node_children<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    concat(ctx, &[
        |c| maybe(c, |c| concat(c, &[
            |c| write_literal(c, "/-"),
            |c| repeat(c, 0, c.conf.extra_space_max, node_space)
        ])),
        |c| write_literal(c, "{"),
        nodes,
        |c| write_literal(c, "}")
    ])
}

// node-space := ws* escline ws* | ws+
fn node_space<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    select(ctx, &[
        |c| concat(c, &[
            |c| repeat(c, 0, c.conf.extra_space_max, ws),
            escline,
            |c| repeat(c, 0, c.conf.extra_space_max, ws),
        ]),
        |c| repeat(c, 1, c.conf.extra_space_max, ws),
    ])
}

// node-terminator := single-line-comment | newline | ';' | eof
fn node_terminator<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    select(ctx, &[
        single_line_comment,
        newline,
        |c| write_literal(c, ";")
    ])
}

// identifier := string | bare-identifier
fn identifier<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    select(ctx, &[string_rule, bare_identifier])
}

// bare-identifier := ((identifier-char - digit - sign) identifier-char*| sign ((identifier-char - digit) identifier-char*)?) - keyword
fn bare_identifier<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    select(ctx, &[
        |c| concat(c, &[
            identifier_char_minus_digit_and_sign,
            |c| repeat(c, 0, c.conf.identifier_len_max - 1, identifier_char)
        ]),
        |c| concat(c, &[
            sign,
            |c| maybe(c, |c| concat(c, &[
                identifier_char_minus_digit,
                |c| repeat(c, 0, c.conf.identifier_len_max - 1, identifier_char)
            ]))
        ])
    ])
}

// identifier-char := unicode - linespace - [\/(){}<>;[]=,"]
//Hax: To avoid generating one of the keywords (true|false|null), we don't use 'u' or 'l'
fn identifier_char<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    write_rand_re(ctx,
                  "[^ul\\\\/\\(\\){}<>;\\[\\]=,\"\
                  \u{000D}\u{000A}\u{000C}\u{0085}\u{2028}\
                  \u{2029}\u{0009}\u{0020}\u{00A0}\u{1680}\
                  \u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\
                  \u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\
                  \u{200A}\u{202F}\u{205F}\u{3000}\u{FEFF}]", 1)
}

fn identifier_char_minus_digit<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    write_rand_re(ctx,
                  "[^ul0-9\\\\/\\(\\){}<>;\\[\\]=,\"\
                  \u{000D}\u{000A}\u{000C}\u{0085}\u{2028}\
                  \u{2029}\u{0009}\u{0020}\u{00A0}\u{1680}\
                  \u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\
                  \u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\
                  \u{200A}\u{202F}\u{205F}\u{3000}\u{FEFF}]", 1)
}

fn identifier_char_minus_digit_and_sign<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    write_rand_re(ctx,
                  "[^-+ul0-9\\\\/\\(\\){}<>;\\[\\]=,\"\
                  \u{000D}\u{000A}\u{000C}\u{0085}\u{2028}\
                  \u{2029}\u{0009}\u{0020}\u{00A0}\u{1680}\
                  \u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\
                  \u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\
                  \u{200A}\u{202F}\u{205F}\u{3000}\u{FEFF}]", 1)
}

// keyword := boolean | 'null'
fn keyword<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    select(ctx, &[
        |c| write_literal(c, "true"),
        |c| write_literal(c, "false"),
        |c| write_literal(c, "null"),
    ])
}

// prop := identifier '=' value
fn prop<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    concat(ctx, &[
        identifier,
        |c| write_literal(c, "="),
        value
    ])
}

// value := type? (string | number | keyword)
fn value<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    concat(ctx, &[
        |c| maybe(c, type_rule),
        |c| select(c, &[string_rule, number, keyword])
    ])
}

// type := '(' identifier ')'
fn type_rule<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    concat(ctx, &[
        |c| write_literal(c, "("),
        identifier,
        |c| write_literal(c, ")"),
    ])
}

// string := raw-string | escaped-string
fn string_rule<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    select(ctx, &[raw_string, escaped_string])
}

// escaped-string := '"' character* '"'
fn escaped_string<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    concat(ctx, &[
        |c| write_literal(c, "\""),
        |c| repeat(c, 0, c.conf.string_len_max, character),
        |c| write_literal(c, "\""),
    ])
}

// character := '\' escape | [^\"]
fn character<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    select(ctx, &[
        |c| concat(c, &[
            |c| write_literal(c, "\\"),
            escape
        ]),
        |c| write_rand_re(c, "[^\\\\\"]", 1)
    ])
}

// escape := ["\\/bfnrt] | 'u{' hex-digit{1, 6} '}'
fn escape<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    select(ctx, &[
        |c| write_rand_re(c, "[\"\\\\/bfnrt]", 1),
        |c| concat(c, &[
            |c| write_literal(c, "u{"),
            |c| write_rand_re(c, "[0-9A-Fa-f]+", 6),
            |c| write_literal(c, "}"),
        ])
    ])
}

// raw-string := 'r' raw-string-hash
fn raw_string<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    concat(ctx, &[
        |c| write_literal(c, "r"),
        raw_string_hash
    ])
}

// raw-string-hash := '#' raw-string-hash '#' | raw-string-quotes
fn raw_string_hash<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    select(ctx, &[
        |c| concat(c, &[
            |c| write_literal(c, "#"),
            raw_string_hash,
            |c| write_literal(c, "#"),
        ]),
        raw_string_quotes
    ])
}

// raw-string-quotes := '"' .* '"'
fn raw_string_quotes<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    concat(ctx, &[
        |c| write_literal(c, "\""),
        |c| write_rand_re(c, ".*", c.conf.string_len_max),
        |c| write_literal(c, "\""),
    ])
}

// number := decimal | hex | octal | binary
fn number<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    select(ctx, &[decimal, hex, octal, binary])
}

// decimal := sign? integer ('.' integer)? exponent?
fn decimal<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    concat(ctx, &[
        |c| maybe(c, sign),
        integer,
        |c| maybe(c, |c| concat(c, &[
            |c| write_literal(c, "."),
            integer
        ])),
        |c| maybe(c, exponent)
    ])
}

// exponent := ('e' | 'E') sign? integer
fn exponent<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    concat(ctx, &[
        |c| select(c, &[
            |c| write_literal(c, "e"),
            |c| write_literal(c, "E"),
        ]),
        |c| maybe(c, sign),
        integer
    ])
}

// integer := digit (digit | '_')*
fn integer<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    write_rand_re(ctx, "[0-9][0-9_]*", ctx.conf.num_len_max)
}

// sign := '+' | '-'
fn sign<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    select(ctx, &[
        |c| write_literal(c, "+"),
        |c| write_literal(c, "-"),
    ])
}

// hex := sign? '0x' hex-digit (hex-digit | '_')*
fn hex<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    concat(ctx, &[
        |c| maybe(c, sign),
        |c| write_literal(c, "0x"),
        |c| write_rand_re(c, "[0-9A-Fa-f][0-9A-Fa-f_]*", c.conf.num_len_max),
    ])
}

// octal := sign? '0o' [0-7] [0-7_]*
fn octal<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    concat(ctx, &[
        |c| maybe(c, sign),
        |c| write_literal(c, "0o"),
        |c| write_rand_re(c, "[0-7][0-7_]*", c.conf.num_len_max),
    ])
}

// binary := sign? '0b' ('0' | '1') ('0' | '1' | '_')*
fn binary<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    concat(ctx, &[
        |c| maybe(c, sign),
        |c| write_literal(c, "0b"),
        |c| write_rand_re(c, "[01][01_]*", c.conf.num_len_max),
    ])
}

// escline := '\\' ws* (single-line-comment | newline)
fn escline<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    concat(ctx, &[
        |c| write_literal(c, "\\"),
        |c| repeat(c, 0, c.conf.extra_space_max, ws),
        |c| select(c, &[single_line_comment, newline])
    ])
}

// linespace := newline | ws | single-line-comment
fn linespace<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    select(ctx, &[
        |c| newline(c),
        |c| ws(c),
        |c| single_line_comment(c)
    ])
}

// newline := See Table (All line-break white_space)
fn newline<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    select(ctx, &[
        |c| write_literal(c, "\u{000D}"),
        |c| write_literal(c, "\u{000A}"),
        |c| write_literal(c, "\u{000D}\u{000A}"),
        |c| write_literal(c, "\u{0085}"),
        |c| write_literal(c, "\u{000C}"),
        |c| write_literal(c, "\u{2028}"),
        |c| write_literal(c, "\u{2029}"),
    ])
}

// ws := bom | unicode-space | multi-line-comment
fn ws<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    select(ctx, &[bom, unicode_space, multi_line_comment])
}

// bom := '\u{FEFF}'
fn bom<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    write_literal(ctx, "\u{FEFF}")
}

// unicode-space := See Table (All White_Space unicode characters which are not `newline`)
fn unicode_space<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    select(ctx, &[
        |c| write_literal(c, "\u{0009}"),
        |c| write_literal(c, "\u{0020}"),
        |c| write_literal(c, "\u{00A0}"),
        |c| write_literal(c, "\u{1680}"),
        |c| write_literal(c, "\u{2000}"),
        |c| write_literal(c, "\u{2001}"),
        |c| write_literal(c, "\u{2002}"),
        |c| write_literal(c, "\u{2003}"),
        |c| write_literal(c, "\u{2004}"),
        |c| write_literal(c, "\u{2005}"),
        |c| write_literal(c, "\u{2006}"),
        |c| write_literal(c, "\u{2007}"),
        |c| write_literal(c, "\u{2008}"),
        |c| write_literal(c, "\u{2009}"),
        |c| write_literal(c, "\u{200A}"),
        |c| write_literal(c, "\u{202F}"),
        |c| write_literal(c, "\u{205F}"),
        |c| write_literal(c, "\u{3000}"),
    ])
}

// single-line-comment := '//' ^newline+ (newline | eof)
fn single_line_comment<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    concat(ctx, &[
        |c| write_literal(c, "//"),
        |c| write_rand_re(c, "[^\u{000D}\u{000A}\u{000C}\u{0085}\u{2028}\u{2029}]+", c.conf.comment_len_max),
        newline
    ])
}

// multi-line-comment := '/*' commented-block
fn multi_line_comment<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    concat(ctx, &[
        |c| write_literal(c, "/*"),
        |c| commented_block(c)
    ])
}

// commented-block := '*/' | (multi-line-comment | '*' | '/' | [^*/]+) commented-block
fn commented_block<T: Write>(ctx: &mut Context<T>) -> io::Result<usize> {
    select(ctx, &[
        |c| write_literal(c, "*/"),
        |c| {
            match select(c, &[
                |c| write_literal(c, "*"),
                |c| write_literal(c, "/"),
                |c| write_rand_re(c, "[^*/]+", c.conf.comment_len_max),
                |c| multi_line_comment(c)
            ]) {
                Err(e) => Err(e),
                Ok(s) => match commented_block(c) {
                    Err(e) => Err(e),
                    Ok(sc) => Ok(s + sc)
                }
            }
        }
    ])
}

fn write_literal<T: Write>(ctx: &mut Context<T>, s: &str) -> io::Result<usize> {
    ctx.write(s.as_bytes())
}

fn write_rand_re<T: Write>(
    ctx: &mut Context<T>, pattern: &str, rep: u32,
) -> io::Result<usize> {
    let s = rand_re(pattern, rep);
    ctx.write(s.as_bytes())
}

fn rand_re(pattern: &str, rep: u32) -> String {
    let mut rng = thread_rng();
    let re = rand_regex::Regex::compile(pattern, rep).unwrap();
    rng.sample(re)
}

fn maybe<T: Write>(
    ctx: &mut Context<T>,
    func: fn(&mut Context<T>) -> io::Result<usize>,
) -> io::Result<usize> {
    let rnd: f32 = thread_rng().gen();
    return if rnd > 0.5 {
        func(ctx)
    } else {
        Ok(0)
    };
}

fn repeat<T: Write>(
    ctx: &mut Context<T>, min_times: u32, max_times: u32,
    func: fn(&mut Context<T>) -> io::Result<usize>,
) -> io::Result<usize> {
    let times = thread_rng().gen_range(min_times..max_times + 1);
    let mut size = 0;
    for _ in 0..times {
        match func(ctx) {
            Err(e) => return Err(e),
            Ok(s) => size += s
        }
    }

    Ok(size)
}

fn select<T: Write>(
    ctx: &mut Context<T>, options: &[fn(&mut Context<T>) -> io::Result<usize>],
) -> io::Result<usize> {
    let idx: usize = thread_rng().gen_range(0..options.len());
    options[idx](ctx)
}

fn concat<T: Write>(
    ctx: &mut Context<T>,
    calls: &[fn(&mut Context<T>) -> io::Result<usize>],
) -> io::Result<usize> {
    let mut size = 0;
    for call in calls {
        match call(ctx) {
            Err(e) => return Err(e),
            Ok(s) => size += s,
        }
    }

    Ok(size)
}
