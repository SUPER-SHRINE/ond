# package

各source fileは先頭に`package`宣言を一つ持ちます。\
同じdirectory内のsourceは同じpackage名を使います。

```ond title="main.ond"
package main

func main() {
    helper()
}

func helper() {
}
```

## packageを宣言する

`package main`でこのfileを`main` packageに所属させます。\
project rootの実行packageは`main`です。

## fileとdirectory

同じdirectoryの複数fileは一つのpackageを構成します。\
subpackageはdirectory名に対応するpackage名を宣言します。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、package構成は[package仕様](../../../../specs/ond/language/v1/names-and-scopes.md#packageとimport)を参照してください。
