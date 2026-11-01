# Language Serverの利用

Ond Language ServerはLanguage Server Protocolを使うeditor向けの補完・診断serverです。\
補完、hover、定義への移動、参照検索、rename、document symbols、semantic tokensを利用できます。\
server起動には`ond-lsp stdio`を指定します。

## 直接起動するeditor

OndのWindows installerまたはLinux archiveには`ond-lsp`が含まれます。\
editorから利用する場合は、実行fileをPATHに置き、Language Serverのcommandを次のように設定してください。

```text
ond-lsp stdio
```

editorはこのprocessの標準入力と標準出力をprotocol通信に使います。\
projectの`ond.toml`があるdirectoryをworkspace rootとして開きます。

対応commandと終了codeは[Language Server command仕様](../../../specs/ond/tools/language-server.md)を参照してください。

## VS Code

Ond VS Code拡張はLanguage Serverを拡張機能内に同梱します。\
拡張機能の導入方法と設定は[拡張機能の案内](https://github.com/SUPER-SHRINE/ond-vscode-extension/blob/main/README.md)を参照してください。\
CLIを使ってcompile、linkする場合は、[Ondのインストール案内](../getting-started/installation.md)からcompilerを別途導入します。

Language Serverの解析、role-aware補完と診断は[Language v1の仕様](../../../specs/ond/language/v1/roles.md#lspの補完と診断)、project/cacheとの関係は[role仕様](../../../specs/ond/build/roles.md)と[cache仕様](../../../specs/ond/build/cache.md)を参照してください。
