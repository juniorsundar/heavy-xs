// Same pattern/subject battery against each engine; prints compile + first match span.
const JS: &str = r"\/(?=(?!\/)(?:(?>[^\\[\/]++|\\.|\[(?:[^\\\]]++|\\.)*+\])*+)++\/[gmiyuvsd]*\s*(?:[\n,;\)\]\}\.]|\/[\/*]))()";
const CASES: &[(&str, &str, &str, &str)] = &[
 // id, pattern, subject, PCRE2 (UTF, UCP off) expected first match or "none"/"error"
 ("atomic", r"(?>a+)a", "aaa", "none"),
 ("possessive", r"a++a", "aaa", "none"),
 ("lookahead", r"a(?=b)", "ab", "0..1"),
 ("lookbehind_fixed", r"(?<=ab)c", "abc", "2..3"),
 ("lookbehind_var", r"(?<=a|bc)d", "bcd", "2..3"),
 ("backref_num", r"(a)\1", "aa", "0..2"),
 ("backref_named", r"(?<x>a)\k<x>", "aa", "0..2"),
 ("inline_flags", r"(?i)ab", "AB", "0..2"),
 ("lazy", r"a+?", "aaa", "0..1"),
 ("empty_capture", r"a()b", "ab", "0..2"),
 ("word_ascii", r"\w", "é", "none"),
 ("digit_ascii", r"\d", "٣", "none"),
 ("space_ascii", r"\s", "\u{a0}", "none"),
 ("dot_utf8", r".", "é", "0..2"),
 ("negclass_utf8", r"[^a]", "é", "0..2"),
 ("icase_utf8", r"(?i)é", "É", "0..2"),
 ("literal_bracket", r"[^\\[/]+", "x[y", "0..1"),
 ("quote", r"\Qa.b\E", "a.b", "0..3"),
 ("h_space", r"\h", " ", "0..1"),
 ("abs_anchor", r"\Aa", "a", "0..1"),
 ("posix", r"[[:alpha:]]+", "abé", "0..2"),
 ("mark", r"(*MARK:t)a", "a", "0..1"),
 ("branch_reset", r"(?|(a)|(b))", "b", "0..1"),
 ("conditional", r"(a)?(?(1)b|c)", "c", "0..1"),
 ("js_rule", JS, "/ab+c/gi;", "0..9"),
];
fn show(r: Result<Option<(usize, usize)>, String>) -> String {
    match r { Ok(Some((a, b))) => format!("{a}..{b}"), Ok(None) => "none".into(), Err(e) => format!("error({})", e.chars().take(50).collect::<String>()) }
}
fn fancy(p: &str, s: &str) -> Result<Option<(usize, usize)>, String> {
    let r = fancy_regex::RegexBuilder::new(p).oniguruma_mode(std::env::var("ONIG").is_ok()).build().map_err(|e| e.to_string())?;
    r.find(s).map(|m| m.map(|m| (m.start(), m.end()))).map_err(|e| e.to_string())
}
fn regress(p: &str, s: &str) -> Result<Option<(usize, usize)>, String> {
    let r = regress::Regex::with_flags(p, "u").map_err(|e| e.to_string())?;
    Ok(r.find(s).map(|m| (m.range().start, m.range().end)))
}
fn resharp(p: &str, s: &str) -> Result<Option<(usize, usize)>, String> {
    let r = resharp::Regex::new(p).map_err(|e| format!("{e:?}"))?;
    let all = r.find_all(s.as_bytes()).map_err(|e| format!("{e:?}"))?;
    Ok(all.first().map(|m| (m.start, m.end)))
}
fn main() {
    let (mut f, mut g, mut h) = (0, 0, 0);
    println!("case\tPCRE2\tfancy-regex\tregress(u)\tresharp");
    for (id, p, s, want) in CASES {
        let (a, b, c) = (show(fancy(p, s)), show(regress(p, s)), show(resharp(p, s)));
        f += (a == *want) as u32; g += (b == *want) as u32; h += (c == *want) as u32;
        println!("{id}\t{want}\t{a}\t{b}\t{c}");
    }
    println!("TOTAL\t{}\t{f}\t{g}\t{h}", CASES.len());
}
