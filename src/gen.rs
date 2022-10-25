use std::io;
use std::io::Write;
use rand::{Rng, RngCore, Error};
use crate::Configuration;

struct Context<'t, T: Write, R: Rng> {
    conf: Configuration,
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

pub fn document<'t, T: Write, R: Rng>(out: &mut T, rng: &mut R, conf: Configuration) -> io::Result<usize> {
    let ctx: &mut Context<T, R> = &mut Context {
        conf,
        out,
        rng,
        depth: 0,
    };

    nodes(ctx)
}

// nodes := linespace* (node nodes?)? linespace*
fn nodes<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    if ctx.depth > ctx.conf.depth_max {
        return Ok(0);
    }

    if ctx.conf.debug {
        write_literal(ctx, "<NODES>").unwrap();
    }

    ctx.depth += 1;
    let result = concat(ctx, &[
        |c| repeat(c, 0, c.conf.blank_lines_max, linespace),
        |c| repeat(c, 0, c.conf.nodes_per_child_max, node),
        |c| repeat(c, 0, c.conf.blank_lines_max, linespace),
    ]);
    ctx.depth -= 1;

    if ctx.conf.debug {
        write_literal(ctx, "</NODES>").unwrap();
    }

    result
}

// node := ('/-' node-space*)? type? identifier (node-space+ node-prop-or-arg)* (node-space* node-children ws*)? node-space* node-terminator
fn node<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    if ctx.conf.debug {
        write_literal(ctx, "<NODE>").unwrap();
    }

    let result = concat(ctx, &[
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
    ]);

    if ctx.conf.debug {
        write_literal(ctx, "</NODE>").unwrap();
    }

    result
}

// node-prop-or-arg := ('/-' node-space*)? (prop | value)
fn node_prop_or_arg<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    if ctx.conf.debug {
        write_literal(ctx, "<NODE-PROP-OR-ARG>").unwrap();
    }

    let result = concat(ctx, &[
        |c| maybe(c, |c| concat(c, &[
            |c| write_literal(c, "/-"),
            |c| repeat(c, 0, c.conf.extra_space_max, node_space)
        ])),
        |c| select(c, &[prop, value])
    ]);

    if ctx.conf.debug {
        write_literal(ctx, "</NODE-PROP-OR-ARG>").unwrap();
    }

    result
}

// node-children := ('/-' node-space*)? {' nodes '}'
fn node_children<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    if ctx.conf.debug {
        write_literal(ctx, "<NODE-CHILDREN>").unwrap();
    }

    let result = concat(ctx, &[
        |c| maybe(c, |c| concat(c, &[
            |c| write_literal(c, "/-"),
            |c| repeat(c, 0, c.conf.extra_space_max, node_space)
        ])),
        |c| write_literal(c, "{"),
        nodes,
        |c| write_literal(c, "}")
    ]);

    if ctx.conf.debug {
        write_literal(ctx, "</NODE-CHILDREN>").unwrap();
    }

    result
}

// node-space := ws* escline ws* | ws+
fn node_space<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    if ctx.conf.debug {
        write_literal(ctx, "<NODE-SPACE>").unwrap();
    }

    let result = select(ctx, &[
        |c| concat(c, &[
            |c| repeat(c, 0, c.conf.extra_space_max, ws),
            escline,
            |c| repeat(c, 0, c.conf.extra_space_max, ws),
        ]),
        |c| repeat(c, 1, c.conf.extra_space_max, ws),
    ]);

    if ctx.conf.debug {
        write_literal(ctx, "</NODE-CHILDREN>").unwrap();
    }

    result
}

// node-terminator := single-line-comment | newline | ';' | eof
fn node_terminator<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    if ctx.conf.debug {
        write_literal(ctx, "<NODE-TERMINATOR>").unwrap();
    }

    let result = select(ctx, &[
        single_line_comment,
        newline,
        |c| write_literal(c, ";")
    ]);

    if ctx.conf.debug {
        write_literal(ctx, "</NODE-TERMINATOR>").unwrap();
    }

    result
}

// identifier := string | bare-identifier
fn identifier<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    if ctx.conf.debug {
        write_literal(ctx, "<IDENTIFIER>").unwrap();
    }

    let result = select(ctx, &[string_rule, bare_identifier]);

    if ctx.conf.debug {
        write_literal(ctx, "</IDENTIFIER>").unwrap();
    }

    result
}

// bare-identifier := ((identifier-char - digit - sign) identifier-char*| sign ((identifier-char - digit) identifier-char*)?) - keyword
fn bare_identifier<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    if ctx.conf.debug {
        write_literal(ctx, "<BARE-IDENTIFIER>").unwrap();
    }

    let result = select(ctx, &[
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
    ]);

    if ctx.conf.debug {
        write_literal(ctx, "</BARE-IDENTIFIER>").unwrap();
    }

    result
}

// identifier-char := unicode - linespace - [\/(){}<>;[]=,"]
//Hax: To avoid generating one of the keywords (true|false|null), we don't use 'u' or 'l'
fn identifier_char<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    write_rand_re(ctx,
                  "[^ul\\\\/\\(\\){}<>;\\[\\]=,\"\
                  \u{000D}\u{000A}\u{000C}\u{0085}\u{2028}\
                  \u{2029}\u{0009}\u{0020}\u{00A0}\u{1680}\
                  \u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\
                  \u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\
                  \u{200A}\u{202F}\u{205F}\u{3000}\u{FEFF}]", 1)
}

fn identifier_char_minus_digit<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    write_rand_re(ctx,
                  "[^ul0-9\\\\/\\(\\){}<>;\\[\\]=,\"\
                  \u{000D}\u{000A}\u{000C}\u{0085}\u{2028}\
                  \u{2029}\u{0009}\u{0020}\u{00A0}\u{1680}\
                  \u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\
                  \u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\
                  \u{200A}\u{202F}\u{205F}\u{3000}\u{FEFF}]", 1)
}

fn identifier_char_minus_digit_and_sign<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    write_rand_re(ctx,
                  "[^-+ul0-9\\\\/\\(\\){}<>;\\[\\]=,\"\
                  \u{000D}\u{000A}\u{000C}\u{0085}\u{2028}\
                  \u{2029}\u{0009}\u{0020}\u{00A0}\u{1680}\
                  \u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\
                  \u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\
                  \u{200A}\u{202F}\u{205F}\u{3000}\u{FEFF}]", 1)
}

// keyword := boolean | 'null'
fn keyword<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    if ctx.conf.debug {
        write_literal(ctx, "<KEYWORD>").unwrap();
    }

    let result = select(ctx, &[
        |c| write_literal(c, "true"),
        |c| write_literal(c, "false"),
        |c| write_literal(c, "null"),
    ]);

    if ctx.conf.debug {
        write_literal(ctx, "</KEYWORD>").unwrap();
    }

    result
}

// prop := identifier '=' value
fn prop<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    if ctx.conf.debug {
        write_literal(ctx, "<PROP>").unwrap();
    }

    let result = concat(ctx, &[
        identifier,
        |c| write_literal(c, "="),
        value
    ]);

    if ctx.conf.debug {
        write_literal(ctx, "</PROP>").unwrap();
    }

    result
}

// value := type? (string | number | keyword)
fn value<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    if ctx.conf.debug {
        write_literal(ctx, "<VALUE>").unwrap();
    }

    let result = concat(ctx, &[
        |c| maybe(c, type_rule),
        |c| select(c, &[string_rule, number, keyword])
    ]);

    if ctx.conf.debug {
        write_literal(ctx, "</VALUE>").unwrap();
    }

    result
}

// type := '(' identifier ')'
fn type_rule<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    if ctx.conf.debug {
        write_literal(ctx, "<TYPE>").unwrap();
    }

    let result = concat(ctx, &[
        |c| write_literal(c, "("),
        identifier,
        |c| write_literal(c, ")"),
    ]);

    if ctx.conf.debug {
        write_literal(ctx, "</TYPE>").unwrap();
    }

    result
}

// string := raw-string | escaped-string
fn string_rule<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    if ctx.conf.debug {
        write_literal(ctx, "<STRING>").unwrap();
    }

    let result = select(ctx, &[raw_string, escaped_string]);

    if ctx.conf.debug {
        write_literal(ctx, "</STRING>").unwrap();
    }

    result
}

// escaped-string := '"' character* '"'
fn escaped_string<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    concat(ctx, &[
        |c| write_literal(c, "\""),
        |c| repeat(c, 0, c.conf.string_len_max, character),
        |c| write_literal(c, "\""),
    ])
}

// character := '\' escape | [^\"]
fn character<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    select(ctx, &[
        |c| concat(c, &[
            |c| write_literal(c, "\\"),
            escape
        ]),
        |c| write_rand_re(c, "[^\\\\\"]", 1)
    ])
}

// escape := ["\\/bfnrt] | 'u{' hex-digit{1, 6} '}'
fn escape<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    select(ctx, &[
        |c| write_rand_re(c, "[\"\\\\/bfnrt]", 1),
        |c| concat(c, &[
            |c| write_literal(c, "u{"),
            |c| write_rand_unicode_hex(c),
            |c| write_literal(c, "}"),
        ])
    ])
}

// raw-string := 'r' raw-string-hash
fn raw_string<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    concat(ctx, &[
        |c| write_literal(c, "r"),
        raw_string_hash
    ])
}

// raw-string-hash := '#' raw-string-hash '#' | raw-string-quotes
fn raw_string_hash<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
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
fn raw_string_quotes<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    concat(ctx, &[
        |c| write_literal(c, "\""),
        |c| write_rand_re(c, ".*", c.conf.string_len_max),
        |c| write_literal(c, "\""),
    ])
}

// number := decimal | hex | octal | binary
fn number<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    if ctx.conf.debug {
        write_literal(ctx, "<NUMBER>").unwrap();
    }

    let result= select(ctx, &[decimal, hex, octal, binary]);

    if ctx.conf.debug {
        write_literal(ctx, "</NUMBER>").unwrap();
    }

    result
}

// decimal := sign? integer ('.' integer)? exponent?
fn decimal<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
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
fn exponent<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
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
fn integer<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    write_rand_re(ctx, "[0-9][0-9_]*", ctx.conf.num_len_max)
}

// sign := '+' | '-'
fn sign<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    select(ctx, &[
        |c| write_literal(c, "+"),
        |c| write_literal(c, "-"),
    ])
}

// hex := sign? '0x' hex-digit (hex-digit | '_')*
fn hex<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    concat(ctx, &[
        |c| maybe(c, sign),
        |c| write_literal(c, "0x"),
        |c| write_rand_re(c, "[0-9A-Fa-f][0-9A-Fa-f_]*", c.conf.num_len_max),
    ])
}

// octal := sign? '0o' [0-7] [0-7_]*
fn octal<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    concat(ctx, &[
        |c| maybe(c, sign),
        |c| write_literal(c, "0o"),
        |c| write_rand_re(c, "[0-7][0-7_]*", c.conf.num_len_max),
    ])
}

// binary := sign? '0b' ('0' | '1') ('0' | '1' | '_')*
fn binary<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    concat(ctx, &[
        |c| maybe(c, sign),
        |c| write_literal(c, "0b"),
        |c| write_rand_re(c, "[01][01_]*", c.conf.num_len_max),
    ])
}

// escline := '\\' ws* (single-line-comment | newline)
fn escline<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    if ctx.conf.debug {
        write_literal(ctx, "<ESCLINE>").unwrap();
    }

    let result = concat(ctx, &[
        |c| write_literal(c, "\\"),
        |c| repeat(c, 0, c.conf.extra_space_max, ws),
        |c| select(c, &[single_line_comment, newline])
    ]);

    if ctx.conf.debug {
        write_literal(ctx, "</ESCLINE>").unwrap();
    }

    result
}

// linespace := newline | ws | single-line-comment
fn linespace<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    if ctx.conf.debug {
        write_literal(ctx, "<LINESPACE>").unwrap();
    }

    let result = select(ctx, &[
        |c| newline(c),
        |c| ws(c),
        |c| single_line_comment(c)
    ]);

    if ctx.conf.debug {
        write_literal(ctx, "</LINESPACE>").unwrap();
    }

    result
}

// newline := See Table (All line-break white_space)
fn newline<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
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
fn ws<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    if ctx.conf.debug {
        write_literal(ctx, "<WS>").unwrap();
    }

    let result = select(ctx, &[bom, unicode_space, multi_line_comment]);

    if ctx.conf.debug {
        write_literal(ctx, "</WS>").unwrap();
    }

    result
}

// bom := '\u{FEFF}'
fn bom<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    write_literal(ctx, "\u{FEFF}")
}

// unicode-space := See Table (All White_Space unicode characters which are not `newline`)
fn unicode_space<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
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
fn single_line_comment<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    if ctx.conf.debug {
        write_literal(ctx, "<SINGLE-LINE-COMMENT>").unwrap();
    }

    let result = concat(ctx, &[
        |c| write_literal(c, "//"),
        |c| write_rand_re(c, "[^\u{000D}\u{000A}\u{000C}\u{0085}\u{2028}\u{2029}]+", c.conf.comment_len_max),
        newline
    ]);

    if ctx.conf.debug {
        write_literal(ctx, "</SINGLE-LINE-COMMENT>").unwrap();
    }

    result
}

// multi-line-comment := '/*' commented-block
fn multi_line_comment<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    if ctx.conf.debug {
        write_literal(ctx, "<MULTI-LINE-COMMENT>").unwrap();
    }

    let result = concat(ctx, &[
        |c| write_literal(c, "/*"),
        |c| commented_block(c)
    ]);

    if ctx.conf.debug {
        write_literal(ctx, "</MULTI-LINE-COMMENT>").unwrap();
    }

    result
}

// commented-block := '*/' | (multi-line-comment | '*' | '/' | [^*/]+) commented-block
fn commented_block<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    select(ctx, &[
        |c| write_literal(c, "*/"),
        |c| concat(c, &[
            |c| select(c, &[
                |c| write_rand_re(c, "\\*[^/]", 1),
                |c| write_rand_re(c, "/[^*]", 1),
                |c| write_rand_re(c, "[^*/]+", c.conf.comment_len_max),
                |c| multi_line_comment(c)
            ]),
            commented_block
        ])
    ])
}

fn write_literal<T: Write, R: Rng>(ctx: &mut Context<T, R>, s: &str) -> io::Result<usize> {
    ctx.write(s.as_bytes())
}

fn write_rand_unicode_hex<T: Write, R: Rng>(ctx: &mut Context<T, R>) -> io::Result<usize> {
    let codepoint: i32 = ctx.gen_range(1..=0x10FFFF);
    write_literal(ctx, &format!("{:#x}", codepoint)[2..]) //Need to slice off the '0x'
}

fn write_rand_re<T: Write, R: Rng>(
    ctx: &mut Context<T, R>, pattern: &str, rep: u32,
) -> io::Result<usize> {
    let s = rand_re(ctx, pattern, rep);
    ctx.write(s.as_bytes())
}

fn rand_re<T: Write, R: Rng>(ctx: &mut Context<T, R>, pattern: &str, rep: u32) -> String {
    let re = rand_regex::Regex::compile(pattern, rep).unwrap();
    ctx.sample(re)
}

fn maybe<T: Write, R: Rng>(
    ctx: &mut Context<T, R>,
    func: fn(&mut Context<T, R>) -> io::Result<usize>,
) -> io::Result<usize> {
    let rnd: f32 = ctx.gen();
    return if rnd > 0.5 {
        func(ctx)
    } else {
        Ok(0)
    };
}

fn repeat<T: Write, R: Rng>(
    ctx: &mut Context<T, R>, min_times: u32, max_times: u32,
    func: fn(&mut Context<T, R>) -> io::Result<usize>,
) -> io::Result<usize> {
    let times = ctx.gen_range(min_times..max_times + 1);
    let mut size = 0;
    for _ in 0..times {
        match func(ctx) {
            Err(e) => return Err(e),
            Ok(s) => size += s
        }
    }

    Ok(size)
}

fn select<T: Write, R: Rng>(
    ctx: &mut Context<T, R>, options: &[fn(&mut Context<T, R>) -> io::Result<usize>],
) -> io::Result<usize> {
    let idx: usize = ctx.gen_range(0..options.len());
    options[idx](ctx)
}

fn concat<T: Write, R: Rng>(
    ctx: &mut Context<T, R>,
    calls: &[fn(&mut Context<T, R>) -> io::Result<usize>],
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
