# 識別子

識別子は変数や関数などを指す名前です。\
\
宣言した名前を同じpackage内で使います。

```ond title="main.ond"
package main

var totalCount: u32 = 1

func main() {
    localCount := totalCount + 2
    _ = localCount
}
```

## 名前を宣言する

`totalCount`はpackage変数、`localCount`は関数内の変数です。\
\
名前の綴りは大文字小文字を区別します。

## 識別子として使える文字

識別子はUnicode文字または`_`で始まり、その後には数字も使えます。\
\
予約語や一部の組み込みbinding名は、通常の宣言名として使えません。\
\
どの名前を宣言できるかは[scope規則](../../../../specs/ond/language/v1/names-and-scopes.md)を参照してください。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../getting-started/first-program.md)を参照してください。

識別子の字句条件は[字句仕様](../../../../specs/ond/language/v1/lexical.md#source形式)に定義されています。
