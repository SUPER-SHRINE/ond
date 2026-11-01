# role

`role`はmanifestで宣言した構成ごとに、選択するsource文を切り替えます。

```text
roles/
├─ ond.toml
└─ main.ond
```

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

## roleを選んでcompileする

project rootで次の各組を実行し、同じsourceを構成ごとにcompileしてlinkします。

```sh
ond compile .
ond link --project .
ond compile . --role Debug
ond link --project .
ond compile . --role Development
ond link --project .
```

role未指定では共通部分が選ばれ、`mode`は0です。\
`--role Debug`では`mode = 1`、`--role Development`では`mode = 2`の文が選ばれます。

**結果:** 3構成それぞれのcompileとlinkはexit `0`で終了し、`.ond/target/roles.bin`と`.ond/target/roles.onddebug`が生成されます。\
例では生成fileを実行しないため、ここで示す値は選択された代入文から分かる値です。

role構文の選択規則は[言語role仕様](../../../../specs/ond/language/v1/roles.md)、manifestと`--role`は[project role仕様](../../../../specs/ond/build/roles.md)を参照してください。
