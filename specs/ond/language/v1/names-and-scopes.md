# Package、import、scope
<!-- ond-spec-id: language/v1-names-and-scopes -->

本章はpackage構成、import path、識別子解決、scope、shadowingを定義する。

## 構文
<!-- ond-spec-id: language/v1-names-and-scopes-grammar -->

次のEBNFは本章の構文を定義する。\
他章のproductionは[文法索引](./lexical.md#文法の定義元)を参照する。

```ebnf
SourceFile    = PackageClause ";" { ImportDecl ";" } { TopLevelDecl ";" } .
PackageClause = "package" identifier .
ImportDecl    = "import" [ identifier ] string_lit .
QualifiedIdent= identifier "." identifier .
```

## Sourceとproject
<!-- ond-spec-id: language/source-and-packages -->

source fileとpackageは次節の構文・名前規則に従う。\
project manifestとsource探索は[package build](../../build/packages.md)、通常/test sourceの可視性は[test build](../../build/test-build.md)を定義元とする。

## Packageとimport
<!-- ond-spec-id: syntax/packages-and-imports -->

各 file は先頭に 1 個の package clause を持つ。

- project root、manifestの名前、source探索は[Manifestとsource探索](../../build/packages.md#manifestとsource探索)に従う
- rootで`package main`を宣言したprojectは実行projectとする。実行projectのrootは`main` packageであり、`main.ond`と`func main`を含む
root直下の全`.ond` fileは`package main`を宣言する。\
実行projectのroot packageの論理package pathは、そのprojectの`[project].name`を`p`とすると`p`である。\
この`p`をrootのpackage成果物・公開interface・debug metadata・具体化identity・test metadataの所属に用いる。\
通常の非generic関数とglobalのlink symbolのnamespaceは[Link symbol規則](../../artifacts.md#link-symbolの名前と所属)に従う。\
library projectはrootの`main.ond`と`func main`がなくても編集・依存として利用できる。\
library rootにsourceがある場合、そのpackage名はproject名と一致する。
- 1 directory を 1 package とし、directory 内の全 `.ond` file は同じ package 名を宣言しなければならない
- project root 以外の package 名は directory 名と一致しなければならない
- project内のsubpackageのimport pathは`project名/相対directory`を`/`区切りで表す
library root packageのimport pathはproject名そのものとする。\
import pathの先頭にproject名がない入力はcompile errorとなる\
実行projectのroot packageをimport対象とする入力はcompile errorとなる
- `import alias "path"`はidentifierのimport aliasを宣言する
  - 構文は本章のImportDeclに従う
- alias を省略した import は、その package の宣言名で参照する
- package間のimportが循環する入力はcompile errorとなる

`test.ond`と`_test.ond`のsource収集と通常/test文脈の分離は、[テストビルド仕様](../../build/test-build.md)で扱う。

package-qualified name は `packageName.ExportedName` とする。

- `packageName` は import した package 名でなければならない
- `ExportedName` は先頭が ASCII 大文字の top-level identifier でなければならない
- package-qualified name は package value ではなく、import 先 package の top-level declaration を指す構文である

## Scopeとshadowing
<!-- ond-spec-id: syntax/scope-and-shadowing -->

- package-level declaration は package scope に属し、同一 package 内の全 file から見える
- import 名の scope は宣言した file に限る
  - 同じ package の別 file でも、使用する package は各 file で import する
- 同一 import path は file ごとに異なる alias で宣言できる
- 異なる import path が同じ参照名になる場合は compile error とし、少なくとも一方へ明示 alias を指定して区別する
- import 名と package-level declaration が衝突した場合は compile error とする
- 関数 parameter、method receiver、関数本体直下の local declaration は、その関数本体の block scope に属する
- 内側の block で宣言した名前は、外側の block、import 名、package-level declaration を shadow できる
- 同一 block 内で同名の `var`、`const`、parameter を重複宣言してはならない
- 同一 package scope 内で同名の top-level declaration を重複宣言してはならない
- method名はreceiverのbase named typeごとのnamespaceに属する
  - 同じbase typeではvalue/pointer receiverを通して一意名とし、top-level関数名のnamespaceから独立する
- `_` は blank identifier とし、宣言または代入の値を破棄する位置で使える
  - 値として参照できず、binding を作らない
- `:=` は現在の block に未宣言の `_` 以外の名前を 1 個以上含まなければならない
  - 既存名の再利用は、その block 内で既に見えている変数に限る
- local `var` と `const` の scope は宣言完了直後から始まる
  - package-level declaration の scope は package 全体とする
- `if` の init で宣言した名前は condition、両 branch から見える
  - `for` の init で宣言した名前は condition、post、loop body から見える
- 組み込み型名`bool`, `u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `f32`をbinding名に使う入力はcompile errorとなる
  - `type`・`var`・`const`・`:=`・関数宣言・関数宣言のparameterでbindingを導入する名前として使用するとcompile errorとする
  - package/localの両scopeへ同じ検査を適用する
- struct field名と関数型の説明用parameter名はmemberまたは説明として扱い、組み込み型名の綴りも成立する
  - 型参照・field selector等では組み込み型名の綴りを通常どおり読み取れる
- 組み込み名`alloc`, `free`, `len`, `trap`, `sizeof`, `alignof`, `thisFile`, `thisLine`, `new`をbinding名に使う入力はcompile errorとなる
  - struct field 名など binding を作らない位置では使用できる
  - `str` は通常の識別子である
