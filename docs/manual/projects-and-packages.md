# プロジェクトとパッケージ

Ondでは`ond.toml`があるdirectoryがproject rootになる。1 directoryが1 packageに対応し、同じdirectoryの`.ond`ファイルは同じpackage名を宣言する。

## project内のpackage

次のprojectでは、rootが`main` package、`math` directoryが`math` packageになる。

```text
game/
├── ond.toml
├── main.ond
└── math/
    └── arithmetic.ond
```

`math/arithmetic.ond`:

```ond
package math

func Add(left: i32, right: i32) -> i32 {
    return left + right
}
```

`main.ond`:

```ond
package main

import "math"

func main() {
    var result = math.Add(20, 22)
    _ = result
}
```

import pathは`/`区切りを使う。aliasを明示することもできる。

```ond
import arithmetic "math"
```

importのscopeはファイル単位である。同じpackageの別ファイルでも、そこで使うpackageはそのファイル自身がimportする。

## 別projectへの依存

再利用するprojectは`ond.toml`で名前を宣言する。

```toml
[project]
name = "shared"
```

利用側の`ond.toml`からpath依存を登録する。

```toml
[dependencies]
common = { path = "../shared" }
```

`common`はmanifest内の登録名であり、sourceのimport pathには使わない。依存先が宣言したproject名を先頭にしてimportする。

```ond
import "shared"
import "shared/collections"
```

相対pathは利用側`ond.toml`のdirectoryを基準に解決される。依存先には`ond.toml`が必要である。

Git依存も宣言できる。

```toml
[dependencies]
common = { git = "https://github.com/example/shared.git", rev = "0123456789abcdef" }
```

Git依存は`.ond/dependencies`へ取得され、解決したcommitは`ond.lock`へ固定される。lock更新の運用や厳密な解決規則は[パッケージビルド契約](../../specs/ond/package-build.md)を参照する。

## 追加library root

project依存ではなく、SDKなどが生成したpackageを一時的に追加する場合は`--library-dir`を使う。

```console
ond compile ./game --library-dir ./generated
```

複数のrootを指定できる。`OND_PATH`環境変数でもOS標準のpath-list区切りを使って指定できる。project、依存project、追加libraryの同名packageを暗黙に混合したり上書きしたりはしない。

## source探索から除外する

既定では`.git`、`.ond`、`target`がsource探索から除外される。生成物などを追加で除外する場合は`ond.toml`へ記載する。

```toml
[resolve]
ignore = [".git", ".ond", "target", "generated"]
```

値はmanifest directoryからの相対directoryであり、絶対path、`..`、globは使えない。

## 公開API

packageを跨いで参照できるのは、ASCII大文字で始まるtop-level宣言、field、methodである。公開interfaceのmethodも大文字で始める。

```ond
package geometry

type Point struct {
    X: i32
    Y: i32
    cached: bool
}

func NewPoint(x: i32, y: i32) -> Point {
    return Point{X: x, Y: y, cached: false}
}
```

利用側は`geometry.Point`、`geometry.NewPoint`、`point.X`を参照できるが、`point.cached`は参照できない。

## 初期化順序

package-level `var`のinitializerはコンパイル時定数だけに限られる。自動実行される`init` hookはなく、必要なruntime初期化は`main`などから明示的に呼び出す。

```ond
package device

var Enabled: bool = false

func Initialize() {
    Enabled = true
}
```

循環importはコンパイルエラーになる。依存方向を一方向に保ち、共有する型や処理は下位のpackageへ分離する。
