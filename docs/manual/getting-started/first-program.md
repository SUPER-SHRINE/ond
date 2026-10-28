# 最初のプログラム

**到達目標:** 最小projectをcompileし、Kagura v1向け実行fileとdebug sidecarを生成します。

生成fileはhost OS上で直接起動するprogramではありません。

最初に[Ondの思想](overview.md)を読むと、CPU・機器・Ondの役割が分かります。

この章の到達点は実行fileの生成までです。

## projectを作る

空のdirectoryに次の2 fileを作ります。

```toml title="ond.toml"
[project]
name = "first"

[link]
startup = "external"
stack-size = 8192
output = "target/first.bin"

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

両commandがexit code `0`で終了し、`target/first.bin`と`target/first.onddebug`が生成されます。

`.bin`はKagura実行fileで、host OSから直接起動する形式ではありません。

このmanifestは`startup = "external"`を指定しています。

設定値の意味と、外部loaderが行う起動準備は[ビルドと起動の手順](build-and-launch.md)で説明します。

実行には外部loaderがload契約を満たしてentryへ制御を渡す必要があります。

entryへのload状態と実行環境の準備は[実行file仕様](../../../specs/ond/artifacts.md#ロード完了時の状態)を参照してください。

このmanualは共通のloader/VM実行手順を定めません。

CLIの入力と出力は[CLI仕様](../../../specs/ond/build/commands.md)、配置設定は[`ond.toml`リファレンス](../reference/manifest.md)を参照してください。



