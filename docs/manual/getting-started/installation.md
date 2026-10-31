# インストール

## Windowsインストーラーで導入する

[GitHub Releases](https://github.com/SUPER-SHRINE/ond/releases)から利用するversionの`ond-<version>-windows-x86_64-setup.exe`を取得します。\
たとえば0.4.0のinstaller名は`ond-0.4.0-windows-x86_64-setup.exe`です。

install後は新しいPowerShellまたはterminalを開き、`ond --help`でusageが表示されることを確認してください。

### アップデート
新版installerを実行すると既存installを更新できます。\
前回の配置先とPATH設定が引き継がれますが、更新時に変更できます。\
更新前にOnd、Ond Language Server、およびそれらを使用するeditorを閉じてください。

### 削除
Ondを削除するには、Windowsの設定からOndをアンインストールします。\
installerが追加したPATH項目も削除されます。

## Linux向けバイナリで導入する

Linux x86_64（glibc）向けの配布archiveは[GitHub Releases](https://github.com/SUPER-SHRINE/ond/releases)から取得できます。\
利用するversionの`ond-<version>-linux-x86_64.tar.gz`を選びます。\
たとえば0.4.0のarchive名は`ond-0.4.0-linux-x86_64.tar.gz`です。

archive内の`ond`と`ond-lsp`を取り出し、PATHの通った`/usr/local/bin`へ配置します。\
以下の例ではversionに合わせてarchive名と作業directoryを読み替えてください。

```sh
mkdir -p ond-0.4.0
tar -xzf ond-0.4.0-linux-x86_64.tar.gz -C ond-0.4.0
sudo install -m 755 ond-0.4.0/ond ond-0.4.0/ond-lsp /usr/local/bin/
ond --help
```

`ond --help`でusageが表示されれば準備完了です。

## Sourceからbuild

### 用意するもの

repositoryが指定するRust toolchainを使います。\
[`rust-toolchain.toml`](../../../rust-toolchain.toml)にはchannel `1.91.1`と`rustfmt`、`clippy` componentが記載されています。\
Rustupでそのtoolchainを利用できる状態にしてください。

### CLIをbuildする

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

### 確認結果

`ond --help`がusageを表示して終了すれば準備完了です。\
build結果はWindowsで`target/debug/ond.exe`、Unix系で`target/debug/ond`です。

CLIのsubcommandと引数は[CLI仕様](../../../specs/ond/build/commands.md)を参照してください。
