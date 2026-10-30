# 診断を読み、修正する

**到達目標:** compile診断のsource位置を使って型errorを修正し、修正版をlinkします。\
また、trap理由がdebug sidecarに記録されることを確認します。

## 修正前

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

`broken` directoryで`ond compile .`を実行すると、exit code `1`と`main.ond`内の該当位置を含む診断がstderrに出ます。\
`u32`の変数へ`bool`値を代入している箇所を確認します。\
診断の`main.ond`と行・列を開き、型宣言と右辺を見比べます。

## 修正後

```toml title="fixed/ond.toml"
[project]
name = "fixed"
[link]
startup = "external"
stack-size = 8192
output = "target/fixed.bin"
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
    if count == 0 {
        trap "count must be nonzero"
    }
    _ = count
}
```

`fixed` directoryで`ond compile .`と`ond link --project .`を実行します。\
どちらもexit `0`となり、`target/fixed.bin`と`target/fixed.onddebug`が生成されます。\
debug sidecarにはtrapの理由`count must be nonzero`が記録されます。\
この章では生成fileを実行しません。

診断位置とexit codeは[CLI仕様](../../../specs/ond/build/commands.md)、trap理由とdebug metadataは[builtin仕様](../../../specs/ond/language/v1/builtins.md#8-trap)および[debug metadata仕様](../../../specs/ond/debug-metadata/v1.md)を参照してください。
