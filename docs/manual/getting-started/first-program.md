# 最初のプログラム

**到達目標:** 最小projectをcompileし、実行fileとdebug sidecarを生成します。

## projectを作る

空のdirectoryに次の2 fileを作ります。

```toml title="ond.toml"
[project]
name = "first"

[link]
startup = "external"
stack-size = 8192
output = ".ond/target/first.bin"

[link.ram]
base = 4096
size = 65536

[link.exit]
mode = "loop"
```

```ond title="main.ond"
package main

func main() {
    var answer: u32 = add(20, 22)
    _ = answer
}

func add(left: u32, right: u32) -> u32 {
    return left + right
}
```

manifestの各fieldについては[`ond.toml`リファレンス](../reference/manifest.md)を参照してください。

## compileとlink

project directoryで次を実行します。

```sh
ond compile .
ond link --project .
```

両commandがexit code `0`で終了し、`.ond/target/first.bin`と`.ond/target/first.onddebug`が生成されます。

`.bin`はKagura規格を満たすVM上で実行するためのfileで、host OSから直接起動するファイルではありません。

entryへのload状態と実行環境の準備は[実行file仕様](../../../specs/ond/artifacts.md#ロード完了時の状態)を参照してください。

CLIの入力と出力は[CLI仕様](../../../specs/ond/build/commands.md)、配置設定は[`ond.toml`リファレンス](../reference/manifest.md)を参照してください。
