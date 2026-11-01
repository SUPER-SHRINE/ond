# package

各source fileは先頭に`package`宣言を一つ持ちます。\
同じdirectory内のsourceは同じpackage名を使います。

```text
app/
├─ ond.toml
├─ main.ond
├─ helper.ond
└─ math/
   └─ add.ond
```

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
    _ = helper()
    _ = math.Add(20, 22)
}
```

```ond title="helper.ond"
package main

func helper() -> u32 {
    return 1
}
```

```ond title="math/add.ond"
package math

func Add(left: u32, right: u32) -> u32 {
    return left + right
}
```

## packageを宣言する

`package main`でroot directoryのfileを`main` packageに所属させます。\
同じdirectoryの`main.ond`と`helper.ond`は一つのpackageを構成します。

`math/add.ond`はsubpackage用のdirectoryに置き、`package math`を宣言します。\
別packageから呼び出す`Add`は大文字で始まる公開名です。

projectのsource構成とimport pathは[package仕様](../../../../specs/ond/build/packages.md)および[名前解決仕様](../../../../specs/ond/language/v1/names-and-scopes.md#packageとimport)に定義されています。

**操作:** project rootで`ond compile .`、続けて`ond link --project .`を実行します。

**結果:** compileとlinkはexit `0`で終了し、`.ond/target/app.bin`と`.ond/target/app.onddebug`が生成されます。\
`helper()`は同じpackageの別fileから呼び出し、`math.Add(20, 22)`はimportしたsubpackageの公開関数を呼び出します。
