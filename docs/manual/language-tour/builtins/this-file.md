# thisFile

`thisFile()`は現在の論理source pathを指す`*u8`を返します。

```ond title="main.ond"
package main

func main() {
    var sourcePath: *u8 = thisFile()
    _ = sourcePath
}
```

## source pathを得る

戻り値は`*u8`です。\
文字列型や長さ付き配列ではありません。

## 論理path

返されるpathは現在の論理source pathです。\
host filesystem上の実行時pathとして扱う契約ではありません。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、builtinの契約は[組み込み仕様](../../../../specs/ond/language/v1/builtins.md#組み込み関数)を参照してください。
