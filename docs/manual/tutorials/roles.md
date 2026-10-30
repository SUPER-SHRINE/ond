# roleで構成を選ぶ

**到達目標:** role未指定の製品構成と、manifestで宣言した`Debug`、`Development`構成をcompileします。

```toml title="ond.toml"
[project]
name = "roles"

[roles]
names = ["Debug", "Development"]

[link]
startup = "external"
stack-size = 8192
output = ".ond/target/roles.bin"

[link.ram]
base = 4096
size = 65536

[link.exit]
mode = "loop"
```

```ond title="main.ond"
package main

func main() {
    var mode: u32 = 0
    role Debug {
        mode = 1
    }
    role Development {
        mode = 2
    }
    _ = mode
}
```

**操作:** project rootで次の各組を実行します。\
構成ごとにcompileし、続けてlinkします。

```sh
ond compile .
ond link --project .
ond compile . --role Debug
ond link --project .
ond compile . --role Development
ond link --project .
```

## この例を読む

role未指定でcompileする構成をこの例の製品構成として使います。\
`mode`は共通部分で0に初期化され、role未指定では追加roleの文が選ばれないため値は0のままです。\
`Debug`を指定したcompileでは1、`Development`では2を代入する文が選ばれます。

3構成は同じsourceから別々にcompileします。\
各compile後にlinkすると設定済みの出力先を更新しますが、ここでは生成fileの実行結果を観察しません。

**結果:** それぞれcompileとlinkがexit `0`で終了し、manifestで設定した`.ond/target/roles.bin`と`.ond/target/roles.onddebug`を生成します。

role構文の選択規則は[言語role仕様](../../../specs/ond/language/v1/roles.md)、manifestと`--role`は[project role仕様](../../../specs/ond/build/roles.md)を参照してください。
