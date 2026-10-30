"""Pygments highlighting for Ond code fences in the documentation site."""

import re
import xml.etree.ElementTree as ET

import markdown
from pygments.lexer import RegexLexer
from pygments.token import Comment, Keyword, Name, Number, Operator, Punctuation, String, Text
from pymdownx.highlight import Highlight, HighlightExtension


class OndLexer(RegexLexer):
    name = "Ond"
    aliases = ["ond"]
    filenames = ["*.ond"]
    flags = re.MULTILINE | re.UNICODE

    tokens = {
        "root": [
            (r"\s+", Text),
            (r"//[^\n]*", Comment.Single),
            (r"/\*[\s\S]*?\*/", Comment.Multiline),
            (r'"(?:\\.|[^"\\\n])*"', String.Double),
            (r'"(?:\\.|[^"\\\n])*', String.Double),
            (r"`[^`]*`", String.Backtick),
            (r"`[^`]*", String.Backtick),
            (r"0[xX](?:_?[0-9A-Fa-f])+(?:\.(?:[0-9A-Fa-f](?:_?[0-9A-Fa-f])*)?)?[pP][+-]?[0-9](?:_?[0-9])*", Number.Float),
            (r"0[xX]\.(?:[0-9A-Fa-f](?:_?[0-9A-Fa-f])*)[pP][+-]?[0-9](?:_?[0-9])*", Number.Float),
            (r"[0-9](?:_?[0-9])*\.(?:[0-9](?:_?[0-9])*)?(?:[eE][+-]?[0-9](?:_?[0-9])*)?", Number.Float),
            (r"\.[0-9](?:_?[0-9])*(?:[eE][+-]?[0-9](?:_?[0-9])*)?", Number.Float),
            (r"[0-9](?:_?[0-9])*[eE][+-]?[0-9](?:_?[0-9])*", Number.Float),
            (r"0[bB]_?[01](?:_?[01])*", Number.Bin),
            (r"0[xX]_?[0-9A-Fa-f](?:_?[0-9A-Fa-f])*", Number.Hex),
            (r"0[oO]_?[0-7](?:_?[0-7])*|0(?:_?[0-7])*", Number.Oct),
            (r"[1-9][0-9]*(?:_?[0-9])*|0", Number.Integer),
            (r"(?:package|import)\b", Keyword.Namespace),
            (r"(?:const|func|struct|type|var)\b", Keyword.Declaration),
            (r"(?:break|continue|defer|else|for|if|return|trap)\b", Keyword),
            (r"(?:as|role|operator|asm|noreturn)\b", Keyword),
            (r"(?:bool|u8|i8|u16|i16|u32|i32|f32)\b", Keyword.Type),
            (r"(?:true|false|nil)\b", Name.Constant),
            (r"(?:alignof|alloc|free|len|new|sizeof|thisFile|thisLine)\b", Name.Builtin),
            (r"[^\W\d][\w]*", Name),
            (r"<->|<<=|>>=|&\^=|\[\]=|\[\]|:=|\+=|-=|\*=|/=|%=|&=|\|=|\^=|\+\+|--|==|!=|<=|>=|<<|>>|&&|\|\||&\^|->|<-|\.\.\.|[+*/%&|^=!<>~-]", Operator),
            (r"[()\[\]{}.,:;]", Punctuation),
            (r".", Text),
        ]
    }


class OndHighlight(Highlight):
    def get_lexer(self, src, language, inline, stripnl):
        if language and language.lower() == "ond":
            return OndLexer(stripnl=stripnl), "ond"
        return super().get_lexer(src, language, inline, stripnl)


class OndHighlightExtension(HighlightExtension):
    def get_pymdownx_highlighter(self):
        return OndHighlight


def makeExtension(**kwargs):
    return OndHighlightExtension(**kwargs)


if __name__ == "__main__":
    from pygments.token import Comment, Keyword, Number, String

    lexed = list(OndLexer().get_tokens(
        'package café\nvar x:u32=0b_10; var y=0o_77; var z=0x_ff; '
        'var a=0_600; var f=1.; var g=.5; var h=1.e2; var j=1e-2; len(x); '
        'var k=1E+2; var m=0.15e+0_2; var n=08.5; var o=0x1p0; '
        'var p=0X_1.Fp+2; var q=0x.8p-0; var r=0x1.p0; var i=0x1_2.3_4P-1_0; '
        'var s="escaped \\" quote"; var raw=`line\nraw`; /* block\ncomment */ // line'
    ))
    assert any(token is Keyword.Namespace and value == "package" for token, value in lexed)
    assert any(token is Keyword.Type and value == "u32" for token, value in lexed)
    assert any(token is Name.Builtin and value == "len" for token, value in lexed)
    assert any(token is Name and value == "café" for token, value in lexed)
    assert all([(token, value) for token, value in lexed if value == literal] == [(expected, literal)]
               for literal, expected in (("0b_10", Number.Bin), ("0o_77", Number.Oct),
                                         ("0x_ff", Number.Hex), ("0_600", Number.Oct)))
    assert all([(token, value) for token, value in lexed if value == literal] == [(Number.Float, literal)]
               for literal in ("1.", ".5", "1.e2", "1e-2", "1E+2", "0.15e+0_2", "08.5",
                               "0x1p0", "0X_1.Fp+2", "0x.8p-0", "0x1.p0", "0x1_2.3_4P-1_0"))
    assert any(token in String and "escaped" in value for token, value in lexed)
    assert any(token in String.Backtick and "raw" in value for token, value in lexed)
    assert any(token in Comment.Multiline and "block" in value for token, value in lexed)
    assert any(token in Comment.Single and value.startswith("//") for token, value in lexed)

    code = 'package main\nfunc main() { var 値:u32=0x2a; var text="<script>"; _=値; _=text } // comment'
    source = f'''```ond title="main.ond" linenums="1" hl_lines="2"
{code}
```

```toml
name = "still toml"
```
'''
    rendered = markdown.markdown(
        source,
        extensions=[OndHighlightExtension(pygments_lang_class=True), "pymdownx.superfences"],
    )
    assert 'main.ond' in rendered and 'language-ond' in rendered
    assert 'class="kd"' in rendered and 'class="kt"' in rendered
    assert 'class="m' in rendered and 'class="c1"' in rendered
    assert "&lt;script&gt;" in rendered and "language-toml" in rendered
    assert "still toml" in rendered
    root = ET.fromstring(f"<root>{rendered}</root>")
    ond_block = root.find(".//table[@class='highlighttable']")
    assert ond_block is not None and 'main.ond' in rendered
    assert "linenos" in rendered and "hll" in rendered
    code_element = ond_block.find(".//td[@class='code']//code")
    assert code_element is not None and "".join(code_element.itertext()).strip() == code
    print("Ond syntax highlighting self-test passed")

