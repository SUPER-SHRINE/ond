# 文字列リテラル

文字列リテラルはdecode後のbyte列を表し、型は`[N]u8`です。\
\
独立した文字列型ではありません。

```ond title="main.ond"
package main

func main() {
    var message: [5]u8 = "hello"
    _ = message[0]
}
```

## 引用符とescape

文字列を二重引用符で囲みます。\
\
改行などを値に含めるには`\n`のようなescapeを使います。

## byte列として扱う

この例の`"hello"`は5 byteの配列です。\
\
長さは表示文字数ではなく、decodeしたbyte数で決まります。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../getting-started/first-program.md)を参照してください。

escapeと値の規則は[字句仕様](../../../../specs/ond/language/v1/lexical.md#source形式)と[式仕様](../../../../specs/ond/language/v1/expressions.md#文字列リテラル)に定義されています。
