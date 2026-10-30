# projectとpackage

**到達目標:** 1つの実行project内にsubpackageを置き、公開関数をimportします。

```toml title="ond.toml"
[project]
name = "app"
[link]
startup = "external"
stack-size = 8192
output = ".ond/target/app.bin"
[link.ram]
base = 4096
size = 65536
[link.exit]
mode = "loop"
```

```ond title="main.ond"
package main

import "app/math"

func main() {
    _ = math.Add(20, 22)
}
```

```ond title="math/add.ond"
package math

func Add(left: u32, right: u32) -> u32 {
    return left + right
}
```

## この例を読む

`ond.toml`があるroot directoryがprojectです。\
rootの`main.ond`は`package main`に属し、`math/add.ond`は`package math`に属します。

`app/math`はproject名とsubpackage pathを組み合わせたimport pathです。\
`Add`は別packageから使うため大文字で始まる公開名です。

**操作:** rootで`ond compile .`、続けて`ond link --project .`を実行します。

**結果:** 両方exit `0`で`.ond/target/app.bin`と`.ond/target/app.onddebug`が生成されます。\
`main.ond`では`app/math`をimportし、`math.Add(20, 22)`で公開関数を呼びます。

project、source探索、import pathとpackage公開名は[package仕様](../../../specs/ond/build/packages.md)と[名前解決仕様](../../../specs/ond/language/v1/names-and-scopes.md#packageとimport)を参照してください。
