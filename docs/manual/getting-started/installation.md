# インストール

この章では、Ond 0.4.0向けCLIをrepositoryのsourceからbuildし、commandを確認します。

## 用意するもの

repositoryが指定するRust toolchainを使います。\
[`rust-toolchain.toml`](../../../rust-toolchain.toml)にはchannel `1.91.1`と`rustfmt`、`clippy` componentが記載されています。\
Rustupでそのtoolchainを利用できる状態にしてください。

## CLIをbuildする

repository rootで次を実行します。

```console
cargo build -p ond-cli --bin ond --locked
```

Windows PowerShellではbuild結果のdirectoryをPATHへ追加します。

```powershell
$env:PATH = "$(Get-Location)\target\debug;$env:PATH"
ond --help
```

Unix系shellでは次のようにします。

```sh
export PATH="$PWD/target/debug:$PATH"
ond --help
```

## 確認結果

`ond --help`がusageを表示してexit code `0`で終了すれば準備完了です。\
build結果はWindowsで`target/debug/ond.exe`、Unix系で`target/debug/ond`です。

CLIのsubcommandと引数は[CLI仕様](../../../specs/ond/build/commands.md)を参照してください。
