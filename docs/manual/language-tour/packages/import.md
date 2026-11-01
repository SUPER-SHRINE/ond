# import

`import`で別packageを参照します。\
import pathはproject名とsubpackage directoryから構成します。

```text
app/
├─ ond.toml
├─ main.ond
└─ math/
   └─ math.ond
```

```ond title="main.ond"
package main

import "app/math"

func main() {
    var result: u32 = math.Double(4)
    _ = result
}
```

```ond title="math/math.ond"
package math

func Double(value: u32) -> u32 {
    return value * 2
}
```

## package pathを指定する

この例ではproject名`app`の`math`directoryをimportします。\
subpackage名`math`を通じてexported functionを参照します。

## path依存のlibraryをimportする

library projectのroot packageはproject名`mathlib`を宣言します。\
実行projectのmanifestでは登録名`calc`でpath依存を設定し、sourceではproject名`mathlib`をimportします。

```text
projects/
├─ app/
│  ├─ ond.toml
│  └─ main.ond
└─ library/
   ├─ ond.toml
   └─ math.ond
```

```toml title="app/ond.toml"
[project]
name = "app"

[dependencies]
calc = { path = "../library" }

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

```ond title="app/main.ond"
package main

import "mathlib"

func main() {
    _ = mathlib.Add(20, 22)
}
```

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

**操作:** `app` directoryから`ond compile .`、続けて`ond link --project .`を実行します。

**結果:** compileとlinkはexit `0`で終了し、`app/.ond/target/app.bin`と`app/.ond/target/app.onddebug`が生成されます。\
manifestの登録名`calc`はimport pathに使わず、libraryの`[project].name`に一致する`mathlib`をimportします。

library root package、path dependencyとimport pathは[package仕様](../../../../specs/ond/build/packages.md#manifestとsource探索)および[名前解決仕様](../../../../specs/ond/language/v1/names-and-scopes.md#packageとimport)を参照してください。

## aliasを付ける

必要なら`import arithmetic "app/math"`のようにaliasを指定し、`arithmetic.Double`と参照できます。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)を参照してください。
