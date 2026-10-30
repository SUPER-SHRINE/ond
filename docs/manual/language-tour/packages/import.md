# import

`import`で別packageをfile単位で参照します。\
import pathはproject名とsubpackage directoryから構成します。

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

## aliasを付ける

必要なら`import arithmetic "app/math"`のようにaliasを指定し、`arithmetic.Double`と参照できます。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、import pathは[package仕様](../../../../specs/ond/language/v1/names-and-scopes.md#packageとimport)を参照してください。
