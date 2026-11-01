# 変数

`var`で再代入できる変数を宣言します。\
型と初期値を指定すると、宣言した時点で値が決まります。

```ond title="main.ond"
package main

func main() {
    var count: u32
    inferred := 3
    var measured = 4
    _ = count
    _ = inferred
    _ = measured
}
```

## 型を指定する

`var count: u32`のように初期値を省略すると、`count`は型のzero valueで初期化されます。

## 初期値から型を推論する

`var measured = 4`は初期値から型を推論し、`measured`の型は`i32`になります。\
`inferred := 3`のような短縮宣言については[短縮変数宣言](short-declarations.md)を参照してください。

## 型errorの位置を確認する

次の例は`u32`の変数に`bool`値を代入するためcompile errorになります。

```toml title="broken/ond.toml"
[project]
name = "broken"
```

```ond title="broken/main.ond"
package main

func main() {
    var count: u32 = true
    _ = count
}
```

`broken` directoryで`ond compile .`を実行すると、exit code `1`と、`main.ond`の行・列を含む診断がstderrに出ます。\
診断位置が示す宣言の型と右辺を見比べます。

次の例では初期値を`u32`に合う整数へ直しています。

```toml title="fixed/ond.toml"
[project]
name = "fixed"

[link]
startup = "external"
stack-size = 8192
output = ".ond/target/fixed.bin"

[link.ram]
base = 4096
size = 65536

[link.exit]
mode = "loop"
```

```ond title="fixed/main.ond"
package main

func main() {
    var count: u32 = 1
    _ = count
}
```

`fixed` directoryで`ond compile .`と`ond link --project .`を実行すると、どちらもexit `0`となり、`.ond/target/fixed.bin`と`.ond/target/fixed.onddebug`が生成されます。

診断位置と成功結果は[宣言仕様](../../../../specs/ond/language/v1/declarations.md#診断と成功結果)、compile/linkのexit codeは[CLI仕様](../../../../specs/ond/build/commands.md)を参照してください。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)を参照してください。
