# libraryと依存

**到達目標:** path依存として登録したlibraryの公開packageを、実行projectからimportします。

`library` projectを用意します。

```toml title="library/ond.toml"
[project]
name = "mathlib"
```

```ond title="library/math.ond"
package mathlib

func Add(left: u32, right: u32) -> u32 {
    return left + right
}
```

実行projectのmanifestはpath依存を登録します。

```toml title="app/ond.toml"
[project]
name = "app"

[dependencies]
calc = { path = "../library" }

[link]
startup = "external"
stack-size = 8192
output = "target/app.bin"

[link.ram]
base = 4096
size = 65536

[link.exit]
mode = "loop"
```

```ond title="app/main.ond"
package main

import "mathlib"

func main() {
    _ = mathlib.Add(20, 22)
}
```

## この例を読む

manifestの登録名`calc`は依存を設定するproject内の名前です。\
sourceではlibrary project名`mathlib`をimportし、そのroot packageの公開関数`Add`を呼びます。

`app`を実行projectとしてcompileすると、pathで指定した`library`も依存として解決されます。\
library単独を実行projectとしてbuildする例ではありません。

**操作:** `app` directoryから`ond compile .`、次に`ond link --project .`を実行します。\
compile対象は実行projectです。

**結果:** exit `0`で`app/target/app.bin`と`app/target/app.onddebug`が生成されます。\
libraryはpath依存としてappのcompileに含まれます。

path dependencyとimportの解決は[package仕様](../../../specs/ond/build/packages.md)を参照してください。
