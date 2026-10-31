# global initialization

package-level `var`はzero value、またはcompile-time constant initializerで初期化します。

```ond title="main.ond"
package main

var initial: u32 = 5
var zeroValue: u32

func main() {
    _ = initial + zeroValue
}
```

## constant initializer

`initial`はconstant initializerの5で初期化されます。\
runtime function callをinitializerに使うことはできません。

## zero value

`zeroValue`はinitializerを省略しているため、型`u32`のzero valueで初期化されます。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、初期化順は[実行仕様](../../../../specs/ond/language/v1/execution.md#初期化とエントリ)を参照してください。
